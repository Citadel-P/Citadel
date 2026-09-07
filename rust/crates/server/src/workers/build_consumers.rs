use citadel_builds::{BuildConsumerClaim, BuildConsumerRuntime, BuildConsumerType, BuildError};
use citadel_deployments::DeploymentService;
use citadel_domain::ActorId;
use citadel_stacks::StackService;
use futures_util::future::BoxFuture;
use std::sync::Arc;
use uuid::Uuid;

/// Composition of existing application services; all Apply ownership and Docker
/// routing remain in those services, including their progress and recovery.
pub(super) struct BuildConsumers {
    pub deployments: Arc<DeploymentService>,
    pub stacks: Arc<StackService>,
    pub realtime: Option<crate::realtime::RealtimeHub>,
}
impl BuildConsumerRuntime for BuildConsumers {
    fn apply<'a>(&'a self, claim: &'a BuildConsumerClaim) -> BoxFuture<'a, Result<(), BuildError>> {
        Box::pin(async move {
            let version = claim
                .expected_version
                .ok_or_else(|| BuildError::Conflict("Build consumer changed.".into()))?;
            let actor = ActorId::new(Uuid::from_u128(1));
            match claim.resource_type {
                BuildConsumerType::Deployment => {
                    let mut output = self
                        .deployments
                        .apply_versioned(actor, true, claim.resource_id, false, Some(version))
                        .await
                        .map_err(failure)?;
                    while let Some(item) = output.recv().await {
                        if item.error.is_some() {
                            return Err(failure("Deployment Apply failed"));
                        }
                    }
                    let current = self
                        .deployments
                        .get(actor, true, claim.resource_id)
                        .await
                        .map_err(failure)?;
                    if current.status != "Healthy"
                        || !matches!(current.spec.image,
                        citadel_deployments::DeploymentImageInfo::Build { applied_build_run_id: Some(run),.. } if run==claim.build_run_id)
                    {
                        return Err(failure("Deployment Apply outcome was not confirmed"));
                    }
                }
                BuildConsumerType::Stack => {
                    let mut output = self
                        .stacks
                        .apply_selected(
                            actor,
                            true,
                            claim.resource_id,
                            citadel_stacks::StackApplyOptions {
                                expected_version: Some(version),
                                service_names: claim.service_names.clone(),
                            },
                        )
                        .await
                        .map_err(failure)?;
                    let mut completed = false;
                    while let Some(item) = output.recv().await {
                        if item.exit_code.is_some_and(|code| code != 0) {
                            return Err(failure("Stack Apply failed"));
                        }
                        completed |= item.exit_code == Some(0);
                    }
                    if !completed {
                        return Err(failure("Stack Apply was interrupted"));
                    }
                }
            }
            Ok(())
        })
    }
    fn changed(&self, resource: BuildConsumerType, id: Uuid) {
        if let Some(realtime) = &self.realtime {
            realtime.publish_resource_change(resource.as_str(), id, "updated");
        }
    }
    fn log(&self, entry: citadel_builds::BuildLogEntry) {
        if let Some(realtime) = &self.realtime {
            realtime.publish_build_log(entry.build_run_id, entry);
        }
    }
}
fn failure(error: impl std::fmt::Display) -> BuildError {
    BuildError::Conflict(error.to_string())
}
