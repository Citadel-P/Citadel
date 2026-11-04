use super::runtime::{CheckSchedule, remaining_delay, run_enabled};
use citadel_updates::{
    CheckState, Release, ReleaseChannel, ReleaseCheckRepository, ReleaseNotice, ReleaseSource,
};
use citadel_updates::{ReleaseCheckPolicy, ReleaseCheckService, UpdateError};
use futures_util::future::BoxFuture;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::{sync::Arc, time::Duration};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

#[derive(Default)]
struct Store {
    state: Mutex<CheckState>,
    pause_commit: AtomicBool,
    committed: Notify,
    resume: Notify,
    writes: AtomicUsize,
}
impl ReleaseCheckRepository for Store {
    fn database_version(&self) -> BoxFuture<'_, Result<u32, UpdateError>> {
        Box::pin(async { Ok(180006) })
    }

    fn observe_core_availability<'a>(
        &'a self,
        _: &'a str,
        _: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(async { Ok(()) })
    }

    fn checks(&self) -> BoxFuture<'_, Result<CheckState, UpdateError>> {
        Box::pin(async { Ok(self.state.lock().unwrap().clone()) })
    }
    fn record_check<'a>(
        &'a self,
        result: Result<Option<&'a ReleaseNotice>, &'a str>,
        now: chrono::DateTime<chrono::Utc>,
    ) -> BoxFuture<'a, Result<bool, UpdateError>> {
        Box::pin(async move {
            {
                let mut state = self.state.lock().unwrap();
                state.last_attempt = Some(now);
                if let Ok(notice) = result {
                    state.available = notice.cloned();
                    state.last_success = Some(now);
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
    pause_first: AtomicBool,
    started: Notify,
    finished: Notify,
    resume: Notify,
}
struct FetchFinished<'a>(&'a Notify);
impl Drop for FetchFinished<'_> {
    fn drop(&mut self) {
        self.0.notify_one();
    }
}
impl ReleaseSource for Source {
    fn releases(&self, _: ReleaseChannel) -> BoxFuture<'_, Result<Vec<Release>, UpdateError>> {
        Box::pin(async {
            let _finished = FetchFinished(&self.finished);
            let first = self.calls.fetch_add(1, Ordering::SeqCst) == 0;
            self.started.notify_one();
            if first && self.pause_first.load(Ordering::SeqCst) {
                self.resume.notified().await;
            }
            Ok(vec![Release {
                tag_name: "v1.1.0".into(),
                draft: false,
                prerelease: false,
            }])
        })
    }
}
struct TestNotifier(Arc<AtomicUsize>);
impl citadel_updates::UpdateChangeNotifier for TestNotifier {
    fn changed(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
fn fixture() -> (
    ReleaseCheckService,
    Arc<Source>,
    Arc<Store>,
    Arc<AtomicUsize>,
) {
    let source = Arc::new(Source::default());
    let store = Arc::new(Store::default());
    let changes = Arc::new(AtomicUsize::new(0));
    let service = ReleaseCheckService::new("1.0.0".into(), source.clone(), store.clone())
        .with_notifier(Arc::new(TestNotifier(changes.clone())));
    (service, source, store, changes)
}

#[test]
fn check_frequency_uses_the_last_attempt_including_failed_checks() {
    let now = chrono::Utc::now();
    for hours in [6, 12, 24] {
        assert_eq!(remaining_delay(None, hours, now), Duration::ZERO);
        assert_eq!(
            remaining_delay(Some(now - chrono::Duration::hours(1)), hours, now),
            Duration::from_secs((hours as u64 - 1) * 3600),
        );
        assert_eq!(
            remaining_delay(Some(now - chrono::Duration::hours(25)), hours, now),
            Duration::ZERO,
        );
    }
}

#[tokio::test(start_paused = true)]
async fn restart_and_frequency_changes_preserve_the_saved_schedule() {
    let (service, source, store, _) = fixture();
    store.state.lock().unwrap().last_attempt =
        Some(chrono::Utc::now() - chrono::Duration::hours(8));
    let cancellation = CancellationToken::new();
    let (schedule, receiver) = tokio::sync::watch::channel(CheckSchedule {
        enabled: true,
        interval_hours: 12,
    });
    let task = tokio::spawn(run_enabled(service, receiver, cancellation.clone()));
    tokio::task::yield_now().await;
    assert_eq!(
        source.calls.load(Ordering::SeqCst),
        0,
        "restart must not check early"
    );
    schedule.send_replace(CheckSchedule {
        enabled: true,
        interval_hours: 24,
    });
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(5 * 3600)).await;
    tokio::task::yield_now().await;
    assert_eq!(
        source.calls.load(Ordering::SeqCst),
        0,
        "old timer must be replaced"
    );
    schedule.send_replace(CheckSchedule {
        enabled: true,
        interval_hours: 6,
    });
    store.committed.notified().await;
    assert_eq!(
        source.calls.load(Ordering::SeqCst),
        1,
        "shorter interval is already due"
    );
    cancellation.cancel();
    task.await.unwrap().unwrap();
}

#[tokio::test(start_paused = true)]
async fn manual_check_moves_the_next_background_check() {
    let (service, source, store, _) = fixture();
    store.state.lock().unwrap().last_attempt =
        Some(chrono::Utc::now() - chrono::Duration::hours(5));
    let cancellation = CancellationToken::new();
    let (_schedule, receiver) = tokio::sync::watch::channel(CheckSchedule {
        enabled: true,
        interval_hours: 6,
    });
    let task = tokio::spawn(run_enabled(service.clone(), receiver, cancellation.clone()));
    tokio::task::yield_now().await;
    service
        .check_now(
            ReleaseCheckPolicy {
                initialized: true,
                checks_allowed: true,
            },
            &cancellation,
        )
        .await
        .unwrap();
    tokio::time::advance(Duration::from_secs(2 * 3600)).await;
    tokio::task::yield_now().await;
    assert_eq!(
        source.calls.load(Ordering::SeqCst),
        1,
        "old due time must not fetch again"
    );
    cancellation.cancel();
    task.await.unwrap().unwrap();
}
#[tokio::test(start_paused = true)]
async fn disabled_worker_stays_idle_until_cancelled() {
    let (service, source, store, _) = fixture();
    let cancellation = CancellationToken::new();
    let (_enabled, receiver) = tokio::sync::watch::channel(CheckSchedule::default());
    let task = tokio::spawn(run_enabled(service, receiver, cancellation.clone()));
    tokio::time::advance(Duration::from_secs(12 * 60 * 60)).await;
    assert!(!task.is_finished());
    assert_eq!(source.calls.load(Ordering::SeqCst), 0);
    assert_eq!(store.writes.load(Ordering::SeqCst), 0);
    cancellation.cancel();
    task.await.unwrap().unwrap();
}
#[tokio::test]
async fn settings_toggle_cancels_fetch_and_resumes_without_restart() {
    let (service, source, store, _) = fixture();
    source.pause_first.store(true, Ordering::SeqCst);
    let cancellation = CancellationToken::new();
    let (enabled, receiver) = tokio::sync::watch::channel(CheckSchedule {
        enabled: true,
        interval_hours: 6,
    });
    let task = tokio::spawn(run_enabled(service.clone(), receiver, cancellation.clone()));
    tokio::time::timeout(Duration::from_secs(5), source.started.notified())
        .await
        .unwrap();
    enabled.send_replace(CheckSchedule::default());
    tokio::time::timeout(Duration::from_secs(5), source.finished.notified())
        .await
        .unwrap();
    assert_eq!(store.writes.load(Ordering::SeqCst), 0);
    enabled.send_replace(CheckSchedule {
        enabled: true,
        interval_hours: 6,
    });
    tokio::time::timeout(Duration::from_secs(5), store.committed.notified())
        .await
        .unwrap();
    assert_eq!(source.calls.load(Ordering::SeqCst), 2);
    assert!(service.available().is_some());
    enabled.send_replace(CheckSchedule::default());
    tokio::time::timeout(Duration::from_secs(5), async {
        while service.available().is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    cancellation.cancel();
    task.await.unwrap().unwrap();
}
#[tokio::test]
async fn shutdown_waits_for_committed_check_to_publish() {
    let (service, _, store, changes) = fixture();
    store.pause_commit.store(true, Ordering::SeqCst);
    let cancellation = CancellationToken::new();
    let (_enabled, receiver) = tokio::sync::watch::channel(CheckSchedule {
        enabled: true,
        interval_hours: 6,
    });
    let task = tokio::spawn(run_enabled(service.clone(), receiver, cancellation.clone()));
    tokio::time::timeout(Duration::from_secs(5), store.committed.notified())
        .await
        .unwrap();
    cancellation.cancel();
    tokio::task::yield_now().await;
    assert!(!task.is_finished());
    assert_eq!(changes.load(Ordering::SeqCst), 0);
    store.resume.notify_one();
    tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(service.available().is_some());
    assert_eq!(changes.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn disabling_checks_during_commit_still_publishes_then_clears_the_notice() {
    let (service, _, store, changes) = fixture();
    store.pause_commit.store(true, Ordering::SeqCst);
    let cancellation = CancellationToken::new();
    let (enabled, receiver) = tokio::sync::watch::channel(CheckSchedule {
        enabled: true,
        interval_hours: 6,
    });
    let task = tokio::spawn(run_enabled(service.clone(), receiver, cancellation.clone()));
    tokio::time::timeout(Duration::from_secs(5), store.committed.notified())
        .await
        .unwrap();
    enabled.send_replace(CheckSchedule::default());
    tokio::task::yield_now().await;
    assert_eq!(changes.load(Ordering::SeqCst), 0);
    store.resume.notify_one();
    tokio::time::timeout(Duration::from_secs(5), async {
        while changes.load(Ordering::SeqCst) < 2 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(service.available().is_none());
    assert!(store.state.lock().unwrap().available.is_some());
    assert_eq!(store.writes.load(Ordering::SeqCst), 1);
    cancellation.cancel();
    task.await.unwrap().unwrap();
}
#[tokio::test]
async fn application_info_only_exposes_updates_to_administrators() {
    use axum::{body::Body, http::StatusCode};
    use citadel_identity::{ActorPrincipal, AuthenticatedPrincipalType};
    use tower::ServiceExt;
    let (checker, _, _, _) = fixture();
    checker
        .check_now(
            ReleaseCheckPolicy {
                initialized: true,
                checks_allowed: true,
            },
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    for (principal_type, roles, expected) in [
        (
            AuthenticatedPrincipalType::User,
            vec!["Admin".to_owned()],
            true,
        ),
        (AuthenticatedPrincipalType::User, vec![], false),
        (
            AuthenticatedPrincipalType::ServiceAccount,
            vec!["Admin".to_owned()],
            false,
        ),
    ] {
        let principal = ActorPrincipal {
            subject_id: uuid::Uuid::nil(),
            actor_id: citadel_primitives::ActorId::new(uuid::Uuid::nil()),
            name: "test".into(),
            principal_type,
            credential_id: None,
            roles,
        };
        let response = crate::application_info_http::router(checker.clone())
            .layer(axum::Extension(principal))
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/v1/application/info")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.get("availableUpdate").is_some(), expected);
    }
}
