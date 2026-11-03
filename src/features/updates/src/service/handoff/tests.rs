use super::*;
use crate::*;
use futures_util::future::BoxFuture;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Default)]
struct Fixture {
    events: Mutex<Vec<&'static str>>,
    closed: AtomicBool,
    busy: AtomicBool,
    fail_ack: AtomicBool,
    fail_grant: AtomicBool,
}
impl UpdateDrainRepository for Fixture {
    fn blockers<'a>(
        &'a self,
        _: &'a UpdateOperation,
    ) -> BoxFuture<'a, Result<Vec<UpdateDrainBlocker>, UpdateError>> {
        Box::pin(async {
            self.events.lock().unwrap().push("database");
            Ok(if self.busy.load(Ordering::SeqCst) {
                vec![UpdateDrainBlocker {
                    kind: "remote cleanup".into(),
                    count: 1,
                }]
            } else {
                vec![]
            })
        })
    }
}
impl UpdateDrainRuntime for Fixture {
    fn close(&self, _: Uuid) -> Result<(), UpdateError> {
        self.closed.store(true, Ordering::SeqCst);
        self.events.lock().unwrap().push("close");
        Ok(())
    }
    fn blockers(&self, _: Uuid) -> Result<Vec<UpdateDrainBlocker>, UpdateError> {
        assert!(self.closed.load(Ordering::SeqCst));
        self.events.lock().unwrap().push("local");
        Ok(vec![])
    }
}
impl UpdateHandoffRuntime for Fixture {
    fn acknowledged<'a>(
        &'a self,
        _: &'a UpdateOperation,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(async {
            self.events.lock().unwrap().push("ack");
            if self.fail_ack.load(Ordering::SeqCst) {
                Err(UpdateError::Conflict("missing helper".into()))
            } else {
                Ok(())
            }
        })
    }
    fn authorize_shutdown<'a>(
        &'a self,
        _: &'a UpdateOperation,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(async {
            self.events.lock().unwrap().push("grant");
            if self.fail_grant.load(Ordering::SeqCst) {
                Err(UpdateError::Conflict("helper disappeared".into()))
            } else {
                Ok(())
            }
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
            source: source.clone(),
            target: CoreIdentity {
                version: "0.1.2".into(),
                ..source.clone()
            },
            helper_image: source.image,
            configuration_sha256: "c".repeat(64),
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
#[tokio::test]
async fn shutdown_requires_acknowledgement_then_a_fresh_owned_drain() {
    for case in 0..4 {
        let f = Arc::new(Fixture::default());
        f.fail_ack.store(case == 1, Ordering::SeqCst);
        f.busy.store(case == 2, Ordering::SeqCst);
        f.fail_grant.store(case == 3, Ordering::SeqCst);
        let service =
            UpdateHandoffService::new(UpdateDrainService::new(f.clone(), f.clone()), f.clone());
        let result = service
            .authorize_shutdown(
                &operation(),
                Duration::from_millis(5),
                &CancellationToken::new(),
            )
            .await;
        assert_eq!(result.is_ok(), case == 0);
        let events = f.events.lock().unwrap();
        assert_eq!(events[0], "ack");
        if case == 1 {
            assert_eq!(*events, ["ack"]);
            assert!(!f.closed.load(Ordering::SeqCst));
        }
        if case == 2 {
            assert!(!events.contains(&"grant"));
        }
        if case == 0 || case == 3 {
            assert_eq!(
                *events,
                [
                    "ack", "database", "close", "local", "database", "local", "grant"
                ]
            );
        }
        if case != 1 {
            assert!(f.closed.load(Ordering::SeqCst));
        }
    }
}
#[tokio::test]
async fn cancellation_cannot_publish_a_shutdown_grant() {
    let f = Arc::new(Fixture::default());
    let service =
        UpdateHandoffService::new(UpdateDrainService::new(f.clone(), f.clone()), f.clone());
    let token = CancellationToken::new();
    token.cancel();
    assert!(matches!(
        service
            .authorize_shutdown(&operation(), Duration::from_secs(1), &token)
            .await,
        Err(UpdateError::Cancelled)
    ));
    assert!(!f.events.lock().unwrap().contains(&"grant"));
}
