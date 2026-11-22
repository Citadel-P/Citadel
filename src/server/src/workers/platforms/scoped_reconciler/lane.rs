use super::*;

pub(crate) async fn run_scope<S, E, C, CF, P, PF>(
    cancel: &CancellationToken,
    receiver: DirtyReceiver<uuid::Uuid>,
    refresh: EventRefresh,
    concurrency: usize,
    retry: Duration,
    collect: &C,
    persist: &P,
) where
    E: std::fmt::Display,
    C: Fn(ScopedEventRequest) -> CF,
    CF: Future<Output = Result<Option<S>, E>>,
    P: Fn(ScopedEventRequest, S) -> PF,
    PF: Future<Output = Result<bool, E>>,
{
    let mut reads = futures_util::stream::FuturesUnordered::new();
    let queue = &receiver;
    loop {
        tokio::select! {
            biased;
            () = cancel.cancelled() => return,
            Some((work, result)) = reads.next(), if !reads.is_empty() => {
                let work: citadel_runtime::dirty_queue::DirtyWork<uuid::Uuid> = work;
                let committed = matches!(result, Ok(true));
                let delay = if let Err(error) = result {
                    refresh_metric(refresh).failures(1);
                    tracing::warn!(%error, platform_id=%work.key, scope=?refresh, "Resource reconciliation failed");
                    Some(retry.max(Duration::from_millis(250)))
                } else { None };
                if !queue.finish(work, committed, delay) && delay.is_none() {
                    outcome_metric(refresh, false).units(1);
                }
            }
            work = queue.next(), if reads.len() < concurrency => {
                if work.retry {
                    outcome_metric(refresh, true).units(1);
                }
                let request = ScopedEventRequest { platform_id: work.key, refresh };
                reads.push(async move {
                    let observation = citadel_runtime::jobs::JobHandle::iteration().map(|i| i.failure_context(
                        "Platform inventory could not be refreshed. Displayed resources may be out of date.",
                        citadel_runtime::jobs::JobTarget {
                            resource_type: "Platform".into(),
                            resource_id: work.key,
                            operation_id: None,
                            container_id: None,
                        },
                    ));
                    let collected = collect(request).await;
                    let result = if queue.current(work) {
                        match collected {
                            Ok(Some(snapshot)) => persist(request, snapshot).await,
                            Ok(None) => Ok(true),
                            Err(error) => Err(error),
                        }
                    } else {
                        Ok(false)
                    };
                    if let Some(observation) = observation {
                        observation.result(&result);
                    }
                    (work, result)
                });
            }
        }
    }
}

fn outcome_metric(refresh: EventRefresh, retry: bool) -> RuntimeWork {
    use citadel_platforms::jobs::ReconciliationScope;
    match (refresh, retry) {
        (EventRefresh::Platform, true) => RuntimeWork::PlatformRefreshRetry,
        (EventRefresh::Platform, false) => RuntimeWork::PlatformRefreshDiscarded,
        (EventRefresh::Resource(ReconciliationScope::Containers), true) => {
            RuntimeWork::ContainerRefreshRetry
        }
        (EventRefresh::Resource(ReconciliationScope::Containers), false) => {
            RuntimeWork::ContainerRefreshDiscarded
        }
        (EventRefresh::Resource(ReconciliationScope::Images), true) => {
            RuntimeWork::ImageRefreshRetry
        }
        (EventRefresh::Resource(ReconciliationScope::Images), false) => {
            RuntimeWork::ImageRefreshDiscarded
        }
        (EventRefresh::Resource(ReconciliationScope::Networks), true) => {
            RuntimeWork::NetworkRefreshRetry
        }
        (EventRefresh::Resource(ReconciliationScope::Networks), false) => {
            RuntimeWork::NetworkRefreshDiscarded
        }
        (EventRefresh::Resource(ReconciliationScope::Volumes), true) => {
            RuntimeWork::VolumeRefreshRetry
        }
        (EventRefresh::Resource(ReconciliationScope::Volumes), false) => {
            RuntimeWork::VolumeRefreshDiscarded
        }
        (EventRefresh::Swarm, true) => RuntimeWork::SwarmRefreshRetry,
        (EventRefresh::Swarm, false) => RuntimeWork::SwarmRefreshDiscarded,
    }
}
