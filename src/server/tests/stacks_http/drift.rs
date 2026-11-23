use super::*;
use citadel_stacks::StackRepository;

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
    let store = PostgresStackRepository::new(pool.clone());
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
        degraded.latest_activity.as_ref().unwrap().info["$type"],
        "StackDriftDetected"
    );
    assert!(degraded.latest_activity.as_ref().unwrap().info["Reason"].is_string());
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
                &original.resource.into(),
                StackReleaseStatus::Healthy,
                citadel_activities::ActivityEventInfo::StackDriftResolved {
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
        recovered.latest_activity.as_ref().unwrap().info["$type"],
        "StackDriftResolved"
    );
    // A release input can change in place while its ID and parent rowversion stay the same.
    let stale = store
        .drift_monitor_candidates(None, 100)
        .await
        .unwrap()
        .into_iter()
        .find(|s| s.id == id)
        .unwrap();
    sqlx::query("UPDATE stackreleases SET spec=jsonb_set(spec,'{ComposeFile}',to_jsonb((spec->>'ComposeFile')||E'\n# edit')) WHERE id=$1")
        .bind(stale.current_stack_release_id).execute(pool).await.unwrap();
    assert!(
        !store
            .record_drift(
                &stale,
                StackReleaseStatus::Degraded,
                citadel_activities::ActivityEventInfo::StackDriftResolved {
                    previous_fingerprint: "stale input".into()
                }
            )
            .await
            .unwrap()
    );
    verify_monitor_pagination(pool, stacks, id, runtime).await;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE resourceid=$1 AND eventtype IN ('StackDriftDetected','StackDriftResolved')").bind(id).fetch_one(pool).await.unwrap();
    assert_eq!(count, 2);
    // Port: StackDriftMonitorJobTests event AutoFix, DetectOnly and intentional stop.
    runtime.runtime_state.store(1, Ordering::Relaxed);
    let calls = runtime.reconcile_calls.load(Ordering::Relaxed);
    stacks.monitor_container_event(id).await.unwrap();
    assert_eq!(runtime.reconcile_calls.load(Ordering::Relaxed), calls);
    // DetectOnly allows an explicit licensed repair without enabling automatic actions.
    let response = request(
        app,
        Method::POST,
        &format!("/api/v1/stacks/{id}/reconcile"),
        Some(admin.clone()),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let manual_policy = runtime
        .reconcile_policies
        .lock()
        .unwrap()
        .last()
        .unwrap()
        .clone();
    assert!(manual_policy.auto_start_stopped_containers);
    assert!(manual_policy.auto_resume_paused_containers);
    assert!(!manual_policy.remove_extra_containers);
    assert_eq!(
        store
            .get_authorized(admin.actor_id, true, id)
            .await
            .unwrap()
            .drift_policy,
        recovered.drift_policy,
        "manual reconciliation must not change the saved policy"
    );
    let calls = runtime.reconcile_calls.load(Ordering::Relaxed);
    runtime.runtime_state.store(0, Ordering::Relaxed);
    let structural = stacks
        .reconcile_drift(admin.actor_id, true, id)
        .await
        .unwrap();
    assert_eq!(
        structural.status,
        citadel_stacks::StackReconciliationStatus::RequiresReapply
    );
    assert_eq!(runtime.reconcile_calls.load(Ordering::Relaxed), calls);
    runtime.runtime_state.store(1, Ordering::Relaxed);
    let mut policy = recovered.drift_policy.clone();
    policy.mode = citadel_stacks::StackDriftMode::AutoFix;
    stacks
        .update_drift_policy(admin.actor_id, true, id, policy.clone())
        .await
        .unwrap();
    let denied_runtime = Arc::new(CompletingStackRuntime::default());
    denied_runtime.runtime_state.store(1, Ordering::Relaxed);
    let alerts = Arc::new(alert_sink::RecordedAlerts::default());
    let unlicensed = StackService::new(
        Arc::new(citadel_server::tasks::stacks::TrackedStackTasks::new(
            citadel_runtime::DynamicTasks::new(tokio_util::sync::CancellationToken::new()),
        )),
        Arc::new(PostgresStackRepository::new(pool.clone())),
        denied_runtime.clone(),
        Arc::new(FixtureBindings),
        Arc::new(NoopStackChangeNotifier),
        CancellationToken::new(),
    )
    .with_entitlements(Arc::new(Entitlements(false)))
    .with_alerts(alerts.clone());
    *denied_runtime.snapshot_barrier.lock().unwrap() = Some(Arc::new(tokio::sync::Barrier::new(2)));
    let (event, periodic) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(
            unlicensed.monitor_container_event(id),
            unlicensed.monitor_drift(None, 100)
        )
    })
    .await
    .expect("both observations should reach the runtime before either finishes");
    *denied_runtime.snapshot_barrier.lock().unwrap() = None;
    event.unwrap();
    let periodic = periodic.unwrap();
    assert_eq!(periodic.checked, 1);
    assert!(periodic.failures.is_empty());
    assert_eq!(
        alerts
            .0
            .lock()
            .unwrap()
            .iter()
            .filter(|o| o.resource_id == id && o.matched)
            .count(),
        2,
        "event and periodic observations must both count towards required matches"
    );
    assert_eq!(denied_runtime.reconcile_calls.load(Ordering::Relaxed), 0);
    assert!(matches!(
        unlicensed.reconcile_drift(admin.actor_id, true, id).await,
        Err(StackError::LicenseRequired("OperationalGuardrails"))
    ));
    assert_eq!(denied_runtime.reconcile_calls.load(Ordering::Relaxed), 0);
    assert_eq!(runtime.reconcile_calls.load(Ordering::Relaxed), calls);
    stacks.monitor_container_event(id).await.unwrap();
    assert_eq!(runtime.reconcile_calls.load(Ordering::Relaxed), calls + 1);
    let automatic_policy = runtime
        .reconcile_policies
        .lock()
        .unwrap()
        .last()
        .unwrap()
        .clone();
    assert_eq!(
        automatic_policy, policy,
        "automatic repair must respect configured options"
    );
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
        .update_drift_policy(admin.actor_id, true, id, recovered.resource.drift_policy)
        .await
        .unwrap();
    // Free DetectOnly still observes, marks drift and recovers without any repair calls.
    denied_runtime.runtime_state.store(2, Ordering::Relaxed);
    unlicensed.monitor_container_event(id).await.unwrap();
    assert_eq!(
        store
            .get_authorized(admin.actor_id, true, id)
            .await
            .unwrap()
            .status,
        StackReleaseStatus::Healthy
    );
    denied_runtime.runtime_state.store(1, Ordering::Relaxed);
    unlicensed.monitor_container_event(id).await.unwrap();
    assert_eq!(
        store
            .get_authorized(admin.actor_id, true, id)
            .await
            .unwrap()
            .status,
        StackReleaseStatus::Degraded
    );
    assert_eq!(denied_runtime.reconcile_calls.load(Ordering::Relaxed), 0);
    let free_detection = unlicensed.monitor_drift(None, 100).await.unwrap();
    assert_eq!(free_detection.checked, 1);
    assert!(free_detection.failures.is_empty());
    assert!(
        alerts
            .0
            .lock()
            .unwrap()
            .iter()
            .any(
                |observation| observation.alert_type == "StackDriftDetected" && observation.matched
            )
    );
    assert!(
        alerts
            .0
            .lock()
            .unwrap()
            .iter()
            .any(|observation| observation.alert_type == "StackDriftDetected"
                && !observation.matched)
    );
    assert!(matches!(
        unlicensed.reconcile_drift(admin.actor_id, true, id).await,
        Err(StackError::LicenseRequired("OperationalGuardrails"))
    ));
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
    for (specific, allowed) in [(0, false), (32, true)] {
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
}

