use super::{Runtime, stack_source};
use citadel_adapters::external::stacks::LocalStackApply;
use citadel_contracts::citadel::stacks::v1::{
    StackApplyRequest, StackApplyResponse, stack_service_server::StackService,
};
use citadel_stacks::{StackCommand, StackOrchestrationMode, StackProgress, StackReleaseStatus};
use futures_util::{StreamExt, stream::BoxStream};
use tonic::{Request, Response, Status};
#[tonic::async_trait]
impl StackService for Runtime {
    type ApplyStream = BoxStream<'static, Result<StackApplyResponse, Status>>;
    async fn apply(
        &self,
        r: Request<StackApplyRequest>,
    ) -> Result<Response<Self::ApplyStream>, Status> {
        let r = r.into_inner();
        let mut prepared = stack_source::prepare(&r, std::path::Path::new("/app/data/stacks"))?;
        let command = |v: citadel_contracts::citadel::stacks::v1::StackCommand| StackCommand {
            commands: v.commands,
            path: if v.path.trim().is_empty() {
                ".".into()
            } else {
                v.path
            },
        };
        let runtime = LocalStackApply {
            docker: self.docker_cli.clone(),
            endpoint: self.docker.endpoint().clone(),
            project_name: r
                .project_name
                .as_ref()
                .filter(|v| !v.is_empty())
                .unwrap_or(&r.stack_name)
                .clone(),
            orchestration: if r.orchestration_mode == 1 {
                StackOrchestrationMode::DockerSwarm
            } else {
                StackOrchestrationMode::DockerCompose
            },
            destroy_before_deploy: r.destroy_before_deploy,
            pre_deploy: r.pre_deploy.map(command),
            post_deploy: r.post_deploy.map(command),
            service_names: r.service_names,
            pull_images: r.pull_images,
        };
        let docker = self.docker.clone();
        let cancel = self.shutdown.child_token();
        let guard = cancel.clone().drop_guard();
        let (tx, mut rx) = tokio::sync::mpsc::channel(8);
        let mut secrets: Vec<_> = r.secret_files.into_iter().map(|v| v.content).collect();
        secrets.extend(r.registry_auth.iter().cloned());
        secrets.extend(
            r.environment_variables
                .iter()
                .filter_map(|v| v.split_once('=').map(|(_, v)| v.to_owned())),
        );
        let progress = StackProgress::new(tx, secrets, cancel.clone());
        Ok(Response::new(Box::pin(async_stream::try_stream! {
            let _guard=guard;
            let outcome={
                let operation=runtime.apply_directory(&prepared.root,&prepared.source,&prepared.environment,r.registry_auth.as_deref().map(|auth|(r.registry_host.as_deref().filter(|v|!v.is_empty()).unwrap_or("https://index.docker.io/v1/"),auth)),r.convert_compose_project_to_swarm,&cancel,Some(&progress));
                tokio::pin!(operation);
                let result=loop{tokio::select!{result=&mut operation=>break result,Some(item)=rx.recv()=>yield event(item)}};
                while let Ok(item)=rx.try_recv(){yield event(item);}
                result
            };
            match outcome{
                Ok(result)=>{
                    let status=if result.status==StackReleaseStatus::Healthy && r.orchestration_mode==0 {
                        prepared.retain_secrets();
                        let settled=tokio::select! { ()=cancel.cancelled()=>Err(Status::cancelled("Stack apply cancelled")), ()=tokio::time::sleep(std::time::Duration::from_secs(4))=>Ok(()) };settled?;
                        observe_compose(&docker,&runtime.project_name).await.map(|v|v.as_str().to_owned())
                    } else {Some(result.status.as_str().into())};
                    yield StackApplyResponse{r#type:4,message:None,exit_code:Some(if result.status==StackReleaseStatus::Healthy{0}else{1}),stack_status:status};
                }
                Err(citadel_stacks::StackError::Cancelled)=>Err(Status::cancelled("Stack apply cancelled"))?,
                Err(error)=>{
                    yield StackApplyResponse{r#type:2,message:Some(error.to_string()),exit_code:None,stack_status:None};
                    yield StackApplyResponse{r#type:4,message:None,exit_code:Some(1),stack_status:Some("Failed".into())};
                }
            }
        })))
    }
}
fn event(v: citadel_stacks::StackProgressItem) -> StackApplyResponse {
    StackApplyResponse {
        r#type: match v.event_type {
            citadel_stacks::StackApplyEventType::Unknown => 0,
            citadel_stacks::StackApplyEventType::StdOut => 1,
            citadel_stacks::StackApplyEventType::StdErr => 2,
            citadel_stacks::StackApplyEventType::SystemMessage => 3,
            citadel_stacks::StackApplyEventType::CommandCompleted => 4,
        },
        message: v.message,
        exit_code: v.exit_code,
        stack_status: v.stack_status.map(|v| v.as_str().into()),
    }
}

async fn observe_compose(
    docker: &citadel_adapters::connectors::docker::DockerClient,
    project: &str,
) -> Option<StackReleaseStatus> {
    let filter =
        serde_json::json!({"label":[format!("com.docker.compose.project={project}")]}).to_string();
    let containers = docker
        .list_container_models(Some(true), None, None, Some(&filter))
        .await
        .ok()?;
    if containers.is_empty() {
        return None;
    }
    let states = futures_util::stream::iter(containers.into_iter().map(|v| async move {
        docker
            .inspect_container(&v.id.unwrap_or_default())
            .await
            .map(|v| v.state)
    }))
    .buffer_unordered(8)
    .collect::<Vec<_>>()
    .await;
    let states = states.into_iter().collect::<Result<Vec<_>, _>>().ok()?;
    Some(compose_status(&states))
}
fn compose_status(
    states: &[citadel_adapters::connectors::docker::projection::ContainerState],
) -> StackReleaseStatus {
    let (mut running, mut pending, mut paused, mut stopped) = (false, false, false, false);
    for s in states {
        let health = s
            .health
            .as_ref()
            .map(|h| h.status.as_str())
            .unwrap_or_default();
        if health == "unhealthy" || matches!(s.status.as_str(), "dead" | "removing") {
            return StackReleaseStatus::Degraded;
        }
        match s.status.as_str() {
            "exited" if s.exit_code > 0 => return StackReleaseStatus::Degraded,
            "exited" => stopped = true,
            "running" if health == "starting" => pending = true,
            "running" => running = true,
            "paused" => paused = true,
            "created" | "restarting" => pending = true,
            _ => return StackReleaseStatus::Degraded,
        }
    }
    if pending {
        StackReleaseStatus::Pending
    } else if paused && (running || stopped) || running && stopped {
        StackReleaseStatus::Degraded
    } else if paused {
        StackReleaseStatus::Paused
    } else if running {
        StackReleaseStatus::Healthy
    } else if stopped {
        StackReleaseStatus::Stopped
    } else {
        StackReleaseStatus::Unknown
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compose_success_is_not_assumed_healthy() {
        use citadel_adapters::connectors::docker::projection::ContainerState;
        let state = |status: &str, exit_code: i64| ContainerState {
            status: status.into(),
            exit_code,
            ..Default::default()
        };
        assert_eq!(
            compose_status(&[state("running", 0)]),
            StackReleaseStatus::Healthy
        );
        assert_eq!(
            compose_status(&[state("running", 0), state("exited", 1)]),
            StackReleaseStatus::Degraded
        );
        assert_eq!(
            compose_status(&[state("running", 0), state("created", 0)]),
            StackReleaseStatus::Pending
        );
        assert_eq!(
            compose_status(&[state("exited", 0)]),
            StackReleaseStatus::Stopped
        );
    }
}
