use super::*;
use citadel_adapters::persistence::postgres::git::accounts::PostgresGitAccountRepository;
use citadel_adapters::security::identity::crypto::AesGcmSecretProtector;
use citadel_git::*;
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use std::sync::atomic::{AtomicUsize, Ordering};
use uuid::Uuid;

#[derive(Default)]
struct SlowStore {
    calls: AtomicUsize,
    completed: AtomicUsize,
    schedules: AtomicUsize,
}
impl GitRepositoryExecutionPersistence for SlowStore {
    fn enqueue_webhook<'a>(
        &'a self,
        _: citadel_primitives::ActorId,
        _: Uuid,
        _: &'a str,
        _: &'a GitRepositoryWebhook,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>> {
        unreachable!()
    }
    fn claim_next(
        &self,
        _: chrono::DateTime<chrono::Utc>,
    ) -> BoxFuture<'_, Result<Option<GitSyncClaim>, GitRepositoryExecutionError>> {
        Box::pin(async {
            self.calls.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_secs(120)).await;
            self.completed.fetch_add(1, Ordering::SeqCst);
            Ok(None)
        })
    }
    fn enqueue_due(&self, _: usize) -> BoxFuture<'_, Result<usize, GitRepositoryExecutionError>> {
        Box::pin(async {
            self.schedules.fetch_add(1, Ordering::SeqCst);
            Ok(0)
        })
    }
    fn get_source(
        &self,
        _: Uuid,
    ) -> BoxFuture<'_, Result<GitRepositorySource, GitRepositoryExecutionError>> {
        unreachable!()
    }
    fn enqueue_sync<'a>(
        &'a self,
        _: ActorId,
        _: Uuid,
        _: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>> {
        unreachable!()
    }
    fn complete<'a>(
        &'a self,
        _: &'a GitSyncClaim,
        _: &'a SyncResult,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>> {
        unreachable!()
    }
    fn fail<'a>(
        &'a self,
        _: &'a GitSyncClaim,
        _: &'a str,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>> {
        unreachable!()
    }
    fn get_ref<'a>(
        &'a self,
        _: Uuid,
        _: &'a str,
    ) -> BoxFuture<'a, Result<Option<GitRepositoryRef>, GitRepositoryExecutionError>> {
        unreachable!()
    }
    fn list_refs(
        &self,
        _: Uuid,
    ) -> BoxFuture<'_, Result<Vec<GitRepositoryRef>, GitRepositoryExecutionError>> {
        unreachable!()
    }
    fn resolve_reference<'a>(
        &'a self,
        _: Uuid,
        _: Option<&'a str>,
    ) -> BoxFuture<'a, Result<String, GitRepositoryExecutionError>> {
        unreachable!()
    }
    fn get_webhook(
        &self,
        _: Uuid,
    ) -> BoxFuture<'_, Result<Option<GitRepositoryWebhook>, GitRepositoryExecutionError>> {
        unreachable!()
    }
}

#[tokio::test(start_paused = true)]
async fn schedule_ticks_do_not_drop_an_in_flight_execution() {
    // Credentials and the CLI are never reached by this pending store read.
    // A lazy pool keeps this a deterministic scheduler unit test without I/O.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://unused:unused@localhost/unused")
        .unwrap();
    let accounts = Arc::new(GitAccountService::new(
        Arc::new(PostgresGitAccountRepository::new(pool)),
        Arc::new(AesGcmSecretProtector::new(&[91; 32]).unwrap()),
    ));
    let store = Arc::new(SlowStore::default());
    let service = Arc::new(GitRepositoryExecutionService::new(
        store.clone(),
        accounts,
        Arc::new(GitCli::new(
            std::sync::Arc::new(citadel_processes::SystemProcess),
            Duration::from_secs(300),
        )),
        std::env::temp_dir(),
        Duration::from_secs(300),
    ));
    let cancellation = CancellationToken::new();
    let worker = tokio::spawn(git_repository_sync(cancellation.clone(), service));
    tokio::task::yield_now().await;
    // Cross the staggered scheduler deadline before the execution completes.
    // Advancing straight to 120s makes both select branches ready together.
    for seconds in [61, 59] {
        tokio::time::advance(Duration::from_secs(seconds)).await;
        tokio::task::yield_now().await;
    }
    assert_eq!(store.calls.load(Ordering::SeqCst), 1);
    assert_eq!(store.completed.load(Ordering::SeqCst), 1);
    assert_eq!(
        store.schedules.load(Ordering::SeqCst),
        1,
        "the initial background schedule is delayed past one full interval"
    );
    cancellation.cancel();
    worker.await.unwrap().unwrap();
}
