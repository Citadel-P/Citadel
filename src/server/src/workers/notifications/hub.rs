//! One LISTEN connection fans out worker wakeups and committed connection events.
use super::super::connection_events::{ConnectionEventHub, ConnectionNotification};
use super::scoped::{ScopedNotifications, ScopedTopic};
use citadel_runtime::RuntimeSignal;
use citadel_runtime::notifications::{NotificationSource, RuntimeNotification};
use std::{sync::Arc, time::Duration};
use tokio::sync::{broadcast, watch};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub(crate) struct DatabaseNotificationHub {
    pub(super) signals: Arc<[watch::Sender<()>; RuntimeSignal::ALL.len()]>,
    pub(super) changes: [broadcast::Sender<RuntimeNotification>; 3],
    copy_work: watch::Sender<()>,
    pub connections: ConnectionEventHub,
}
impl DatabaseNotificationHub {
    pub fn new() -> Self {
        Self {
            signals: Arc::new(std::array::from_fn(|_| watch::channel(()).0)),
            connections: ConnectionEventHub::new(256),
            changes: std::array::from_fn(|_| broadcast::channel(256).0),
            copy_work: watch::channel(()).0,
        }
    }
    pub fn subscribe(&self, signal: RuntimeSignal) -> watch::Receiver<()> {
        self.signals[signal as usize].subscribe()
    }
    pub fn copy_work(&self) -> watch::Receiver<()> {
        self.copy_work.subscribe()
    }
    pub fn scoped(&self, topic: ScopedTopic) -> Box<dyn NotificationSource> {
        Box::new(ScopedNotifications(
            self.changes[topic as usize].subscribe(),
        ))
    }
    pub async fn run(
        self,
        mut listener: Box<dyn NotificationSource>,
        token: CancellationToken,
    ) -> Result<(), std::convert::Infallible> {
        loop {
            let result = tokio::select! { ()=token.cancelled()=>return Ok(()), result=listener.recv()=>result };
            match result {
                Ok(RuntimeNotification::Connection(event)) => self
                    .connections
                    .publish(ConnectionNotification::Changed(event)),
                Ok(RuntimeNotification::ConnectionResync) => {
                    self.connections.publish(ConnectionNotification::Resync)
                }
                Ok(RuntimeNotification::Signal(signal)) => {
                    self.signals[signal as usize].send_modify(|_| {})
                }
                Ok(RuntimeNotification::Resync) => {
                    self.connections.publish(ConnectionNotification::Resync);
                    for changes in &self.changes {
                        let _ = changes.send(RuntimeNotification::Resync);
                    }
                    self.copy_work.send_modify(|_| {});
                    citadel_runtime::runtime_metrics::RuntimeWork::NotificationReconnect.units(1);
                    for signal in self.signals.iter() {
                        signal.send_modify(|_| {});
                    }
                }
                Ok(event @ RuntimeNotification::ResourceCopy(_)) => {
                    self.copy_work.send_modify(|_| {});
                    let _ = self.changes[ScopedTopic::Copies as usize].send(event);
                }
                Ok(event @ RuntimeNotification::BackupCoverage(_)) => {
                    let _ = self.changes[ScopedTopic::Coverage as usize].send(event);
                }
                Ok(event @ RuntimeNotification::StackDrift(_)) => {
                    let _ = self.changes[ScopedTopic::StackDrift as usize].send(event);
                }
                Ok(_) => {}
                Err(error) => {
                    citadel_runtime::runtime_metrics::RuntimeWork::NotificationReconnect.units(1);
                    tracing::warn!(%error, "Database notification hub reconnecting; fallback timers remain active");
                    tokio::select! { ()=token.cancelled()=>return Ok(()), _=tokio::time::sleep(Duration::from_secs(2))=>{} }
                }
            }
        }
    }
}

/// A signal arriving during execution stays pending until this wait. A missed
/// notification can delay persisted work by at most the fallback duration.
pub(crate) async fn wait(
    receiver: &mut watch::Receiver<()>,
    token: &CancellationToken,
    fallback: Duration,
) {
    tokio::select! {
        () = token.cancelled() => {},
        _ = tokio::time::sleep(fallback) => {},
        changed = receiver.changed() => if changed.is_err() {
            tokio::select! { ()=token.cancelled()=>{}, _=tokio::time::sleep(fallback)=>{} }
        },
    }
}
