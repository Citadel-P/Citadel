//! Shared request construction and bounded reply handling for both Agent transports.
use super::*;

pub(crate) fn deployment_result(
    command: &RuntimeDeploymentCommand,
    response: citadel_contracts::citadel::deployments::v1::ApplyDeploymentResponse,
) -> RuntimeDeploymentResult {
    let state = match DeployedContainerState::try_from(response.deployed_container_state) {
        Ok(DeployedContainerState::Running) => RuntimeContainerState::Running,
        Ok(DeployedContainerState::Exited) => RuntimeContainerState::Exited,
        Ok(DeployedContainerState::Timeout) | Err(_) => RuntimeContainerState::Timeout,
    };
    RuntimeDeploymentResult {
        docker_container_id: response.container_id,
        docker_image_id: command.image_id.clone(),
        state,
    }
}

pub(crate) fn stack_request(
    claim: &citadel_stacks::StackOperationClaim,
    source: &citadel_stacks::StackApplySource,
    environment: &[String],
    registry: Option<&AgentStackRegistry>,
) -> StackApplyRequest {
    StackApplyRequest {
        stack_name: claim.name.clone(),
        compose_file_content: None,
        project_name: Some(claim.project_name.clone()),
        environment_file_path: claim.spec.common().env_file_path.clone(),
        registry_auth: registry.map(|value| value.auth.to_string()),
        registry_name: registry.map(|value| value.name.clone()),
        registry_host: registry.map(|value| value.host.clone()),
        destroy_before_deploy: claim.spec.common().destroy_before_deploy
            && claim.service_names.is_empty(),
        environment_variables: environment.to_vec(),
        pre_deploy: claim
            .spec
            .common()
            .pre_deploy
            .as_ref()
            .map(|command| ProtoStackCommand {
                commands: command.commands.clone(),
                path: command.path.clone(),
            }),
        post_deploy: claim
            .spec
            .common()
            .post_deploy
            .as_ref()
            .map(|command| ProtoStackCommand {
                commands: command.commands.clone(),
                path: command.path.clone(),
            }),
        service_names: claim.service_names.clone(),
        pull_images: true,
        source_working_directory: Some(source.working_directory.clone()),
        source_compose_file_paths: source.compose_paths.clone(),
        source_env_file_paths: source.env_file_paths.clone(),
        labels_override_file_path: source.labels_override_path.clone(),
        generated_files_directory: source
            .labels_override_path
            .as_deref()
            .and_then(|path| path.rsplit_once('/').map(|(parent, _)| parent.to_owned())),
        secret_files: Vec::new(),
        secret_target_service_names: Vec::new(),
        orchestration_mode: if claim.platform_type == "DockerSwarm" {
            ProtoStackOrchestrationMode::DockerSwarm as i32
        } else {
            ProtoStackOrchestrationMode::DockerCompose as i32
        },
        source_files: source
            .files
            .iter()
            .map(|file| ProtoStackSourceFile {
                relative_path: file.relative_path.clone(),
                content: file.content.clone(),
            })
            .collect(),
        retained_swarm_secrets: Vec::new(),
        retained_swarm_configs: Vec::new(),
        convert_compose_project_to_swarm: false,
    }
}

pub(crate) fn deployment_request(command: &RuntimeDeploymentCommand) -> ApplyDeploymentRequest {
    let spec = &command.spec;
    let life_cycle_spec = spec
        .life_cycle_spec
        .as_ref()
        .map(|life_cycle| ProtoLifeCycleSpec {
            stop_timeout: life_cycle.stop_timeout,
            stop_signal: life_cycle.stop_signal.map(|signal| match signal {
                StopSignal::SIGTERM => ProtoStopSignal::Sigterm as i32,
                StopSignal::SIGKILL => ProtoStopSignal::Sigkill as i32,
                StopSignal::SIGINT => ProtoStopSignal::Sigint as i32,
                StopSignal::SIGQUIT => ProtoStopSignal::Sigquit as i32,
            }),
            restart_policy: match life_cycle.restart_policy {
                ContainerRestartPolicy::No => ProtoRestartPolicy::No as i32,
                ContainerRestartPolicy::Always => ProtoRestartPolicy::Always as i32,
                ContainerRestartPolicy::OnFailure => ProtoRestartPolicy::OnFailure as i32,
                ContainerRestartPolicy::UnlessStopped => ProtoRestartPolicy::UnlessStopped as i32,
            },
        });
    let resource_spec = spec
        .resource_spec
        .as_ref()
        .map(|resource| ProtoResourceSpec {
            nano_cpus: resource.nano_cpus,
            memory_limit: resource.memory_limit,
        });
    ApplyDeploymentRequest {
        image_id: command.image_id.clone(),
        name: command.name.clone(),
        spec: Some(ProtoDeploymentSpec {
            image_id: command.image_id.clone(),
            life_cycle_spec,
            resource_spec,
            labels: spec
                .labels
                .clone()
                .unwrap_or_default()
                .into_iter()
                .collect(),
            ports: spec.ports.clone().unwrap_or_default(),
            env_vars: command.environment_variables.clone(),
            volumes: spec.volumes.clone().unwrap_or_default(),
            networks: spec.networks.clone().unwrap_or_default(),
            command: spec.command.clone().unwrap_or_default(),
        }),
    }
}

