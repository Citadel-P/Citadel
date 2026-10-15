use super::*;
use citadel_stacks::{
    ApplyStack, CreateStack, ImageUpdateState, RecreateStackOnNewImageState, StackRepository,
    StackSpec, StackUpdateScanner, StackUpdateState,
};

#[path = "applied_images/lifecycle.rs"]
mod lifecycle;

struct AppliedImageScanner {
    pool: sqlx::PgPool,
    mode: AtomicU8,
    calls: AtomicU8,
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
    scan_token: Mutex<Option<CancellationToken>>,
}
impl AppliedImageScanner {
    fn new(pool: sqlx::PgPool) -> Self {
        Self {
            pool,
            mode: AtomicU8::new(0),
            calls: AtomicU8::new(0),
            entered: Default::default(),
            release: Default::default(),
            scan_token: Default::default(),
        }
    }
}
impl StackUpdateScanner for AppliedImageScanner {
    fn scan<'a>(
        &'a self,
        _: &'a citadel_stacks::Stack,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackUpdateState, StackError>> {
        Box::pin(async { panic!("post-Apply must only check images, without synchronizing Git") })
    }

    fn scan_images<'a>(
        &'a self,
        stack: &'a citadel_stacks::Stack,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackUpdateState, StackError>> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::Relaxed);
            assert_eq!(stack.status, StackReleaseStatus::Healthy);
            assert_eq!(
                stack.control_state,
                citadel_primitives::ResourceControlState::Idle
            );
            let persisted: String =
                sqlx::query_scalar("SELECT controlstate FROM stacks WHERE id=$1")
                    .bind(stack.id)
                    .fetch_one(&self.pool)
                    .await
                    .unwrap();
            assert_eq!(
                persisted, "Idle",
                "Apply must commit before its observation is refreshed"
            );
            match self.mode.load(Ordering::Relaxed) {
                1 => return Err(StackError::Runtime("Registry unavailable".into())),
                2 => {
                    sqlx::query("UPDATE stacks SET description='concurrent edit',rowversion=rowversion+1 WHERE id=$1")
                        .bind(stack.id).execute(&self.pool).await.unwrap();
                }
                5 => {
                    *self.scan_token.lock().unwrap() = Some(cancel.clone());
                    self.entered.notify_one();
                    self.release.notified().await;
                }
                _ => {}
            }
            let mut state = stack.stack_update_state.clone();
            let (StackUpdateState::Git {
                recreate_stack_on_new_image_state,
                ..
            }
            | StackUpdateState::WebEditor {
                recreate_stack_on_new_image_state,
            }) = &mut state;
            // Model a tag advancing again after Docker pulls the advertised update.
            *recreate_stack_on_new_image_state = images(
                "sha256:pulled",
                if self.mode.load(Ordering::Relaxed) == 3 {
                    "sha256:newer"
                } else {
                    "sha256:pulled"
                },
            );
            Ok(state)
        })
    }
}

fn images(current: &str, remote: &str) -> RecreateStackOnNewImageState {
    RecreateStackOnNewImageState {
        auto_update_states: vec![ImageUpdateState {
            service_name: "web".into(),
            image_name: "nginx:alpine".into(),
            current_digest: current.into(),
            remote_digest: Some(remote.into()),
            last_checked_at: chrono::Utc::now(),
            update_available: current != remote,
        }],
    }
}

