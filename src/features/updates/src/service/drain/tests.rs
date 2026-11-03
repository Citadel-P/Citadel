use crate::*;
use futures_util::future::BoxFuture;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Default)]
struct Fixture {
    closed: AtomicBool,
    reads: AtomicUsize,
    fail_at: AtomicUsize,
    pending_read: AtomicBool,
    handoff_during_read: AtomicBool,
    local_active: AtomicBool,
    blockers: Mutex<Vec<UpdateDrainBlocker>>,
}
impl UpdateDrainRepository for Fixture {
    fn blockers<'a>(
        &'a self,
        _: &'a UpdateOperation,
    ) -> BoxFuture<'a, Result<Vec<UpdateDrainBlocker>, UpdateError>> {
        Box::pin(async {
            let read = self.reads.fetch_add(1, Ordering::SeqCst) + 1;
            if read == self.fail_at.load(Ordering::SeqCst) {
                return Err(UpdateError::Conflict("ownership changed".into()));
            }
            if self.pending_read.load(Ordering::SeqCst) {
                std::future::pending::<()>().await;
            }
            if read == 2 && self.handoff_during_read.load(Ordering::SeqCst) {
                self.blockers.lock().unwrap().push(UpdateDrainBlocker {
                    kind: "Remote reservation".into(),
                    count: 1,
                });
                self.local_active.store(false, Ordering::SeqCst);
                // The snapshot preceded the durable handoff and cannot see it.
                return Ok(vec![]);
            }
            Ok(self.blockers.lock().unwrap().clone())
        })
    }
}
impl UpdateDrainRuntime for Fixture {
    fn close(&self, _: Uuid) -> Result<(), UpdateError> {
        assert!(self.reads.load(Ordering::SeqCst) > 0);
        self.closed.store(true, Ordering::SeqCst);
        Ok(())
    }
    fn blockers(&self, _: Uuid) -> Result<Vec<UpdateDrainBlocker>, UpdateError> {
        assert!(self.closed.load(Ordering::SeqCst));
        Ok(if self.local_active.load(Ordering::SeqCst) {
            vec![UpdateDrainBlocker {
                kind: "Local execution".into(),
                count: 1,
            }]
        } else {
            vec![]
        })
    }
}
fn operation() -> UpdateOperation {
    let now = chrono::Utc::now();
    let source = CoreIdentity {
        version: "0.1.1".into(),
        image: format!("ghcr.io/citadel-p/citadel@sha256:{}", "a".repeat(64)),
        catalog_sha256: "b".repeat(64),
        architecture: "amd64".into(),
    };
    UpdateOperation::approve(
        UpdatePlan {
            id: Uuid::now_v7(),
            instance_id: Uuid::now_v7(),
            target: CoreIdentity {
                version: "0.1.2".into(),
                image: format!("ghcr.io/citadel-p/citadel@sha256:{}", "c".repeat(64)),
                ..source.clone()
            },
            helper_image: source.image.clone(),
            source,
            configuration_sha256: "d".repeat(64),
            registration_id: Uuid::now_v7(),
            reviewed_at: now,
            expires_at: now + chrono::Duration::minutes(10),
            checkpoint_directory: "/private/checkpoints".into(),
            agent_requirements: vec![],
        },
        citadel_primitives::ActorId::new(Uuid::now_v7()),
        Uuid::now_v7(),
        now,
    )
    .unwrap()
}
fn service(f: &Arc<Fixture>) -> UpdateDrainService {
    UpdateDrainService::new(f.clone(), f.clone())
}

#[tokio::test(start_paused = true)]
async fn local_completion_during_database_snapshot_requires_another_observation() {
    let f = Arc::new(Fixture::default());
    f.local_active.store(true, Ordering::SeqCst);
    f.handoff_during_read.store(true, Ordering::SeqCst);
    assert_eq!(
        service(&f)
            .drain(
                &operation(),
                Duration::from_secs(3),
                &CancellationToken::new()
            )
            .await
            .unwrap(),
        UpdateDrainOutcome::Busy(vec![UpdateDrainBlocker {
            kind: "Remote reservation".into(),
            count: 1
        }])
    );
    assert!(f.reads.load(Ordering::SeqCst) >= 3);
}

