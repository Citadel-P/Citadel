//! Dedicated LISTEN connections never occupy the request/worker query pool.
use citadel_runtime::RuntimeSignal;
use citadel_runtime::notifications::{NotificationError, NotificationSource, RuntimeNotification};
use futures_util::{FutureExt, future::BoxFuture};
use sqlx::{PgPool, postgres::PgListener};

pub struct PostgresNotifications(PgListener);
impl PostgresNotifications {
    pub async fn connect(pool: &PgPool, channel: &str) -> Result<Self, sqlx::Error> {
        let dedicated = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy_with((*pool.connect_options()).clone().application_name(channel));
        let mut listener = PgListener::connect_with(&dedicated).await?;
        listener.listen(channel).await?;
        Ok(Self(listener))
    }
    pub async fn listen(&mut self, channel: &str) -> Result<(), sqlx::Error> {
        self.0.listen(channel).await
    }
}
impl NotificationSource for PostgresNotifications {
    fn recv(&mut self) -> BoxFuture<'_, Result<RuntimeNotification, NotificationError>> {
        async move {
            loop {
                let notification = self
                    .0
                    .try_recv()
                    .await
                    .map_err(|e| NotificationError(e.to_string()))?;
                // SQLx restores subscriptions before reporting a reconnect.
                let Some(notification) = notification else {
                    return Ok(RuntimeNotification::Resync);
                };
                if let Some(event) = decode(notification.channel(), notification.payload()) {
                    return Ok(event);
                }
            }
        }
        .boxed()
    }
}
fn decode(channel: &str, payload: &str) -> Option<RuntimeNotification> {
    use RuntimeNotification::*;
    match channel {
        super::connection_events::CHANNEL => Some(
            serde_json::from_str(payload)
                .map(Connection)
                .unwrap_or_else(|error| {
                    tracing::warn!(%error, "Invalid connection event; refreshing subscribers");
                    ConnectionResync
                }),
        ),
        "citadel_container_created" => serde_json::from_str(payload).ok().map(ContainerCreated),
        "citadel_stack_drift" => payload.parse().ok().map(StackDrift),
        "citadel_swarm_prune" => Some(SwarmPrune),
        "citadel_job_alerts" => Some(JobAlerts),
        _ => RuntimeSignal::ALL
            .into_iter()
            .find(|s| s.channel() == channel)
            .map(Signal),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decodes_scoped_events_and_recovers_invalid_connection_events() {
        let id = uuid::Uuid::now_v7();
        assert!(
            matches!(decode("citadel_stack_drift", &id.to_string()), Some(RuntimeNotification::StackDrift(received)) if received == id)
        );
        assert!(decode("citadel_stack_drift", "invalid").is_none());
        assert!(decode("citadel_container_created", "{}").is_none());
        assert!(matches!(
            decode(super::super::connection_events::CHANNEL, "invalid"),
            Some(RuntimeNotification::ConnectionResync)
        ));
        assert!(decode("unknown_channel", "{}").is_none());
        for signal in RuntimeSignal::ALL {
            assert!(
                matches!(decode(signal.channel(), ""), Some(RuntimeNotification::Signal(received)) if received as usize == signal as usize)
            );
        }
    }
}
