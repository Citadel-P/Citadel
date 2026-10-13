//! Application notification presentation shared by every delivery transport.
use crate::api::resources::alerts::views::alert_resource_path;
use citadel_alerts::{AlertChannel, AlertDelivery, AlertError, AlertEvent};
use futures_util::future::BoxFuture;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub struct LinkedAlertDelivery {
    inner: Arc<dyn AlertDelivery>,
    public_url: String,
}

impl LinkedAlertDelivery {
    pub fn new(inner: Arc<dyn AlertDelivery>, public_url: String) -> Self {
        Self { inner, public_url }
    }
}

impl AlertDelivery for LinkedAlertDelivery {
    fn send<'a>(
        &'a self,
        channel: &'a AlertChannel,
        event: &'a AlertEvent,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), AlertError>> {
        Box::pin(async move {
            let mut notification = event.clone();
            if let Some(path) = alert_resource_path(event) {
                notification.message.push_str(&format!(
                    "\n{}{}",
                    self.public_url.trim_end_matches('/'),
                    path
                ));
            }
            self.inner.send(channel, &notification, cancellation).await
        })
    }
}

#[cfg(test)]
mod tests;
