use super::*;
use crate::realtime::topic::Topic;
use citadel_platforms::{ContainerView, logs::ContainerLogPort};
use futures_util::StreamExt;
use tokio_util::sync::CancellationToken;

static LOG_STREAM_SLOTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(128);

impl ApplicationGroupReader {
    pub(super) async fn log_targets(
        &self,
        p: &ActorPrincipal,
        g: &Group,
    ) -> Result<Vec<ContainerView>, RealtimeReadError> {
        if matches!(g.topic(), Topic::StackLog(..)) {
            let id = g.id().ok_or(RealtimeReadError::Authorization)?;
            self.permission(
                p,
                ResourceType::Stack,
                Some(id),
                Some(SpecificPermission::Logs),
            )
            .await?;
            let stack = self
                .stacks
                .get_authorized(p.actor_id, p.is_administrator(), id)
                .await
                .map_err(failure)?;
            let containers = self
                .platforms
                .list_containers(
                    stack
                        .platform_id
                        .ok_or_else(|| failure("The Stack has no applied Platform."))?,
                )
                .await
                .map_err(failure)?;
            let containers: Vec<_> = containers
                .into_iter()
                .filter(|c| c.stack_id == Some(id) && c.state.eq_ignore_ascii_case("running"))
                .collect();
            if containers.len() > 32 {
                return Err(failure(
                    "At most 32 container log streams can be opened per Stack.",
                ));
            }
            return Ok(containers);
        }
        let container = self
            .platforms
            .get_container_by_reference(g.reference().unwrap_or_default())
            .await
            .map_err(failure)?
            .ok_or(RealtimeReadError::Authorization)?;
        for (kind, id) in [
            (ResourceType::Platform, Some(container.platform_id)),
            (ResourceType::Deployment, container.deployment_id),
            (ResourceType::Stack, container.stack_id),
        ] {
            if let Some(id) = id
                && self
                    .permission(p, kind, Some(id), Some(SpecificPermission::Logs))
                    .await
                    .is_ok()
            {
                return Ok(vec![container]);
            }
        }
        Err(RealtimeReadError::Authorization)
    }
    pub(super) async fn logs(
        &self,
        p: &ActorPrincipal,
        g: &Group,
        cancel: &CancellationToken,
    ) -> Result<Option<GroupStream>, RealtimeReadError> {
        if !matches!(g.topic(), Topic::ContainerLog(..) | Topic::StackLog(..)) {
            return Ok(None);
        }
        // Subscribe before reading the projection so changes during initialization
        // are not lost. Only new/replaced containers open a new daemon stream.
        let mut changes = self.docker.realtime.as_ref().map(|hub| hub.subscribe());
        let targets = self.log_targets(p, g).await?;
        let platform_id = if matches!(g.topic(), Topic::StackLog(..)) {
            self.stacks
                .get_authorized(
                    p.actor_id,
                    p.is_administrator(),
                    g.id().ok_or(RealtimeReadError::Authorization)?,
                )
                .await
                .map_err(failure)?
                .platform_id
        } else {
            targets.first().map(|target| target.platform_id)
        };
        let mut streams = tokio_stream::StreamMap::new();
        let mut identities = BTreeMap::new();
        for target in &targets {
            streams.insert(
                target.id,
                self.open_log(target, matches!(g.topic(), Topic::StackLog(..)), cancel)
                    .await?,
            );
            identities.insert(target.id, log_identity(target));
        }
        let reader = self.clone();
        let principal = p.clone();
        let group = g.clone();
        let cancel = cancel.clone();
        Ok(Some(Box::pin(async_stream::try_stream! {
            loop {
                let (change,item)=tokio::select! {
                    biased;
                    ()=cancel.cancelled()=>break,
                    change=async {match &mut changes {Some(receiver)=>receiver.recv().await,None=>std::future::pending().await}}=>(Some(change),None),
                    Some((_,item))=streams.next(),if !streams.is_empty()=>(None,Some(item)),
                    else=>break,
                };
                if let Some(item)=item {yield item?;}
                if let Some(change)=change {
                        let change=change.map_err(failure)?;
                        if change.platform_id!=platform_id || change.event_kind!="runtimeChanged" || change.payload["dockerResourceType"]=="containerStats" {continue;}
                        // Keep forwarding logs while the projection read is pending.
                        let read=reader.log_targets(&principal,&group);
                        tokio::pin!(read);
                        let targets=loop {
                            tokio::select! {
                                biased;
                                ()=cancel.cancelled()=>return,
                                targets=&mut read=>break targets,
                                Some((_,item))=streams.next(),if !streams.is_empty()=>yield item?,
                            }
                        };
                        let targets=targets?;
                        identities.retain(|id,_|{
                            let keep=targets.iter().any(|t|t.id==*id);
                            if !keep {streams.remove(id);}
                            keep
                        });
                        for target in targets {
                            if target.projection_stale_since.is_some() {Err(failure("Container node data is stale."))?;}
                            let identity=log_identity(&target);
                            if identities.get(&target.id)!=Some(&identity) || (!streams.contains_key(&target.id) && target.state.eq_ignore_ascii_case("running")) {
                                // Release the previous command/slot before opening its replacement.
                                streams.remove(&target.id);
                                streams.insert(target.id,reader.open_log(&target,matches!(group.topic(), Topic::StackLog(..)),&cancel).await?);
                                identities.insert(target.id,identity);
                            }
                        }
                }
            }
        })))
    }

