//! Complete native Swarm use cases; callers only authorize and present the result.
use super::*;
use crate::{
    InventoryProjectionStore, PlatformDetails, PlatformReadService,
    runtime_provider::PlatformRuntimeProvider,
};

pub enum SwarmOperation {
    UpdateNode {
        id: String,
        input: UpdateSwarmNodeInput,
    },
    UpdateAvailability(UpdateSwarmNodesAvailabilityInput),
    RestartService(String),
    CreateMaterial {
        secret: bool,
        input: CreateSwarmMaterialInput,
    },
    UpdateLabels {
        secret: bool,
        id: String,
        input: UpdateSwarmResourceLabelsInput,
    },
    Delete {
        kind: SwarmResourceKind,
        input: DeleteSwarmResourcesInput,
    },
}

pub struct SwarmOperations<'a> {
    pub reads: &'a PlatformReadService,
    pub services: &'a dyn citadel_swarm_services::SwarmServiceRepository,
    pub runtime: &'a dyn PlatformRuntimeProvider,
    pub projections: &'a dyn InventoryProjectionStore,
    pub changed: &'a (dyn Fn(&str, &str, &str) + Send + Sync),
}
impl From<RuntimeCapabilityError> for SwarmCompletionError {
    fn from(error: RuntimeCapabilityError) -> Self {
        Self::Runtime(error)
    }
}
impl From<crate::AuthorizedReadError> for SwarmCompletionError {
    fn from(error: crate::AuthorizedReadError) -> Self {
        let kind = match &error {
            crate::AuthorizedReadError::NotFound => RuntimeErrorKind::NotFound,
            crate::AuthorizedReadError::Conflict(_) => RuntimeErrorKind::Conflict,
            crate::AuthorizedReadError::Storage(_) => RuntimeErrorKind::Remote,
        };
        Self::Runtime(RuntimeCapabilityError::new(kind, error.to_string(), false))
    }
}
fn valid(result: Result<(), &'static str>) -> Result<(), SwarmCompletionError> {
    result.map_err(|message| {
        RuntimeCapabilityError::new(RuntimeErrorKind::InvalidRequest, message, false).into()
    })
}
impl SwarmOperations<'_> {
    pub async fn execute(
        &self,
        platform: &PlatformDetails,
        mut operation: SwarmOperation,
        shutdown: &CancellationToken,
    ) -> Result<(), SwarmCompletionError> {
        let pid = platform.id;
        if platform.platform_type != crate::PlatformKind::DockerSwarm {
            return Err(RuntimeCapabilityError::new(
                RuntimeErrorKind::Conflict,
                "A Swarm platform is required.",
                false,
            )
            .into());
        }
        // Preflight the entire selection before the first external mutation.
        let mut updates = Vec::new();
        match &mut operation {
            SwarmOperation::UpdateNode { id, input } => {
                valid(resource_id(id).and(input.validate()))?;
                let node = self
                    .reads
                    .get_swarm_node(pid, id)
                    .await?
                    .ok_or(crate::AuthorizedReadError::NotFound)?;
                check_node(&node, input.version_index)?;
            }
            SwarmOperation::UpdateAvailability(input) => {
                valid(input.validate())?;
                updates = availability_updates(self.reads, pid, input).await?;
                if updates.is_empty() {
                    return Ok(());
                }
            }
            SwarmOperation::RestartService(id) => {
                valid(resource_id(id))?;
                service_guard(self.reads, self.services, pid, id).await?;
            }
            SwarmOperation::CreateMaterial { secret, input } => valid(input.validate(*secret))?,
            SwarmOperation::UpdateLabels { secret, id, input } => {
                valid(resource_id(id).and(input.validate()))?;
                material_guard(
                    self.reads,
                    pid,
                    id,
                    *secret,
                    false,
                    Some(input.version_index),
                )
                .await?;
            }
            SwarmOperation::Delete { kind, input } => {
                valid(input.validate())?;
                for id in &input.ids {
                    if *kind == SwarmResourceKind::Service {
                        service_guard(self.reads, self.services, pid, id).await?;
                    } else {
                        material_guard(
                            self.reads,
                            pid,
                            id,
                            *kind == SwarmResourceKind::Secret,
                            true,
                            None,
                        )
                        .await?;
                    }
                }
            }
        }
        let cancel = shutdown.child_token();
        let _guard = cancel.clone().drop_guard();
        let runtime = self.runtime.swarm(pid, &cancel).await?;
        let mut removed = Vec::new();
        let result = bounded(async {
            manager_identity(runtime.as_ref(), platform, &cancel).await?;
            match &operation {
                SwarmOperation::UpdateNode { id, input } => {
                    runtime.update_node(id, input, &cancel).await
                }
                SwarmOperation::UpdateAvailability(_) => {
                    update_nodes(runtime.as_ref(), &updates, &cancel).await
                }
                SwarmOperation::RestartService(id) => runtime.restart_service(id, &cancel).await,
                SwarmOperation::CreateMaterial { secret, input } => {
                    runtime.create_material(*secret, input, &cancel).await
                }
                SwarmOperation::UpdateLabels { secret, id, input } => {
                    runtime.update_labels(*secret, id, input, &cancel).await
                }
                SwarmOperation::Delete { kind, input } => {
                    delete_resources(runtime.as_ref(), &input.ids, *kind, &cancel, &mut removed)
                        .await
                }
            }
        })
        .await;
        match operation {
            SwarmOperation::Delete { kind, .. } => {
                finish_deletion(
                    self.projections,
                    self.runtime,
                    platform,
                    result,
                    kind,
                    &removed,
                    self.changed,
                )
                .await
            }
            _ => finish_mutation(self.projections, self.runtime, platform, result, |kind| {
                (self.changed)(kind, "update", "")
            })
            .await
            .map_err(Into::into),
        }
    }
}

pub async fn bounded<T>(
    future: impl std::future::Future<Output = Result<T, RuntimeCapabilityError>>,
) -> Result<T, RuntimeCapabilityError> {
    tokio::time::timeout(std::time::Duration::from_secs(30), future)
        .await
        .unwrap_or_else(|_| {
            Err(RuntimeCapabilityError::new(
                RuntimeErrorKind::Timeout,
                "Swarm operation timed out; inventory will be reconciled.",
                false,
            ))
        })
}
