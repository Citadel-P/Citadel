use super::*;
use citadel_stacks::StackStore;

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
