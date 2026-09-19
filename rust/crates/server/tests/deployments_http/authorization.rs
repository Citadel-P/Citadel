use super::*;
use citadel_deployments::DeploymentStore;
use citadel_deployments::permissions::{ApplyDeployment, CreateDeployment, ReadDeployment};
use citadel_identity::IdentityError;

pub(super) async fn verify(
    app: &Router,
    pool: &sqlx::PgPool,
    identity: &IdentityService,
    admin: &ActorPrincipal,
    reader: &ActorPrincipal,
    deployment_id: Uuid,
    applied: &AtomicBool,
) {
    assert!(
        identity
            .require_scope::<CreateDeployment>(admin)
            .await
            .is_ok()
    );
    assert!(matches!(
        identity.require_scope::<CreateDeployment>(reader).await,
        Err(IdentityError::Forbidden)
    ));
    assert!(
        identity
            .require_resource::<ReadDeployment>(reader, deployment_id)
            .await
            .is_ok()
    );
    assert!(matches!(
        identity
            .require_resource::<ReadDeployment>(reader, Uuid::now_v7())
            .await,
        Err(IdentityError::Forbidden)
    ));
    assert!(
        identity
            .require_resource::<ReadDeployment>(admin, Uuid::now_v7())
            .await
            .is_ok()
    );
    assert_eq!(
        request(
            app,
            Method::GET,
            &format!("/api/v1/deployments/{}", Uuid::now_v7()),
            Some(admin.clone()),
            None
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );

    applied.store(false, Ordering::Release);
    let denied = request(
        app,
        Method::POST,
        "/api/v1/deployments/apply",
        Some(reader.clone()),
        Some(json!({"id":deployment_id})),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    let problem = response_json(denied).await;
    assert_eq!(problem["status"], 403);
    assert!(
        !applied.load(Ordering::Acquire),
        "denial must precede stream/runtime creation"
    );

    // The only added right is Apply. Read remains 1, so this catches Execute drift
    // independently at the HTTP precheck and transactional claim boundary.
    sqlx::query("UPDATE resourceaccesses SET permissionlevel=1,specificpermissions=4 WHERE actorid=$1 AND resourceid=$2")
        .bind(reader.actor_id.value()).bind(deployment_id).execute(pool).await.unwrap();
    assert!(
        identity
            .require_resource::<ApplyDeployment>(reader, deployment_id)
            .await
            .is_ok()
    );
    let view = response_json(
        request(
            app,
            Method::GET,
            &format!("/api/v1/deployments/{deployment_id}"),
            Some(reader.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(view["capabilities"]["canApply"], true);
    assert_eq!(view["capabilities"]["canExecute"], false);
    let allowed = request(
        app,
        Method::POST,
        "/api/v1/deployments/apply",
        Some(reader.clone()),
        Some(json!({"id":deployment_id})),
    )
    .await;
    assert_eq!(allowed.status(), StatusCode::OK);
    let _progress = response_json(allowed).await;
    assert!(applied.load(Ordering::Acquire));

    // Revocation after a successful precheck must still fail inside persistence.
    sqlx::query(
        "UPDATE resourceaccesses SET specificpermissions=0 WHERE actorid=$1 AND resourceid=$2",
    )
    .bind(reader.actor_id.value())
    .bind(deployment_id)
    .execute(pool)
    .await
    .unwrap();
    let store = PostgresDeploymentStore::new(pool.clone());
    assert!(matches!(
        store
            .claim_apply(reader.actor_id, false, deployment_id)
            .await,
        Err(DeploymentError::Forbidden)
    ));

    let mut service_account = reader.clone();
    service_account.principal_type = AuthenticatedPrincipalType::ServiceAccount;
    service_account.roles = vec!["Admin".into()];
    assert!(matches!(
        identity
            .require_resource::<ApplyDeployment>(&service_account, deployment_id)
            .await,
        Err(IdentityError::Forbidden)
    ));
    sqlx::query("UPDATE actors SET isenabled=FALSE WHERE id=$1")
        .bind(reader.actor_id.value())
        .execute(pool)
        .await
        .unwrap();
    assert!(matches!(
        identity
            .require_resource::<ReadDeployment>(reader, deployment_id)
            .await,
        Err(IdentityError::Forbidden)
    ));
    assert!(matches!(
        identity.require_scope::<CreateDeployment>(reader).await,
        Err(IdentityError::Forbidden)
    ));
    sqlx::query("UPDATE actors SET isenabled=TRUE WHERE id=$1")
        .bind(reader.actor_id.value())
        .execute(pool)
        .await
        .unwrap();

    // Optional instrumentation is enabled in the isolated verification database.
    // Ordinary integration environments do not need pg_stat_statements installed.
    let instrumented: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname='pg_stat_statements')",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert!(
        instrumented || std::env::var_os("CITADEL_REQUIRE_QUERY_COUNTS").is_none(),
        "CI query-count checks require pg_stat_statements"
    );
    if instrumented {
        let extra_resources: Vec<Uuid> =
            sqlx::query_scalar("SELECT id FROM deployments WHERE id<>$1 ORDER BY id LIMIT 2")
                .bind(deployment_id)
                .fetch_all(pool)
                .await
                .unwrap();
        assert_eq!(extra_resources.len(), 2);
        let mut extra_grants = Vec::new();
        for id in extra_resources {
            let grant = Uuid::now_v7();
            sqlx::query("INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,1,$3,1,0)")
                .bind(grant).bind(reader.actor_id.value()).bind(id).execute(pool).await.unwrap();
            extra_grants.push(grant);
        }

        for (path, expected) in [
            ("/api/v1/deployments".to_owned(), 3_i64),
            (format!("/api/v1/deployments/{deployment_id}"), 2_i64),
        ] {
            let before = query_count(pool).await;
            let response = request(app, Method::GET, &path, Some(reader.clone()), None).await;
            assert_eq!(response.status(), StatusCode::OK);
            let body = response_json(response).await;
            let count = query_count(pool).await - before;
            if path == "/api/v1/deployments" {
                assert_eq!(body["deployments"].as_array().unwrap().len(), 3);
            }
            assert_eq!(
                count, expected,
                "authorization must not amplify queries for {path}: {body}"
            );
            println!("authorization query count {path}: {count}");
        }
        sqlx::query("DELETE FROM resourceaccesses WHERE id=ANY($1)")
            .bind(extra_grants)
            .execute(pool)
            .await
            .unwrap();
    }
}

async fn query_count(pool: &sqlx::PgPool) -> i64 {
    sqlx::query_scalar("SELECT COALESCE(sum(calls),0)::bigint FROM pg_stat_statements WHERE dbid=(SELECT oid FROM pg_database WHERE datname=current_database()) AND query NOT LIKE '%pg_stat_statements%'")
        .fetch_one(pool).await.unwrap()
}
