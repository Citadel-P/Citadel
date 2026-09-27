//! UnmanagedContainerAlertJob: grace, deduplication, then re-read ownership.
use citadel_alerts::{AlertEventSink, AlertObservation};
use serde::Deserialize;
use sqlx::{PgPool, Row};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Hash, Eq, PartialEq)]
struct Request {
    platform: Uuid,
    container: String,
    node: Option<String>,
}

pub(super) async fn run(
    token: CancellationToken,
    pool: PgPool,
    alerts: Arc<dyn AlertEventSink>,
    mut listener: sqlx::postgres::PgListener,
) -> Result<(), std::convert::Infallible> {
    let mut pending = HashMap::<Request, Instant>::new();
    loop {
        if token.is_cancelled() {
            return Ok(());
        }
        if let Err(error) =
            listen(&token, &pool, alerts.as_ref(), &mut pending, &mut listener).await
        {
            tracing::warn!(%error, "Unmanaged container alert listener failed");
        }
        tokio::select! { ()=token.cancelled()=>return Ok(()), _=tokio::time::sleep(Duration::from_secs(2))=>{} }
    }
}

async fn listen(
    token: &CancellationToken,
    pool: &PgPool,
    alerts: &dyn AlertEventSink,
    pending: &mut HashMap<Request, Instant>,
    listener: &mut sqlx::postgres::PgListener,
) -> Result<(), sqlx::Error> {
    loop {
        let deadline = pending.values().copied().min();
        tokio::select! {
            ()=token.cancelled()=>return Ok(()),
            notification=listener.recv()=> {
                let notification = notification?;
                if let Ok(mut request) = serde_json::from_str::<Request>(notification.payload()) {
                    schedule(pending, &mut request, Instant::now());
                }
            }
            ()=async {
                // No periodic wakeup when the grace queue is empty.
                match deadline {
                    Some(deadline) => tokio::time::sleep_until(deadline).await,
                    None => std::future::pending().await,
                }
            }=> {
                let due: Vec<_> = pending.iter().filter(|(_, due)| **due <= Instant::now()).map(|(request,_)|request.clone()).take(256).collect();
                for request in due {
                    match observe(pool, alerts, &request).await {
                        Ok(())=>{pending.remove(&request);},
                        Err(error)=>{ tracing::warn!(%error, "Unmanaged container Alert evaluation failed"); pending.insert(request, Instant::now()+Duration::from_secs(30)); }
                    }
                }
            }
        }
    }
}

async fn observe(
    pool: &PgPool,
    alerts: &dyn AlertEventSink,
    request: &Request,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let row = sqlx::query("SELECT c.id,c.name,p.name platformname,p.address FROM containers c JOIN platforms p ON p.id=c.platformid WHERE c.platformid=$1 AND lower(c.dockercontainerid)=$2 AND c.dockernodeid IS NOT DISTINCT FROM $3 AND NOT c.issystem AND NOT c.isswarmtask AND c.deploymentid IS NULL AND c.stackid IS NULL")
        .bind(request.platform).bind(&request.container).bind(&request.node).fetch_optional(pool).await?;
    let Some(row) = row else {
        return Ok(());
    };
    let id: Uuid = row.try_get("id")?;
    let name: String = row.try_get("name")?;
    alerts
        .observe(&AlertObservation {
            alert_type: "UnmanagedContainerCreated".into(),
            info: serde_json::json!({"Id":id,"ContainerId":request.container,"ContainerName":name,
            "PlatformId":request.platform,"PlatformName":row.try_get::<String,_>("platformname")?,
            "PlatformAddress":row.try_get::<String,_>("address")?,
            "HumanMessage":format!("Unmanaged container '{name}' was created.")}),
            resource_id: request.platform,
            resource_name: row.try_get("platformname")?,
            resource_type: "Platform".into(),
            deduplication_component: request.container.clone(),
            observed_at: chrono::Utc::now(),
            value: None,
            matched: true,
        })
        .await?;
    Ok(())
}

fn schedule(pending: &mut HashMap<Request, Instant>, request: &mut Request, now: Instant) {
    request.container = request.container.trim().to_ascii_lowercase();
    pending.insert(request.clone(), now + Duration::from_secs(30));
}

#[cfg(test)]
mod tests;
