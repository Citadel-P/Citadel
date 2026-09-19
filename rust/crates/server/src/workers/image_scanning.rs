use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

pub async fn run(
    cancel: CancellationToken,
    scanner: Arc<citadel_adapters::image_scanner::ImageScanner>,
    deployments: Arc<citadel_deployments::DeploymentService>,
    stacks: Arc<citadel_stacks::StackService>,
    services: Arc<citadel_swarm_services::SwarmServiceService>,
) -> Result<(), std::convert::Infallible> {
    let mut ticker = tokio::time::interval_at(
        tokio::time::Instant::now() + Duration::from_millis(5733),
        Duration::from_secs(90 * 60),
    );
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut first = true;
    loop {
        tokio::select! {biased;()=cancel.cancelled()=>return Ok(()),_=ticker.tick()=>{}}
        let _iteration = citadel_application::runtime_metrics::RuntimeWork::ImageScan.start();
        match scanner.run_cycle(&cancel).await {
            Ok(count) => tracing::debug!(count, "Registry image scan completed"),
            Err(error) => {
                tracing::warn!(%error,"Registry image scan failed");
                continue;
            }
        }
        if first {
            first = false;
            // Initial consumers wait for the scanner's first complete pass;
            // subsequent checks retain their independent two-hour cadence.
            if let Err(error) = deployments.run_scheduled_update_checks(&cancel).await {
                tracing::warn!(%error,"Initial Deployment update check failed");
            }
            if let Err(error) = stacks.run_update_checks(true, &cancel).await {
                tracing::warn!(%error,"Initial Stack update check failed");
            }
            if let Err(error) = services.run_scheduled_update_checks(&cancel).await {
                tracing::warn!(%error,"Initial Service update check failed");
            }
        }
    }
}
