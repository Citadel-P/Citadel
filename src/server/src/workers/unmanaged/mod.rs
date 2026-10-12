//! UnmanagedContainerAlertJob: grace, deduplication, then re-read ownership.
use citadel_alerts::{AlertEventSink, AlertObservation};
use citadel_platforms::jobs::background::ContainerMaintenanceReader;
use citadel_runtime::notifications::{NotificationError, NotificationSource, RuntimeNotification};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
#[cfg(test)]
use uuid::Uuid;

use citadel_runtime::notifications::ContainerCreated as Request;

pub(crate) async fn run(
    token: CancellationToken,
    store: Arc<dyn ContainerMaintenanceReader>,
    alerts: Arc<dyn AlertEventSink>,
    mut listener: Box<dyn NotificationSource>,
) -> Result<(), std::convert::Infallible> {
    let mut pending = HashMap::<Request, Instant>::new();
    loop {
        if token.is_cancelled() {
            return Ok(());
        }
        if let Err(error) = listen(
            &token,
            store.as_ref(),
            alerts.as_ref(),
            &mut pending,
            &mut listener,
        )
        .await
        {
            tracing::warn!(%error, "Unmanaged container alert listener failed");
        }
        tokio::select! { ()=token.cancelled()=>return Ok(()), _=tokio::time::sleep(Duration::from_secs(2))=>{} }
    }
}

async fn listen(
    token: &CancellationToken,
    store: &dyn ContainerMaintenanceReader,
    alerts: &dyn AlertEventSink,
    pending: &mut HashMap<Request, Instant>,
    listener: &mut Box<dyn NotificationSource>,
) -> Result<(), NotificationError> {
    loop {
        let deadline = pending.values().copied().min();
        tokio::select! {
            ()=token.cancelled()=>return Ok(()),
            notification=listener.recv()=> {
                let notification = notification?;
                if let RuntimeNotification::ContainerCreated(mut request) = notification {
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
                    match observe(store, alerts, &request).await {
                        Ok(())=>{pending.remove(&request);},
                        Err(error)=>{ tracing::warn!(%error, "Unmanaged container Alert evaluation failed"); pending.insert(request, Instant::now()+Duration::from_secs(30)); }
                    }
                }
            }
        }
    }
}

async fn observe(
    store: &dyn ContainerMaintenanceReader,
    alerts: &dyn AlertEventSink,
    request: &Request,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let Some(container) = store
        .unmanaged_container(
            request.platform,
            &request.container,
            request.node.as_deref(),
        )
        .await?
    else {
        return Ok(());
    };
    let id = container.id;
    let name = container.name;
    alerts
        .observe(&AlertObservation {
            alert_type: "UnmanagedContainerCreated".into(),
            info: serde_json::json!({"Id":id,"ContainerId":request.container,"ContainerName":name,
            "PlatformId":request.platform,"PlatformName":container.platform_name,
            "PlatformAddress":container.platform_address,
            "HumanMessage":format!("Unmanaged container '{name}' was created.")}),
            resource_id: request.platform,
            resource_name: container.platform_name,
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
