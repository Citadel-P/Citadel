use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueOverflowPolicy {
    Wait,
    Reject,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum QueueSendError<T> {
    #[error("queue is full")]
    Full(T),
    #[error("queue is closed")]
    Closed(T),
    #[error("queue send was cancelled")]
    Cancelled(T),
}

pub struct BoundedSender<T> {
    inner: mpsc::Sender<T>,
    overflow_policy: QueueOverflowPolicy,
}

impl<T> Clone for BoundedSender<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            overflow_policy: self.overflow_policy,
        }
    }
}

pub struct BoundedReceiver<T> {
    inner: mpsc::Receiver<T>,
}

#[must_use]
pub fn bounded_channel<T>(
    capacity: usize,
    overflow_policy: QueueOverflowPolicy,
) -> (BoundedSender<T>, BoundedReceiver<T>) {
    assert!(
        capacity > 0,
        "bounded queue capacity must be greater than zero"
    );
    let (sender, receiver) = mpsc::channel(capacity);
    (
        BoundedSender {
            inner: sender,
            overflow_policy,
        },
        BoundedReceiver { inner: receiver },
    )
}

impl<T> BoundedSender<T> {
    pub fn try_send(&self, value: T) -> Result<(), QueueSendError<T>> {
        self.inner.try_send(value).map_err(|error| match error {
            mpsc::error::TrySendError::Full(value) => QueueSendError::Full(value),
            mpsc::error::TrySendError::Closed(value) => QueueSendError::Closed(value),
        })
    }

    pub async fn send(
        &self,
        value: T,
        cancellation: &CancellationToken,
    ) -> Result<(), QueueSendError<T>> {
        match self.overflow_policy {
            QueueOverflowPolicy::Reject => self.try_send(value),
            QueueOverflowPolicy::Wait => tokio::select! {
                biased;
                () = cancellation.cancelled() => Err(QueueSendError::Cancelled(value)),
                permit = self.inner.reserve() => match permit {
                    Ok(permit) => {
                        permit.send(value);
                        Ok(())
                    }
                    Err(_) => Err(QueueSendError::Closed(value)),
                },
            },
        }
    }
}

impl<T> BoundedReceiver<T> {
    pub async fn recv(&mut self, cancellation: &CancellationToken) -> Option<T> {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => None,
            value = self.inner.recv() => value,
        }
    }

    pub fn try_recv(&mut self) -> Option<T> {
        self.inner.try_recv().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reject_policy_never_exceeds_capacity() {
        let cancellation = CancellationToken::new();
        let (sender, _receiver) = bounded_channel(1, QueueOverflowPolicy::Reject);
        sender.send(1, &cancellation).await.unwrap();
        assert_eq!(
            sender.send(2, &cancellation).await,
            Err(QueueSendError::Full(2))
        );
    }

    #[tokio::test]
    async fn wait_policy_observes_cancellation_when_full() {
        let cancellation = CancellationToken::new();
        let (sender, _receiver) = bounded_channel(1, QueueOverflowPolicy::Wait);
        sender.send(1, &cancellation).await.unwrap();
        cancellation.cancel();
        assert_eq!(
            sender.send(2, &cancellation).await,
            Err(QueueSendError::Cancelled(2))
        );
    }
}