    async fn open_log(
        &self,
        target: &ContainerView,
        stack: bool,
        cancel: &CancellationToken,
    ) -> Result<GroupStream, RealtimeReadError> {
        let permit = LOG_STREAM_SLOTS
            .try_acquire()
            .map_err(|_| failure("Log stream capacity exceeded."))?;
        if target.projection_stale_since.is_some() {
            return Err(failure("Container node data is stale."));
        }
        let runtime = crate::platforms_http::runtime_for_node(
            &self.docker,
            target.platform_id,
            target.docker_node_id.as_deref(),
        )
        .await
        .map_err(failure)?;
        use crate::platforms_http::RuntimeRef;
        let stream = match runtime {
            RuntimeRef::Local(r) => r.container_logs(&target.container_id, cancel).await,
            RuntimeRef::Agent(r) => r.container_logs(&target.container_id, cancel).await,
            RuntimeRef::Edge(r) => r.container_logs(&target.container_id, cancel).await,
        }
        .map_err(failure)?;
        let mut lines = log_lines(
            stream,
            if stack {
                Some(target.name.clone())
            } else {
                None
            },
        );
        Ok(Box::pin(async_stream::stream! {
            let _permit=permit;
            while let Some(line)=lines.next().await {yield line;}
        }))
    }
    pub(super) async fn log_invocation(
        &self,
        p: &ActorPrincipal,
        target: &str,
        args: &[Value],
    ) -> Result<Option<Group>, RealtimeReadError> {
        let [value] = args else { return Ok(None) };
        let Some(id) = value.as_str() else {
            return Ok(None);
        };
        let group = match target {
            "StartContainerLogs" => {
                let reference = if Uuid::parse_str(id).is_ok() {
                    id
                } else if id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit()) {
                    &id[..12]
                } else {
                    id
                };
                Group::parse(&format!("container-log:{}", reference.to_ascii_lowercase()))
            }
            "StartStackLogs" => Group::parse(&format!("stack-log:{id}")),
            "StartDeploymentLogs" => {
                let id = Uuid::parse_str(id).map_err(|_| RealtimeReadError::Authorization)?;
                self.deployment_permission::<citadel_deployments::permissions::ViewDeploymentLogs>(
                    p, id,
                )
                .await?;
                let deployment = self
                    .deployments
                    .get_authorized(p.actor_id, p.is_administrator(), id)
                    .await
                    .map_err(failure)?;
                // Match the container selected by the Deployment read model, including
                // stopped containers: Docker retains their historical logs.
                let reference = deployment.docker_container_id.as_deref().ok_or_else(|| {
                    failure("The Deployment has no container available for logs.")
                })?;
                let reference = reference.get(..12).unwrap_or(reference);
                Group::parse(&format!("container-log:{}", reference.to_ascii_lowercase()))
            }
            _ => return Ok(None),
        };
        if let Some(group) = &group {
            self.log_targets(p, group).await?;
        }
        Ok(group)
    }
}

