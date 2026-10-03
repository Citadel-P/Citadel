//! Count and byte bounds for the connection's shared output queue.
use super::{AgentEnvelope, MAX_ENVELOPE, OUTGOING_CAPACITY};
use prost::Message;
use std::{io, sync::Arc, time::Duration};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc};

#[derive(Clone)]
pub(super) struct Outgoing {
    sender: mpsc::Sender<QueuedEnvelope>,
    budget: Arc<Semaphore>,
}
pub(super) struct QueuedEnvelope {
    pub message: AgentEnvelope,
    _budget: OwnedSemaphorePermit,
}
impl Outgoing {
    pub fn channel() -> (Self, mpsc::Receiver<QueuedEnvelope>) {
        let (sender, receiver) = mpsc::channel(OUTGOING_CAPACITY);
        (
            Self {
                sender,
                budget: Arc::new(Semaphore::new(2 * MAX_ENVELOPE)),
            },
            receiver,
        )
    }
    pub async fn send(&self, message: AgentEnvelope) -> io::Result<()> {
        let size = message.encoded_len();
        if size > MAX_ENVELOPE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Edge envelope exceeds 16 MiB",
            ));
        }
        tokio::time::timeout(Duration::from_secs(10), async {
            let permit = self
                .budget
                .clone()
                .acquire_many_owned(size.max(1) as u32)
                .await
                .map_err(|_| io::Error::other("Edge outgoing byte budget closed"))?;
            self.sender
                .send(QueuedEnvelope {
                    message,
                    _budget: permit,
                })
                .await
                .map_err(|_| io::Error::other("Edge outgoing stream closed"))
        })
        .await
        .map_err(|_| io::Error::other("Edge outgoing stream stalled"))?
    }
}
