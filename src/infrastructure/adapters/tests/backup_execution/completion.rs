//! Fault injection delegates completion to the real PostgreSQL transaction.
use super::*;
use chrono::DateTime;
use citadel_backups::*;
use futures_util::future::BoxFuture;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct Store {
    inner: PostgresBackupPersistence,
    pool: sqlx::PgPool,
    mode: usize,
    claims: AtomicUsize,
    finishes: AtomicUsize,
    cleaned: Arc<AtomicBool>,
    entered: tokio::sync::Notify,
    resume: tokio::sync::Notify,
}
impl BackupPersistence for Store {
    fn defer_restore<'a>(
        &'a self,
        claim: &'a RestoreClaim,
    ) -> BoxFuture<'a, Result<bool, BackupError>> {
        self.inner.defer_restore(claim)
    }
    fn renew_leases(
        &self,
        repository: Uuid,
        owner: Uuid,
        attempt: Uuid,
    ) -> BoxFuture<'_, Result<bool, BackupError>> {
        self.inner.renew_leases(repository, owner, attempt)
    }
    fn defer_backup<'a>(
        &'a self,
        claim: &'a BackupClaim,
    ) -> BoxFuture<'a, Result<bool, BackupError>> {
        self.inner.defer_backup(claim)
    }

    fn create_repository<'a>(
        &'a self,
        actor: ActorId,
        input: &'a BackupRepositoryConfiguration,
    ) -> BoxFuture<'a, Result<BackupRepository, BackupError>> {
        self.inner.create_repository(actor, input)
    }
    fn list_repositories(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<BackupRepository>, BackupError>> {
        self.inner.list_repositories(actor, administrator)
    }
    fn get_repository(&self, id: Uuid) -> BoxFuture<'_, Result<BackupRepository, BackupError>> {
        self.inner.get_repository(id)
    }
    fn update_repository<'a>(
        &'a self,
        id: Uuid,
        patch: &'a Value,
    ) -> BoxFuture<'a, Result<BackupRepository, BackupError>> {
        self.inner.update_repository(id, patch)
    }
    fn commit_repository_credentials<'a>(
        &'a self,
        id: Uuid,
        operation_id: Uuid,
        row_version: i64,
        credentials: &'a BackupRepositoryCredentials,
    ) -> BoxFuture<'a, Result<BackupRepository, BackupError>> {
        self.inner
            .commit_repository_credentials(id, operation_id, row_version, credentials)
    }
    fn archive_repository(&self, id: Uuid) -> BoxFuture<'_, Result<(), BackupError>> {
        self.inner.archive_repository(id)
    }
    fn record_repository_operation<'a>(
        &'a self,
        id: Uuid,
        result: BackupRepositoryOperation<'a>,
    ) -> BoxFuture<'a, Result<(BackupRepository, BackupRepositoryValidation), BackupError>> {
        self.inner.record_repository_operation(id, result)
    }
    fn acquire_repository_operation(
        &self,
        repository_id: Uuid,
        operation_id: Uuid,
        operation: &str,
        expires_at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<bool, BackupError>> {
        self.inner
            .acquire_repository_operation(repository_id, operation_id, operation, expires_at)
    }
    fn release_repository_operation(
        &self,
        repository_id: Uuid,
        operation_id: Uuid,
    ) -> BoxFuture<'_, Result<(), BackupError>> {
        self.inner
            .release_repository_operation(repository_id, operation_id)
    }
    fn create_policy<'a>(
        &'a self,
        actor: ActorId,
        input: &'a BackupPolicyConfiguration,
    ) -> BoxFuture<'a, Result<BackupPolicy, BackupError>> {
        self.inner.create_policy(actor, input)
    }
    fn list_policies(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<BackupPolicy>, BackupError>> {
        self.inner.list_policies(actor, administrator)
    }
    fn get_policy(&self, id: Uuid) -> BoxFuture<'_, Result<BackupPolicy, BackupError>> {
        self.inner.get_policy(id)
    }
    fn update_policy<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        row_version: i64,
        input: &'a BackupPolicyConfiguration,
    ) -> BoxFuture<'a, Result<BackupPolicy, BackupError>> {
        self.inner.update_policy(actor, id, row_version, input)
    }
    fn platform_summaries<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        platform_ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<runs::read_models::PlatformBackupSummary>, BackupError>> {
        self.inner
            .platform_summaries(actor, administrator, platform_ids)
    }
    fn rename_policy<'a>(
        &'a self,
        actor: ActorId,
        input: &'a policies::metadata::RenameBackupPolicyInput,
    ) -> BoxFuture<'a, Result<BackupPolicy, BackupError>> {
        self.inner.rename_policy(actor, input)
    }
    fn update_policy_description<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        description: Option<&'a str>,
    ) -> BoxFuture<'a, Result<BackupPolicy, BackupError>> {
        self.inner.update_policy_description(actor, id, description)
    }
    fn archive_policy(&self, id: Uuid) -> BoxFuture<'_, Result<(), BackupError>> {
        self.inner.archive_policy(id)
    }
    fn enqueue_backup(
        &self,
        actor: ActorId,
        policy_id: Uuid,
        trigger: &str,
        expected_row_version: Option<i64>,
    ) -> BoxFuture<'_, Result<BackupRun, BackupError>> {
        self.inner
            .enqueue_backup(actor, policy_id, trigger, expected_row_version)
    }
    fn enqueue_webhook<'a>(
        &'a self,
        policy_id: Uuid,
        expected_webhook: &'a citadel_primitives::WebhookConfig,
    ) -> BoxFuture<'a, Result<BackupRun, BackupError>> {
        self.inner.enqueue_webhook(policy_id, expected_webhook)
    }
    fn list_scheduled_policies(&self) -> BoxFuture<'_, Result<Vec<BackupPolicy>, BackupError>> {
        self.inner.list_scheduled_policies()
    }
    fn enqueue_scheduled_backup(
        &self,
        policy_id: Uuid,
        scheduled_minute: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<bool, BackupError>> {
        self.inner
            .enqueue_scheduled_backup(policy_id, scheduled_minute)
    }
    fn claim_backup(
        &self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<BackupClaim>, BackupError>> {
        self.claims.fetch_add(1, Ordering::SeqCst);
        self.inner.claim_backup(stale_before)
    }
    fn prepare_backup_items<'a>(
        &'a self,
        claim: &'a BackupClaim,
        plan: &'a BackupSourcePlan,
    ) -> BoxFuture<'a, Result<(), BackupError>> {
        if self.mode == 6 {
            return Box::pin(async {
                Err(BackupError::Conflict("source already reserved".into()))
            });
        }
        self.inner.prepare_backup_items(claim, plan)
    }
    fn finish_backup<'a>(
        &'a self,
        claim: &'a BackupClaim,
        result: &'a BackupExecutionResult,
    ) -> BoxFuture<'a, Result<bool, BackupError>> {
        Box::pin(async move {
            let attempt = self.finishes.fetch_add(1, Ordering::SeqCst);
            if self.mode == 4 {
                return Err(BackupError::Conflict("definitive fixture rejection".into()));
            }
            self.entered.notify_one();
            assert!(
                self.cleaned.load(Ordering::SeqCst),
                "executor cleanup must precede finish"
            );
            if attempt == 0 && self.mode == 0 {
                self.resume.notified().await;
            }
            if self.mode == 3 || (matches!(self.mode, 0 | 6) && attempt == 0) {
                return Err(BackupError::Storage("injected before commit".into()));
            }
            if self.mode == 2 && attempt == 0 {
                citadel_adapters::persistence::postgres::maintenance::recover_on_startup(
                    &self.pool,
                )
                .await
                .unwrap();
            }

            let committed = self.inner.finish_backup(claim, result).await?;
            if self.mode == 1 && attempt == 0 {
                return Err(BackupError::Storage(
                    "injected lost commit acknowledgment".into(),
                ));
            }
            Ok(committed)
        })
    }
    fn list_runs(
        &self,
        actor: ActorId,
        administrator: bool,
        policy_id: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'_, Result<Vec<BackupRun>, BackupError>> {
        self.inner.list_runs(actor, administrator, policy_id, limit)
    }
    fn get_run(&self, id: Uuid) -> BoxFuture<'_, Result<BackupRun, BackupError>> {
        self.inner.get_run(id)
    }
    fn backup_logs(&self, id: Uuid) -> BoxFuture<'_, Result<Vec<BackupLog>, BackupError>> {
        self.inner.backup_logs(id)
    }
    fn cancel_backup(&self, id: Uuid) -> BoxFuture<'_, Result<bool, BackupError>> {
        self.inner.cancel_backup(id)
    }
    fn enqueue_restore(
        &self,
        request: BackupRestoreRequest,
    ) -> BoxFuture<'_, Result<BackupRestoreRun, BackupError>> {
        self.inner.enqueue_restore(request)
    }
    fn claim_restore(
        &self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<RestoreClaim>, BackupError>> {
        self.claims.fetch_add(1, Ordering::SeqCst);
        self.inner.claim_restore(stale_before)
    }
    fn finish_restore<'a>(
        &'a self,
        claim: &'a RestoreClaim,
        result: &'a RestoreExecutionResult,
    ) -> BoxFuture<'a, Result<bool, BackupError>> {
        Box::pin(async move {
            let attempt = self.finishes.fetch_add(1, Ordering::SeqCst);
            if self.mode == 4 {
                return Err(BackupError::Conflict("definitive fixture rejection".into()));
            }
            self.entered.notify_one();
            assert!(
                self.cleaned.load(Ordering::SeqCst),
                "executor cleanup must precede finish"
            );
            if attempt == 0 && self.mode == 0 {
                self.resume.notified().await;
            }
            if self.mode == 3 || (matches!(self.mode, 0 | 6) && attempt == 0) {
                return Err(BackupError::Storage("injected before commit".into()));
            }
            if self.mode == 2 && attempt == 0 {
                citadel_adapters::persistence::postgres::maintenance::recover_on_startup(
                    &self.pool,
                )
                .await
                .unwrap();
            }

            let committed = self.inner.finish_restore(claim, result).await?;
            if self.mode == 1 && attempt == 0 {
                return Err(BackupError::Storage(
                    "injected lost commit acknowledgment".into(),
                ));
            }
            Ok(committed)
        })
    }
    fn list_restores(
        &self,
        actor: ActorId,
        administrator: bool,
        backup_run_id: Option<Uuid>,
        policy_id: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'_, Result<Vec<BackupRestoreRun>, BackupError>> {
        self.inner
            .list_restores(actor, administrator, backup_run_id, policy_id, limit)
    }
    fn get_restore(&self, id: Uuid) -> BoxFuture<'_, Result<BackupRestoreRun, BackupError>> {
        self.inner.get_restore(id)
    }
    fn restore_logs(&self, id: Uuid) -> BoxFuture<'_, Result<Vec<BackupLog>, BackupError>> {
        self.inner.restore_logs(id)
    }
    fn cancel_restore(&self, id: Uuid) -> BoxFuture<'_, Result<bool, BackupError>> {
        self.inner.cancel_restore(id)
    }
}

