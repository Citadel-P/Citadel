//! SwarmTaskContainerPruner: use the normal node-aware mutation path, never force
//! deletion or remove volumes. Claims recheck state after the candidate query.
use citadel_platforms::containers::{ContainerAction, ContainerMutationService};
use citadel_runtime::notifications::NotificationSource;

use citadel_platforms::containers::DeleteContainerOptions;

use citadel_platforms::jobs::background::ContainerMaintenanceReader;

use std::sync::Arc;
#[cfg(test)]
use std::time::Duration;

use tokio_util::sync::CancellationToken;

pub(crate) async fn run(
    token: CancellationToken,
    store: Arc<dyn ContainerMaintenanceReader>,
    containers: Arc<ContainerMutationService>,
    events: Box<dyn NotificationSource>,
) -> Result<(), std::convert::Infallible> {
    let mut tick = super::schedule::interval("pruning", super::recovery::FALLBACK);
    let mut startup = true;
    let mut listener = events;
    loop {
        if token.is_cancelled() {
            return Ok(());
        }
        if !std::mem::take(&mut startup) {
            tokio::select! {
                ()=token.cancelled()=>return Ok(()),
                _=tick.tick()=>{},
                event=listener.recv() => {
                    if let Err(error) = event { tracing::warn!(%error, "Swarm prune event listener failed"); tokio::select! { ()=token.cancelled()=>return Ok(()), _=tokio::time::sleep(std::time::Duration::from_secs(2))=>{} } }
                }
            }
        }
        let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::Pruning.start();
        let candidates = store.pruning_candidates().await;
        match candidates {
            Ok(ids) => {
                for id in ids {
                    if token.is_cancelled() {
                        return Ok(());
                    }
                    if let Err(error) = containers
                        .execute_background(
                            citadel_primitives::ActorId::new(citadel_identity::SYSTEM_ACTOR_ID),
                            vec![id.to_string()],
                            ContainerAction::Delete(DeleteContainerOptions::default()),
                            token.child_token(),
                        )
                        .await
                    {
                        tracing::warn!(%error, %id, "Historical Swarm task pruning failed");
                    }
                }
            }
            Err(error) => tracing::warn!(%error, "Swarm task pruning lookup failed"),
        }
    }
}

#[cfg(test)]
mod tests;