fn log_identity(container: &ContainerView) -> (String, Option<String>, String) {
    (
        container.container_id.clone(),
        container.docker_node_id.clone(),
        container.name.clone(),
    )
}

fn log_lines(
    mut input: citadel_platforms::logs::RuntimeLogStream,
    name: Option<String>,
) -> GroupStream {
    Box::pin(async_stream::try_stream! {
        let mut line=Vec::new();
        while let Some(chunk)=input.next().await {
            for segment in chunk.map_err(failure)?.split_inclusive(|b|*b==b'\n') {
                if line.len()+segment.len()>citadel_platforms::logs::MAX_LOG_FRAME {Err(failure("Log line exceeds the limit."))?;}
                line.extend_from_slice(segment);
                if line.last()==Some(&b'\n') {
                    yield log_event(&line,name.as_deref());
                    line.clear();
                }
            }
        }
        if !line.is_empty(){line.push(b'\n');yield log_event(&line,name.as_deref());}
    })
}

fn log_event(line: &[u8], name: Option<&str>) -> ClientEvent {
    if let Some(name) = name {
        // Keep the Docker timestamp first for the existing viewer's date and container filters.
        let offset = line.iter().position(|b| *b == b' ').map_or(0, |p| p + 1);
        let mut value = Vec::with_capacity(line.len() + name.len() + 3);
        value.extend_from_slice(&line[..offset]);
        value.push(b'[');
        value.extend_from_slice(name.as_bytes());
        value.extend_from_slice(b"] ");
        value.extend_from_slice(&line[offset..]);
        ClientEvent::new("SendStackLogs", vec![json!(value)])
    } else {
        ClientEvent::new("SendContainerLogs", vec![json!(line)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn stack_lines_preserve_timestamp_filter_prefix_and_split_utf8_including_final_line() {
        let input = "2026-09-06T12:00:00Z héllo\n2026-09-06T12:00:01Z final".as_bytes();
        let chunks = input.iter().map(|b| Ok(vec![*b])).collect::<Vec<_>>();
        let output: Vec<_> = log_lines(
            Box::pin(futures_util::stream::iter(chunks)),
            Some("web".into()),
        )
        .collect()
        .await;
        assert_eq!(output.len(), 2);
        for (item, expected) in output.into_iter().zip([
            "2026-09-06T12:00:00Z [web] héllo\n",
            "2026-09-06T12:00:01Z [web] final\n",
        ]) {
            let event = item.unwrap();
            assert_eq!(event.target, "SendStackLogs");
            let data: Vec<u8> = serde_json::from_value(event.arguments[0].clone()).unwrap();
            assert_eq!(String::from_utf8(data).unwrap(), expected);
        }
    }
    #[tokio::test]
    async fn container_lines_keep_the_existing_viewer_payload_and_reject_unbounded_lines() {
        let output = log_lines(
            Box::pin(futures_util::stream::iter([Ok(b"date line\n".to_vec())])),
            None,
        )
        .next()
        .await
        .unwrap()
        .unwrap();
        assert_eq!(output.target, "SendContainerLogs");
        assert_eq!(output.arguments[0], json!(b"date line\n".to_vec()));
        let output = log_lines(
            Box::pin(futures_util::stream::iter([Ok(vec![
                b'x';
                citadel_platforms::logs::MAX_LOG_FRAME
                    + 1
            ])])),
            None,
        )
        .next()
        .await
        .unwrap();
        assert!(output.is_err());
    }
}
