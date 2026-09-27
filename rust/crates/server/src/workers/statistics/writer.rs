//! One bounded queue and one retained retry batch per writer, across all sources.
use citadel_platforms::stats_ingestion::StatsBatchStore;
use citadel_runtime::runtime_metrics::RuntimeWork;
use std::{sync::Arc, time::Duration};
use tokio::{sync::mpsc, time::Instant};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy)]
pub struct WriterSettings {
    pub batch_size: usize,
    pub flush_interval: Duration,
    pub shutdown_timeout: Duration,
}

pub struct Ingress<T> {
    sender: mpsc::Sender<T>,
    metric: RuntimeWork,
}
impl<T> Clone for Ingress<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            metric: self.metric,
        }
    }
}
impl<T> Ingress<T> {
    pub async fn send(&self, sample: T, cancel: &CancellationToken) -> bool {
        let started = Instant::now();
        let saturated = self.sender.capacity() == 0;
        self.metric.permit_wait(Duration::ZERO, saturated);
        let permit = tokio::select! {
            biased;
            () = cancel.cancelled() => return false,
            permit = self.sender.reserve() => match permit { Ok(p) => p, Err(_) => return false },
        };
        permit.send(sample);
        self.metric.permit_wait(started.elapsed(), false);
        self.metric.units(1);
        self.metric
            .queued(self.sender.max_capacity() - self.sender.capacity());
        true
    }
}

pub fn channel<T>(capacity: usize, metric: RuntimeWork) -> (Ingress<T>, mpsc::Receiver<T>) {
    let (sender, receiver) = mpsc::channel(capacity.max(1));
    (Ingress { sender, metric }, receiver)
}

pub async fn run<T: Send + Sync + 'static>(
    mut receiver: mpsc::Receiver<T>,
    store: Arc<dyn StatsBatchStore<T>>,
    cancel: CancellationToken,
    settings: WriterSettings,
    ingress: RuntimeWork,
    flush: RuntimeWork,
    stale: RuntimeWork,
) -> Result<(), std::convert::Infallible> {
    let mut batch = Vec::with_capacity(settings.batch_size.max(1));
    let mut deadline = None;
    let mut shutdown = None;
    let mut closed = false;
    loop {
        if cancel.is_cancelled() && shutdown.is_none() {
            // Producer admission observes the same cancellation before reserving.
            // Close now, retain accepted rows, then drain under one total deadline.
            receiver.close();
            shutdown = Some(Instant::now() + settings.shutdown_timeout);
        }
        if shutdown.is_some_and(|end| Instant::now() >= end) {
            break;
        }
        if shutdown.is_some() {
            while batch.len() < settings.batch_size.max(1) {
                match receiver.try_recv() {
                    Ok(sample) => batch.push(sample),
                    Err(mpsc::error::TryRecvError::Disconnected) => {
                        closed = true;
                        break;
                    }
                    Err(mpsc::error::TryRecvError::Empty) => break,
                }
            }
            deadline = Some(Instant::now());
        }
        let due = deadline.is_some_and(|end| Instant::now() >= end);
        if !batch.is_empty() && (batch.len() >= settings.batch_size.max(1) || due || closed) {
            let mut delay = Duration::from_secs(1);
            loop {
                if cancel.is_cancelled() && shutdown.is_none() {
                    receiver.close();
                    shutdown = Some(Instant::now() + settings.shutdown_timeout);
                }
                let result = tokio::select! {
                    biased;
                    () = cancel.cancelled(), if shutdown.is_none() => continue,
                    () = until(shutdown) => break,
                    result = async {
                        let _timer = flush.start();
                        store.persist_batch(&batch).await
                    } => result,
                };
                match result {
                    Ok(outcome) => {
                        flush.success();
                        flush.units(outcome.persisted as u64);
                        stale.units(outcome.stale as u64);
                        batch.clear();
                        deadline = None;
                        break;
                    }
                    Err(error) => {
                        flush.failures(1);
                        RuntimeWork::StatsRetry.units(1);
                        tracing::warn!(%error, ?flush, "Statistics flush failed; retaining bounded batch");
                        tokio::select! {
                            biased;
                            () = cancel.cancelled(), if shutdown.is_none() => {},
                            () = until(shutdown) => break,
                            () = tokio::time::sleep(delay) => {},
                        }
                        delay = (delay * 2).min(Duration::from_secs(30));
                    }
                }
            }
            flush.buffered(batch.len());
            if shutdown.is_some_and(|end| Instant::now() >= end) {
                break;
            }
            continue;
        }
        if closed {
            break;
        }
        let next = tokio::select! {
            biased;
            () = cancel.cancelled(), if shutdown.is_none() => continue,
            () = until(shutdown) => break,
            () = until(deadline), if !batch.is_empty() => continue,
            next = receiver.recv() => next,
        };
        match next {
            Some(sample) => {
                batch.push(sample);
                deadline.get_or_insert(Instant::now() + settings.flush_interval);
                flush.buffered(batch.len());
                ingress.queued(receiver.len());
            }
            None => closed = true,
        }
        // During shutdown consume the finite closed queue without waiting for
        // the normal partial-batch deadline.
        if shutdown.is_some() {
            deadline = Some(Instant::now());
        }
    }
    RuntimeWork::StatsShutdownDrop.units((batch.len() + receiver.len()) as u64);
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

#[cfg(test)]
mod tests;