#[tokio::test]
#[ignore = "requires CITADEL_WORKLOAD_DATABASE_URL"]
async fn successful_apply_refreshes_images_without_masking_failures_or_overwriting_edits() {
    let database_url = std::env::var("CITADEL_WORKLOAD_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .unwrap();
    let actor = ActorId::new(citadel_identity::SYSTEM_ACTOR_ID);
    let repository = Uuid::now_v7();
    sqlx::query("INSERT INTO gitrepositories(id,createdbyactorid,defaultbranch,name,status,syncmode,url,controlstate) VALUES($1,$2,'main',$3,'Healthy','Manual','https://example.test/repository.git','Idle')")
        .bind(repository).bind(actor.value()).bind(repository.to_string()).execute(&pool).await.unwrap();
    let store = Arc::new(PostgresStackRepository::new(pool.clone()));
    let runtime = Arc::new(CompletingStackRuntime::default());
    let scanner = Arc::new(AppliedImageScanner::new(pool.clone()));
    let service = StackService::new(
        Arc::new(citadel_server::tasks::stacks::TrackedStackTasks::new(
            citadel_runtime::DynamicTasks::new(CancellationToken::new()),
        )),
        store.clone(),
        runtime.clone(),
        Arc::new(FixtureBindings),
        Arc::new(NoopStackChangeNotifier),
        CancellationToken::new(),
    )
    .with_update_scanner(scanner.clone())
    .with_source_materializer(Arc::new(updates::GitSource));

    for platform_type in ["DockerStandalone", "DockerSwarm"] {
        let platform = Uuid::now_v7();
        sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$4,'Local',0,0,0,$2,0,$3,'Online',0)")
            .bind(platform).bind(platform.to_string()).bind(json!({"$type":platform_type}))
            .bind(format!("http://stack-status-{platform}.test")).execute(&pool).await.unwrap();
        for git in [false, true] {
            let mut spec: StackSpec = serde_json::from_value(if git {
                json!({"$type":"Git","gitRepoId":repository,"branch":"main","composePaths":["compose.yml"],"updateBehavior":"Disabled"})
            } else {
                json!({"$type":"WebEditor","composeFile":"services:\n  web:\n    image: nginx:alpine\n","updateBehavior":"Disabled"})
            }).unwrap();
            spec.common_mut().registry_id = Some(Uuid::from_u128(0x100));
            let created = store
                .create(
                    actor,
                    true,
                    &CreateStack {
                        name: format!("applied-images-{}", Uuid::now_v7()),
                        platform_id: platform,
                        stack_source: if git {
                            citadel_stacks::StackSource::Git
                        } else {
                            citadel_stacks::StackSource::WebEditor
                        },
                        spec,
                        description: None,
                        drift_policy: None,
                        tag_ids: vec![],
                        duplicate_source: None,
                    },
                )
                .await
                .unwrap();
            // Success, failed scan, concurrent edit, registry advances, and failed Docker apply.
            for mode in 0..=4 {
                scanner.mode.store(mode, Ordering::Relaxed);
                runtime
                    .apply_failure
                    .store(if mode == 4 { 2 } else { 0 }, Ordering::Relaxed);
                let mut old = created.stack_update_state.clone();
                let (StackUpdateState::Git {
                    recreate_stack_on_new_image_state,
                    ..
                }
                | StackUpdateState::WebEditor {
                    recreate_stack_on_new_image_state,
                }) = &mut old;
                *recreate_stack_on_new_image_state = images("sha256:old", "sha256:advertised");
                sqlx::query("UPDATE stacks SET stackupdatestate=$2 WHERE id=$1")
                    .bind(created.id)
                    .bind(old.to_storage_value().unwrap())
                    .execute(&pool)
                    .await
                    .unwrap();
                let calls = scanner.calls.load(Ordering::Relaxed);
                let mut progress = service
                    .apply(
                        actor,
                        true,
                        ApplyStack {
                            id: created.id,
                            recreate: None,
                        },
                    )
                    .await
                    .unwrap();
                let mut events = Vec::new();
                while let Some(item) = progress.recv().await {
                    events.push(item);
                }
                let expected_status = if mode == 4 {
                    StackReleaseStatus::Failed
                } else {
                    StackReleaseStatus::Healthy
                };
                assert_eq!(events.last().unwrap().stack_status, Some(expected_status));
                let persisted = store.get_authorized(actor, true, created.id).await.unwrap();
                assert_eq!(persisted.status, expected_status);
                assert_eq!(
                    scanner.calls.load(Ordering::Relaxed),
                    calls + u8::from(mode != 4)
                );
                if mode == 1 || mode == 2 {
                    assert!(events.iter().any(|item| {
                        item.message
                            .as_deref()
                            .is_some_and(|message| message.contains("Check for updates to retry"))
                    }));
                }
                if mode == 2 {
                    assert_eq!(persisted.description.as_deref(), Some("concurrent edit"));
                }
                if git && mode != 4 {
                    let StackUpdateState::Git {
                        recreate_stack_on_new_commit_state,
                        ..
                    } = &persisted.stack_update_state
                    else {
                        panic!("Git state lost")
                    };
                    assert_eq!(
                        recreate_stack_on_new_commit_state.current_commit_sha,
                        "c".repeat(40)
                    );
                }
                let (StackUpdateState::Git {
                    recreate_stack_on_new_image_state,
                    ..
                }
                | StackUpdateState::WebEditor {
                    recreate_stack_on_new_image_state,
                }) = &persisted.stack_update_state;
                let image = &recreate_stack_on_new_image_state.auto_update_states[0];
                assert_eq!(
                    image.current_digest,
                    if mode == 0 || mode == 3 {
                        "sha256:pulled"
                    } else {
                        "sha256:old"
                    }
                );
                assert_eq!(image.update_available, mode != 0);
            }
        }
    }
    pool.close().await;
}
