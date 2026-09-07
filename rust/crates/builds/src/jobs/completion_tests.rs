use super::*;
use std::{
    collections::VecDeque,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

struct Store {
    queued: Mutex<VecDeque<BuildConsumerClaim>>,
    messages: Mutex<Vec<String>>,
}
impl BuildCompletionStore for Store {
    fn claim_next(&self) -> BoxFuture<'_, Result<Option<BuildConsumerClaim>, BuildError>> {
        Box::pin(async { Ok(self.queued.lock().unwrap().pop_front()) })
    }
    fn complete<'a>(
        &'a self,
        _: &'a BuildConsumerClaim,
        message: &'a str,
    ) -> BoxFuture<'a, Result<Option<crate::BuildLogEntry>, BuildError>> {
        Box::pin(async move {
            self.messages.lock().unwrap().push(message.into());
            Ok(None)
        })
    }
    fn recover(&self) -> BoxFuture<'_, Result<(), BuildError>> {
        Box::pin(async { Ok(()) })
    }
}
struct Runtime {
    calls: AtomicUsize,
    fail: bool,
}
impl BuildConsumerRuntime for Runtime {
    fn apply<'a>(&'a self, _: &'a BuildConsumerClaim) -> BoxFuture<'a, Result<(), BuildError>> {
        Box::pin(async {
            self.calls.fetch_add(1, Ordering::Relaxed);
            if self.fail {
                Err(BuildError::Conflict(
                    "secret-like upstream diagnostic".into(),
                ))
            } else {
                Ok(())
            }
        })
    }
    fn changed(&self, _: BuildConsumerType, _: Uuid) {}
}
struct License(bool);
impl BuildEntitlements for License {
    fn enabled(&self, _: LicenseCapability) -> BoxFuture<'_, Result<bool, BuildError>> {
        Box::pin(async { Ok(self.0) })
    }
}
fn setup(
    count: usize,
    licensed: bool,
    fail: bool,
) -> (BuildCompletionService, Arc<Store>, Arc<Runtime>) {
    let store = Arc::new(Store {
        queued: Mutex::new(
            (0..count)
                .map(|_| BuildConsumerClaim {
                    id: Uuid::now_v7(),
                    build_run_id: Uuid::now_v7(),
                    resource_id: Uuid::now_v7(),
                    resource_type: BuildConsumerType::Deployment,
                    expected_version: Some(3),
                    redeploy: true,
                    service_names: Vec::new(),
                })
                .collect(),
        ),
        messages: Mutex::new(vec![]),
    });
    let runtime = Arc::new(Runtime {
        calls: AtomicUsize::new(0),
        fail,
    });
    (
        BuildCompletionService::new(store.clone(), runtime.clone(), Arc::new(License(licensed))),
        store,
        runtime,
    )
}
#[tokio::test]
async fn batches_are_bounded_and_entitlements_are_checked_before_apply() {
    let (service, store, runtime) = setup(27, false, false);
    assert_eq!(
        service
            .process_batch(&CancellationToken::new())
            .await
            .unwrap(),
        25
    );
    assert_eq!(store.queued.lock().unwrap().len(), 2);
    assert_eq!(runtime.calls.load(Ordering::Relaxed), 0);
    assert!(
        store
            .messages
            .lock()
            .unwrap()
            .iter()
            .all(|message| message.contains("license"))
    );
}
#[tokio::test]
async fn failed_consumer_apply_is_logged_without_failing_or_replaying_the_build() {
    let (service, store, runtime) = setup(1, true, true);
    assert_eq!(
        service
            .process_batch(&CancellationToken::new())
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        service
            .process_batch(&CancellationToken::new())
            .await
            .unwrap(),
        0
    );
    assert_eq!(runtime.calls.load(Ordering::Relaxed), 1);
    let messages = store.messages.lock().unwrap();
    assert!(messages[0].contains("did not complete"));
    assert!(!messages[0].contains("secret-like"));
}
#[tokio::test]
async fn shutdown_does_not_claim_more_work() {
    let (service, store, runtime) = setup(1, true, false);
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert_eq!(service.process_batch(&cancel).await.unwrap(), 0);
    assert_eq!(store.queued.lock().unwrap().len(), 1);
    assert_eq!(runtime.calls.load(Ordering::Relaxed), 0);
}
