//! Best-effort, bounded diagnostic persistence. Never participates in execution admission.
use citadel_instance::{
    JobDiagnosticSource, JobHistoryBatch, JobHistoryBucket, JobHistoryEvent, JobHistoryRepository,
    JobHistorySession, JobsSnapshot, history_buckets,
};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub async fn run(
    source: Arc<dyn JobDiagnosticSource>,
    store: Arc<dyn JobHistoryRepository>,
    core_id: Uuid,
    version: String,
    cancel: CancellationToken,
) -> Result<(), citadel_instance::InstanceError> {
    let mut tick = tokio::time::interval(Duration::from_secs(30));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut previous: Option<JobsSnapshot> = None;
    let mut through = 0;
    let mut lost = 0;
    let mut windows: BTreeMap<(String, u64), JobHistoryBucket> = BTreeMap::new();
    let mut pending = None;
    let mut last_checkpoint = 0;
    let mut sequence = 1;
    loop {
        let stopping = tokio::select! { () = cancel.cancelled() => true, _ = tick.tick() => false };
        // Retrying the identical batch makes ambiguous commit recovery idempotent.
        if pending.is_none() {
            let capture = source.capture(through);
            let snapshot = capture.snapshot;
            let checkpoint =
                stopping || last_checkpoint / 300_000 != snapshot.observed_at / 300_000;
            for delta in history_buckets(previous.as_ref(), &snapshot) {
                let key = (delta.job_id.clone(), delta.at);
                if let Some(w) = windows.get_mut(&key) {
                    merge(w, &delta);
                } else {
                    windows.insert(key, delta);
                }
            }
            let mut events = capture.events;
            if capture.lost_events > 0 {
                events.insert(
                    0,
                    JobHistoryEvent {
                        sequence: events.first().map_or(capture.through, |e| e.sequence - 1),
                        job_id: String::new(),
                        at: snapshot.observed_at,
                        kind: "observationsLost".into(),
                        problem: None,
                    },
                );
            }
            if previous.is_none() {
                events.insert(
                    0,
                    JobHistoryEvent {
                        sequence: 0,
                        job_id: String::new(),
                        at: snapshot.started_at,
                        kind: "coreStarted".into(),
                        problem: None,
                    },
                );
            }
            pending = Some((
                JobHistoryBatch {
                    sequence,
                    session: JobHistorySession {
                        core_id,
                        version: version.clone(),
                        heartbeat_at: snapshot.observed_at,
                        stopped_at: stopping.then_some(snapshot.observed_at),
                        lost_events: lost + capture.lost_events,
                        snapshot,
                    },
                    events,
                    buckets: if checkpoint {
                        windows.values().cloned().collect()
                    } else {
                        Vec::new()
                    },
                    checkpoint,
                },
                capture.through,
            ));
        }
        let (batch, next_through) = pending.as_ref().expect("captured batch");
        let iteration = citadel_runtime::jobs::JobHandle::iteration();
        let result = tokio::time::timeout(Duration::from_secs(5), store.persist(batch)).await;
        if let Some(iteration) = iteration {
            iteration.finish(
                if matches!(&result, Ok(Ok(()))) {
                    citadel_runtime::jobs::JobOutcome::Completed
                } else {
                    citadel_runtime::jobs::JobOutcome::Failed
                },
                None,
            );
        }
        match result {
            Ok(Ok(())) => {
                if batch.checkpoint {
                    last_checkpoint = batch.session.heartbeat_at;
                    windows.retain(|(_, at), _| *at == last_checkpoint / 300_000 * 300_000);
                }
                lost = batch.session.lost_events;
                through = *next_through;
                let stopped = batch.session.stopped_at.is_some();
                previous = Some(batch.session.snapshot.clone());
                pending = None;
                sequence += 1;
                source.committed();
                if stopping && !stopped {
                    continue;
                }
            }
            _ => tracing::warn!(
                "Job diagnostic persistence unavailable; workers continue, buffered transitions may be lost"
            ),
        }
        if stopping {
            break;
        }
        if let Some(job) = citadel_runtime::jobs::JobHandle::current() {
            job.policy(
                "Diagnostics flush every 30 seconds; checkpoints every five minutes",
                None,
            );
            job.waiting(
                "Next diagnostic flush",
                Some(tokio::time::Instant::now() + Duration::from_secs(30)),
            );
        }
    }
    Ok(())
}
fn merge(target: &mut JobHistoryBucket, delta: &JobHistoryBucket) {
    target.through = delta.through;
    target.iterations += delta.iterations;
    target.productive += delta.productive;
    target.no_work += delta.no_work;
    target.failures += delta.failures;
    target.elapsed_ms += delta.elapsed_ms;
    target.productive_ms += delta.productive_ms;
    target.no_work_ms += delta.no_work_ms;
    target.units = target.units.zip(delta.units).map(|(a, b)| a + b);
    target.backlog = delta.backlog;
    target.backlog_is_lower_bound = delta.backlog_is_lower_bound;
    target.gap |= delta.gap;
}
