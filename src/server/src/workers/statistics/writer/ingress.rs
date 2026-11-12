use citadel_runtime::runtime_metrics::RuntimeWork;
use std::{sync::Arc, time::Duration};
use tokio::{sync::mpsc, time::Instant};
use tokio_util::sync::CancellationToken;

pub struct Envelope<T> {
    pub value: T,
    pub lease: Option<Arc<super::super::budget::Lease>>,
    pub encoded: usize,
}
impl<T> std::ops::Deref for Envelope<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}
#[derive(Clone, Copy)]
pub struct WriterSettings {
    pub batch_size: usize,
    pub flush_interval: Duration,
    pub shutdown_timeout: Duration,
}

pub struct Ingress<T> {
    sender: mpsc::Sender<Envelope<T>>,
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
    #[cfg(test)]
    pub async fn send(&self, sample: T, cancel: &CancellationToken) -> bool {
        self.send_owned(sample, None, 1, cancel).await
    }
    pub async fn send_owned(
        &self,
        sample: T,
        lease: Option<Arc<super::super::budget::Lease>>,
        encoded: usize,
        cancel: &CancellationToken,
    ) -> bool {
        if encoded > super::super::budget::BATCH_BYTES {
            return false;
        }
        let started = Instant::now();
        let saturated = self.sender.capacity() == 0;
        self.metric.permit_wait(Duration::ZERO, saturated);
        let permit = tokio::select! {
            biased;
            () = cancel.cancelled() => return false,
            permit = self.sender.reserve() => match permit { Ok(p) => p, Err(_) => return false },
        };
        permit.send(Envelope {
            value: sample,
            lease,
            encoded,
        });
        self.metric.permit_wait(started.elapsed(), false);
        self.metric.units(1);
        self.metric
            .queued(self.sender.max_capacity() - self.sender.capacity());
        true
    }
}

pub fn channel<T>(
    capacity: usize,
    metric: RuntimeWork,
) -> (Ingress<T>, mpsc::Receiver<Envelope<T>>) {
    let (sender, receiver) = mpsc::channel(capacity.max(1));
    (Ingress { sender, metric }, receiver)
}
