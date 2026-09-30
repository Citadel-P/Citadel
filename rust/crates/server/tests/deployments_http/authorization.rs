use super::*;
use citadel_adapters::persistence::postgres::identity::{
    actors::repository::PostgresActorRepository, users::repository::PostgresUserRepository,
};
use citadel_deployments::DeploymentRepository;
use citadel_deployments::permissions::{ApplyDeployment, CreateDeployment, ReadDeployment};
use citadel_identity::{
    ActorRepository, IdentityError, ResourceAccessInput, UserPatchMutation, UserRepository,
};
use citadel_primitives::{PermissionLevel, ResourceType, SpecificPermission};

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
    set_read_access(pool, reader, &[deployment_id], true).await;
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
    set_read_access(pool, reader, &[deployment_id], false).await;
    let store = PostgresDeploymentRepository::new(pool.clone());
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
    PostgresActorRepository::new(pool.clone())
        .set_enabled(reader.actor_id.value(), false)
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
    PostgresActorRepository::new(pool.clone())
        .set_enabled(reader.actor_id.value(), true)
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
        let mut grants = extra_resources;
        grants.push(deployment_id);
        set_read_access(pool, reader, &grants, false).await;

        for (principal, path, expected) in [
            (reader, "/api/v1/deployments".to_owned(), 3_i64),
            (
                reader,
                format!("/api/v1/deployments/{deployment_id}"),
                2_i64,
            ),
            (admin, "/api/v1/deployments".to_owned(), 1_i64),
            (admin, format!("/api/v1/deployments/{deployment_id}"), 1_i64),
        ] {
            let before = query_count(pool).await;
            let response = request(app, Method::GET, &path, Some(principal.clone()), None).await;
            assert_eq!(response.status(), StatusCode::OK);
            let body = response_json(response).await;
            let count = query_count(pool).await - before;
            if path == "/api/v1/deployments" && !principal.is_administrator() {
                assert_eq!(body["deployments"].as_array().unwrap().len(), 3);
            }
            assert_eq!(
                count, expected,
                "authorization must not amplify queries for {path}: {body}"
            );
            println!(
                "authorization query count admin={} {path}: {count}",
                principal.is_administrator()
            );
        }
        set_read_access(pool, reader, &[deployment_id], false).await;
    }
}

async fn query_count(pool: &sqlx::PgPool) -> i64 {
    sqlx::query_scalar("SELECT COALESCE(sum(calls),0)::bigint FROM pg_stat_statements WHERE dbid=(SELECT oid FROM pg_database WHERE datname=current_database()) AND query NOT LIKE '%pg_stat_statements%'")
        .fetch_one(pool).await.unwrap()
}

// After an authorization read, fixture ACL writes must use the application
// repository so the committed change invalidates cached grants and denials.
pub(super) async fn set_read_access(
    pool: &sqlx::PgPool,
    reader: &ActorPrincipal,
    ids: &[Uuid],
    apply: bool,
) {
    let patch = UserPatchMutation {
        resource_accesses: Some(
            ids.iter()
                .map(|id| ResourceAccessInput {
                    resource_type: ResourceType::Deployment,
                    resource_id: *id,
                    permission_level: PermissionLevel::Read,
                    specific_permissions: if apply {
                        vec![SpecificPermission::Apply]
                    } else {
                        vec![]
                    },
                })
                .collect(),
        ),
        ..Default::default()
    };
    PostgresUserRepository::new(pool.clone())
        .patch(
            reader.subject_id,
            &patch,
            ActorId::new(SYSTEM_ACTOR_ID),
            chrono::Utc::now(),
            true,
        )
        .await
        .unwrap();
}
