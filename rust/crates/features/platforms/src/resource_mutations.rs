//! Native resource deletion policies. Hosting authorizes the request and publishes
//! each confirmed removal; adapters own transport selection.
use crate::{
    AuthorizedReadError, NetworkMutationPort, NetworkObservationPort, PlatformReadService,
    RuntimeCapabilityError, RuntimeErrorKind, VolumeMutationPort,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum NetworkDeletionError {
    #[error(transparent)]
    Read(#[from] AuthorizedReadError),
    #[error(transparent)]
    Runtime(#[from] RuntimeCapabilityError),
}

pub async fn delete_networks(
    reads: &PlatformReadService,
    runtime: &(impl NetworkObservationPort + NetworkMutationPort + ?Sized),
    platform: Uuid,
    is_swarm: bool,
    ids: &[String],
    cancel: &CancellationToken,
    removed: impl Fn(&str) + Send + Sync,
) -> Result<(), NetworkDeletionError> {
    // Finish every preflight before sending the first irreversible operation.
    for id in ids {
        let network = runtime.inspect_network(id, cancel).await?;
        if network
            .labels
            .get("com.citadel.system")
            .is_some_and(|v| v == "true")
        {
            return Err(conflict(format!(
                "System network '{}' cannot be deleted.",
                network.name
            ))
            .into());
        }
        if network.container_count != 0 {
            return Err(conflict(format!(
                "Network '{}' is in use and cannot be deleted.",
                network.name
            ))
            .into());
        }
        if network.labels.contains_key("com.docker.stack.namespace")
            || network.labels.contains_key("com.citadel.stack-id")
        {
            return Err(conflict(format!(
                "Stack-owned network '{}' cannot be deleted independently.",
                network.name
            ))
            .into());
        }
        if is_swarm && !network.scope.eq_ignore_ascii_case("swarm") {
            return Err(conflict("Node-local Network deletion requires an explicit Node target and is not available.".into()).into());
        }
        if is_swarm {
            reads
                .validate_swarm_network_deletion(platform, id, &network.name)
                .await?;
        }
    }
    for (deleted, id) in ids.iter().enumerate() {
        match runtime.delete_network(id, cancel).await {
            Ok(()) => {}
            Err(error) if error.kind == RuntimeErrorKind::NotFound => {}
            Err(error) => return Err(partial(error, deleted, ids.len(), "Networks").into()),
        }
        removed(id);
    }
    Ok(())
}

pub async fn delete_volumes(
    runtime: &(impl VolumeMutationPort + ?Sized),
    names: &[String],
    force: bool,
    cancel: &CancellationToken,
    removed: impl Fn(&str) + Send + Sync,
) -> Result<(), RuntimeCapabilityError> {
    for (deleted, name) in names.iter().enumerate() {
        match runtime.delete_volume(name, force, cancel).await {
            Ok(()) => {}
            Err(error) if error.kind == RuntimeErrorKind::NotFound => {}
            Err(error) => return Err(partial(error, deleted, names.len(), "Volumes")),
        }
        removed(name);
    }
    Ok(())
}

fn partial(
    error: RuntimeCapabilityError,
    deleted: usize,
    total: usize,
    kind: &str,
) -> RuntimeCapabilityError {
    if deleted == 0 {
        return error;
    }
    conflict(format!(
        "Deleted {deleted} of {total} {kind} before Docker rejected the operation: {}",
        error.message
    ))
}
fn conflict(message: String) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Conflict, message, false)
}
