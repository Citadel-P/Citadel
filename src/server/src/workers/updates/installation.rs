use citadel_updates::{UpdateInstallCommand, UpdateInstallService};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub async fn run_installation(
    service: UpdateInstallService,
    mut commands: mpsc::Receiver<UpdateInstallCommand>,
    cancellation: CancellationToken,
) -> Result<(), std::convert::Infallible> {
    loop {
        let command = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Ok(()),
            command = commands.recv() => match command { Some(command) => command, None => return Ok(()) },
        };
        // Await cancellation cleanup; never drop a claimed operation future.
        if let Err(error) = service.execute(command, &cancellation).await {
            tracing::error!(%error, "Citadel update stopped; retained maintenance and journal require recovery");
        }
    }
}
