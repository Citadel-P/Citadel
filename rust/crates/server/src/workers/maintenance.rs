use crate::realtime::RealtimeHub;
use sqlx::PgPool;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub(super) async fn run(
    token: CancellationToken,
    pool: PgPool,
    realtime: Option<RealtimeHub>,
    build_retention_days: Option<i32>,
) -> Result<(), std::convert::Infallible> {
    let mut tick = tokio::time::interval(Duration::from_secs(60));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! { ()=token.cancelled()=>return Ok(()), _=tick.tick()=>{} }
        match citadel_adapters::maintenance_store::reconcile(&pool).await {
            Ok(changed) => {
                if let Some(hub) = &realtime {
                    for resource in changed {
                        hub.publish_resource_change(resource, uuid::Uuid::nil(), "reconciled");
                    }
                }
            }
            Err(error) => tracing::warn!(%error, "Resource reconciliation failed"),
        }
        if let Err(error) =
            citadel_adapters::maintenance_store::cleanup(&pool, build_retention_days).await
        {
            tracing::warn!(%error, "Scheduled retention failed");
        }
    }
}
