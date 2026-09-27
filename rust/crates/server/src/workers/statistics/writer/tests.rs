use super::*;
use citadel_platforms::{
    RuntimeCapabilityError, RuntimeErrorKind, stats_ingestion::StatsWriteOutcome,
};
use futures_util::future::BoxFuture;
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Default)]
struct Store {
    calls: Mutex<Vec<Vec<usize>>>,
    fail: AtomicUsize,
    hang: bool,
}
impl StatsBatchStore<usize> for Store {
    fn persist_batch<'a>(
        &'a self,
        samples: &'a [usize],
    ) -> BoxFuture<'a, Result<StatsWriteOutcome, RuntimeCapabilityError>> {
        Box::pin(async move {
            self.calls.lock().unwrap().push(samples.to_vec());
            if self.hang {
                return std::future::pending().await;
            }
            if self
                .fail
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
                .is_ok()
            {
                return Err(RuntimeCapabilityError::new(
                    RuntimeErrorKind::Remote,
                    "fixture outage",
                    true,
                ));
            }
            Ok(StatsWriteOutcome {
                persisted: samples.len(),
                stale: 0,
            })
        })
    }
}
fn start(
    store: Arc<Store>,
    batch_size: usize,
    capacity: usize,
) -> (
    Ingress<usize>,
    CancellationToken,
    tokio::task::JoinHandle<Result<(), std::convert::Infallible>>,
) {
    let token = CancellationToken::new();
    let (sender, queue) = channel(capacity, RuntimeWork::PlatformStatsIngress);
    let task = tokio::spawn(run(
        queue,
        store,
        token.clone(),
        WriterSettings {
            batch_size,
            flush_interval: Duration::from_secs(60),
            shutdown_timeout: Duration::from_secs(5),
        },
        RuntimeWork::PlatformStatsIngress,
        RuntimeWork::PlatformStatsFlush,
        RuntimeWork::PlatformStatsStale,
    ));
    (sender, token, task)
}
async fn settle() {
    for _ in 0..5 {
        tokio::task::yield_now().await;
    }
}

#[tokio::test(start_paused = true)]
async fn threshold_flushes_many_samples_once_and_deadline_flushes_partial_batch() {
    let store = Arc::new(Store::default());
    let (sender, token, task) = start(store.clone(), 3, 4);
    sender.send(1, &token).await;
    sender.send(2, &token).await;
    settle().await;
    tokio::time::advance(Duration::from_secs(59)).await;
    assert!(store.calls.lock().unwrap().is_empty());
    sender.send(3, &token).await;
    settle().await;
    assert_eq!(*store.calls.lock().unwrap(), [vec![1, 2, 3]]);
    sender.send(4, &token).await;
    settle().await;
    tokio::time::advance(Duration::from_secs(60)).await;
    settle().await;
    assert_eq!(*store.calls.lock().unwrap(), [vec![1, 2, 3], vec![4]]);
    token.cancel();
    task.await.unwrap().unwrap();
}

#[tokio::test(start_paused = true)]
async fn retry_retains_order_while_accepting_a_bounded_follow_up() {
    let store = Arc::new(Store {
        fail: AtomicUsize::new(1),
        ..Default::default()
    });
    let (sender, token, task) = start(store.clone(), 2, 1);
    sender.send(1, &token).await;
    sender.send(2, &token).await;
    settle().await;
    sender.send(3, &token).await;
    assert!(sender.send(4, &token).await);
    settle().await;
    assert_eq!(*store.calls.lock().unwrap(), [vec![1, 2]]);
    tokio::time::advance(Duration::from_secs(1)).await;
    settle().await;
    assert_eq!(
        *store.calls.lock().unwrap(),
        [vec![1, 2], vec![1, 2], vec![3, 4]]
    );
    token.cancel();
    task.await.unwrap().unwrap();
}

#[tokio::test(start_paused = true)]
async fn saturated_admission_is_cancellable_and_shutdown_is_bounded_when_database_hangs() {
    let store = Arc::new(Store {
        hang: true,
        ..Default::default()
    });
    let (sender, token, task) = start(store.clone(), 1, 1);
    sender.send(1, &token).await;
    settle().await;
    sender.send(2, &token).await;
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    assert!(!sender.send(3, &cancelled).await);
    let start = Instant::now();
    token.cancel();
    task.await.unwrap().unwrap();
    assert!(Instant::now() - start <= Duration::from_secs(5));
    assert!(!sender.send(4, &CancellationToken::new()).await);
}

#[tokio::test(start_paused = true)]
async fn shutdown_flushes_accepted_queue_and_partial_buffer_before_deadline() {
    let store = Arc::new(Store::default());
    let (sender, token, task) = start(store.clone(), 10, 4);
    sender.send(1, &token).await;
    settle().await;
    sender.send(2, &token).await;
    token.cancel();
    task.await.unwrap().unwrap();
    assert_eq!(*store.calls.lock().unwrap(), [vec![1, 2]]);
}

#[tokio::test(start_paused = true)]
async fn failed_platform_does_not_block_healthy_partition() {
    struct PartitionStore(Mutex<Vec<usize>>);
    impl StatsBatchStore<usize> for PartitionStore {
        fn partition(&self, sample: &usize) -> uuid::Uuid {
            uuid::Uuid::from_u128(*sample as u128)
        }
        fn persist_batch<'a>(
            &'a self,
            samples: &'a [usize],
        ) -> BoxFuture<'a, Result<StatsWriteOutcome, RuntimeCapabilityError>> {
            Box::pin(async move {
                self.0.lock().unwrap().extend(samples);
                if samples[0] == 1 {
                    Err(RuntimeCapabilityError::new(
                        RuntimeErrorKind::Remote,
                        "locked platform",
                        true,
                    ))
                } else {
                    Ok(StatsWriteOutcome {
                        persisted: samples.len(),
                        stale: 0,
                    })
                }
            })
        }
    }
    let store = Arc::new(PartitionStore(Mutex::new(vec![])));
    let cancel = CancellationToken::new();
    let (sender, queue) = channel(4, RuntimeWork::ContainerStatsIngress);
    let worker = tokio::spawn(run(
        queue,
        store.clone(),
        cancel.clone(),
        WriterSettings {
            batch_size: 1,
            flush_interval: Duration::from_secs(1),
            shutdown_timeout: Duration::from_secs(1),
        },
        RuntimeWork::ContainerStatsIngress,
        RuntimeWork::ContainerStatsFlush,
        RuntimeWork::ContainerStatsStale,
    ));
    sender.send(1, &cancel).await;
    settle().await;
    sender.send(2, &cancel).await;
    settle().await;
    assert_eq!(
        *store.0.lock().unwrap(),
        [1, 2],
        "healthy platform must commit before failed platform retries"
    );
    cancel.cancel();
    worker.await.unwrap().unwrap();
}
