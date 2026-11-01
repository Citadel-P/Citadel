use super::{Runtime, stack_source};
use citadel_contracts::citadel::stacks::v1::{
    StackApplyRequest, StackApplyResponse, stack_service_server::StackService,
};
use citadel_docker::stacks::LocalStackApply;
use citadel_stacks::{StackCommand, StackOrchestrationMode, StackProgress, StackReleaseStatus};
use futures_util::stream::BoxStream;
use tonic::{Request, Response, Status};
#[cfg(all(test, unix))]
mod tests;
#[tonic::async_trait]
impl StackService for Runtime {
    type ApplyStream = BoxStream<'static, Result<StackApplyResponse, Status>>;
    async fn apply(
        &self,
        r: Request<StackApplyRequest>,
    ) -> Result<Response<Self::ApplyStream>, Status> {
        let r = r.into_inner();
        let permit = self
            .mutations
            .clone()
            .try_acquire_owned()
            .map_err(|_| Status::resource_exhausted("Agent mutation limit reached"))?;
        let stack_root = self.stack_root.clone();
        let (r, prepared) = tokio::task::spawn_blocking(move || {
            let prepared = stack_source::prepare_admitted(&r, &stack_root, Some(permit))?;
            Ok::<_, Status>((r, prepared))
        })
        .await
        .map_err(|_| Status::internal("Stack preparation task failed"))??;
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
                    if result.status==StackReleaseStatus::Healthy && r.orchestration_mode==0 {
                        let _retained = tokio::task::spawn_blocking(move || {
                            let mut prepared = prepared;
                            prepared.retain_secrets();
                            prepared
                        }).await.map_err(|_| Status::internal("Stack secret retention task failed"))?;
                    }
                    // Apply reports command completion. Core observes container health separately.
                    yield StackApplyResponse{r#type:4,message:None,exit_code:Some(if result.status==StackReleaseStatus::Healthy{0}else{1}),stack_status:Some(result.status.as_str().into())};
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