pub(crate) async fn consume_stack_stream<S>(
    mut stream: S,
    cancellation: &CancellationToken,
) -> Result<citadel_stacks::StackRuntimeResult, RuntimeCapabilityError>
where
    S: futures_util::Stream<
            Item = Result<citadel_contracts::citadel::stacks::v1::StackApplyResponse, Status>,
        > + Unpin,
{
    let mut status = citadel_stacks::StackReleaseStatus::Failed;
    let mut messages = Vec::new();
    let mut retained_bytes = 0usize;
    let mut failed = false;
    loop {
        let next = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(cancelled_error()),
            item = stream.next() => item,
        };
        let Some(item) = next else { break };
        let item = item.map_err(normalize_status)?;
        // A later successful cleanup/rollback command must not erase a
        // failed Apply. Docker acceptance and rollback are not Apply success.
        failed |= item.exit_code.is_some_and(|code| code != 0)
            || item.stack_status.as_deref() == Some("Failed");
        if let Some(value) = item.stack_status.as_deref() {
            status = citadel_stacks::StackReleaseStatus::parse(value).map_err(|error| {
                RuntimeCapabilityError::new(RuntimeErrorKind::Remote, error.to_string(), false)
            })?;
        } else if item.exit_code == Some(0) {
            status = citadel_stacks::StackReleaseStatus::Healthy;
        }
        let event_type = match ProtoStackApplyEventType::try_from(item.r#type) {
            Ok(ProtoStackApplyEventType::StdOut) => citadel_stacks::StackApplyEventType::StdOut,
            Ok(ProtoStackApplyEventType::StdErr) => citadel_stacks::StackApplyEventType::StdErr,
            Ok(ProtoStackApplyEventType::SystemMessage) => {
                citadel_stacks::StackApplyEventType::SystemMessage
            }
            Ok(ProtoStackApplyEventType::CommandCompleted) => {
                citadel_stacks::StackApplyEventType::CommandCompleted
            }
            _ => citadel_stacks::StackApplyEventType::Unknown,
        };
        let cost = item.message.as_ref().map_or(0, |message| message.len()) + 64;
        if retained_bytes.saturating_add(cost) > 512 * 1024 {
            continue;
        }
        retained_bytes += cost;
        messages.push(citadel_stacks::StackStreamItem {
            event_type,
            message: item.message,
            exit_code: item.exit_code,
            stack_status: Some(status),
            severity: None,
        });
    }
    if failed {
        status = citadel_stacks::StackReleaseStatus::Failed;
    }
    Ok(citadel_stacks::StackRuntimeResult { status, messages })
}

#[cfg(test)]
mod tests {
    use super::*;
    use citadel_contracts::citadel::stacks::v1::StackApplyResponse;

    // StackServiceTests: rejected Compose-to-Swarm deploy followed by a
    // successful Compose restoration must still report the Apply as failed.
    #[tokio::test]
    async fn cleanup_success_does_not_erase_apply_failure() {
        let replies = [0, 1, 0].map(|code| {
            Ok(StackApplyResponse {
                r#type: ProtoStackApplyEventType::CommandCompleted as i32,
                exit_code: Some(code),
                ..Default::default()
            })
        });
        let result = consume_stack_stream(
            futures_util::stream::iter(replies),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(result.status, citadel_stacks::StackReleaseStatus::Failed);
    }

    #[tokio::test]
    async fn output_is_bounded_but_final_failure_is_still_observed() {
        let replies = (0..1024)
            .map(|_| {
                Ok(StackApplyResponse {
                    message: Some("x".repeat(1024)),
                    ..Default::default()
                })
            })
            .chain(std::iter::once(Ok(StackApplyResponse {
                exit_code: Some(1),
                ..Default::default()
            })));
        let result = consume_stack_stream(
            futures_util::stream::iter(replies),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(result.status, citadel_stacks::StackReleaseStatus::Failed);
        assert!(result.messages.len() < 512);
        assert!(
            result
                .messages
                .iter()
                .map(|item| item.message.as_ref().map_or(0, String::len) + 64)
                .sum::<usize>()
                <= 512 * 1024
        );
    }

    #[tokio::test]
    async fn disconnected_or_cancelled_stack_does_not_report_success() {
        let replies = [
            Ok(StackApplyResponse {
                exit_code: Some(0),
                ..Default::default()
            }),
            Err(Status::unavailable("disconnected")),
        ];
        assert!(
            consume_stack_stream(
                futures_util::stream::iter(replies),
                &CancellationToken::new()
            )
            .await
            .is_err()
        );
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        assert!(
            consume_stack_stream(futures_util::stream::pending(), &cancellation)
                .await
                .is_err()
        );
    }
}
