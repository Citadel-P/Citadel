use super::*;
use citadel_runtime::DynamicTasks;
use citadel_stacks::{RollbackStack, StackProgressItem, UpdateStack};
use std::time::Duration as StdDuration;

struct Fixture {
    pool: sqlx::PgPool,
    actor: ActorId,
    id: Uuid,
    store: Arc<PostgresStackRepository>,
    runtime: Arc<CompletingStackRuntime>,
    scanner: Arc<AppliedImageScanner>,
    service: StackService,
    shutdown: CancellationToken,
    owner: DynamicTasks,
}
impl Fixture {
    async fn new(registry: Option<Uuid>) -> Self {
        let database_url = std::env::var("CITADEL_WORKLOAD_DATABASE_URL").unwrap();
        MigrationRunner::migrate(&database_url).await.unwrap();
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect(&database_url)
            .await
            .unwrap();
        let actor = ActorId::new(SYSTEM_ACTOR_ID);
        let platform = Uuid::now_v7();
        sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$2,'Local',0,0,0,$2,0,'{\"$type\":\"DockerStandalone\"}','Online',0)")
            .bind(platform).bind(format!("http://stack-lifecycle-{platform}.test")).execute(&pool).await.unwrap();
        let mut spec: StackSpec = serde_json::from_value(json!({"$type":"WebEditor","composeFile":"services:\n  web:\n    image: nginx:alpine\n","updateBehavior":"Disabled"})).unwrap();
        spec.common_mut().registry_id = registry;
        let store = Arc::new(PostgresStackRepository::new(pool.clone()));
        let stack = store
            .create(
                actor,
                true,
                &CreateStack {
                    name: format!("image-lifecycle-{platform}"),
                    platform_id: platform,
                    stack_source: citadel_stacks::StackSource::WebEditor,
                    spec,
                    description: None,
                    drift_policy: None,
                    tag_ids: vec![],
                    duplicate_source: None,
                },
            )
            .await
            .unwrap();
        let runtime = Arc::new(CompletingStackRuntime::default());
        let scanner = Arc::new(AppliedImageScanner::new(pool.clone()));
        let shutdown = CancellationToken::new();
        let owner = DynamicTasks::new(shutdown.clone());
        let service = StackService::new(
            Arc::new(citadel_server::tasks::stacks::TrackedStackTasks::new(
                owner.clone(),
            )),
            store.clone(),
            runtime.clone(),
            Arc::new(FixtureBindings),
            Arc::new(NoopStackChangeNotifier),
            shutdown.clone(),
        )
        .with_update_scanner(scanner.clone());
        let fixture = Self {
            pool,
            actor,
            id: stack.id,
            store,
            runtime,
            scanner,
            service,
            shutdown,
            owner,
        };
        fixture.seed_stale_images().await;
        fixture
    }

    async fn seed_stale_images(&self) {
        let state = StackUpdateState::WebEditor {
            recreate_stack_on_new_image_state: images("sha256:old", "sha256:advertised"),
        };
        sqlx::query("UPDATE stacks SET stackupdatestate=$2 WHERE id=$1")
            .bind(self.id)
            .bind(state.to_storage_value().unwrap())
            .execute(&self.pool)
            .await
            .unwrap();
    }

    async fn apply(&self) -> tokio::sync::mpsc::Receiver<StackProgressItem> {
        self.service
            .apply(
                self.actor,
                true,
                ApplyStack {
                    id: self.id,
                    recreate: None,
                },
            )
            .await
            .unwrap()
    }

    async fn assert_images(&self, digest: &str, available: bool) {
        let stack = self
            .store
            .get_authorized(self.actor, true, self.id)
            .await
            .unwrap();
        assert_eq!(stack.status, StackReleaseStatus::Healthy);
        assert_eq!(
            stack.control_state,
            citadel_primitives::ResourceControlState::Idle
        );
        let StackUpdateState::WebEditor {
            recreate_stack_on_new_image_state,
        } = &stack.stack_update_state
        else {
            panic!("Expected Web Editor")
        };
        let image = &recreate_stack_on_new_image_state.auto_update_states[0];
        assert_eq!(image.current_digest, digest);
        assert_eq!(image.update_available, available);
    }

    async fn dispose(self) {
        self.shutdown.cancel();
        self.owner.drain(StdDuration::from_secs(5)).await.unwrap();
        // The following tests run global drift scans in the same disposable DB.
        // Remove this fixture's Stack only after all its owned work has settled.
        sqlx::query("DELETE FROM stacks WHERE id=$1")
            .bind(self.id)
            .execute(&self.pool)
            .await
            .unwrap();
        self.pool.close().await;
    }
}

