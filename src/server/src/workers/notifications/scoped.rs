//! Bounded payload fanout; a slow subscriber reconciles from persisted state.
use citadel_runtime::notifications::{NotificationError, NotificationSource, RuntimeNotification};
use futures_util::future::BoxFuture;
use tokio::sync::broadcast;

#[derive(Clone, Copy)]
pub(crate) enum ScopedTopic {
    Copies,
    Coverage,
    StackDrift,
}

pub(super) struct ScopedNotifications(pub broadcast::Receiver<RuntimeNotification>);
impl NotificationSource for ScopedNotifications {
    fn recv(&mut self) -> BoxFuture<'_, Result<RuntimeNotification, NotificationError>> {
        Box::pin(async move {
            match self.0.recv().await {
                Ok(event) => Ok(event),
                Err(broadcast::error::RecvError::Lagged(_)) => Ok(RuntimeNotification::Resync),
                // The supervisor owns both ends. A closed producer must not spin
                // consumers; their cancellation/fallback paths remain available.
                Err(broadcast::error::RecvError::Closed) => std::future::pending().await,
            }
        })
    }
}
