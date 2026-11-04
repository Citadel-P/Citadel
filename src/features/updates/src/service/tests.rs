use super::*;
use crate::{
    CheckState, Release, ReleaseChannel, ReleaseCheckRepository, ReleaseNotice, ReleaseSource,
    UpdateError,
};
use futures_util::future::BoxFuture;
use std::sync::Arc;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

const POLICY: ReleaseCheckPolicy = ReleaseCheckPolicy {
    initialized: true,
    checks_allowed: true,
};

#[tokio::test]
async fn scheduled_check_rechecks_due_time_after_a_concurrent_manual_check() {
    let (service, store, source, _) = fixture();
    source.pause.store(true, Ordering::SeqCst);
    let token = CancellationToken::new();
    let manual = service.check_now(POLICY, &token);
    tokio::pin!(manual);
    tokio::select! { _ = source.started.notified() => {}, _ = &mut manual => panic!("manual fetch must pause") }
    let scheduled = service.check_scheduled(POLICY, chrono::Duration::hours(6), &token);
    tokio::pin!(scheduled);
    tokio::select! { biased; _ = &mut scheduled => panic!("scheduled check must wait"), () = std::future::ready(()) => {} }
    source.resume.notify_one();
    let completed = manual.await.unwrap();
    assert_eq!(scheduled.await.unwrap(), completed);
    assert_eq!(source.calls.load(Ordering::SeqCst), 1);
    assert_eq!(store.writes.load(Ordering::SeqCst), 1);
    // A genuinely due attempt must still fetch again.
    source.pause.store(false, Ordering::SeqCst);
    store.state.lock().unwrap().last_attempt =
        Some(chrono::Utc::now() - chrono::Duration::hours(7));
    service
        .check_scheduled(POLICY, chrono::Duration::hours(6), &token)
        .await
        .unwrap();
    assert_eq!(source.calls.load(Ordering::SeqCst), 2);
}

