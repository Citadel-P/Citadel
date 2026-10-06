//! HTTP/PostgreSQL ports of CheckStackUpdates: access, persisted update state,
//! read failures and edit/Apply races, without introducing a frontend workaround.
use super::*;
use citadel_stacks::{StackUpdateScanner, StackUpdateState};

pub(super) struct Scanner {
    pool: sqlx::PgPool,
    mode: AtomicU8,
    calls: std::sync::atomic::AtomicUsize,
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
}
impl Scanner {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self {
            pool,
            mode: AtomicU8::new(0),
            calls: Default::default(),
            entered: Default::default(),
            release: Default::default(),
        }
    }
}
impl StackUpdateScanner for Scanner {
    fn scan<'a>(
        &'a self,
        stack: &'a citadel_stacks::Stack,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackUpdateState, StackError>> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            match self.mode.load(Ordering::Relaxed) {
                1 => {
                    sqlx::query("UPDATE stacks SET rowversion=rowversion+1 WHERE id=$1")
                        .bind(stack.id)
                        .execute(&self.pool)
                        .await
                        .unwrap();
                }
                2 => return Err(StackError::Runtime("Registry is unavailable.".into())),
                3 => {
                    self.entered.notify_one();
                    self.release.notified().await;
                }
                _ => {}
            }
            if matches!(stack.spec, Some(citadel_stacks::StackSpec::Git { .. })) {
                return Ok(StackUpdateState::Git {
                    recreate_stack_on_new_image_state: Default::default(),
                    recreate_stack_on_new_commit_state:
                        citadel_stacks::RecreateStackOnNewCommitState {
                            current_commit_sha: "a".repeat(40),
                            remote_commit_sha: Some("b".repeat(40)),
                            last_checked_at: chrono::Utc::now(),
                        },
                });
            }
            Ok(StackUpdateState::WebEditor {
                recreate_stack_on_new_image_state: citadel_stacks::RecreateStackOnNewImageState {
                    auto_update_states: vec![citadel_stacks::ImageUpdateState {
                        service_name: "web".into(),
                        image_name: "nginx:alpine".into(),
                        current_digest: "sha256:old".into(),
                        remote_digest: Some("sha256:new".into()),
                        last_checked_at: chrono::Utc::now(),
                        update_available: true,
                    }],
                },
            })
        })
    }
}
pub(super) async fn verify(
    app: &Router,
    pool: &sqlx::PgPool,
    stacks: &StackService,
    admin: &ActorPrincipal,
    id: Uuid,
    scanner: &Scanner,
) {
    let url = format!("/api/v1/stacks/{id}/check-updates");
    assert_eq!(
        request(app, Method::POST, &url, None, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    let mut denied = admin.clone();
    denied.roles.clear();
    denied.actor_id = ActorId::new(Uuid::now_v7());
    assert_eq!(
        request(app, Method::POST, &url, Some(denied), None)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let response = request(app, Method::POST, &url, Some(admin.clone()), None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["controlState"], "Idle");
    assert_eq!(
        body["stackUpdateState"]["recreateStackOnNewImageState"]["autoUpdateStates"][0]["updateAvailable"],
        true
    );
    let before: Value = sqlx::query_scalar("SELECT stackupdatestate FROM stacks WHERE id=$1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap();
    for (mode, status) in [(1, StatusCode::CONFLICT), (2, StatusCode::BAD_GATEWAY)] {
        scanner.mode.store(mode, Ordering::Relaxed);
        assert_eq!(
            request(app, Method::POST, &url, Some(admin.clone()), None)
                .await
                .status(),
            status
        );
        assert_eq!(
            sqlx::query_scalar::<_, Value>("SELECT stackupdatestate FROM stacks WHERE id=$1")
                .bind(id)
                .fetch_one(pool)
                .await
                .unwrap(),
            before
        );
    }
    let version = body["rowVersion"].as_i64().unwrap();
    assert!(
        stacks
            .apply_versioned(admin.actor_id, true, id, version)
            .await
            .is_err(),
        "a configuration checked before an edit must not be automatically applied"
    );
    scanner.mode.store(0, Ordering::Relaxed);
    verify_single_flight(stacks, admin.actor_id, id, scanner).await;
    verify_selected_apply(pool, admin, id).await;
    verify_update_producers(pool, id, false).await;
    verify_git_update_producers(pool, admin, id).await;
}

async fn verify_single_flight(stacks: &StackService, actor: ActorId, id: Uuid, scanner: &Scanner) {
    for cancel in [false, true] {
        scanner.mode.store(3, Ordering::Relaxed);
        let before = scanner.calls.load(Ordering::SeqCst);
        let token = CancellationToken::new();
        let first = stacks.check_updates(actor, true, id, &token);
        tokio::pin!(first);
        tokio::select! {
            result = &mut first => panic!("scan unexpectedly finished: {result:?}"),
            () = scanner.entered.notified() => {},
        }
        for _ in 0..8 {
            assert!(matches!(
                stacks
                    .check_updates(actor, true, id, &CancellationToken::new())
                    .await,
                Err(StackError::Conflict(_))
            ));
        }
        assert_eq!(scanner.calls.load(Ordering::SeqCst), before + 1);
        if cancel {
            token.cancel();
            assert!(matches!(first.await, Err(StackError::Cancelled)));
        } else {
            scanner.release.notify_one();
            first.await.unwrap();
        }
        scanner.mode.store(0, Ordering::Relaxed);
        stacks
            .check_updates(actor, true, id, &CancellationToken::new())
            .await
            .unwrap();
    }
    // Dropping an HTTP-like future must also release both the key and budget.
    scanner.mode.store(3, Ordering::Relaxed);
    {
        let token = CancellationToken::new();
        let first = stacks.check_updates(actor, true, id, &token);
        tokio::pin!(first);
        tokio::select! {
            result = &mut first => panic!("scan unexpectedly finished: {result:?}"),
            () = scanner.entered.notified() => {},
        }
    }
    scanner.mode.store(0, Ordering::Relaxed);
    stacks
        .check_updates(actor, true, id, &CancellationToken::new())
        .await
        .unwrap();
}

struct GitSource;
impl citadel_stacks::StackSourceMaterializerPort for GitSource {
    fn materialize<'a>(
        &'a self,
        claim: &'a StackOperationClaim,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackApplySource, StackError>> {
        Box::pin(async move {
            if claim.operation == "Drift" {
                let citadel_stacks::StackSpec::Git { commit_sha, .. } = &claim.spec else {
                    panic!("expected Git")
                };
                assert_eq!(commit_sha.as_deref(), Some("c".repeat(40).as_str()));
            }
            Ok(StackApplySource {
                files: vec![citadel_stacks::StackSourceFile {
                    relative_path: "compose.yml".into(),
                    content: b"services:\n  web:\n    image: nginx:alpine\n".to_vec(),
                }],
                compose_paths: vec!["compose.yml".into()],
                env_file_paths: vec![],
                working_directory: ".".into(),
                labels_override_path: None,
                resolved_commit_sha: Some("c".repeat(40)),
            })
        })
    }
}

async fn verify_git_update_producers(pool: &sqlx::PgPool, admin: &ActorPrincipal, existing: Uuid) {
    use citadel_stacks::{StackRepository, StackSpec};
    let store = Arc::new(PostgresStackRepository::new(pool.clone()));
    let platform = store
        .get_authorized(admin.actor_id, true, existing)
        .await
        .unwrap()
        .platform_id;
    let repo = Uuid::now_v7();
    sqlx::query("INSERT INTO gitrepositories(id,createdbyactorid,defaultbranch,name,status,syncmode,url,controlstate) VALUES($1,$2,'main',$3,'Healthy','Manual','https://example.test/repository.git','Idle')")
        .bind(repo).bind(admin.actor_id.value()).bind(repo.to_string()).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO gitrepositoryrefs(id,branch,gitrepositoryid,lastsyncedat,status,resolvedcommitsha) VALUES($1,'main',$2,now(),'Healthy',$3)")
        .bind(Uuid::now_v7()).bind(repo).bind("b".repeat(40)).execute(pool).await.unwrap();
    let input: citadel_stacks::CreateStack = serde_json::from_value::<citadel_server::api::resources::stacks::requests::CreateStackInput>(json!({"name":format!("git-updates-{repo}"),"platformId":platform,"stackSource":"Git",
        "spec":{"$type":"Git","gitRepoId":repo,"branch":"main","composePaths":["compose.yml"],"updateBehavior":"Notify"}})).unwrap().try_into().unwrap();
    let stack = store.create(admin.actor_id, true, &input).await.unwrap();
    let runtime = Arc::new(CompletingStackRuntime::default());
    let alerts = Arc::new(alert_sink::RecordedAlerts::default());
    let service = StackService::new(
        Arc::new(citadel_server::tasks::stacks::TrackedStackTasks::new(
            citadel_runtime::DynamicTasks::new(tokio_util::sync::CancellationToken::new()),
        )),
        store.clone(),
        runtime.clone(),
        Arc::new(FixtureBindings),
        Arc::new(NoopStackChangeNotifier),
        CancellationToken::new(),
    )
    .with_update_scanner(Arc::new(Scanner::new(pool.clone())))
    .with_source_materializer(Arc::new(GitSource))
    .with_entitlements(Arc::new(Entitlements {
        automatic: true.into(),
        guardrails: true.into(),
    }))
    .with_alerts(alerts.clone());
    for (behavior, failed, kind) in [
        ("Notify", false, "StackGitUpdateAvailable"),
        ("StackAutoDeploy", false, "StackGitAutoUpdated"),
        ("StackAutoDeploy", true, "StackGitAutoDeployFailed"),
    ] {
        let spec: StackSpec = serde_json::from_value(json!({"$type":"Git","gitRepoId":repo,"branch":"main","composePaths":["compose.yml"],"updateBehavior":behavior})).unwrap();
        sqlx::query("UPDATE stackreleases SET status='Healthy',spec=$2 WHERE id=$1")
            .bind(stack.current_stack_release_id)
            .bind(spec.to_storage_value().unwrap())
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("UPDATE gitrepositoryrefs SET lastsyncedat=now()+interval '1 minute' WHERE gitrepositoryid=$1")
            .bind(repo).execute(pool).await.unwrap();
        runtime
            .apply_failure
            .store(u8::from(failed), Ordering::Relaxed);
        alerts.0.lock().unwrap().clear();
        let calls = runtime.apply_calls.lock().unwrap().len();
        service
            .run_update_checks(false, &CancellationToken::new())
            .await
            .unwrap();
        let persisted = store
            .get_authorized(admin.actor_id, true, stack.id)
            .await
            .unwrap();
        if kind == "StackGitAutoUpdated" {
            runtime.runtime_state.store(2, Ordering::Relaxed);
            assert!(
                !service
                    .drift(admin.actor_id, true, stack.id)
                    .await
                    .unwrap()
                    .has_drift
            );
        }
        let observations = alerts.0.lock().unwrap();
        let observation = observations
            .iter()
            .find(|a| a.resource_id == stack.id && a.alert_type == kind)
            .expect(kind);
        assert_eq!(observation.info["RemoteCommitSha"], "b".repeat(40));
        if kind == "StackGitAutoUpdated" {
            assert_eq!(observation.info["UpdatedCommitSha"], "c".repeat(40));
            assert_eq!(
                persisted.source.as_ref().unwrap().resolved_commit_sha,
                "c".repeat(40)
            );
        } else {
            assert!(observation.info["UpdatedCommitSha"].is_null());
        }
        if behavior == "Notify" {
            assert_eq!(runtime.apply_calls.lock().unwrap().len(), calls);
        }
    }
    sqlx::query("DELETE FROM stacks WHERE id=$1")
        .bind(stack.id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM gitrepositories WHERE id=$1")
        .bind(repo)
        .execute(pool)
        .await
        .unwrap();
}

struct Entitlements {
    automatic: std::sync::atomic::AtomicBool,
    guardrails: std::sync::atomic::AtomicBool,
}
impl citadel_stacks::StackEntitlements for Entitlements {
    fn automated_operations(&self) -> BoxFuture<'_, Result<bool, StackError>> {
        Box::pin(async { Ok(self.automatic.load(Ordering::Relaxed)) })
    }
    fn operational_guardrails(&self) -> BoxFuture<'_, Result<bool, StackError>> {
        Box::pin(async { Ok(self.guardrails.load(Ordering::Relaxed)) })
    }
}
pub(super) async fn verify_update_producers(pool: &sqlx::PgPool, id: Uuid, swarm: bool) {
    let runtime = Arc::new(CompletingStackRuntime::default());
    let alerts = Arc::new(alert_sink::RecordedAlerts::default());
    let entitlements = Arc::new(Entitlements {
        automatic: std::sync::atomic::AtomicBool::new(true),
        guardrails: std::sync::atomic::AtomicBool::new(true),
    });
    let service = StackService::new(
        Arc::new(citadel_server::tasks::stacks::TrackedStackTasks::new(
            citadel_runtime::DynamicTasks::new(tokio_util::sync::CancellationToken::new()),
        )),
        Arc::new(PostgresStackRepository::new(pool.clone())),
        runtime.clone(),
        Arc::new(FixtureBindings),
        Arc::new(NoopStackChangeNotifier),
        CancellationToken::new(),
    )
    .with_update_scanner(Arc::new(Scanner::new(pool.clone())))
    .with_entitlements(entitlements.clone())
    .with_alerts(alerts.clone());
    for (behavior, guardrails, rejected, kind) in [
        ("Disabled", true, false, ""),
        ("Notify", true, false, "StackImageUpdateAvailable"),
        ("StackAutoDeploy", false, false, "StackImageUpdateAvailable"),
        (
            "ServiceAutoDeploy",
            false,
            false,
            "StackImageUpdateAvailable",
        ),
        ("ServiceAutoDeploy", true, false, "StackServiceAutoUpdated"),
        ("StackAutoDeploy", true, false, "StackAutoUpdated"),
        (
            "ServiceAutoDeploy",
            true,
            true,
            "StackServiceAutoDeployFailed",
        ),
        ("StackAutoDeploy", true, true, "StackAutoDeployFailed"),
    ] {
        sqlx::query("UPDATE stackreleases SET status='Healthy',spec=jsonb_set(spec::jsonb,'{UpdateBehavior}',to_jsonb($2::text))::json WHERE id=(SELECT currentstackreleaseid FROM stacks WHERE id=$1)")
            .bind(id).bind(behavior).execute(pool).await.unwrap();
        entitlements.guardrails.store(guardrails, Ordering::Relaxed);
        // Notify and entitlement fallback must keep scanning on Community.
        entitlements
            .automatic
            .store(guardrails && behavior != "Notify", Ordering::Relaxed);
        runtime
            .apply_failure
            .store(u8::from(rejected), Ordering::Relaxed);
        let calls = runtime.apply_calls.lock().unwrap().len();
        alerts.0.lock().unwrap().clear();
        service
            .run_update_checks(true, &CancellationToken::new())
            .await
            .unwrap();
        let observations = alerts.0.lock().unwrap();
        if behavior == "Disabled" {
            assert!(!observations.iter().any(|a| a.resource_id == id));
            assert_eq!(runtime.apply_calls.lock().unwrap().len(), calls);
            continue;
        }
        // Persisted legacy service-only settings must not trigger unsupported Swarm mutations.
        let kind = if swarm && behavior == "ServiceAutoDeploy" {
            "StackImageUpdateAvailable"
        } else {
            kind
        };
        let alert = observations
            .iter()
            .find(|a| a.resource_id == id && a.alert_type == kind)
            .expect(kind);
        assert_eq!(alert.info["Updates"][0]["ServiceName"], "web");
        assert_eq!(alert.resource_type, "Stack");
        assert!(!alert.info.to_string().contains("private-runtime-error"));
        let apply_calls = runtime.apply_calls.lock().unwrap();
        if kind == "StackImageUpdateAvailable" {
            assert_eq!(apply_calls.len(), calls);
        } else if !rejected {
            assert_eq!(apply_calls.len(), calls + 1);
            let names = &apply_calls.last().unwrap().service_names;
            if behavior == "ServiceAutoDeploy" {
                assert_eq!(names, &["web"]);
            } else {
                assert!(names.is_empty());
            }
        }
    }
    sqlx::query("UPDATE stackreleases SET status='Healthy',spec=jsonb_set(spec::jsonb,'{UpdateBehavior}','\"Disabled\"'::jsonb)::json WHERE id=(SELECT currentstackreleaseid FROM stacks WHERE id=$1)")
        .bind(id).execute(pool).await.unwrap();
}

async fn verify_selected_apply(pool: &sqlx::PgPool, admin: &ActorPrincipal, id: Uuid) {
    use citadel_stacks::{StackApplyOptions, StackRepository};
    let store = Arc::new(PostgresStackRepository::new(pool.clone()));
    let runtime = Arc::new(CompletingStackRuntime::default());
    let service = StackService::new(
        Arc::new(citadel_server::tasks::stacks::TrackedStackTasks::new(
            citadel_runtime::DynamicTasks::new(tokio_util::sync::CancellationToken::new()),
        )),
        store.clone(),
        runtime.clone(),
        Arc::new(FixtureBindings),
        Arc::new(NoopStackChangeNotifier),
        CancellationToken::new(),
    );
    let before = store
        .get_authorized(admin.actor_id, true, id)
        .await
        .unwrap();
    assert!(
        service
            .apply_selected(
                admin.actor_id,
                true,
                id,
                StackApplyOptions {
                    expected_version: Some(before.row_version),
                    service_names: vec!["missing".into()]
                }
            )
            .await
            .is_err()
    );
    let mut progress = service
        .apply_selected(
            admin.actor_id,
            true,
            id,
            StackApplyOptions {
                expected_version: Some(before.row_version),
                service_names: vec!["web".into()],
            },
        )
        .await
        .unwrap();
    let mut completed = false;
    while let Some(item) = progress.recv().await {
        completed |= item.exit_code == Some(0);
    }
    assert!(completed);
    assert_eq!(
        runtime.apply_calls.lock().unwrap()[0].service_names,
        vec!["web"]
    );
    let snapshot = store
        .get_authorized(admin.actor_id, true, id)
        .await
        .unwrap();
    let claim = store
        .claim_apply_versioned(
            admin.actor_id,
            true,
            id,
            None,
            None,
            StackApplyOptions {
                expected_version: Some(snapshot.row_version),
                service_names: vec!["web".into()],
            },
        )
        .await
        .unwrap();
    store
        .fail_apply(admin.actor_id, &claim, "transport disconnected", true)
        .await
        .unwrap();
    assert!(
        store
            .complete_apply(admin.actor_id, &claim, &healthy(), &[], None, None)
            .await
            .is_err(),
        "a completion with an old operation version cannot win recovery"
    );
    let recovery = store
        .stale_apply_claims(i64::MAX, 25)
        .await
        .unwrap()
        .into_iter()
        .find(|(_, claim)| claim.stack_id == id)
        .unwrap()
        .1;
    assert_eq!(recovery.service_names, vec!["web"]);
    store
        .complete_apply(admin.actor_id, &recovery, &healthy(), &[], None, None)
        .await
        .unwrap();
    assert!(
        sqlx::query_scalar::<_, Vec<String>>("SELECT applyservices FROM stacks WHERE id=$1")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap()
            .is_empty()
    );
}
