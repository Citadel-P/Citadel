use super::*;
use citadel_runtime::notifications::NotificationError;
use std::collections::VecDeque;

struct Events(VecDeque<RuntimeNotification>);
impl NotificationSource for Events {
    fn recv(&mut self) -> BoxFuture<'_, Result<RuntimeNotification, NotificationError>> {
        Box::pin(async move {
            match self.0.pop_front() {
                Some(event) => Ok(event),
                None => std::future::pending().await,
            }
        })
    }
}
fn source(events: Vec<RuntimeNotification>) -> PostgresCoverageChanges {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .unwrap();
    PostgresCoverageChanges::from_notifications(&pool, Box::new(Events(events.into())))
}
#[tokio::test(start_paused = true)]
async fn repeated_platforms_coalesce_and_distinct_platforms_remain_bounded() {
    let id = Uuid::now_v7();
    let mut events = vec![RuntimeNotification::BackupCoverage(id); 1000];
    let second = Uuid::now_v7();
    events.push(RuntimeNotification::ResourceCopy(Uuid::now_v7()));
    events.push(RuntimeNotification::BackupCoverage(second));
    let mut changes = source(events);
    assert!(changes.collect().await.unwrap());
    assert_eq!(changes.pending, BTreeSet::from([id, second]));
    let mut changes = source(
        (0..300)
            .map(|_| RuntimeNotification::BackupCoverage(Uuid::now_v7()))
            .collect(),
    );
    assert!(changes.collect().await.unwrap());
    assert_eq!(changes.pending.len(), 256);
    changes.pending.clear();
    assert!(changes.collect().await.unwrap());
    assert_eq!(changes.pending.len(), 44);
}
#[tokio::test(start_paused = true)]
async fn resync_discards_partial_batch_without_querying() {
    let mut changes = source(vec![
        RuntimeNotification::BackupCoverage(Uuid::now_v7()),
        RuntimeNotification::Resync,
    ]);
    assert!(changes.next().await.unwrap().is_none());
    assert!(changes.pending.is_empty());
}