struct Runtime {
    calls: AtomicUsize,
    plans: AtomicUsize,
    cleanup: AtomicUsize,
    cleaned: Arc<AtomicBool>,
    started: tokio::sync::Notify,
    mode: usize,
}
impl Runtime {
    async fn execute(&self, cancel: &CancellationToken) {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.started.notify_one();
        if self.mode == 5 {
            cancel.cancelled().await;
        }
        tokio::task::yield_now().await;
        self.cleanup.fetch_add(1, Ordering::SeqCst);
        self.cleaned.store(true, Ordering::SeqCst);
    }
}
impl BackupExecutor for Runtime {
    fn repository<'a>(
        &'a self,
        _: &'a BackupRepository,
        _operation_id: Uuid,
        _: &'a str,
        _: &'a str,
        _: Option<Uuid>,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<BackupLog>, BackupError>> {
        unreachable!()
    }
    fn backup<'a>(
        &'a self,
        _: &'a BackupClaim,
        plan: &'a BackupSourcePlan,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, BackupExecutionResult> {
        Box::pin(async move {
            self.execute(cancel).await;
            {
                let mut result = super::success_result("a", Some(plan.items[0].id));
                result.logs.push(BackupLog {
                    stream: "stdout".into(),
                    message: "backed up once".into(),
                });
                result
            }
        })
    }
    fn restore<'a>(
        &'a self,
        _: &'a RestoreClaim,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, RestoreExecutionResult> {
        Box::pin(async move {
            self.execute(cancel).await;
            RestoreExecutionResult {
                status: BackupRestoreStatus::Succeeded,
                exit_code: Some(0),
                error_code: None,
                error_message: None,
                logs: vec![BackupLog {
                    stream: "stdout".into(),
                    message: "restored once".into(),
                }],
            }
        })
    }
}
impl BackupSourcePlanner for Runtime {
    fn cleanup_staging<'a>(
        &'a self,
        _: &'a BackupSourcePlan,
    ) -> BoxFuture<'a, Result<(), BackupError>> {
        Box::pin(async move {
            self.cleanup.fetch_add(1, Ordering::SeqCst);
            self.cleaned.store(true, Ordering::SeqCst);
            Ok(())
        })
    }
    fn preview<'a>(
        &'a self,
        _: policies::read_models::BackupPreviewKind,
        _: Uuid,
        _: ActorId,
        _: bool,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<policies::read_models::BackupSourcePreview, BackupError>> {
        unreachable!()
    }
    fn validate_source<'a>(
        &'a self,
        _: &'a spec::BackupSourceSpec,
        _: &'a BackupRepository,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), BackupError>> {
        Box::pin(async { Ok(()) })
    }
    fn plan<'a>(
        &'a self,
        claim: &'a BackupClaim,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<BackupSourcePlan, BackupError>> {
        self.plans.fetch_add(1, Ordering::SeqCst);
        let spec::BackupSourceSpec::DockerVolume {
            platform_id,
            volume_name,
            ..
        } = &claim.policy.source
        else {
            unreachable!()
        };
        Box::pin(async move {
            Ok(BackupSourcePlan {
                display_name: "fixture".into(),
                items: vec![BackupSourceItem::new(
                    *platform_id,
                    volume_name.clone(),
                    None,
                    None,
                )],
                warnings: vec![],
                local_directory: None,
            })
        })
    }
}
struct Authorization;
impl BackupRunAuthorizer for Authorization {
    fn authorize_backup<'a>(
        &'a self,
        _: &'a BackupClaim,
    ) -> BoxFuture<'a, Result<(), BackupError>> {
        Box::pin(async { Ok(()) })
    }
    fn authorize_plan<'a>(
        &'a self,
        _: &'a BackupClaim,
        _: &'a BackupSourcePlan,
    ) -> BoxFuture<'a, Result<(), BackupError>> {
        Box::pin(async { Ok(()) })
    }
    fn authorize_restore<'a>(
        &'a self,
        _: &'a RestoreClaim,
    ) -> BoxFuture<'a, Result<(), BackupError>> {
        Box::pin(async { Ok(()) })
    }
}

