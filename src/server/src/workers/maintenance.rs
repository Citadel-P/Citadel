use crate::realtime::RealtimeHub;
use sqlx::PgPool;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub(super) async fn run(
    token: CancellationToken,
    pool: PgPool,
    realtime: Option<RealtimeHub>,
    build_retention_days: Option<i32>,
    retention_interval: Duration,
) -> Result<(), std::convert::Infallible> {
    // Startup recovery already ran under the exclusive Core lease. Historical
    // retention is independent: its transactions cannot delay operation recovery.
    let recovery = async {
        let mut tick = super::schedule::interval("recovery", super::recovery::FALLBACK);
        loop {
            tokio::select! { ()=token.cancelled()=>break, _=tick.tick()=>{} }
            match citadel_adapters::persistence::postgres::maintenance::reconcile(&pool).await {
                Ok(changed) => {
                    if let Some(hub) = &realtime {
                        for resource in changed {
                            hub.publish_resource_change(resource, uuid::Uuid::nil(), "reconciled");
                        }
                    }
                }
                Err(error) => tracing::warn!(%error, "Resource reconciliation failed"),
            }
        }
    };
    let expiry = async {
        let mut tick = super::schedule::interval("lease-expiry", Duration::from_secs(30));
        loop {
            tokio::select! { ()=token.cancelled()=>break, _=tick.tick()=>{} }
            if let Err(error) =
                citadel_adapters::persistence::postgres::maintenance::expire_leases(&pool).await
            {
                tracing::warn!(%error, "Lease expiry failed");
            }
        }
    };
    let retention = async {
        let mut tick = super::schedule::interval("retention", retention_interval);
        loop {
            tokio::select! { ()=token.cancelled()=>break, _=tick.tick()=>{} }
            let drain = async {
                // Drain backlogs with short commits, at most 128 passes / ten seconds.
                // One pass deletes at most 5,000 rows per historical table (1,000 builds).
                for _ in 0..128 {
                    if citadel_adapters::persistence::postgres::maintenance::cleanup(
                        &pool,
                        build_retention_days,
                    )
                    .await?
                        == 0
                    {
                        break;
                    }
                }
                Ok::<_, sqlx::Error>(())
            };
            tokio::select! {
                () = token.cancelled() => break,
                result = tokio::time::timeout(Duration::from_secs(10), drain) => match result {
                    Ok(Ok(())) => {},
                    Ok(Err(error)) => tracing::warn!(%error, "Scheduled retention failed"),
                    Err(_) => tracing::info!("Retention budget exhausted; remaining rows wait for the next maintenance pass"),
                }
            }
        }
    };
    tokio::join!(recovery, expiry, retention);
    Ok(())
}
