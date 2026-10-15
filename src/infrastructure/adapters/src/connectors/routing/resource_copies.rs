use super::platforms::runtime::PlatformRuntimeRouter;
use citadel_platforms::{RuntimeErrorKind, VolumeMutationPort, VolumeObservationPort};
use citadel_resource_copies::*;
use futures_util::future::BoxFuture;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
fn storage_error(e: impl std::fmt::Display) -> CopyError {
    CopyError::Storage(e.to_string())
}
fn conflict(message: &str) -> CopyError {
    CopyError::Conflict(message.into())
}
mod current;
pub struct CopyStorageRuntime {
    pub image: String,
    pub runtime: Arc<PlatformRuntimeRouter>,
}
impl CopyStorage for CopyStorageRuntime {
    fn copy_current<'a>(
        &'a self,
        plan: &'a CopyPlan,
        volume: &'a CopyRestore,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), CopyError>> {
        Box::pin(self.current(plan, volume, cancel))
    }

    fn cleanup<'a>(
        &'a self,
        plan: &'a CopyPlan,
        restores: &'a [CopyRestore],
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), CopyError>> {
        Box::pin(async move {
            for restore in restores {
                if plan.request.data_mode == CopyDataMode::CurrentData {
                    self.cleanup_current_helper(plan, restore, cancel).await?;
                }

                if cancel.is_cancelled() {
                    return Err(conflict(
                        "Cleanup interrupted; remaining targets will be checked again on restart.",
                    ));
                }
                let target = plan
                    .preview
                    .volumes
                    .iter()
                    .find(|v| v.target == restore.target)
                    .ok_or_else(|| {
                        conflict("Cleanup receipt does not match the reviewed destination.")
                    })?;
                let runtime = self
                    .runtime
                    .volumes(plan.target_platform(), plan.target_node(target), cancel)
                    .await
                    .map_err(storage_error)?;
                let volume = match runtime.inspect_volume(&target.target, cancel).await {
                    Ok(volume) => volume,
                    Err(error) if error.kind == RuntimeErrorKind::NotFound => continue,
                    Err(error) => return Err(storage_error(error)),
                };
                crate::external::backups::restore_storage::RestoreVolumeOwner::new(
                    plan.target_platform(),
                    restore.id,
                )
                .verify(&target.target, &volume)
                .map_err(CopyError::Conflict)?;
                // force=false also refuses volumes referenced by stopped containers.
                runtime.delete_volume(&target.target,false,cancel).await.map_err(|_|conflict("A destination volume is in use or could not be removed. It was retained; inspect its consumers before retrying cleanup."))?;
            }
            Ok(())
        })
    }

    fn check<'a>(
        &'a self,
        plan: &'a CopyPlan,
        restores: Option<&'a [CopyRestore]>,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), CopyError>> {
        Box::pin(self.inspect(plan, restores, cancel))
    }
}
impl CopyStorageRuntime {
    async fn inspect(
        &self,
        plan: &CopyPlan,
        restores: Option<&[CopyRestore]>,
        cancellation: &CancellationToken,
    ) -> Result<(), CopyError> {
        tokio::time::timeout(std::time::Duration::from_secs(30),async {
            for volume in &plan.preview.volumes {
                let runtime=self.runtime.volumes(plan.target_platform(),plan.target_node(volume),cancellation).await.map_err(storage_error)?;
                if let Some(restores)=restores {
                    let restore=restores.iter().find(|r|r.target==volume.target).ok_or_else(||conflict("Restore receipt is missing."))?;
                    let target=runtime.inspect_volume(&volume.target,cancellation).await.map_err(|_|conflict("Restored volume is unavailable. No workload was created."))?;
                    crate::external::backups::restore_storage::RestoreVolumeOwner::new(plan.target_platform(),restore.id).verify(&volume.target,&target).map_err(CopyError::Conflict)?;
                } else {
                    if plan.request.data_mode == CopyDataMode::CurrentData {
                        let source=runtime.inspect_volume(&volume.source,cancellation).await.map_err(|_|conflict("A source volume could not be inspected. Bring its Platform or exact Swarm Node online and retry."))?;
                        if source.content_access_allowed != Some(true) || source.driver!="local" || source.scope!="local" || !source.options.is_empty() {
                            return Err(conflict("Only plain local Docker volumes can be copied. Custom drivers and driver options are unsupported."));
                        }
                    }
                    match runtime.inspect_volume(&volume.target,cancellation).await {
                        Err(error) if error.kind==RuntimeErrorKind::NotFound => {},
                        Err(_)=>return Err(conflict("Destination volume availability could not be verified.")),
                        Ok(_)=>return Err(conflict("A destination volume already exists. Review again to allocate fresh names.")),
                    }
                }
            }
            Ok(())
        }).await.map_err(|_|conflict("Storage inspection timed out. Check the Platform connection and retry."))?
    }
}

pub async fn verify_destination(
    pool: &sqlx::PgPool,
    runtime: &PlatformRuntimeRouter,
    id: uuid::Uuid,
    platform_id: uuid::Uuid,
    node_id: Option<&str>,
    cancel: &CancellationToken,
) -> Result<(), String> {
    let receipts =
        crate::persistence::postgres::resource_copies::storage::destination_receipts(pool, id)
            .await
            .map_err(|e| e.to_string())?;
    for receipt in receipts {
        receipt
            .verify_location(platform_id, node_id)
            .map_err(|e| e.to_string())?;
        let volumes = runtime
            .volumes(receipt.platform_id, receipt.node_id.as_deref(), cancel)
            .await
            .map_err(|e| e.to_string())?;
        let volume=volumes.inspect_volume(&receipt.name,cancel).await.map_err(|_|format!("Copied volume {} is missing or unavailable. Apply was blocked to avoid creating empty replacement storage.",receipt.name))?;
        crate::external::backups::restore_storage::RestoreVolumeOwner::new(
            receipt.platform_id,
            receipt.restore_id,
        )
        .verify(&receipt.name, &volume)?;
    }
    Ok(())
}
