//! SwarmTaskContainerPruner: use the normal node-aware mutation path, never force
//! deletion or remove volumes. Claims recheck state after the candidate query.
use citadel_platforms::containers::{
    ContainerAction, ContainerMutationService, DeleteContainerOptions,
};
use sqlx::PgPool;
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

pub(super) async fn run(
    token: CancellationToken,
    pool: PgPool,
    containers: Arc<ContainerMutationService>,
    events: sqlx::postgres::PgListener,
) -> Result<(), std::convert::Infallible> {
    let mut tick = tokio::time::interval(Duration::from_secs(10));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut listener = Some(events);
    loop {
        if listener.is_none() {
            match super::listener(&pool, "citadel_swarm_prune").await {
                Ok(value) => listener = Some(value),
                Err(error) => tracing::warn!(%error, "Swarm prune event subscription failed"),
            }
        }
        tokio::select! {
            ()=token.cancelled()=>return Ok(()),
            _=tick.tick()=>{},
            event=async { match listener.as_mut() { Some(l)=>l.recv().await.map(|_|()), None=>std::future::pending().await } } => {
                if let Err(error) = event { tracing::warn!(%error, "Swarm prune event listener failed"); listener=None; }
            }
        }
        let _iteration = citadel_application::runtime_metrics::RuntimeWork::Pruning.start();
        let candidates: Result<Vec<uuid::Uuid>, _> = sqlx::query_scalar("SELECT c.id FROM containers c JOIN platforms p ON p.id=c.platformid WHERE p.prunehistoricalswarmtaskcontainers AND lower(p.platformdescriptor->>'$type')='dockerswarm' AND p.status='Online' AND c.isswarmtask AND NOT c.issystem AND lower(c.state) IN('exited','dead') AND c.controlstate='Idle' AND c.projectionstalesince IS NULL ORDER BY c.updated,c.id LIMIT 100")
            .fetch_all(&pool).await;
        match candidates {
            Ok(ids) => {
                for id in ids {
                    if token.is_cancelled() {
                        return Ok(());
                    }
                    if let Err(error) = containers
                        .execute_background(
                            citadel_domain::ActorId::new(citadel_identity::SYSTEM_ACTOR_ID),
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
#[path = "pruning_tests.rs"]
mod tests;
