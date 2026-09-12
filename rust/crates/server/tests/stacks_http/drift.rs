use super::*;
use citadel_stacks::StackStore;

pub(super) struct Entitlements(pub bool);
impl citadel_stacks::StackEntitlements for Entitlements {
    fn automated_operations(&self) -> BoxFuture<'_, Result<bool, StackError>> {
        Box::pin(async { Ok(false) })
    }
    fn operational_guardrails(&self) -> BoxFuture<'_, Result<bool, StackError>> {
        Box::pin(async move { Ok(self.0) })
    }
}

pub(super) async fn verify(
    app: &Router,
    pool: &sqlx::PgPool,
    stacks: &StackService,
    admin: &ActorPrincipal,
    id: Uuid,
    runtime: &CompletingStackRuntime,
) {
    let store = PostgresStackStore::new(pool.clone());
    let original = store
        .get_authorized(admin.actor_id, true, id)
        .await
        .unwrap();
    let detail = response_json(
        request(
            app,
            Method::GET,
            &format!("/api/v1/stacks/{id}"),
            Some(admin.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(detail["capabilities"]["canViewResourceBindings"], true);
    assert!(detail["capabilities"].get("canResourceBindings").is_none());
    runtime.runtime_state.store(1, Ordering::Relaxed);
    let report = response_json(
        request(
            app,
            Method::GET,
            &format!("/api/v1/stacks/{id}/drift"),
            Some(admin.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(report["hasStructuralDrift"], false);
    assert_eq!(
        report["drifts"],
        json!([{"$type":"ContainerStopped","containerId":"docker-web","serviceName":"web"}])
    );
    let result = stacks.monitor_drift(None, 100).await.unwrap();
    assert!(result.failures.is_empty(), "{:?}", result.failures);
    let degraded = store
        .get_authorized(admin.actor_id, true, id)
        .await
        .unwrap();
    assert_eq!(degraded.status, StackReleaseStatus::Degraded);
    assert_eq!(
        degraded.latest_activity_view.as_ref().unwrap()["info"]["$type"],
        "StackDriftDetected"
    );
    assert!(degraded.latest_activity_view.as_ref().unwrap()["info"]["reason"].is_string());
    stacks.monitor_drift(None, 100).await.unwrap();
    let repeated = store
        .get_authorized(admin.actor_id, true, id)
        .await
        .unwrap();
    assert_eq!(
        repeated.row_version, degraded.row_version,
        "same drift must not rewrite or duplicate activity"
    );
    assert!(
        !store
            .record_drift(
                &original,
                StackReleaseStatus::Healthy,
                citadel_domain::ActivityEventInfo::StackDriftResolved {
                    previous_fingerprint: "stale".into()
                }
            )
            .await
            .unwrap()
    );
    assert_eq!(
        store
            .get_authorized(admin.actor_id, true, id)
            .await
            .unwrap()
            .status,
        StackReleaseStatus::Degraded
    );
    runtime.runtime_state.store(2, Ordering::Relaxed);
    let result = stacks.monitor_drift(None, 100).await.unwrap();
    assert!(result.failures.is_empty(), "{:?}", result.failures);
    let recovered = store
        .get_authorized(admin.actor_id, true, id)
        .await
        .unwrap();
    assert_eq!(recovered.status, StackReleaseStatus::Healthy);
    assert_eq!(
        recovered.latest_activity_view.as_ref().unwrap()["info"]["$type"],
        "StackDriftResolved"
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE resourceid=$1 AND eventtype IN ('StackDriftDetected','StackDriftResolved')").bind(id).fetch_one(pool).await.unwrap();
    assert_eq!(count, 2);
    // Port: StackDriftMonitorJobTests event AutoFix, DetectOnly and intentional stop.
    runtime.runtime_state.store(1, Ordering::Relaxed);
    let calls = runtime.reconcile_calls.load(Ordering::Relaxed);
    stacks.monitor_container_event(id).await.unwrap();
    assert_eq!(runtime.reconcile_calls.load(Ordering::Relaxed), calls);
    let mut policy = recovered.drift_policy.clone();
    policy.mode = citadel_stacks::StackDriftMode::AutoFix;
    stacks
        .update_drift_policy(admin.actor_id, true, id, policy.clone())
        .await
        .unwrap();
    let denied_runtime = Arc::new(CompletingStackRuntime::default());
    let unlicensed = StackService::new(
        Arc::new(PostgresStackStore::new(pool.clone())),
        denied_runtime.clone(),
        Arc::new(FixtureBindings),
        Arc::new(NoopStackChangeNotifier),
        CancellationToken::new(),
    )
    .with_entitlements(Arc::new(Entitlements(false)));
    unlicensed.monitor_container_event(id).await.unwrap();
    assert_eq!(
        unlicensed.monitor_drift(None, 100).await.unwrap().checked,
        0
    );
    assert_eq!(denied_runtime.reconcile_calls.load(Ordering::Relaxed), 0);
    assert_eq!(runtime.reconcile_calls.load(Ordering::Relaxed), calls);
    stacks.monitor_container_event(id).await.unwrap();
    assert_eq!(runtime.reconcile_calls.load(Ordering::Relaxed), calls + 1);
    for status in ["Stopped", "Paused", "Created"] {
        sqlx::query("UPDATE stackreleases SET status=$2 WHERE id=(SELECT currentstackreleaseid FROM stacks WHERE id=$1)")
            .bind(id).bind(status).execute(pool).await.unwrap();
        stacks.monitor_container_event(id).await.unwrap();
        assert_eq!(
            runtime.reconcile_calls.load(Ordering::Relaxed),
            calls + 1,
            "{status}"
        );
    }
    sqlx::query("UPDATE stackreleases SET status='Healthy' WHERE id=(SELECT currentstackreleaseid FROM stacks WHERE id=$1)")
        .bind(id).execute(pool).await.unwrap();
    stacks
        .update_drift_policy(admin.actor_id, true, id, recovered.drift_policy)
        .await
        .unwrap();
    let exhausted = stacks
        .monitor_drift(Some(Uuid::from_u128(u128::MAX)), 1)
        .await
        .unwrap();
    assert_eq!(
        exhausted.checked, 0,
        "an exhausted page must not wrap to the first page"
    );
    runtime.runtime_state.store(0, Ordering::Relaxed);
    bindings_permissions(app, pool, id).await;
}

async fn bindings_permissions(app: &Router, pool: &sqlx::PgPool, id: Uuid) {
    let actor = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'User')")
        .bind(actor)
        .execute(pool)
        .await
        .unwrap();
    let reader = ActorPrincipal {
        subject_id: actor,
        actor_id: ActorId::new(actor),
        name: "bindings-reader".into(),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec![],
    };
    let access = Uuid::now_v7();
    sqlx::query("INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,1,$3,2,0)").bind(access).bind(actor).bind(id).execute(pool).await.unwrap();
    for (specific, allowed) in [(0, false), (32, true)] {
        sqlx::query("UPDATE resourceaccesses SET specificpermissions=$2 WHERE id=$1")
            .bind(access)
            .bind(specific)
            .execute(pool)
            .await
            .unwrap();
        let detail = response_json(
            request(
                app,
                Method::GET,
                &format!("/api/v1/stacks/{id}"),
                Some(reader.clone()),
                None,
            )
            .await,
        )
        .await;
        assert_eq!(detail["capabilities"]["canViewResourceBindings"], allowed);
    }
    sqlx::query("DELETE FROM resourceaccesses WHERE id=$1")
        .bind(access)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM actors WHERE id=$1")
        .bind(actor)
        .execute(pool)
        .await
        .unwrap();
}
