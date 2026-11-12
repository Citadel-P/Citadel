//! SwarmTaskContainerPruner: use the normal node-aware mutation path, never force
//! deletion or remove volumes. Claims recheck state after the candidate query.
use citadel_platforms::containers::{ContainerAction, ContainerMutationService};

use citadel_platforms::containers::DeleteContainerOptions;

use citadel_platforms::jobs::background::ContainerMaintenanceReader;

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

pub(crate) async fn run(
    admission: citadel_runtime::admission::AdmissionGate,
    token: CancellationToken,
    store: Arc<dyn ContainerMaintenanceReader>,
    containers: Arc<ContainerMutationService>,
    mut wake: tokio::sync::watch::Receiver<()>,
) -> Result<(), std::convert::Infallible> {
    let mut tick = super::super::schedule::interval("pruning", super::super::recovery::FALLBACK);
    let mut startup = true;
    let mut wake_open = true;
    loop {
        if token.is_cancelled() {
            return Ok(());
        }
        if !std::mem::take(&mut startup) {
            tokio::select! {
                ()=token.cancelled()=>return Ok(()),
                _=tick.tick()=>{},
                changed=wake.changed(), if wake_open => {
                    if changed.is_err() {
                        // A stopped hub must not cause a busy loop; the scheduled
                        // reconciliation remains authoritative for missed wakeups.
                        wake_open = false;
                        continue;
                    }
                }
            }
        }
        let Some(_admission) = admission.enter("Container pruning", &token).await else {
            return Ok(());
        };
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
