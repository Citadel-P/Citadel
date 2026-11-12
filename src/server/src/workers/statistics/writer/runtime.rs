use super::{Envelope, WriterSettings};
use citadel_platforms::stats_ingestion::StatsBatchStore;
use citadel_runtime::runtime_metrics::RuntimeWork;
use std::{sync::Arc, time::Duration};
use tokio::{sync::mpsc, time::Instant};
use tokio_util::sync::CancellationToken;

pub async fn run<T: Send + Sync + 'static>(
    mut receiver: mpsc::Receiver<Envelope<T>>,
    store: Arc<dyn StatsBatchStore<T>>,
    cancel: CancellationToken,
    settings: WriterSettings,
    ingress: RuntimeWork,
    flush: RuntimeWork,
    stale: RuntimeWork,
) -> Result<(), std::convert::Infallible> {
    struct Pending<T> {
        key: uuid::Uuid,
        combined: bool,
        rows: Vec<Envelope<T>>,
        at: Instant,
        expires: Instant,
        delay: Duration,
    }
    let limit = settings.batch_size.max(1);
    // Retain at most four batches in addition to the bounded producer queue.
    // Telemetry older than two minutes is explicitly expired, never retried forever.
    let mut pending = std::collections::VecDeque::<Pending<T>>::new();
    let mut batch = Vec::<Envelope<T>>::new();
    let mut batch_bytes = 0usize;
    let mut deadline = None;
    let mut carry = None::<Envelope<T>>;
    let mut shutdown = None;
    let mut closed = false;
    loop {
        if cancel.is_cancelled() && shutdown.is_none() {
            receiver.close();
            shutdown = Some(Instant::now() + settings.shutdown_timeout);
        }
        if shutdown.is_some_and(|end| Instant::now() >= end) {
            break;
        }
        if batch.is_empty()
            && let Some(row) = carry.take()
        {
            batch_bytes = batch_bytes.saturating_add(row.encoded);
            batch.push(row);
            deadline = Some(Instant::now() + settings.flush_interval);
        }
        if shutdown.is_some() {
            while batch.len() < limit && carry.is_none() {
                match receiver.try_recv() {
                    Ok(row) => {
                        if batch_bytes.saturating_add(row.encoded)
                            > super::super::budget::BATCH_BYTES
                            && !batch.is_empty()
                        {
                            carry = Some(row);
                            break;
                        }
                        batch_bytes = batch_bytes.saturating_add(row.encoded);
                        batch.push(row);
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                        closed = true;
                        break;
                    }
                    Err(_) => break,
                }
            }
        }
        let due = deadline.is_some_and(|end| Instant::now() >= end);
        if !batch.is_empty()
            && (batch.len() >= limit || carry.is_some() || due || closed || shutdown.is_some())
        {
            let combined = pending.is_empty();
            let mut groups = std::collections::BTreeMap::<uuid::Uuid, Vec<Envelope<T>>>::new();
            for row in std::mem::take(&mut batch) {
                let key = if combined {
                    uuid::Uuid::nil()
                } else {
                    store.partition(&row)
                };
                groups.entry(key).or_default().push(row);
            }
            for (key, rows) in groups {
                pending.push_back(Pending {
                    key,
                    combined,
                    rows,
                    at: Instant::now(),
                    expires: Instant::now() + Duration::from_secs(120),
                    delay: Duration::from_secs(1),
                });
            }
            deadline = None;
            batch_bytes = 0;
        }
        let now = Instant::now();
        // The first outstanding batch for a platform always precedes its later batches.
        let mut seen = std::collections::BTreeSet::new();
        let ready = pending.iter().position(|p| {
            seen.insert(p.key) && (p.at <= now || p.expires <= now || shutdown.is_some())
        });
        if let Some(index) = ready {
            let mut work = pending.remove(index).unwrap();
            flush.age(now.saturating_duration_since(work.expires - Duration::from_secs(120)));
            if work.expires <= now {
                RuntimeWork::StatsRetentionDrop.units(work.rows.len() as u64);
                tracing::warn!(
                    samples = work.rows.len(),
                    "Expired retained statistics after two minutes"
                );
                continue;
            }
            let (rows, ownership): (Vec<_>, Vec<_>) = work
                .rows
                .drain(..)
                .map(|row| (row.value, (row.lease, row.encoded)))
                .unzip();
            let result = tokio::select! {
                biased;
                () = cancel.cancelled(), if shutdown.is_none() => None,
                () = until(shutdown) => None,
                result = async { let _timer = flush.start(); tokio::time::timeout(Duration::from_secs(10), store.persist_batch(&rows)).await } => Some(result.unwrap_or_else(|_| Err(citadel_platforms::RuntimeCapabilityError::new(citadel_platforms::RuntimeErrorKind::Timeout, "Statistics flush timed out", true)))),
            };
            work.rows = rows
                .into_iter()
                .zip(ownership)
                .map(|(sample, (lease, encoded))| Envelope {
                    value: sample,
                    lease,
                    encoded,
                })
                .collect();
            let Some(result) = result else {
                pending.insert(index, work);
                continue;
            };
            match result {
                Ok(outcome) => {
                    flush.success();
                    flush.units(outcome.persisted as u64);
                    stale.units(outcome.stale as u64);
                }
                Err(error) if work.combined => {
                    // Preserve cross-platform batching on success; isolate failures only.
                    flush.failures(1);
                    RuntimeWork::StatsRetry.units(1);
                    let mut groups =
                        std::collections::BTreeMap::<uuid::Uuid, Vec<Envelope<T>>>::new();
                    for row in work.rows {
                        groups.entry(store.partition(&row)).or_default().push(row);
                    }
                    let split = groups.len() > 1;
                    for (key, rows) in groups.into_iter().rev() {
                        pending.insert(
                            index,
                            Pending {
                                key,
                                rows,
                                combined: false,
                                at: Instant::now()
                                    + if split { Duration::ZERO } else { work.delay },
                                expires: work.expires,
                                delay: work.delay * 2,
                            },
                        );
                    }
                    tracing::warn!(%error, "Statistics failure retained by platform");
                }
                Err(error) if !error.retryable => {
                    flush.failures(1);
                    RuntimeWork::StatsRejected.units(work.rows.len() as u64);
                    tracing::error!(%error, samples=work.rows.len(), "Rejected permanent statistics write failure");
                }
                Err(error) => {
                    flush.failures(1);
                    RuntimeWork::StatsRetry.units(1);
                    tracing::warn!(%error, "Statistics partition retained; other platforms may continue");
                    work.at = Instant::now() + work.delay;
                    work.delay = (work.delay * 2).min(Duration::from_secs(30));
                    pending.insert(index, work);
                    // Shutdown retries still wait instead of spinning on a failed store.
                    if shutdown.is_some() {
                        tokio::select! { ()=until(shutdown)=>break, _=tokio::time::sleep(Duration::from_millis(100))=>{} }
                    }
                }
            }
            // Receive available rows before choosing another due retry.
            while batch.len() < limit
                && carry.is_none()
                && pending.iter().map(|p| p.rows.len()).sum::<usize>() + batch.len()
                    < limit.saturating_mul(4)
            {
                match receiver.try_recv() {
                    Ok(row) => {
                        if batch_bytes.saturating_add(row.encoded)
                            > super::super::budget::BATCH_BYTES
                            && !batch.is_empty()
                        {
                            carry = Some(row);
                            break;
                        }
                        batch_bytes = batch_bytes.saturating_add(row.encoded);
                        batch.push(row);
                        deadline.get_or_insert(Instant::now() + settings.flush_interval);
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                        closed = true;
                        break;
                    }
                    Err(_) => break,
                }
            }
            continue;
        }
        if batch.is_empty() && carry.is_some() {
            continue;
        }
        if closed && pending.is_empty() && batch.is_empty() {
            break;
        }
        let retained = pending.iter().map(|p| p.rows.len()).sum::<usize>() + batch.len();
        flush.buffered(retained);
        ingress.queued(receiver.len());
        let mut seen = std::collections::BTreeSet::new();
        let next_retry = pending
            .iter()
            .filter(|p| seen.insert(p.key))
            .map(|p| p.at.min(p.expires))
            .min();
        tokio::select! {
            biased;
            ()=cancel.cancelled(), if shutdown.is_none()=>{},
            ()=until(shutdown)=>break,
            ()=until(deadline), if !batch.is_empty()=>{},
            ()=until(next_retry)=>{},
            row=receiver.recv(), if !closed=>match row {
                Some(row)=>{
                    if retained >= limit.saturating_mul(4) && let Some(oldest) = pending.pop_front() {
                        RuntimeWork::StatsRetentionDrop.units(oldest.rows.len() as u64);
                    }
                    if batch_bytes.saturating_add(row.encoded) > super::super::budget::BATCH_BYTES && !batch.is_empty() {
                        carry = Some(row);
                    } else {
                        batch_bytes = batch_bytes.saturating_add(row.encoded);
                        batch.push(row);
                    }
                    deadline.get_or_insert(Instant::now()+settings.flush_interval); },
                None=>closed=true,
            },
        }
    }
    RuntimeWork::StatsShutdownDrop.units(
        (batch.len()
            + usize::from(carry.is_some())
            + receiver.len()
            + pending.iter().map(|p| p.rows.len()).sum::<usize>()) as u64,
    );
    flush.buffered(0);
    ingress.queued(0);
    Ok(())
}

async fn until(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,
        None => std::future::pending().await,
    }
}
