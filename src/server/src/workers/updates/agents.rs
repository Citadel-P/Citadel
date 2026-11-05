use citadel_updates::AgentUpdateService;
use tokio_util::sync::CancellationToken;

/// Owned by Core's existing supervisor and single-database background lease.
pub async fn run_agents(
    service: AgentUpdateService,
    cancellation: CancellationToken,
) -> Result<(), std::convert::Infallible> {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {biased;()=cancellation.cancelled()=>return Ok(()),_=interval.tick()=>{}}
        // Let a claimed attempt settle; shutdown never detaches its cleanup.
        if let Err(error) = service.tick(&cancellation).await {
            tracing::warn!(%error,"Agent update worker could not finish its current check");
        }
    }
}