#[derive(Default)]
struct Store {
    state: Mutex<CheckState>,
    availability: Mutex<Vec<Option<String>>>,
    reads: AtomicUsize,
    writes: AtomicUsize,
    fail: AtomicBool,
    pause_commit: AtomicBool,
    committed: Notify,
    resume: Notify,
}
impl ReleaseCheckRepository for Store {
    fn observe_core_availability<'a>(
        &'a self,
        _: &'a str,
        available: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(async move {
            self.availability
                .lock()
                .unwrap()
                .push(available.map(Into::into));
            Ok(())
        })
    }

    fn checks(&self) -> BoxFuture<'_, Result<CheckState, UpdateError>> {
        Box::pin(async {
            self.reads.fetch_add(1, Ordering::SeqCst);
            Ok(self.state.lock().unwrap().clone())
        })
    }
    fn record_check<'a>(
        &'a self,
        result: Result<Option<&'a ReleaseNotice>, &'a str>,
        now: chrono::DateTime<chrono::Utc>,
    ) -> BoxFuture<'a, Result<bool, UpdateError>> {
        Box::pin(async move {
            if self.fail.load(Ordering::SeqCst) {
                return Err(UpdateError::Storage("fixture failure".into()));
            }
            {
                let mut state = self.state.lock().unwrap();
                state.last_attempt = Some(now);
                match result {
                    Ok(notice) => {
                        state.last_success = Some(now);
                        state.available = notice.cloned();
                        state.error = None;
                    }
                    Err(error) => state.error = Some(error.into()),
                }
            }
            self.writes.fetch_add(1, Ordering::SeqCst);
            self.committed.notify_one();
            if self.pause_commit.load(Ordering::SeqCst) {
                self.resume.notified().await;
            }
            Ok(true)
        })
    }
}
#[derive(Default)]
struct Source {
    calls: AtomicUsize,
    fail: AtomicBool,
    pause: AtomicBool,
    started: Notify,
    resume: Notify,
}
impl ReleaseSource for Source {
    fn releases(
        &self,
        channel: ReleaseChannel,
    ) -> BoxFuture<'_, Result<Vec<Release>, UpdateError>> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.started.notify_one();
            if self.pause.load(Ordering::SeqCst) {
                self.resume.notified().await;
            }
            if self.fail.load(Ordering::SeqCst) {
                return Err(UpdateError::Unavailable("fixture source error".into()));
            }
            Ok(vec![Release {
                tag_name: match channel {
                    ReleaseChannel::Stable => "v1.1.0",
                    ReleaseChannel::Development => "v1.1.0-dev.2",
                }
                .into(),
                draft: false,
                prerelease: channel == ReleaseChannel::Development,
            }])
        })
    }
}
struct TestNotifier {
    notices: Arc<AtomicUsize>,
    store: Arc<Store>,
}
impl crate::UpdateChangeNotifier for TestNotifier {
    fn changed(&self) {
        assert!(self.store.writes.load(Ordering::SeqCst) > 0);
        self.notices.fetch_add(1, Ordering::SeqCst);
    }
}
fn fixture() -> (
    ReleaseCheckService,
    Arc<Store>,
    Arc<Source>,
    Arc<AtomicUsize>,
) {
    let store = Arc::new(Store::default());
    let source = Arc::new(Source::default());
    let notices = Arc::new(AtomicUsize::new(0));
    let service = ReleaseCheckService::new("1.0.0".into(), source.clone(), store.clone())
        .with_notifier(Arc::new(TestNotifier {
            notices: notices.clone(),
            store: store.clone(),
        }));
    (service, store, source, notices)
}
#[tokio::test]
async fn check_returns_one_authoritative_read_and_notifies_every_committed_check() {
    let (service, store, source, notices) = fixture();
    let first = service
        .check_now(POLICY, &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(first.available, service.available());
    assert!(first.last_success.is_some());
    assert_eq!(store.reads.load(Ordering::SeqCst), 1);
    assert_eq!(notices.load(Ordering::SeqCst), 1);
    let second = service
        .check_now(POLICY, &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(notices.load(Ordering::SeqCst), 2);
    source.fail.store(true, Ordering::SeqCst);
    let failed = service
        .check_now(POLICY, &CancellationToken::new())
        .await
        .unwrap();
    assert!(failed.error.is_some());
    assert_eq!(failed.last_success, second.last_success);
    assert_eq!(failed.available, first.available);
    assert_eq!(service.available(), first.available);
    assert_eq!(notices.load(Ordering::SeqCst), 3);
}
#[tokio::test]
async fn development_installations_check_the_development_channel() {
    let service = ReleaseCheckService::new(
        "1.0.0-dev.1".into(),
        Arc::new(Source::default()),
        Arc::new(Store::default()),
    );
    let state = service
        .check_now(POLICY, &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(state.available.unwrap().version, "1.1.0-dev.2");
}

#[tokio::test]
async fn failed_persistence_never_publishes_or_updates_the_cache() {
    let (service, store, _, notices) = fixture();
    store.fail.store(true, Ordering::SeqCst);
    assert!(matches!(
        service.check_now(POLICY, &CancellationToken::new()).await,
        Err(UpdateError::Storage(_))
    ));
    assert!(service.available().is_none());
    assert_eq!(notices.load(Ordering::SeqCst), 0);
    assert_eq!(store.reads.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn policy_denial_and_cancelled_fetch_do_not_persist() {
    let (service, store, source, notices) = fixture();
    for policy in [
        ReleaseCheckPolicy {
            initialized: false,
            ..POLICY
        },
        ReleaseCheckPolicy {
            checks_allowed: false,
            ..POLICY
        },
    ] {
        assert!(matches!(
            service.check_now(policy, &CancellationToken::new()).await,
            Err(UpdateError::Unavailable(_))
        ));
    }
    assert_eq!(source.calls.load(Ordering::SeqCst), 0);
    source.pause.store(true, Ordering::SeqCst);
    let token = CancellationToken::new();
    let attempt = service.check_now(POLICY, &token);
    tokio::pin!(attempt);
    tokio::select! { _ = source.started.notified() => {}, _ = &mut attempt => panic!("fetch should be blocked") }
    token.cancel();
    assert!(matches!(attempt.await, Err(UpdateError::Cancelled)));
    assert_eq!(store.writes.load(Ordering::SeqCst), 0);
    assert_eq!(notices.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn cancellation_after_commit_finishes_publication_and_cancels_waiting_checks() {
    let (service, store, source, notices) = fixture();
    store.pause_commit.store(true, Ordering::SeqCst);
    let token = CancellationToken::new();
    let attempt = service.check_now(POLICY, &token);
    tokio::pin!(attempt);
    tokio::select! { _ = store.committed.notified() => {}, _ = &mut attempt => panic!("commit response should be blocked") }
    assert_eq!(notices.load(Ordering::SeqCst), 0);
    let waiting = service.check_now(POLICY, &token);
    tokio::pin!(waiting);
    tokio::select! { biased; _ = &mut waiting => panic!("second check must wait"), () = std::future::ready(()) => {} }
    token.cancel();
    assert!(matches!(waiting.await, Err(UpdateError::Cancelled)));
    store.resume.notify_one();
    let state = attempt.await.unwrap();
    assert_eq!(service.available(), state.available);
    assert_eq!(source.calls.load(Ordering::SeqCst), 1);
    assert_eq!(store.writes.load(Ordering::SeqCst), 1);
    assert_eq!(notices.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn restored_notices_are_filtered_by_installed_version_and_channel() {
    let store = Arc::new(Store::default());
    store.state.lock().unwrap().available = Some(ReleaseNotice {
        version: "1.1.0".into(),
        release_url: "https://untrusted.invalid".into(),
    });
    for (version, expected) in [("1.0.0", true), ("1.1.0", false), ("1.0.0-dev.1", false)] {
        let service =
            ReleaseCheckService::new(version.into(), Arc::new(Source::default()), store.clone());
        service.restore_notice().await.unwrap();
        assert_eq!(service.available().is_some(), expected);
        if let Some(notice) = service.available() {
            assert_eq!(
                notice.release_url,
                "https://github.com/Citadel-P/Citadel/releases/tag/v1.1.0"
            );
        }
        service.clear_notice();
        assert!(service.available().is_none());
    }
}

#[tokio::test]
async fn core_availability_requires_successful_check_and_transition_qualification() {
    let (service, store, source, _) = fixture();
    let token = CancellationToken::new();
    service.check_now(POLICY, &token).await.unwrap();
    assert!(
        store
            .availability
            .lock()
            .unwrap()
            .iter()
            .all(Option::is_none)
    );
    let mut installer = super::installation::tests::Fixture::new();
    installer.plan.source.version = "1.0.0".into();
    installer.plan.target.version = "1.1.0".into();
    let (installation, _receiver) = installer.service();
    let service = service.with_installer(Some(installation));
    service.check_now(POLICY, &token).await.unwrap();
    assert_eq!(
        store.availability.lock().unwrap().last(),
        Some(&Some("1.1.0".into()))
    );
    assert_eq!(
        installer.state.lock().unwrap().events,
        ["source", "verify", "inspect"]
    );
    assert!(installer.state.lock().unwrap().operation.is_none());
    store.availability.lock().unwrap().clear();
    source.fail.store(true, Ordering::SeqCst);
    service.check_now(POLICY, &token).await.unwrap();
    assert!(
        store
            .availability
            .lock()
            .unwrap()
            .iter()
            .all(Option::is_none)
    );
    source.fail.store(false, Ordering::SeqCst);
    installer.state.lock().unwrap().agent_drift = true;
    service.check_now(POLICY, &token).await.unwrap();
    assert!(
        store
            .availability
            .lock()
            .unwrap()
            .iter()
            .all(Option::is_none)
    );
}
