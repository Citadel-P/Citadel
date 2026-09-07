use super::*;
use crate::platforms_http::{RuntimeRef, runtime_for_node};
use citadel_platforms::{ContainerView, StatisticsReadStore, SwarmTaskRuntimePort, terminal::*};
use futures_util::StreamExt;
use tokio_util::sync::CancellationToken;

static TERMINAL_SLOTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(32);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Owner {
    Container,
    Deployment,
    Stack,
    Task,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Operation {
    Start,
    Input,
    Resize,
}

impl ApplicationGroupReader {
    pub(super) async fn terminal_target(
        &self,
        p: &ActorPrincipal,
        g: &Group,
    ) -> Result<ContainerView, RealtimeReadError> {
        let container = if g.kind == "swarm-task-exec" {
            let platform = g.id.ok_or(RealtimeReadError::Authorization)?;
            self.permission(
                p,
                ResourceType::Platform,
                Some(platform),
                Some(SpecificPermission::Terminal),
            )
            .await?;
            let task = g
                .reference
                .as_deref()
                .ok_or(RealtimeReadError::Authorization)?;
            let projection = self
                .platforms
                .get_swarm_task(platform, task)
                .await
                .map_err(failure)?
                .ok_or(RealtimeReadError::Authorization)?;
            let cancel = CancellationToken::new();
            let runtime = runtime_for_node(&self.docker, platform, None)
                .await
                .map_err(failure)?;
            let live = match runtime {
                RuntimeRef::Local(r) => r.inspect_task(task, &cancel).await,
                RuntimeRef::Agent(r) => r.inspect_task(task, &cancel).await,
                RuntimeRef::Edge(r) => r.inspect_task(task, &cancel).await,
            }
            .map_err(failure)?;
            let docker_id =
                citadel_platforms::validate_running_task(&projection, &live).map_err(failure)?;
            let store = citadel_adapters::statistics_read_store::PostgresStatisticsReadStore::new(
                self.docker.pool.clone(),
            );
            let target = store
                .task_container(platform, &live.node_id, docker_id)
                .await
                .map_err(failure)?
                .ok_or(RealtimeReadError::Authorization)?;
            self.platforms
                .get_container(target.id)
                .await
                .map_err(failure)?
                .ok_or(RealtimeReadError::Authorization)?
        } else {
            let c = self
                .platforms
                .get_container_by_reference(g.reference.as_deref().unwrap_or_default())
                .await
                .map_err(failure)?
                .ok_or(RealtimeReadError::Authorization)?;
            let mut allowed = false;
            for (kind, id) in [
                (ResourceType::Platform, Some(c.platform_id)),
                (ResourceType::Deployment, c.deployment_id),
                (ResourceType::Stack, c.stack_id),
            ] {
                if let Some(id) = id
                    && self
                        .permission(p, kind, Some(id), Some(SpecificPermission::Terminal))
                        .await
                        .is_ok()
                {
                    allowed = true;
                    break;
                }
            }
            if !allowed {
                return Err(RealtimeReadError::Authorization);
            }
            c
        };
        if container.projection_stale_since.is_some()
            || !container.state.eq_ignore_ascii_case("running")
        {
            return Err(failure(
                "The terminal target is not running or its node data is stale.",
            ));
        }
        Ok(container)
    }

    pub(super) async fn open_terminal(
        &self,
        p: &ActorPrincipal,
        g: &Group,
        shell: TerminalShell,
        cancel: &CancellationToken,
    ) -> Result<GroupTerminal, RealtimeReadError> {
        let permit = TERMINAL_SLOTS
            .try_acquire()
            .map_err(|_| failure("Terminal capacity exceeded."))?;
        // Subscribe before resolving the target. A session must never silently move
        // to a replacement container or continue using a stale node projection.
        let mut changes = self.docker.realtime.as_ref().map(|hub| hub.subscribe());
        let target = self.terminal_target(p, g).await?;
        let runtime = runtime_for_node(
            &self.docker,
            target.platform_id,
            target.docker_node_id.as_deref(),
        )
        .await
        .map_err(failure)?;
        let TerminalSession { input, mut output } = match runtime {
            RuntimeRef::Local(r) => {
                r.container_terminal(&target.container_id, shell, cancel)
                    .await
            }
            RuntimeRef::Agent(r) => {
                r.container_terminal(&target.container_id, shell, cancel)
                    .await
            }
            RuntimeRef::Edge(r) => {
                r.container_terminal(&target.container_id, shell, cancel)
                    .await
            }
        }
        .map_err(failure)?;
        let reader = self.clone();
        let principal = p.clone();
        let group = g.clone();
        let cancel = cancel.clone();
        let output = Box::pin(async_stream::try_stream! {
            let _permit=permit;
            loop {
                let (change, item) = tokio::select! {
                    biased;
                    () = cancel.cancelled() => break,
                    change = async { match &mut changes { Some(receiver) => receiver.recv().await, None => std::future::pending().await } } => (Some(change), None),
                    item = output.next() => match item { Some(item) => (None, Some(item)), None => break },
                };
                if let Some(change) = change {
                    let change = change.map_err(failure)?;
                    if change.platform_id != Some(target.platform_id) || change.event_kind != "runtimeChanged" || change.payload["dockerResourceType"] == "containerStats" { continue; }
                    let current = reader.terminal_target(&principal, &group).await?;
                    if current.id != target.id || current.container_id != target.container_id || current.platform_id != target.platform_id || current.docker_node_id != target.docker_node_id {
                        Err(failure("The terminal target changed. Reconnect to the current container."))?;
                    }
                    continue;
                }
                let Some(item) = item else { continue };
                let bytes=match item.map_err(failure)? {
                    TerminalOutput::Data(bytes)=>bytes,
                    TerminalOutput::Exit(code)=>format!("\r\n[process exited: {code}]\r\n").into_bytes(),
                };
                yield ClientEvent::new("SendContainerExec",vec![json!(bytes)]);
            }
        });
        Ok(GroupTerminal { input, output })
    }

    pub(super) async fn resolve_terminal_invocation(
        &self,
        p: &ActorPrincipal,
        method: &str,
        args: &[Value],
    ) -> Result<Option<TerminalInvocation>, RealtimeReadError> {
        let (owner, mode) = match method {
            "StartExecProcess" => (Owner::Container, Operation::Start),
            "SendExecInput" => (Owner::Container, Operation::Input),
            "ResizeExec" => (Owner::Container, Operation::Resize),
            "StartDeploymentExecProcess" => (Owner::Deployment, Operation::Start),
            "SendDeploymentExecInput" => (Owner::Deployment, Operation::Input),
            "ResizeDeploymentExec" => (Owner::Deployment, Operation::Resize),
            "StartStackExecProcess" => (Owner::Stack, Operation::Start),
            "SendStackExecInput" => (Owner::Stack, Operation::Input),
            "ResizeStackExec" => (Owner::Stack, Operation::Resize),
            "StartSwarmTaskExecProcess" => (Owner::Task, Operation::Start),
            "SendSwarmTaskExecInput" => (Owner::Task, Operation::Input),
            "ResizeSwarmTaskExec" => (Owner::Task, Operation::Resize),
            _ => return Ok(None),
        };
        let prefix = if matches!(owner, Owner::Stack | Owner::Task) {
            2
        } else {
            1
        };
        if args.len() != prefix + if mode == Operation::Resize { 3 } else { 2 } {
            return Err(RealtimeReadError::Authorization);
        }
        let text = |index: usize| args[index].as_str().ok_or(RealtimeReadError::Authorization);
        let session = text(prefix)?;
        let mut reference = text(0)?.to_owned();
        let mut expected_stack = None;
        if owner == Owner::Deployment {
            let id = Uuid::parse_str(&reference).map_err(failure)?;
            self.permission(
                p,
                ResourceType::Deployment,
                Some(id),
                Some(SpecificPermission::Terminal),
            )
            .await?;
            let deployment = self
                .deployments
                .get_authorized(p.actor_id, p.is_administrator(), id)
                .await
                .map_err(failure)?;
            let containers = self
                .platforms
                .list_containers(deployment.platform_id)
                .await
                .map_err(failure)?;
            reference = containers
                .into_iter()
                .find(|c| c.deployment_id == Some(id) && c.state.eq_ignore_ascii_case("running"))
                .ok_or(RealtimeReadError::Authorization)?
                .container_id;
        } else if owner == Owner::Stack {
            let id = Uuid::parse_str(&reference).map_err(failure)?;
            self.permission(
                p,
                ResourceType::Stack,
                Some(id),
                Some(SpecificPermission::Terminal),
            )
            .await?;
            expected_stack = Some(id);
            reference = text(1)?.into();
        }
        if reference.len() == 64 && reference.bytes().all(|c| c.is_ascii_hexdigit()) {
            reference.truncate(12);
        }
        let name = if owner == Owner::Task {
            format!("swarm-task-exec:{}:{}:{session}", text(0)?, text(1)?)
        } else {
            format!(
                "container-exec:{}:{session}",
                reference.to_ascii_lowercase()
            )
        };
        let group = Group::parse(&name).ok_or(RealtimeReadError::Authorization)?;
        // Input is accepted only for this connection's already-open session. Do
        // not inspect Docker again for every keystroke; the stream fences target
        // changes and the connection periodically rechecks authorization.
        if mode == Operation::Start || expected_stack.is_some() {
            let target = self.terminal_target(p, &group).await?;
            if expected_stack.is_some() && target.stack_id != expected_stack {
                return Err(RealtimeReadError::Authorization);
            }
        }
        let value = &args[prefix + 1];
        let action = match mode {
            Operation::Start => TerminalAction::Start(match value.as_str() {
                Some("sh") => TerminalShell::Sh,
                Some("bash") => TerminalShell::Bash,
                _ => return Err(failure("Unsupported terminal shell.")),
            }),
            Operation::Input => TerminalAction::Input(TerminalInput::Stdin(input_bytes(value)?)),
            Operation::Resize => TerminalAction::Input(TerminalInput::Resize {
                cols: value
                    .as_u64()
                    .and_then(|v| u16::try_from(v).ok())
                    .ok_or(RealtimeReadError::Authorization)?,
                rows: args[prefix + 2]
                    .as_u64()
                    .and_then(|v| u16::try_from(v).ok())
                    .ok_or(RealtimeReadError::Authorization)?,
            }),
        };
        if let TerminalAction::Input(input) = &action {
            input.validate().map_err(failure)?;
        }
        Ok(Some(TerminalInvocation { group, action }))
    }
}

fn input_bytes(value: &Value) -> Result<Vec<u8>, RealtimeReadError> {
    // JSON.stringify(Uint8Array) produces numeric object keys in the existing transport.
    let len = value
        .as_array()
        .map(Vec::len)
        .or_else(|| value.as_object().map(|o| o.len()))
        .ok_or(RealtimeReadError::Authorization)?;
    if len == 0 || len > MAX_TERMINAL_INPUT {
        return Err(failure("Terminal input exceeds the limit."));
    }
    (0..len)
        .map(|index| {
            let v = if value.is_array() {
                &value[index]
            } else {
                &value[index.to_string()]
            };
            v.as_u64()
                .and_then(|v| u8::try_from(v).ok())
                .ok_or(RealtimeReadError::Authorization)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn existing_typed_array_and_plain_array_inputs_are_bounded_and_validated() {
        assert_eq!(input_bytes(&json!({"0":65,"1":10})).unwrap(), b"A\n");
        assert_eq!(input_bytes(&json!([65, 10])).unwrap(), b"A\n");
        for bad in [
            json!({"1":65}),
            json!([256]),
            json!([-1]),
            json!([]),
            json!(vec![0; MAX_TERMINAL_INPUT + 1]),
        ] {
            assert!(input_bytes(&bad).is_err());
        }
    }
}
