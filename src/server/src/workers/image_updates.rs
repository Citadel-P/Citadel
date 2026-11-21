//! Read-only registry checks owned by Core's single background-job lease.
use citadel_platforms::image_updates::ImageUpdateService;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub(crate) async fn sweep(
    cancel: CancellationToken,
    service: Arc<ImageUpdateService>,
) -> Result<(), String> {
    let mut failures = 0;
    let mut after = Uuid::nil();
    loop {
        let observation = citadel_runtime::jobs::JobHandle::iteration();
        let result = tokio::select! { biased; () = cancel.cancelled() => return Ok(()), result = service.run_scheduled_batch(after, &cancel) => result };
        match result {
            Ok(batch) => {
                if let Some(i) = observation {
                    i.finish(
                        if batch.failed > 0 {
                            citadel_runtime::jobs::JobOutcome::Failed
                        } else if batch.checked == 0 {
                            citadel_runtime::jobs::JobOutcome::NoWork
                        } else {
                            citadel_runtime::jobs::JobOutcome::Productive
                        },
                        Some((batch.checked as u64, "image checks")),
                    );
                }
                failures += batch.failed;
                if batch.failed > 0 {
                    tracing::warn!(
                        failed = batch.failed,
                        "Some inventory image checks could not complete"
                    );
                }
                if batch.checked > 0 {
                    tracing::debug!(checked = batch.checked, "Inventory image checks completed");
                }
                match batch.next {
                    Some(next) => after = next,
                    None => break,
                }
            }
            Err(error) => {
                if let Some(i) = observation {
                    i.finish(citadel_runtime::jobs::JobOutcome::Failed, None);
                }
                return Err(error.to_string());
            }
        }
    }
    if failures > 0 {
        return Err(format!("{failures} inventory image checks failed"));
    }
    Ok(())
}