#[tokio::test]
async fn rejects_unowned_or_changed_claim_and_never_reopens_after_closing() {
    for fail_at in [1, 2] {
        let f = Arc::new(Fixture::default());
        f.fail_at.store(fail_at, Ordering::SeqCst);
        assert!(matches!(
            service(&f)
                .drain(
                    &operation(),
                    Duration::from_secs(1),
                    &CancellationToken::new()
                )
                .await,
            Err(UpdateError::Conflict(_))
        ));
        assert_eq!(f.closed.load(Ordering::SeqCst), fail_at == 2);
    }
}

#[tokio::test(start_paused = true)]
async fn remote_reservations_block_even_when_no_local_work_remains() {
    let f = Arc::new(Fixture::default());
    let blocker = UpdateDrainBlocker {
        kind: "Script executor reservations".into(),
        count: 1,
    };
    f.blockers.lock().unwrap().push(blocker.clone());
    let cancellation = CancellationToken::new();
    assert_eq!(
        service(&f)
            .drain(&operation(), Duration::from_secs(3), &cancellation)
            .await
            .unwrap(),
        UpdateDrainOutcome::Busy(vec![blocker])
    );
    assert!(!cancellation.is_cancelled());
    assert!(f.closed.load(Ordering::SeqCst));
    assert!(f.reads.load(Ordering::SeqCst) > 2);
}

#[tokio::test]
async fn idle_observation_preserves_the_claim_and_closed_gate() {
    let f = Arc::new(Fixture::default());
    let operation = operation();
    assert_eq!(
        service(&f)
            .drain(
                &operation,
                Duration::from_secs(1),
                &CancellationToken::new()
            )
            .await
            .unwrap(),
        UpdateDrainOutcome::Idle
    );
    assert!(f.closed.load(Ordering::SeqCst));
    assert_eq!(operation.phase, UpdatePhase::Preparing);
    assert_eq!(operation.revision, 0);
}

#[tokio::test]
async fn cancellation_and_dropped_waiters_keep_maintenance_closed() {
    let f = Arc::new(Fixture::default());
    f.blockers.lock().unwrap().push(UpdateDrainBlocker {
        kind: "Backups".into(),
        count: 1,
    });
    let service = service(&f);
    let operation = operation();
    let cancellation = CancellationToken::new();
    let mut drain = Box::pin(service.drain(&operation, Duration::from_secs(30), &cancellation));
    assert!(futures_util::poll!(&mut drain).is_pending());
    assert!(matches!(
        service
            .drain(
                &operation,
                Duration::from_secs(1),
                &CancellationToken::new()
            )
            .await,
        Err(UpdateError::Conflict(_))
    ));
    cancellation.cancel();
    assert!(matches!(drain.await, Err(UpdateError::Cancelled)));
    let another = CancellationToken::new();
    let mut drain = Box::pin(service.drain(&operation, Duration::from_secs(30), &another));
    assert!(futures_util::poll!(&mut drain).is_pending());
    drop(drain);
    assert!(f.closed.load(Ordering::SeqCst));
}

#[tokio::test(start_paused = true)]
async fn unresponsive_database_is_not_reported_as_idle() {
    let f = Arc::new(Fixture::default());
    f.pending_read.store(true, Ordering::SeqCst);
    assert!(matches!(
        service(&f)
            .drain(
                &operation(),
                Duration::from_secs(1),
                &CancellationToken::new()
            )
            .await,
        Err(UpdateError::Unavailable(_))
    ));
    assert!(!f.closed.load(Ordering::SeqCst));
}
