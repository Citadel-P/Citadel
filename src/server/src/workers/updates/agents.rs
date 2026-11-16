use citadel_updates::AgentUpdateService;
use tokio_util::sync::CancellationToken;

/// Owned by Core's existing supervisor and single-database background lease.
pub async fn run_agents(
    service: AgentUpdateService,
    cancellation: CancellationToken,
) -> Result<(), std::convert::Infallible> {
    let mut wake = service.work_changes();
    loop {
        if cancellation.is_cancelled() {
            return Ok(());
        }
        // Let a claimed attempt settle; shutdown never detaches its cleanup.
        let delay = match citadel_runtime::jobs::observe(service.poll(&cancellation)).await {
            Ok(delay) => delay,
            Err(error) => {
                tracing::warn!(%error,"Agent update worker could not finish its current check");
                // Errors must not create a notification-driven retry loop.
                tokio::select! { ()=cancellation.cancelled()=>return Ok(()), ()=tokio::time::sleep(std::time::Duration::from_secs(5))=>{} }
                continue;
            }
        };
        super::super::notifications::wait(&mut wake, &cancellation, delay).await;
    }
}