/// Mirror a production-sized worker page, including an observation that cannot parse its Compose file.
async fn verify_monitor_pagination(
    pool: &sqlx::PgPool,
    service: &StackService,
    template: Uuid,
    runtime: &CompletingStackRuntime,
) {
    let start = u128::MAX - 200;
    let ids: Vec<_> = (1..=101).map(|n| Uuid::from_u128(start + n)).collect();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO stacks(id,name,stacksource,stackupdatestate,driftpolicy,createdbyactorid)
        SELECT item,'drift-page-'||i,s.stacksource,s.stackupdatestate,
            CASE WHEN i%2=0 THEN jsonb_set(s.driftpolicy::jsonb,'{Mode}','\"AutoFix\"'::jsonb)::json ELSE s.driftpolicy END,s.createdbyactorid
        FROM stacks s CROSS JOIN unnest($2::uuid[]) WITH ORDINALITY page(item,i) WHERE s.id=$1")
        .bind(template).bind(&ids).execute(&mut *tx).await.unwrap();
    sqlx::query("WITH releases AS (INSERT INTO stackreleases(id,stackid,createdbyactorid,platformid,status,version,spec)
        SELECT gen_random_uuid(),item,r.createdbyactorid,r.platformid,'Healthy',1,
          CASE WHEN i=1 THEN jsonb_set(r.spec,'{ComposeFile}',to_jsonb('invalid: ['::text)) ELSE r.spec END
        FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid
        CROSS JOIN unnest($2::uuid[]) WITH ORDINALITY page(item,i) WHERE s.id=$1 RETURNING id,stackid)
        UPDATE stacks s SET currentstackreleaseid=r.id FROM releases r WHERE s.id=r.stackid")
        .bind(template).bind(&ids).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    let before_groups = runtime.group_checks.load(Ordering::Relaxed);
    let before = query_counts::count(pool).await;
    let first = service
        .monitor_drift(Some(Uuid::from_u128(start)), 100)
        .await
        .unwrap();
    query_counts::assert_single_query(pool, before).await;
    assert_eq!(first.checked, 99);
    assert_eq!(first.failures.len(), 1);
    assert_eq!(first.failures[0].stack_id, ids[0]);
    assert_eq!(first.next_cursor, Some(ids[99]));
    let last = service.monitor_drift(first.next_cursor, 100).await.unwrap();
    assert_eq!(last.checked, 1);
    assert!(last.failures.is_empty());
    assert!(last.next_cursor.is_none());
    assert_eq!(
        runtime.group_checks.load(Ordering::Relaxed) - before_groups,
        1
    );

    let other = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount)
        VALUES($1,$1::text,'Agent',1,0,1024,$1::text,0,'{\"$type\":\"Docker\"}','Online',0)")
        .bind(other).execute(pool).await.unwrap();
    sqlx::query("UPDATE stackreleases SET platformid=$1 WHERE stackid=ANY($2)")
        .bind(other)
        .bind(&ids[50..100])
        .execute(pool)
        .await
        .unwrap();
    for unavailable in [false, true] {
        *runtime.unavailable_group.lock().unwrap() = unavailable.then_some(other);
        let before = runtime.group_checks.load(Ordering::Relaxed);
        let page = service
            .monitor_drift(Some(Uuid::from_u128(start)), 100)
            .await
            .unwrap();
        assert_eq!(runtime.group_checks.load(Ordering::Relaxed) - before, 2);
        let runtime_failures = if unavailable { 50 } else { 0 };
        assert_eq!(page.checked, 99 - runtime_failures);
        assert_eq!(page.failures.len(), 1 + runtime_failures);
        assert_eq!(page.next_cursor, Some(ids[99]));
    }
    *runtime.unavailable_group.lock().unwrap() = None;
    sqlx::query("DELETE FROM stacks WHERE id=ANY($1)")
        .bind(&ids)
        .execute(pool)
        .await
        .unwrap();
}