pub(super) async fn verify(pool: &sqlx::PgPool, actor: ActorId, platform: Uuid, repository: Uuid) {
    let inner = PostgresBackupPersistence::new(pool.clone());
    let mut input = BackupPolicyConfiguration {
        name: format!("completion-{}", Uuid::now_v7()),
        description: None,
        source: serde_json::from_value(
            json!({"$type":"DockerVolume","platformId":platform,"volumeName":"completion-volume"}),
        )
        .unwrap(),
        backup_repository_id: repository,
        enabled: true,
        cron: None,
        time_zone: None,
        webhook: None,
        keep_last_successful: Some(50),
        timeout_seconds: Some(60),
        alert_on_failure: false,
        run_as_actor_id: None,
        tag_ids: vec![],
    };
    input.validate(actor).unwrap();
    let policy = inner.create_policy(actor, &input).await.unwrap();
    let mut source = None;
    for restore in [false, true] {
        // precommit, lost acknowledgment, recovery, backoff shutdown,
        // definitive rejection, execution shutdown, failed preparation cleanup.
        for mode in [0, 1, 2, 3, 4, 5, 6] {
            if restore && mode == 6 {
                continue;
            }
            let cleaned = Arc::new(AtomicBool::new(false));
            let store = Arc::new(Store {
                inner: PostgresBackupPersistence::new(pool.clone()),
                pool: pool.clone(),
                mode,
                claims: AtomicUsize::new(0),
                finishes: AtomicUsize::new(0),
                cleaned: cleaned.clone(),
                entered: tokio::sync::Notify::new(),
                resume: tokio::sync::Notify::new(),
            });
            let runtime = Arc::new(Runtime {
                calls: AtomicUsize::new(0),
                plans: AtomicUsize::new(0),
                cleanup: AtomicUsize::new(0),
                cleaned: cleaned.clone(),
                started: tokio::sync::Notify::new(),
                mode,
            });
            let run_id = if restore {
                let source: &BackupRun = source.as_ref().unwrap();
                store
                    .enqueue_restore(BackupRestoreRequest {
                        actor,
                        backup_run_id: source.id,
                        target_platform_id: platform,
                        target_volume_name: format!("completion-target-{mode}"),
                        target_host_path: None,
                        overwrite_existing: false,
                        target_docker_node_id: None,
                        source_backup_run_item_id: Some(source.items[0].id),
                    })
                    .await
                    .unwrap()
                    .id
            } else {
                store
                    .enqueue_backup(actor, policy.id, "Manual", None)
                    .await
                    .unwrap()
                    .id
            };
            let service = BackupService::new(
                store.clone(),
                runtime.clone(),
                runtime.clone(),
                Duration::minutes(5),
                Arc::new(Authorization),
            )
            .with_tracking_slots(1);
            let mut progress = service.subscribe_progress();
            let shutdown = CancellationToken::new();
            let observer = async {
                if matches!(mode, 0 | 3) {
                    store.entered.notified().await;
                    while let Ok(event) = progress.try_recv() {
                        assert!(
                            !citadel_backups::runs::progress::is_terminal(
                                event.status.as_deref().unwrap_or_default()
                            ),
                            "terminal progress before commit"
                        );
                    }
                    assert_eq!(store.claims.load(Ordering::SeqCst), 1);
                    // Completion retries still own the shared tracking permit.
                    let waiting = service.claim_restore(&shutdown);
                    tokio::pin!(waiting);
                    assert!(futures_util::poll!(&mut waiting).is_pending());
                    assert_eq!(store.claims.load(Ordering::SeqCst), 1);
                    if mode == 3 {
                        shutdown.cancel();
                    }
                    store.resume.notify_one();
                } else if mode == 5 {
                    runtime.started.notified().await;
                    shutdown.cancel();
                }
            };
            let process = async {
                if mode % 2 == 0 {
                    let work = if restore {
                        service.claim_restore(&shutdown).await?
                    } else {
                        service.claim_backup(&shutdown).await?
                    }
                    .expect("queued work");
                    // A claim owns capacity before its controller is spawned.
                    // The other queue shares the same limit and cannot touch SQL.
                    let waiting_token = shutdown.child_token();
                    let waiting = async {
                        if restore {
                            service.claim_backup(&waiting_token).await
                        } else {
                            service.claim_restore(&waiting_token).await
                        }
                    };
                    tokio::pin!(waiting);
                    assert!(futures_util::poll!(&mut waiting).is_pending());
                    assert_eq!(store.claims.load(Ordering::SeqCst), 1);
                    waiting_token.cancel();
                    assert!(waiting.await?.is_none());
                    service.execute_claim(work, &shutdown).await
                } else if restore {
                    service.process_restore(&shutdown).await
                } else {
                    service.process_backup(&shutdown).await
                }
            };
            let (result, ()) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
                tokio::join!(process, observer)
            })
            .await
            .unwrap();
            if mode == 4 {
                assert!(matches!(result, Err(BackupError::Conflict(_))));
            } else {
                assert!(result.unwrap());
            }
            assert_eq!(runtime.calls.load(Ordering::SeqCst), usize::from(mode != 6));
            assert_eq!(
                runtime.cleanup.load(Ordering::SeqCst),
                usize::from(mode != 6)
            );
            assert_eq!(runtime.plans.load(Ordering::SeqCst), usize::from(!restore));
            assert_eq!(store.claims.load(Ordering::SeqCst), 1);
            let status = if restore {
                store.get_restore(run_id).await.unwrap().status.as_str()
            } else {
                store.get_run(run_id).await.unwrap().status.as_str()
            };
            let expected = match mode {
                2 => "Interrupted",
                3 | 4 => "Running",
                6 => "Queued",
                _ => "Succeeded",
            };
            assert_eq!(status, expected, "restore={restore}, mode={mode}");
            let leases: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM backuprepositoryleases WHERE ownerrunid=$1",
            )
            .bind(run_id)
            .fetch_one(pool)
            .await
            .unwrap();
            assert_eq!(
                leases,
                i64::from(matches!(mode, 2..=4)),
                "recovery retains ownership until external cleanup is verified"
            );
            if !restore {
                let terminal_activity: i64 = sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE eventtype='BackupRunCompleted' AND info::jsonb->>'RunId'=$1")
                    .bind(run_id.to_string()).fetch_one(pool).await.unwrap();
                assert_eq!(terminal_activity, i64::from(!matches!(mode, 3 | 4 | 6)));
            }
            let mut terminals = Vec::new();
            while let Ok(event) = progress.try_recv() {
                if citadel_backups::runs::progress::is_terminal(
                    event.status.as_deref().unwrap_or_default(),
                ) {
                    terminals.push(event.status.clone().unwrap());
                }
            }
            assert_eq!(
                terminals,
                if matches!(mode, 3 | 4 | 6) {
                    vec![]
                } else {
                    vec![expected.to_owned()]
                }
            );
            if matches!(mode, 0 | 1) {
                assert_eq!(store.finishes.load(Ordering::SeqCst), 2);
                let logs = if restore {
                    store.restore_logs(run_id).await.unwrap()
                } else {
                    store.backup_logs(run_id).await.unwrap()
                };
                assert_eq!(logs.len(), 1, "lost acknowledgment must not duplicate logs");
                if !restore {
                    let persisted = store.get_run(run_id).await.unwrap();
                    assert_eq!(
                        persisted.restic_snapshot_id.as_deref(),
                        Some("a".repeat(64).as_str())
                    );
                    assert_eq!(persisted.bytes_processed, Some(1));
                    source = Some(persisted);
                }
            }
            citadel_adapters::persistence::postgres::maintenance::recover_on_startup(pool)
                .await
                .unwrap();
            // The fake executor has finished and its cleanup was asserted above.
            // Production recovery retains these locks until executor proof arrives;
            // explicitly clean this fixture before exercising the next fault mode.
            sqlx::query("DELETE FROM backupsourceleases WHERE ownerrunid=$1")
                .bind(run_id)
                .execute(pool)
                .await
                .unwrap();
            sqlx::query("DELETE FROM backuprepositoryleases WHERE ownerrunid=$1")
                .bind(run_id)
                .execute(pool)
                .await
                .unwrap();
            sqlx::query("UPDATE backuppolicies SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL WHERE currentrunid=$1").bind(run_id).execute(pool).await.unwrap();
            if mode == 6 {
                store.cancel_backup(run_id).await.unwrap();
            }
        }
    }
    sqlx::query("DELETE FROM activityevents WHERE resourceid=$1")
        .bind(policy.id)
        .execute(pool)
        .await
        .unwrap();
    // These additional fixtures must not alter the parent test's catalog/counts.
    sqlx::query("DELETE FROM backuprestoreruns WHERE backuprunid IN (SELECT id FROM backupruns WHERE backuppolicyid=$1)").bind(policy.id).execute(pool).await.unwrap();
    sqlx::query("DELETE FROM backupruns WHERE backuppolicyid=$1")
        .bind(policy.id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM backuppolicies WHERE id=$1")
        .bind(policy.id)
        .execute(pool)
        .await
        .unwrap();
}
