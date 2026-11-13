//! One supervised first-install task; no recurring registration or HTTP-triggered work.
use citadel_identity::IdentityService;
use citadel_updates::UpdateChangeNotifier;
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

pub async fn run_automatic_setup(
    registration: Arc<dyn citadel_updates::UpdateRegistrationRuntime>,
    identity: Arc<IdentityService>,
    realtime: Option<crate::realtime::RealtimeHub>,
    cancellation: CancellationToken,
) -> Result<(), std::convert::Infallible> {
    let mut changes = identity.setup_changes();
    loop {
        let initialized = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Ok(()),
            status = identity.setup_status() => status.is_ok_and(|s| !s.requires_setup),
        };
        if initialized {
            break;
        }
        tokio::select! {
            () = cancellation.cancelled() => return Ok(()),
            result = changes.changed() => if result.is_err() { return Ok(()); },
            () = tokio::time::sleep(Duration::from_secs(30)) => {},
        }
    }
    // Docker health can lag HTTP setup. Bounded retries also cover brief daemon
    // unavailability, never replacement or a change to persisted setup inputs.
    for attempt in 0..6 {
        if cancellation.is_cancelled() {
            return Ok(());
        }
        match registration.register(&cancellation).await {
            Ok(()) => {
                crate::realtime::notifiers::UpdatesRealtimeNotifier::new(realtime).changed();
                tracing::info!(
                    "Automatic setup completed; Core and PostgreSQL in-app updates are configured"
                );
                cancellation.cancelled().await;
                return Ok(());
            }
            Err(error) if !cancellation.is_cancelled() => {
                tracing::warn!(%error, attempt, "Automatic update setup could not complete; installation files and data were preserved")
            }
            _ => {}
        }
        tokio::select! { () = cancellation.cancelled() => return Ok(()), () = tokio::time::sleep(Duration::from_secs(30)) => {} }
    }
    tracing::warn!(
        "Automatic update setup stopped after bounded retries. Check installation permissions and configuration, then restart Core to retry."
    );
    // Completed workers must remain owned until shutdown (supervisor contract).
    cancellation.cancelled().await;
    Ok(())
}