async fn completed(
    mut progress: tokio::sync::mpsc::Receiver<StackProgressItem>,
) -> Vec<StackProgressItem> {
    tokio::time::timeout(StdDuration::from_secs(5), async {
        let mut items = Vec::new();
        while let Some(item) = progress.recv().await {
            items.push(item);
        }
        assert_eq!(
            items.last().unwrap().stack_status,
            Some(StackReleaseStatus::Healthy)
        );
        items
    })
    .await
    .expect("Apply must settle")
}

#[tokio::test]
#[ignore = "requires CITADEL_WORKLOAD_DATABASE_URL"]
async fn apply_without_registry_skips_refresh_without_a_warning() {
    let f = Fixture::new(None).await;
    let events = completed(f.apply().await).await;
    assert_eq!(f.scanner.calls.load(Ordering::Relaxed), 0);
    assert!(!events.iter().any(|item| {
        item.message
            .as_deref()
            .is_some_and(|message| message.contains("could not be refreshed"))
    }));
    f.assert_images("sha256:old", true).await;
    f.dispose().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_WORKLOAD_DATABASE_URL"]
async fn rollback_refreshes_the_observed_images_of_the_restored_release() {
    let f = Fixture::new(Some(Uuid::from_u128(0x100))).await;
    completed(f.apply().await).await;
    let current = f.store.get_authorized(f.actor, true, f.id).await.unwrap();
    let mut spec = current.spec.clone().unwrap();
    let StackSpec::WebEditor { compose_file, .. } = &mut spec else {
        unreachable!()
    };
    *compose_file =
        "services:\n  web:\n    image: nginx:alpine\n    command: sleep infinity\n".into();
    f.store
        .update(
            f.actor,
            true,
            f.id,
            current.row_version,
            &UpdateStack {
                name: None,
                platform_id: None,
                description: None,
                stack_source: None,
                spec: None,
                drift_policy: None,
                row_version: Some(current.row_version),
            },
            &spec,
        )
        .await
        .unwrap();
    completed(f.apply().await).await;
    let releases = f.store.releases(f.actor, true, f.id).await.unwrap();
    let old_release = releases
        .iter()
        .find(|release| {
            !release
                .spec
                .compose_file()
                .unwrap()
                .contains("sleep infinity")
        })
        .unwrap();
    f.seed_stale_images().await;
    f.scanner.mode.store(3, Ordering::Relaxed);
    completed(
        f.service
            .rollback(
                f.actor,
                true,
                RollbackStack {
                    stack_id: f.id,
                    release_id: old_release.id,
                },
            )
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(f.scanner.calls.load(Ordering::Relaxed), 3);
    f.assert_images("sha256:pulled", true).await;
    let persisted = f.store.get_authorized(f.actor, true, f.id).await.unwrap();
    assert!(
        !persisted
            .spec
            .as_ref()
            .unwrap()
            .compose_file()
            .unwrap()
            .contains("sleep infinity")
    );
    f.dispose().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_WORKLOAD_DATABASE_URL"]
async fn recovery_refreshes_images_without_replaying_docker_apply() {
    let f = Fixture::new(Some(Uuid::from_u128(0x100))).await;
    f.store
        .claim_apply(f.actor, true, f.id, None, None)
        .await
        .unwrap();
    assert_eq!(
        f.service
            .reconcile_stale_operations(StdDuration::ZERO, 100)
            .await
            .unwrap(),
        1
    );
    f.assert_images("sha256:pulled", false).await;
    assert_eq!(f.scanner.calls.load(Ordering::Relaxed), 1);
    assert!(f.runtime.apply_calls.lock().unwrap().is_empty());
    assert_eq!(
        f.service
            .reconcile_stale_operations(StdDuration::ZERO, 100)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        f.scanner.calls.load(Ordering::Relaxed),
        1,
        "settled recovery is not replayed"
    );
    f.dispose().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_WORKLOAD_DATABASE_URL"]
async fn browser_disconnect_does_not_cancel_the_post_apply_refresh() {
    let f = Fixture::new(Some(Uuid::from_u128(0x100))).await;
    f.scanner.mode.store(5, Ordering::Relaxed);
    let progress = f.apply().await;
    tokio::time::timeout(StdDuration::from_secs(5), f.scanner.entered.notified())
        .await
        .unwrap();
    drop(progress);
    f.scanner.release.notify_one();
    f.owner.drain(StdDuration::from_secs(5)).await.unwrap();
    f.assert_images("sha256:pulled", false).await;
    assert_eq!(f.runtime.apply_calls.lock().unwrap().len(), 1);
    f.dispose().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_WORKLOAD_DATABASE_URL"]
async fn shutdown_cancels_refresh_but_preserves_the_successful_apply() {
    verify_interrupted_refresh(false).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_WORKLOAD_DATABASE_URL"]
async fn refresh_timeout_cancels_scanner_and_preserves_the_successful_apply() {
    verify_interrupted_refresh(true).await;
}

async fn verify_interrupted_refresh(timeout: bool) {
    let f = Fixture::new(Some(Uuid::from_u128(0x100))).await;
    f.scanner.mode.store(5, Ordering::Relaxed);
    let progress = f.apply().await;
    tokio::time::timeout(StdDuration::from_secs(5), f.scanner.entered.notified())
        .await
        .unwrap();
    if timeout {
        // Pause only after the database commit; the scanner is suspended on Notify.
        tokio::time::pause();
        tokio::time::advance(StdDuration::from_secs(61)).await;
        tokio::time::resume();
    } else {
        f.shutdown.cancel();
    }
    let events = completed(progress).await;
    assert!(events.iter().any(|item| {
        item.message
            .as_deref()
            .is_some_and(|message| message.contains("Check for updates to retry"))
    }));
    assert!(
        f.scanner
            .scan_token
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .is_cancelled()
    );
    f.assert_images("sha256:old", true).await;
    assert_eq!(f.runtime.apply_calls.lock().unwrap().len(), 1);
    f.dispose().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_WORKLOAD_DATABASE_URL"]
async fn refresh_cannot_overwrite_a_new_apply_claim() {
    let f = Fixture::new(Some(Uuid::from_u128(0x100))).await;
    f.scanner.mode.store(5, Ordering::Relaxed);
    let progress = f.apply().await;
    tokio::time::timeout(StdDuration::from_secs(5), f.scanner.entered.notified())
        .await
        .unwrap();
    let next = f
        .store
        .claim_apply(f.actor, true, f.id, None, None)
        .await
        .unwrap();
    f.scanner.release.notify_one();
    completed(progress).await;
    let current = f.store.get_authorized(f.actor, true, f.id).await.unwrap();
    assert_eq!(current.current_stack_release_id, next.release_id);
    assert_eq!(current.row_version, next.row_version);
    assert_eq!(current.status, StackReleaseStatus::Applying);
    assert_eq!(
        current.control_state,
        citadel_primitives::ResourceControlState::Processing
    );
    let StackUpdateState::WebEditor {
        recreate_stack_on_new_image_state,
    } = &current.stack_update_state
    else {
        unreachable!()
    };
    assert_eq!(
        recreate_stack_on_new_image_state.auto_update_states[0].current_digest,
        "sha256:old"
    );
    assert!(recreate_stack_on_new_image_state.auto_update_states[0].update_available);
    f.store
        .fail_apply(f.actor, &next, "Disposable fixture cleanup", false)
        .await
        .unwrap();
    f.dispose().await;
}
