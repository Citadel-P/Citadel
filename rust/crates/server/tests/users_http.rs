use std::sync::{Arc, OnceLock};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::CACHE_CONTROL;
use axum::http::{Method, Request, Response, StatusCode};
use chrono::{Duration, Utc};
use citadel_adapters::crypto::{
    Argon2PasswordHasher, JwtSessionTokenCodec, OpaqueServiceAccountTokenCodec,
};
use citadel_adapters::identity_store::{PostgresIdentityStore, StaticEntitlementService};
use citadel_adapters::user_store::PostgresUserReadStore;
use citadel_database::MigrationRunner;
use citadel_domain::{ActorId, AuthenticatedPrincipalType, PermissionLevel, ResourceType};
use citadel_identity::{ADMIN_ROLE_ID, ActorPrincipal, SYSTEM_ACTOR_ID};
use citadel_identity::{
    IdentityService, NoopServiceAccountLastUsedTracker, PasswordHasher, SystemClock,
    UserMutationService, UserReadService,
};
use citadel_server::users_http::{self, UsersHttpState};
use serde_json::Value;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::{Mutex, OwnedMutexGuard};
use tower::ServiceExt;
use uuid::Uuid;

struct Fixture {
    app: Router,
    pool: PgPool,
    administrator: ActorPrincipal,
    identity: Arc<IdentityService>,
    mutations: Arc<UserMutationService>,
    _guard: OwnedMutexGuard<()>,
}

static TEST_LOCK: OnceLock<Arc<Mutex<()>>> = OnceLock::new();
#[path = "users_http/actors.rs"]
mod actors;
const VIEWER_ROLE_ID: Uuid = Uuid::from_u128(0x30000000000000000000000000000003);

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn list_users_returns_the_dotnet_paging_projection_and_capabilities() {
    let fixture = fixture().await;
    let prefix = format!("list-{}", Uuid::now_v7().simple());
    let first = seed_user(&fixture.pool, &format!("{prefix}-a"), true).await;
    let second = seed_user(&fixture.pool, &format!("{prefix}-b"), true).await;

    let response = send(
        &fixture,
        &format!("/api/v1/users?Name={prefix}&Page=1&PageSize=10"),
        Some(fixture.administrator.clone()),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get(CACHE_CONTROL).unwrap(), "no-store");
    let body = json(response).await;
    let page = &body["pagedResult"];
    assert_eq!(page["totalCount"], 2);
    assert_eq!(page["page"], 1);
    assert_eq!(page["pageSize"], 10);
    let ids = page["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|user| user["id"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(ids.contains(&first.to_string().as_str()));
    assert!(ids.contains(&second.to_string().as_str()));
    assert_eq!(page["items"][0]["resourceAccesses"], Value::Null);
    assert_eq!(body["capabilities"]["canRead"], true);
    assert_eq!(body["capabilities"]["canWrite"], true);
    assert_eq!(body["capabilities"]["canExecute"], true);
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn search_users_matches_name_or_email_and_honors_the_limit() {
    let fixture = fixture().await;
    let marker = Uuid::now_v7().simple().to_string();
    let first = seed_user_with_email(
        &fixture.pool,
        &format!("match-{marker}"),
        &format!("first-{marker}@example.test"),
        true,
    )
    .await;
    let second = seed_user_with_email(
        &fixture.pool,
        &format!("operator-{marker}"),
        &format!("second-match-{marker}@example.test"),
        true,
    )
    .await;
    let non_match = seed_user_with_email(
        &fixture.pool,
        &format!("operator-other-{marker}"),
        &format!("other-{marker}@example.test"),
        true,
    )
    .await;

    let response = send(
        &fixture,
        &format!("/api/v1/users/search?Query=match-{marker}&Limit=10"),
        Some(fixture.administrator.clone()),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json(response).await;
    let ids = body
        .as_array()
        .unwrap()
        .iter()
        .map(|user| user["id"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(ids.contains(&first.to_string().as_str()));
    assert!(ids.contains(&second.to_string().as_str()));
    assert!(!ids.contains(&non_match.to_string().as_str()));
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn get_user_returns_enabled_state_roles_teams_and_resource_accesses() {
    let fixture = fixture().await;
    let marker = Uuid::now_v7().simple().to_string();
    let user_id = seed_user(&fixture.pool, &format!("detail-{marker}"), false).await;
    let actor_id = sqlx::query_scalar::<_, Uuid>("SELECT actorid FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    let role_id = Uuid::now_v7();
    let team_id = Uuid::now_v7();
    let team_actor_id = Uuid::now_v7();
    let access_id = Uuid::now_v7();
    let resource_id = Uuid::now_v7();
    let mut transaction = fixture.pool.begin().await.unwrap();
    sqlx::query("INSERT INTO roles (id, name, roletype) VALUES ($1, $2, 'Custom')")
        .bind(role_id)
        .bind(format!("role-{marker}"))
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query("INSERT INTO actorroles (actorid, roleid) VALUES ($1, $2)")
        .bind(actor_id)
        .bind(role_id)
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query("INSERT INTO actors (id, isenabled, type) VALUES ($1, TRUE, 'Team')")
        .bind(team_actor_id)
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query("INSERT INTO teams (id, actorid, name) VALUES ($1, $2, $3)")
        .bind(team_id)
        .bind(team_actor_id)
        .bind(format!("team-{marker}"))
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query("INSERT INTO actorteammemberships (teamid, memberactorid) VALUES ($1, $2)")
        .bind(team_id)
        .bind(actor_id)
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query(
        r#"
INSERT INTO resourceaccesses
    (id, actorid, permissionlevel, resourceid, resourcetype, specificpermissions)
VALUES ($1, $2, 1, $3, 8, 1)
"#,
    )
    .bind(access_id)
    .bind(actor_id)
    .bind(resource_id)
    .execute(&mut *transaction)
    .await
    .unwrap();
    transaction.commit().await.unwrap();

    let response = send(
        &fixture,
        &format!("/api/v1/users/{user_id}"),
        Some(fixture.administrator.clone()),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json(response).await;
    assert_eq!(body["id"], user_id.to_string());
    assert_eq!(body["isEnabled"], false);
    assert_eq!(body["roles"][0]["id"], role_id.to_string());
    assert_eq!(body["teams"][0]["id"], team_id.to_string());
    assert_eq!(body["resourceAccesses"][0]["id"], access_id.to_string());
    assert_eq!(
        body["resourceAccesses"][0]["resourceId"],
        resource_id.to_string()
    );
    assert_eq!(body["resourceAccesses"][0]["resourceType"], "User");
    assert_eq!(body["resourceAccesses"][0]["permissionLevel"], "Read");
    assert_eq!(
        body["resourceAccesses"][0]["specificPermissions"][0],
        "Logs"
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn user_reads_require_a_human_administrator_even_with_user_execute_permission() {
    let fixture = fixture().await;
    let role_id = Uuid::now_v7();
    let actor_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let marker = Uuid::now_v7().simple().to_string();
    let mut transaction = fixture.pool.begin().await.unwrap();
    sqlx::query("INSERT INTO roles (id, name, roletype) VALUES ($1, $2, 'Custom')")
        .bind(role_id)
        .bind(format!("user-manager-{marker}"))
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions) VALUES ($1, 4, 8, $2, 0)",
    )
    .bind(Uuid::now_v7())
    .bind(role_id)
    .execute(&mut *transaction)
    .await
    .unwrap();
    sqlx::query("INSERT INTO actors (id, isenabled, type) VALUES ($1, TRUE, 'User')")
        .bind(actor_id)
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO users (id, actorid, createdbyactorid, email, name) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(user_id)
    .bind(actor_id)
    .bind(SYSTEM_ACTOR_ID)
    .bind(format!("manager-{marker}@example.test"))
    .bind(format!("manager-{marker}"))
    .execute(&mut *transaction)
    .await
    .unwrap();
    sqlx::query("INSERT INTO actorroles (actorid, roleid) VALUES ($1, $2)")
        .bind(actor_id)
        .bind(role_id)
        .execute(&mut *transaction)
        .await
        .unwrap();
    transaction.commit().await.unwrap();
    let non_administrator = ActorPrincipal {
        subject_id: user_id,
        actor_id: ActorId::new(actor_id),
        name: format!("manager-{marker}"),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec![format!("user-manager-{marker}")],
    };

    let response = send(
        &fixture,
        "/api/v1/users?Page=1&PageSize=10",
        Some(non_administrator),
    )
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let service_account = ActorPrincipal {
        subject_id: Uuid::now_v7(),
        actor_id: ActorId::new(Uuid::now_v7()),
        name: "automation".to_owned(),
        principal_type: AuthenticatedPrincipalType::ServiceAccount,
        credential_id: Some(Uuid::now_v7()),
        roles: vec!["Admin".to_owned()],
    };
    let response = send(
        &fixture,
        "/api/v1/users?Page=1&PageSize=10",
        Some(service_account),
    )
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn user_reads_return_compatible_problem_statuses_for_invalid_or_missing_requests() {
    let fixture = fixture().await;
    let response = send(&fixture, "/api/v1/users", None).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    for uri in [
        "/api/v1/users?Page=0&PageSize=10",
        "/api/v1/users?Page=1&PageSize=501",
        "/api/v1/users/search?Query=x&Limit=10",
        "/api/v1/users/search?Limit=10",
    ] {
        let response = send(&fixture, uri, Some(fixture.administrator.clone())).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{uri}");
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/problem+json"
        );
    }

    let response = send(
        &fixture,
        &format!("/api/v1/users/{}", Uuid::now_v7()),
        Some(fixture.administrator.clone()),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let response = send(
        &fixture,
        "/api/v1/users/not-a-uuid",
        Some(fixture.administrator.clone()),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/problem+json"
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn user_mutations_normalize_invalid_json_without_bypassing_authorization() {
    let fixture = fixture().await;

    let response = send_raw_json(
        &fixture,
        Method::POST,
        "/api/v1/users",
        "{",
        Some(fixture.administrator.clone()),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/problem+json"
    );

    let response = send_raw_json(&fixture, Method::POST, "/api/v1/users", "{", None).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/problem+json"
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn user_mutations_reject_non_administrators_and_service_accounts() {
    let fixture = fixture().await;
    let marker = Uuid::now_v7().simple().to_string();
    for principal in [
        ActorPrincipal {
            subject_id: Uuid::now_v7(),
            actor_id: ActorId::new(Uuid::now_v7()),
            name: "operator".to_owned(),
            principal_type: AuthenticatedPrincipalType::User,
            credential_id: None,
            roles: Vec::new(),
        },
        ActorPrincipal {
            subject_id: Uuid::now_v7(),
            actor_id: ActorId::new(Uuid::now_v7()),
            name: "automation".to_owned(),
            principal_type: AuthenticatedPrincipalType::ServiceAccount,
            credential_id: Some(Uuid::now_v7()),
            roles: vec!["Admin".to_owned()],
        },
    ] {
        let response = send_json(
            &fixture,
            Method::POST,
            "/api/v1/users",
            serde_json::json!({
                "name": format!("denied-{marker}"),
                "email": format!("denied-{marker}@example.test"),
                "password": "denied-user-password-123"
            }),
            principal,
            "application/json",
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn create_user_persists_an_enabled_actor_and_safe_activity() {
    let fixture = fixture().await;
    let marker = Uuid::now_v7().simple().to_string();
    let name = format!("create-{marker}");
    let password = "create-user-password-123";
    let response = send_json(
        &fixture,
        Method::POST,
        "/api/v1/users",
        serde_json::json!({
            "name": name,
            "email": format!("create-{marker}@example.test"),
            "password": password
        }),
        fixture.administrator.clone(),
        "application/json",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json(response).await;
    assert_eq!(body["isEnabled"], true);
    let user_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();
    let row = sqlx::query(
        "SELECT actor.isenabled, users.password FROM users JOIN actors actor ON actor.id = users.actorid WHERE users.id = $1",
    )
    .bind(user_id)
    .fetch_one(&fixture.pool)
    .await
    .unwrap();
    assert!(sqlx::Row::get::<bool, _>(&row, "isenabled"));
    let hash = sqlx::Row::get::<String, _>(&row, "password");
    assert!(Argon2PasswordHasher::default().verify(password, &hash));
    let info = activity_info(&fixture.pool, user_id, "UserCreated").await;
    assert!(!info.contains(password));
    assert!(!info.to_ascii_lowercase().contains("passwordhash"));
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn create_user_with_assignments_persists_teams_roles_and_resource_access() {
    let fixture = fixture().await;
    let marker = Uuid::now_v7().simple().to_string();
    let team_id = seed_team(&fixture.pool, &format!("team-{marker}")).await;
    let role_id = seed_role(&fixture.pool, &format!("role-{marker}"), "Custom").await;
    let resource_id = Uuid::now_v7();
    let response = send_json(
        &fixture,
        Method::POST,
        "/api/v1/users",
        serde_json::json!({
            "name": format!("assigned-{marker}"),
            "email": format!("assigned-{marker}@example.test"),
            "password": "assigned-user-password-123",
            "teamIds": [team_id],
            "roleIds": [role_id],
            "resourceAccesses": [{
                "resourceType": "Deployment",
                "resourceId": resource_id,
                "permissionLevel": "Read",
                "specificPermissions": []
            }]
        }),
        fixture.administrator.clone(),
        "application/json",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json(response).await;
    let actor_id = Uuid::parse_str(body["actorId"].as_str().unwrap()).unwrap();
    assert_eq!(body["teams"][0]["id"], team_id.to_string());
    assert_eq!(body["roles"][0]["id"], role_id.to_string());
    assert_eq!(
        body["resourceAccesses"][0]["resourceId"],
        resource_id.to_string()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM actorteammemberships WHERE memberactorid = $1 AND teamid = $2",
        )
        .bind(actor_id)
        .bind(team_id)
        .fetch_one(&fixture.pool)
        .await
        .unwrap(),
        1
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn create_user_with_duplicate_name_returns_conflict() {
    let fixture = fixture().await;
    let marker = Uuid::now_v7().simple().to_string();
    let name = format!("duplicate-name-{marker}");
    seed_user(&fixture.pool, &name, true).await;
    let response =
        create_user_request(&fixture, &name, &format!("other-{marker}@example.test")).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn create_user_with_duplicate_email_returns_conflict() {
    let fixture = fixture().await;
    let marker = Uuid::now_v7().simple().to_string();
    let email = format!("duplicate-email-{marker}@example.test");
    seed_user_with_email(
        &fixture.pool,
        &format!("email-first-{marker}"),
        &email,
        true,
    )
    .await;
    let response = create_user_request(&fixture, &format!("email-second-{marker}"), &email).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn patch_user_updates_email_password_enabled_state_and_revokes_sessions() {
    let fixture = fixture().await;
    let marker = Uuid::now_v7().simple().to_string();
    let user_id = seed_user(&fixture.pool, &format!("patch-{marker}"), true).await;
    seed_refresh_token(&fixture.pool, user_id).await;
    let password = "updated-user-password-123";
    let response = send_json(
        &fixture,
        Method::PATCH,
        &format!("/api/v1/users/{user_id}"),
        serde_json::json!({
            "email": format!("patched-{marker}@example.test"),
            "password": password,
            "isEnabled": false
        }),
        fixture.administrator.clone(),
        "application/merge-patch+json",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json(response).await;
    assert_eq!(body["isEnabled"], false);
    let hash = sqlx::query_scalar::<_, String>("SELECT password FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    assert!(Argon2PasswordHasher::default().verify(password, &hash));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM refreshtokens WHERE userid = $1")
            .bind(user_id)
            .fetch_one(&fixture.pool)
            .await
            .unwrap(),
        0
    );
    let info = activity_info(&fixture.pool, user_id, "UserUpdated").await;
    assert!(!info.contains(password));
    assert!(info.contains("\"PasswordChanged\":true"));
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn patch_user_password_alone_revokes_refresh_sessions() {
    let fixture = fixture().await;
    let marker = Uuid::now_v7().simple().to_string();
    let user_id = seed_user(&fixture.pool, &format!("password-only-{marker}"), true).await;
    seed_refresh_token(&fixture.pool, user_id).await;
    let response = send_json(
        &fixture,
        Method::PATCH,
        &format!("/api/v1/users/{user_id}"),
        serde_json::json!({ "password": "password-only-correct-horse-123" }),
        fixture.administrator.clone(),
        "application/merge-patch+json",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json(response).await["isEnabled"], true);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM refreshtokens WHERE userid = $1")
            .bind(user_id)
            .fetch_one(&fixture.pool)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn patch_user_validates_password_against_the_persisted_identity() {
    let fixture = fixture().await;
    let name = format!("predictable-{}", Uuid::now_v7().simple());
    let user_id = seed_user(&fixture.pool, &name, true).await;
    let response = send_json(
        &fixture,
        Method::PATCH,
        &format!("/api/v1/users/{user_id}"),
        serde_json::json!({ "password": name }),
        fixture.administrator.clone(),
        "application/merge-patch+json",
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        activity_count(&fixture.pool, user_id, "UserUpdated").await,
        0
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn rename_user_updates_name_and_records_the_rename() {
    let fixture = fixture().await;
    let marker = Uuid::now_v7().simple().to_string();
    let old_name = format!("rename-old-{marker}");
    let new_name = format!("rename-new-{marker}");
    let user_id = seed_user(&fixture.pool, &old_name, true).await;
    let response = send_json(
        &fixture,
        Method::POST,
        "/api/v1/users/rename",
        serde_json::json!({ "id": user_id, "name": new_name }),
        fixture.administrator.clone(),
        "application/json",
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json(response).await["name"], new_name);
    let info = activity_info(&fixture.pool, user_id, "UserRenamed").await;
    assert!(info.contains(&old_name));
    assert!(info.contains(&new_name));
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn add_user_role_assigns_the_role_and_audits_once() {
    let fixture = fixture().await;
    let marker = Uuid::now_v7().simple().to_string();
    let user_id = seed_user(&fixture.pool, &format!("role-add-{marker}"), true).await;
    let response = send_json(
        &fixture,
        Method::POST,
        &format!("/api/v1/users/{user_id}/roles"),
        serde_json::json!({ "roleId": VIEWER_ROLE_ID }),
        fixture.administrator.clone(),
        "application/json",
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json(response).await["roles"][0]["id"],
        VIEWER_ROLE_ID.to_string()
    );
    assert_eq!(
        activity_count(&fixture.pool, user_id, "UserUpdated").await,
        1
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn remove_user_role_unassigns_the_role() {
    let fixture = fixture().await;
    let marker = Uuid::now_v7().simple().to_string();
    let (user_id, actor_id) =
        seed_user_record(&fixture.pool, &format!("role-remove-{marker}"), true).await;
    assign_role(&fixture.pool, actor_id, VIEWER_ROLE_ID).await;
    let response = send_json(
        &fixture,
        Method::DELETE,
        &format!("/api/v1/users/{user_id}/roles/{VIEWER_ROLE_ID}"),
        Value::Null,
        fixture.administrator.clone(),
        "application/json",
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(json(response).await["roles"].as_array().unwrap().is_empty());
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn add_and_remove_user_resource_access_changes_effective_authorization() {
    let fixture = fixture().await;
    let marker = Uuid::now_v7().simple().to_string();
    let (user_id, actor_id) =
        seed_user_record(&fixture.pool, &format!("access-{marker}"), true).await;
    let resource_id = Uuid::now_v7();
    let payload = serde_json::json!({
        "resourceType": "Deployment",
        "resourceId": resource_id,
        "permissionLevel": "Read",
        "specificPermissions": []
    });
    let add = send_json(
        &fixture,
        Method::POST,
        &format!("/api/v1/users/{user_id}/resource-accesses"),
        payload.clone(),
        fixture.administrator.clone(),
        "application/json",
    )
    .await;
    assert_eq!(add.status(), StatusCode::OK);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM resourceaccesses WHERE actorid = $1 AND resourceid = $2",
        )
        .bind(actor_id)
        .bind(resource_id)
        .fetch_one(&fixture.pool)
        .await
        .unwrap(),
        1
    );
    let user_principal = ActorPrincipal {
        subject_id: user_id,
        actor_id: ActorId::new(actor_id),
        name: format!("access-{marker}"),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: Vec::new(),
    };
    assert!(
        fixture
            .identity
            .authorize_resource(
                &user_principal,
                ResourceType::Deployment,
                resource_id,
                PermissionLevel::Read,
                None,
            )
            .await
            .is_ok()
    );
    let remove = send_json(
        &fixture,
        Method::DELETE,
        &format!("/api/v1/users/{user_id}/resource-accesses"),
        payload,
        fixture.administrator.clone(),
        "application/json",
    )
    .await;
    assert_eq!(remove.status(), StatusCode::OK);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM resourceaccesses WHERE actorid = $1 AND resourceid = $2",
        )
        .bind(actor_id)
        .bind(resource_id)
        .fetch_one(&fixture.pool)
        .await
        .unwrap(),
        0
    );
    assert!(
        fixture
            .identity
            .authorize_resource(
                &user_principal,
                ResourceType::Deployment,
                resource_id,
                PermissionLevel::Read,
                None,
            )
            .await
            .is_err()
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn patch_user_replaces_teams_roles_and_resource_accesses() {
    let fixture = fixture().await;
    let marker = Uuid::now_v7().simple().to_string();
    let (user_id, actor_id) =
        seed_user_record(&fixture.pool, &format!("replace-{marker}"), true).await;
    let old_team = seed_team(&fixture.pool, &format!("old-team-{marker}")).await;
    let new_team = seed_team(&fixture.pool, &format!("new-team-{marker}")).await;
    let old_role = seed_role(&fixture.pool, &format!("old-role-{marker}"), "Custom").await;
    let new_role = seed_role(&fixture.pool, &format!("new-role-{marker}"), "Custom").await;
    let old_resource = Uuid::now_v7();
    let new_resource = Uuid::now_v7();
    assign_team(&fixture.pool, actor_id, old_team).await;
    assign_role(&fixture.pool, actor_id, old_role).await;
    seed_access(&fixture.pool, actor_id, old_resource).await;
    let response = send_json(
        &fixture,
        Method::PATCH,
        &format!("/api/v1/users/{user_id}"),
        serde_json::json!({
            "teamIds": [new_team],
            "roleIds": [new_role],
            "resourceAccesses": [{
                "resourceType": "Deployment", "resourceId": new_resource,
                "permissionLevel": "Read", "specificPermissions": []
            }]
        }),
        fixture.administrator.clone(),
        "application/merge-patch+json",
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json(response).await;
    assert_eq!(body["teams"][0]["id"], new_team.to_string());
    assert_eq!(body["roles"][0]["id"], new_role.to_string());
    assert_eq!(
        body["resourceAccesses"][0]["resourceId"],
        new_resource.to_string()
    );
    assert!(!body.to_string().contains(&old_resource.to_string()));
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn patch_user_with_empty_assignment_collections_clears_them() {
    let fixture = fixture().await;
    let marker = Uuid::now_v7().simple().to_string();
    let (user_id, actor_id) =
        seed_user_record(&fixture.pool, &format!("clear-{marker}"), true).await;
    assign_role(&fixture.pool, actor_id, VIEWER_ROLE_ID).await;
    seed_access(&fixture.pool, actor_id, Uuid::now_v7()).await;
    let response = send_json(
        &fixture,
        Method::PATCH,
        &format!("/api/v1/users/{user_id}"),
        serde_json::json!({ "teamIds": [], "roleIds": [], "resourceAccesses": [] }),
        fixture.administrator.clone(),
        "application/merge-patch+json",
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json(response).await;
    assert!(body["teams"].as_array().unwrap().is_empty());
    assert!(body["roles"].as_array().unwrap().is_empty());
    assert!(body["resourceAccesses"].as_array().unwrap().is_empty());
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn delete_user_removes_the_user_and_sessions_but_keeps_safe_activity() {
    let fixture = fixture().await;
    let marker = Uuid::now_v7().simple().to_string();
    let user_id = seed_user(&fixture.pool, &format!("delete-{marker}"), true).await;
    seed_refresh_token(&fixture.pool, user_id).await;
    let response = send_json(
        &fixture,
        Method::DELETE,
        "/api/v1/users",
        serde_json::json!({ "ids": [user_id] }),
        fixture.administrator.clone(),
        "application/json",
    )
    .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&fixture.pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        activity_count(&fixture.pool, user_id, "UserDeleted").await,
        1
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn deleting_the_last_administrator_returns_conflict_and_rolls_back() {
    let fixture = fixture().await;
    demote_other_administrators(&fixture.pool, fixture.administrator.actor_id.value()).await;
    let response = send_json(
        &fixture,
        Method::DELETE,
        "/api/v1/users",
        serde_json::json!({ "ids": [fixture.administrator.subject_id] }),
        fixture.administrator.clone(),
        "application/json",
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE id = $1")
            .bind(fixture.administrator.subject_id)
            .fetch_one(&fixture.pool)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        activity_count(
            &fixture.pool,
            fixture.administrator.subject_id,
            "UserDeleted"
        )
        .await,
        0
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn concurrent_administrator_deletes_keep_one_enabled_administrator() {
    let fixture = fixture().await;
    let marker = Uuid::now_v7().simple().to_string();
    let first = seed_user_record(&fixture.pool, &format!("admin-first-{marker}"), true).await;
    let second = seed_user_record(&fixture.pool, &format!("admin-second-{marker}"), true).await;
    assign_role(&fixture.pool, first.1, ADMIN_ROLE_ID).await;
    assign_role(&fixture.pool, second.1, ADMIN_ROLE_ID).await;
    sqlx::query("DELETE FROM actorroles WHERE roleid = $1 AND actorid NOT IN ($2, $3)")
        .bind(ADMIN_ROLE_ID)
        .bind(first.1)
        .bind(second.1)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let first_delete = fixture
        .mutations
        .delete(vec![first.0], ActorId::new(SYSTEM_ACTOR_ID));
    let second_delete = fixture
        .mutations
        .delete(vec![second.0], ActorId::new(SYSTEM_ACTOR_ID));
    let (first_result, second_result) = tokio::join!(first_delete, second_delete);
    assert_ne!(first_result.is_ok(), second_result.is_ok());
    assert_eq!(enabled_administrator_count(&fixture.pool).await, 1);
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn duplicate_role_assignment_returns_conflict_without_a_ghost_activity() {
    let fixture = fixture().await;
    let marker = Uuid::now_v7().simple().to_string();
    let (user_id, actor_id) =
        seed_user_record(&fixture.pool, &format!("ghost-{marker}"), true).await;
    assign_role(&fixture.pool, actor_id, VIEWER_ROLE_ID).await;
    let response = send_json(
        &fixture,
        Method::POST,
        &format!("/api/v1/users/{user_id}/roles"),
        serde_json::json!({ "roleId": VIEWER_ROLE_ID }),
        fixture.administrator.clone(),
        "application/json",
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        activity_count(&fixture.pool, user_id, "UserUpdated").await,
        0
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn create_user_with_missing_assignment_rolls_back_every_row() {
    let fixture = fixture().await;
    let marker = Uuid::now_v7().simple().to_string();
    let name = format!("rollback-{marker}");
    let actor_count_before = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM actors")
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    let response = send_json(
        &fixture,
        Method::POST,
        "/api/v1/users",
        serde_json::json!({
            "name": name,
            "email": format!("rollback-{marker}@example.test"),
            "password": "rollback-user-password-123",
            "roleIds": [Uuid::now_v7()]
        }),
        fixture.administrator.clone(),
        "application/json",
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE name = $1")
            .bind(&name)
            .fetch_one(&fixture.pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM actors")
            .fetch_one(&fixture.pool)
            .await
            .unwrap(),
        actor_count_before
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn create_user_with_custom_access_requires_the_license_and_rolls_back() {
    let fixture = fixture_with_custom_access(false).await;
    let marker = Uuid::now_v7().simple().to_string();
    let name = format!("unlicensed-{marker}");
    let actor_count_before = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM actors")
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    let response = send_json(
        &fixture,
        Method::POST,
        "/api/v1/users",
        serde_json::json!({
            "name": name,
            "email": format!("unlicensed-{marker}@example.test"),
            "password": "unlicensed-user-password-123",
            "resourceAccesses": [{
                "resourceType": "Deployment",
                "resourceId": Uuid::now_v7(),
                "permissionLevel": "Read"
            }]
        }),
        fixture.administrator.clone(),
        "application/json",
    )
    .await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE name = $1")
            .bind(&name)
            .fetch_one(&fixture.pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM actors")
            .fetch_one(&fixture.pool)
            .await
            .unwrap(),
        actor_count_before
    );
}

async fn fixture() -> Fixture {
    fixture_with_custom_access(true).await
}

async fn fixture_with_custom_access(custom_access_control_enabled: bool) -> Fixture {
    let guard = TEST_LOCK
        .get_or_init(|| Arc::new(Mutex::new(())))
        .clone()
        .lock_owned()
        .await;
    let database_url = std::env::var("CITADEL_PHASE3_DATABASE_URL")
        .expect("CITADEL_PHASE3_DATABASE_URL is required for this fixture");
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .unwrap();
    let administrator = seed_administrator(&pool).await;
    let identity_store = Arc::new(PostgresIdentityStore::new(pool.clone()));
    let passwords = Arc::new(Argon2PasswordHasher::default());
    let entitlements = Arc::new(StaticEntitlementService::new(custom_access_control_enabled));
    let clock = Arc::new(SystemClock);
    let identity = Arc::new(IdentityService::new(
        identity_store,
        passwords.clone(),
        Arc::new(
            JwtSessionTokenCodec::new(
                &[9_u8; 32],
                "users-http-fixture".to_owned(),
                "users-http-fixture".to_owned(),
            )
            .unwrap(),
        ),
        Arc::new(OpaqueServiceAccountTokenCodec),
        entitlements.clone(),
        clock.clone(),
        Arc::new(NoopServiceAccountLastUsedTracker),
        Duration::minutes(15),
        Duration::days(30),
    ));
    let store = Arc::new(PostgresUserReadStore::new(pool.clone()));
    let users = Arc::new(UserReadService::new(store.clone()));
    let mutations = Arc::new(UserMutationService::new(
        store,
        passwords,
        entitlements,
        clock,
    ));
    let app = users_http::router(UsersHttpState {
        identity: identity.clone(),
        users,
        mutations: mutations.clone(),
    });
    Fixture {
        app,
        pool,
        administrator,
        identity,
        mutations,
        _guard: guard,
    }
}

async fn seed_administrator(pool: &PgPool) -> ActorPrincipal {
    let actor_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let name = format!("http-admin-{}", user_id.simple());
    let mut transaction = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO actors (id, isenabled, type) VALUES ($1, TRUE, 'User')")
        .bind(actor_id)
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO users (id, actorid, createdat, createdbyactorid, email, name) VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(user_id)
    .bind(actor_id)
    .bind(Utc::now())
    .bind(SYSTEM_ACTOR_ID)
    .bind(format!("{name}@example.test"))
    .bind(&name)
    .execute(&mut *transaction)
    .await
    .unwrap();
    sqlx::query("INSERT INTO actorroles (actorid, roleid) VALUES ($1, $2)")
        .bind(actor_id)
        .bind(ADMIN_ROLE_ID)
        .execute(&mut *transaction)
        .await
        .unwrap();
    transaction.commit().await.unwrap();
    ActorPrincipal {
        subject_id: user_id,
        actor_id: ActorId::new(actor_id),
        name,
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec!["Admin".to_owned()],
    }
}

async fn seed_user(pool: &PgPool, name: &str, is_enabled: bool) -> Uuid {
    seed_user_with_email(pool, name, &format!("{name}@example.test"), is_enabled).await
}

async fn seed_user_record(pool: &PgPool, name: &str, is_enabled: bool) -> (Uuid, Uuid) {
    let user_id = seed_user(pool, name, is_enabled).await;
    let actor_id = sqlx::query_scalar("SELECT actorid FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap();
    (user_id, actor_id)
}

async fn seed_user_with_email(pool: &PgPool, name: &str, email: &str, is_enabled: bool) -> Uuid {
    let actor_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let mut transaction = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO actors (id, isenabled, type) VALUES ($1, $2, 'User')")
        .bind(actor_id)
        .bind(is_enabled)
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO users (id, actorid, createdbyactorid, email, name) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(user_id)
    .bind(actor_id)
    .bind(SYSTEM_ACTOR_ID)
    .bind(email)
    .bind(name)
    .execute(&mut *transaction)
    .await
    .unwrap();
    transaction.commit().await.unwrap();
    user_id
}

async fn seed_team(pool: &PgPool, name: &str) -> Uuid {
    let actor_id = Uuid::now_v7();
    let team_id = Uuid::now_v7();
    let mut transaction = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO actors (id, isenabled, type) VALUES ($1, TRUE, 'Team')")
        .bind(actor_id)
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query("INSERT INTO teams (id, actorid, name) VALUES ($1, $2, $3)")
        .bind(team_id)
        .bind(actor_id)
        .bind(name)
        .execute(&mut *transaction)
        .await
        .unwrap();
    transaction.commit().await.unwrap();
    team_id
}

async fn seed_role(pool: &PgPool, name: &str, role_type: &str) -> Uuid {
    let role_id = Uuid::now_v7();
    sqlx::query("INSERT INTO roles (id, name, roletype) VALUES ($1, $2, $3)")
        .bind(role_id)
        .bind(name)
        .bind(role_type)
        .execute(pool)
        .await
        .unwrap();
    role_id
}

async fn assign_role(pool: &PgPool, actor_id: Uuid, role_id: Uuid) {
    sqlx::query("INSERT INTO actorroles (actorid, roleid) VALUES ($1, $2)")
        .bind(actor_id)
        .bind(role_id)
        .execute(pool)
        .await
        .unwrap();
}

async fn assign_team(pool: &PgPool, actor_id: Uuid, team_id: Uuid) {
    sqlx::query("INSERT INTO actorteammemberships (memberactorid, teamid) VALUES ($1, $2)")
        .bind(actor_id)
        .bind(team_id)
        .execute(pool)
        .await
        .unwrap();
}

async fn seed_access(pool: &PgPool, actor_id: Uuid, resource_id: Uuid) {
    sqlx::query(
        r#"
INSERT INTO resourceaccesses
    (id, actorid, permissionlevel, resourceid, resourcetype, specificpermissions)
VALUES ($1, $2, 1, $3, 1, 0)
"#,
    )
    .bind(Uuid::now_v7())
    .bind(actor_id)
    .bind(resource_id)
    .execute(pool)
    .await
    .unwrap();
}

async fn seed_refresh_token(pool: &PgPool, user_id: Uuid) {
    let now = Utc::now();
    sqlx::query(
        r#"
INSERT INTO refreshtokens (id, createdat, expiresat, lastseenat, userid)
VALUES ($1, $2, $3, $2, $4)
"#,
    )
    .bind(Uuid::now_v7())
    .bind(now)
    .bind(now + Duration::days(1))
    .bind(user_id)
    .execute(pool)
    .await
    .unwrap();
}

async fn activity_count(pool: &PgPool, user_id: Uuid, event_type: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM activityevents WHERE resourceid = $1 AND eventtype = $2",
    )
    .bind(user_id)
    .bind(event_type)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn activity_info(pool: &PgPool, user_id: Uuid, event_type: &str) -> String {
    sqlx::query_scalar(
        "SELECT info FROM activityevents WHERE resourceid = $1 AND eventtype = $2 ORDER BY createdat DESC LIMIT 1",
    )
    .bind(user_id)
    .bind(event_type)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn demote_other_administrators(pool: &PgPool, retained_actor_id: Uuid) {
    sqlx::query("DELETE FROM actorroles WHERE roleid = $1 AND actorid <> $2")
        .bind(ADMIN_ROLE_ID)
        .bind(retained_actor_id)
        .execute(pool)
        .await
        .unwrap();
}

async fn enabled_administrator_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar(
        r#"
SELECT COUNT(*)
FROM users
JOIN actors ON actors.id = users.actorid AND actors.isenabled
JOIN actorroles ON actorroles.actorid = users.actorid AND actorroles.roleid = $1
"#,
    )
    .bind(ADMIN_ROLE_ID)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn create_user_request(fixture: &Fixture, name: &str, email: &str) -> Response<Body> {
    send_json(
        fixture,
        Method::POST,
        "/api/v1/users",
        serde_json::json!({
            "name": name,
            "email": email,
            "password": "create-request-password-123"
        }),
        fixture.administrator.clone(),
        "application/json",
    )
    .await
}

async fn send(fixture: &Fixture, uri: &str, principal: Option<ActorPrincipal>) -> Response<Body> {
    let mut request = Request::get(uri).body(Body::empty()).unwrap();
    if let Some(principal) = principal {
        request.extensions_mut().insert(principal);
    }
    fixture.app.clone().oneshot(request).await.unwrap()
}

async fn send_json(
    fixture: &Fixture,
    method: Method,
    uri: &str,
    body: Value,
    principal: ActorPrincipal,
    content_type: &str,
) -> Response<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", content_type)
        .body(Body::from(body.to_string()))
        .unwrap();
    request.extensions_mut().insert(principal);
    fixture.app.clone().oneshot(request).await.unwrap()
}

async fn send_raw_json(
    fixture: &Fixture,
    method: Method,
    uri: &str,
    body: &str,
    principal: Option<ActorPrincipal>,
) -> Response<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_owned()))
        .unwrap();
    if let Some(principal) = principal {
        request.extensions_mut().insert(principal);
    }
    fixture.app.clone().oneshot(request).await.unwrap()
}

async fn json(response: Response<Body>) -> Value {
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}
