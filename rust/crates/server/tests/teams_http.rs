use std::sync::{Arc, OnceLock};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, Response, StatusCode};
use chrono::{Duration, Utc};
use citadel_adapters::crypto::{
    Argon2PasswordHasher, JwtSessionTokenCodec, OpaqueServiceAccountTokenCodec,
};
use citadel_adapters::identity_store::{PostgresIdentityStore, StaticEntitlementService};
use citadel_adapters::team_store::PostgresTeamStore;
use citadel_database::MigrationRunner;
use citadel_domain::{ActorId, AuthenticatedPrincipalType};
use citadel_identity::{ADMIN_ROLE_ID, ActorPrincipal, SYSTEM_ACTOR_ID};
use citadel_identity::{
    IdentityService, NoopServiceAccountLastUsedTracker, SystemClock, TeamMutationService,
    TeamReadService,
};
use citadel_server::teams_http::{self, TeamsHttpState};
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
    mutations: Arc<TeamMutationService>,
    _guard: OwnedMutexGuard<()>,
}

static TEST_LOCK: OnceLock<Arc<Mutex<()>>> = OnceLock::new();

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn list_search_and_get_preserve_the_team_projection() {
    let fixture = fixture(true).await;
    let marker = Uuid::now_v7().simple().to_string();
    let (team_id, team_actor_id) = seed_team(&fixture.pool, &format!("ops-{marker}"), false).await;
    let (user_id, user_actor_id) = seed_user(&fixture.pool, &format!("member-{marker}")).await;
    let role_id = seed_role(&fixture.pool, &format!("viewer-{marker}"), "System").await;
    sqlx::query("INSERT INTO actorteammemberships (teamid, memberactorid) VALUES ($1, $2)")
        .bind(team_id)
        .bind(user_actor_id)
        .execute(&fixture.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO actorroles (actorid, roleid) VALUES ($1, $2)")
        .bind(team_actor_id)
        .bind(role_id)
        .execute(&fixture.pool)
        .await
        .unwrap();

    let list = send(
        &fixture,
        Method::GET,
        &format!("/api/v1/teams?Name={marker}&Page=1&PageSize=10"),
        None,
    )
    .await;
    assert_eq!(list.status(), StatusCode::OK);
    let list = json(list).await;
    assert_eq!(list["pagedResult"]["totalCount"], 1);
    assert_eq!(list["pagedResult"]["items"][0]["totalMembers"], 1);
    assert_eq!(
        list["pagedResult"]["items"][0]["resourceAccesses"],
        Value::Null
    );
    assert_eq!(list["capabilities"]["canExecute"], true);

    let search = send(
        &fixture,
        Method::GET,
        &format!("/api/v1/teams/search?Query=ops-{marker}&Limit=10"),
        None,
    )
    .await;
    assert_eq!(search.status(), StatusCode::OK);
    assert_eq!(json(search).await[0]["id"], team_id.to_string());

    let get = send(
        &fixture,
        Method::GET,
        &format!("/api/v1/teams/{team_id}"),
        None,
    )
    .await;
    assert_eq!(get.status(), StatusCode::OK);
    let get = json(get).await;
    assert_eq!(get["isEnabled"], false);
    assert_eq!(get["users"][0]["id"], user_id.to_string());
    assert_eq!(get["members"][0]["actorId"], user_actor_id.to_string());
    assert_eq!(get["members"][0]["principalType"], "User");
    assert_eq!(get["roles"][0]["id"], role_id.to_string());
    assert_eq!(get["resourceAccesses"], serde_json::json!([]));
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn create_with_assignments_is_atomic_and_writes_safe_activity() {
    let fixture = fixture(true).await;
    let marker = Uuid::now_v7().simple().to_string();
    let (user_id, user_actor_id) =
        seed_user(&fixture.pool, &format!("create-member-{marker}")).await;
    let role_id = seed_role(&fixture.pool, &format!("create-role-{marker}"), "Custom").await;
    let resource_id = Uuid::now_v7();
    let response = send_json(&fixture, Method::POST, "/api/v1/teams", serde_json::json!({
        "name": format!("create-{marker}"), "userIds": [user_id], "roleIds": [role_id],
        "resourceAccesses": [{ "resourceType": "Deployment", "resourceId": resource_id, "permissionLevel": "Read", "specificPermissions": ["Logs"] }]
    }), "application/json").await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json(response).await;
    let team_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();
    assert_eq!(body["totalMembers"], 1);
    assert_eq!(
        body["resourceAccesses"][0]["resourceId"],
        resource_id.to_string()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM actorteammemberships WHERE teamid = $1 AND memberactorid = $2"
        )
        .bind(team_id)
        .bind(user_actor_id)
        .fetch_one(&fixture.pool)
        .await
        .unwrap(),
        1
    );
    let info: String = sqlx::query_scalar(
        "SELECT info FROM activityevents WHERE resourceid = $1 AND eventtype = 'TeamCreated'",
    )
    .bind(team_id)
    .fetch_one(&fixture.pool)
    .await
    .unwrap();
    assert!(info.contains("\"$type\":\"TeamCreated\""));
    assert!(info.contains(&user_actor_id.to_string()));
    assert!(!info.contains("password"));
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn create_conflicts_and_missing_assignments_leave_no_partial_rows() {
    let fixture = fixture(true).await;
    let name = format!("rollback-{}", Uuid::now_v7().simple());
    seed_team(&fixture.pool, &name, true).await;
    let conflict = send_json(
        &fixture,
        Method::POST,
        "/api/v1/teams",
        serde_json::json!({ "name": name }),
        "application/json",
    )
    .await;
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    let missing_name = format!("missing-{}", Uuid::now_v7().simple());
    let actor_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM actors")
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    let missing = send_json(
        &fixture,
        Method::POST,
        "/api/v1/teams",
        serde_json::json!({ "name": missing_name, "userIds": [Uuid::now_v7()] }),
        "application/json",
    )
    .await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM teams WHERE name = $1")
            .bind(&missing_name)
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
        actor_count
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn patch_replaces_and_clears_assignments_without_removing_service_accounts() {
    let fixture = fixture(true).await;
    let marker = Uuid::now_v7().simple().to_string();
    let (team_id, team_actor_id) = seed_team(&fixture.pool, &format!("patch-{marker}"), true).await;
    let (old_user, old_actor) = seed_user(&fixture.pool, &format!("old-{marker}")).await;
    let (new_user, new_actor) = seed_user(&fixture.pool, &format!("new-{marker}")).await;
    let (_, service_actor) =
        seed_service_account(&fixture.pool, &format!("service-{marker}"), false).await;
    let old_role = seed_role(&fixture.pool, &format!("old-role-{marker}"), "Custom").await;
    let new_role = seed_role(&fixture.pool, &format!("new-role-{marker}"), "Custom").await;
    sqlx::query(
        "INSERT INTO actorteammemberships (teamid, memberactorid) VALUES ($1, $2), ($1, $3)",
    )
    .bind(team_id)
    .bind(old_actor)
    .bind(service_actor)
    .execute(&fixture.pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO actorroles (actorid, roleid) VALUES ($1, $2)")
        .bind(team_actor_id)
        .bind(old_role)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let resource_id = Uuid::now_v7();
    sqlx::query("INSERT INTO resourceaccesses (id, actorid, permissionlevel, resourceid, resourcetype, specificpermissions) VALUES ($1, $2, 1, $3, 1, 0)").bind(Uuid::now_v7()).bind(team_actor_id).bind(Uuid::now_v7()).execute(&fixture.pool).await.unwrap();
    let patch = send_json(&fixture, Method::PATCH, &format!("/api/v1/teams/{team_id}"), serde_json::json!({
        "isEnabled": false, "userIds": [new_user], "roleIds": [new_role],
        "resourceAccesses": [{ "resourceType": "Deployment", "resourceId": resource_id, "permissionLevel": "Read" }]
    }), "application/merge-patch+json").await;
    assert_eq!(patch.status(), StatusCode::OK);
    let body = json(patch).await;
    assert_eq!(body["isEnabled"], false);
    let members = body["members"].as_array().unwrap();
    assert!(
        members
            .iter()
            .any(|member| member["actorId"] == new_actor.to_string())
    );
    assert!(
        members
            .iter()
            .any(|member| member["actorId"] == service_actor.to_string())
    );
    assert!(
        !members
            .iter()
            .any(|member| member["actorId"] == old_actor.to_string())
    );
    let clear = send_json(
        &fixture,
        Method::PATCH,
        &format!("/api/v1/teams/{team_id}"),
        serde_json::json!({ "userIds": [], "roleIds": [], "resourceAccesses": [] }),
        "application/merge-patch+json",
    )
    .await;
    assert_eq!(clear.status(), StatusCode::OK);
    let clear = json(clear).await;
    assert_eq!(clear["roles"], serde_json::json!([]));
    assert_eq!(clear["resourceAccesses"], serde_json::json!([]));
    assert_eq!(clear["members"].as_array().unwrap().len(), 1);
    assert_eq!(clear["members"][0]["principalType"], "ServiceAccount");
    let _ = old_user;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn incremental_member_role_and_resource_access_routes_persist_and_remove() {
    let fixture = fixture(true).await;
    let marker = Uuid::now_v7().simple().to_string();
    let (team_id, _) = seed_team(&fixture.pool, &format!("incremental-{marker}"), true).await;
    let (_, member_actor_id) = seed_user(&fixture.pool, &format!("member-{marker}")).await;
    let role_id = seed_role(&fixture.pool, &format!("role-{marker}"), "System").await;
    let resource_id = Uuid::now_v7();
    for (uri, body) in [
        (
            format!("/api/v1/teams/{team_id}/members"),
            serde_json::json!({ "memberActorId": member_actor_id }),
        ),
        (
            format!("/api/v1/teams/{team_id}/roles"),
            serde_json::json!({ "roleId": role_id }),
        ),
        (
            format!("/api/v1/teams/{team_id}/resource-accesses"),
            serde_json::json!({ "resourceType": "Deployment", "resourceId": resource_id, "permissionLevel": "Read" }),
        ),
    ] {
        assert_eq!(
            send_json(&fixture, Method::POST, &uri, body, "application/json")
                .await
                .status(),
            StatusCode::OK
        );
    }
    assert_eq!(
        activity_count(&fixture.pool, team_id, "TeamUpdated").await,
        3
    );
    assert_eq!(
        send(
            &fixture,
            Method::DELETE,
            &format!("/api/v1/teams/{team_id}/members/{member_actor_id}"),
            None
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        send(
            &fixture,
            Method::DELETE,
            &format!("/api/v1/teams/{team_id}/roles/{role_id}"),
            None
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(send_json(&fixture, Method::DELETE, &format!("/api/v1/teams/{team_id}/resource-accesses"), serde_json::json!({ "resourceType": "Deployment", "resourceId": resource_id, "permissionLevel": "Read" }), "application/json").await.status(), StatusCode::OK);
    assert_eq!(
        activity_count(&fixture.pool, team_id, "TeamUpdated").await,
        6
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn custom_access_expansion_and_service_account_membership_require_the_license() {
    let fixture = fixture(false).await;
    let marker = Uuid::now_v7().simple().to_string();
    let (team_id, _) = seed_team(&fixture.pool, &format!("unlicensed-{marker}"), true).await;
    let (_, service_actor_id) =
        seed_service_account(&fixture.pool, &format!("service-{marker}"), false).await;
    let member = send_json(
        &fixture,
        Method::POST,
        &format!("/api/v1/teams/{team_id}/members"),
        serde_json::json!({ "memberActorId": service_actor_id }),
        "application/json",
    )
    .await;
    assert_eq!(member.status(), StatusCode::FORBIDDEN);
    let access = send_json(&fixture, Method::POST, &format!("/api/v1/teams/{team_id}/resource-accesses"), serde_json::json!({ "resourceType": "Deployment", "resourceId": Uuid::now_v7(), "permissionLevel": "Read" }), "application/json").await;
    assert_eq!(access.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM actorteammemberships WHERE teamid = $1")
            .bind(team_id)
            .fetch_one(&fixture.pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        activity_count(&fixture.pool, team_id, "TeamUpdated").await,
        0
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn last_administrator_guard_rolls_back_team_member_removal() {
    let fixture = fixture(true).await;
    let (team_id, team_actor_id) = seed_team(
        &fixture.pool,
        &format!("admin-team-{}", Uuid::now_v7().simple()),
        true,
    )
    .await;
    sqlx::query("INSERT INTO actorroles (actorid, roleid) VALUES ($1, $2)")
        .bind(team_actor_id)
        .bind(ADMIN_ROLE_ID)
        .execute(&fixture.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO actorteammemberships (teamid, memberactorid) VALUES ($1, $2)")
        .bind(team_id)
        .bind(fixture.administrator.actor_id.value())
        .execute(&fixture.pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM actorroles WHERE roleid = $1 AND actorid <> $2")
        .bind(ADMIN_ROLE_ID)
        .bind(team_actor_id)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let response = send(
        &fixture,
        Method::DELETE,
        &format!(
            "/api/v1/teams/{team_id}/members/{}",
            fixture.administrator.actor_id.value()
        ),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM actorteammemberships WHERE teamid = $1 AND memberactorid = $2"
        )
        .bind(team_id)
        .bind(fixture.administrator.actor_id.value())
        .fetch_one(&fixture.pool)
        .await
        .unwrap(),
        1
    );
    assert_eq!(
        activity_count(&fixture.pool, team_id, "TeamUpdated").await,
        0
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn concurrent_team_deletes_keep_one_enabled_administrator() {
    let fixture = fixture(true).await;
    let marker = Uuid::now_v7().simple().to_string();
    let (second_user_id, second_actor_id) =
        seed_user(&fixture.pool, &format!("second-admin-{marker}")).await;
    let (first_team_id, first_team_actor_id) =
        seed_team(&fixture.pool, &format!("first-admin-{marker}"), true).await;
    let (second_team_id, second_team_actor_id) =
        seed_team(&fixture.pool, &format!("second-admin-{marker}"), true).await;
    let mut transaction = fixture.pool.begin().await.unwrap();
    for actor_id in [first_team_actor_id, second_team_actor_id] {
        sqlx::query("INSERT INTO actorroles (actorid, roleid) VALUES ($1, $2)")
            .bind(actor_id)
            .bind(ADMIN_ROLE_ID)
            .execute(&mut *transaction)
            .await
            .unwrap();
    }
    for (team_id, member_actor_id) in [
        (first_team_id, fixture.administrator.actor_id.value()),
        (second_team_id, second_actor_id),
    ] {
        sqlx::query("INSERT INTO actorteammemberships (teamid, memberactorid) VALUES ($1, $2)")
            .bind(team_id)
            .bind(member_actor_id)
            .execute(&mut *transaction)
            .await
            .unwrap();
    }
    sqlx::query("DELETE FROM actorroles WHERE roleid = $1 AND actorid NOT IN ($2, $3)")
        .bind(ADMIN_ROLE_ID)
        .bind(first_team_actor_id)
        .bind(second_team_actor_id)
        .execute(&mut *transaction)
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    let (first, second) = tokio::join!(
        fixture
            .mutations
            .delete(vec![first_team_id], fixture.administrator.actor_id),
        fixture
            .mutations
            .delete(vec![second_team_id], fixture.administrator.actor_id)
    );

    assert_eq!(first.is_ok() as u8 + second.is_ok() as u8, 1);
    assert!(
        [first, second]
            .into_iter()
            .any(|result| matches!(result, Err(citadel_identity::IdentityError::Conflict(_))))
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM teams WHERE id = ANY($1)")
            .bind(vec![first_team_id, second_team_id])
            .fetch_one(&fixture.pool)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM activityevents WHERE resourceid = ANY($1) AND eventtype = 'TeamDeleted'",
        )
        .bind(vec![first_team_id, second_team_id])
        .fetch_one(&fixture.pool)
        .await
        .unwrap(),
        1
    );
    let _ = second_user_id;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn rename_and_delete_persist_lifecycle_activity_without_ghost_rows() {
    let fixture = fixture(true).await;
    let (team_id, _) = seed_team(
        &fixture.pool,
        &format!("rename-{}", Uuid::now_v7().simple()),
        true,
    )
    .await;
    let new_name = format!("renamed-{}", Uuid::now_v7().simple());
    let rename = send_json(
        &fixture,
        Method::POST,
        "/api/v1/teams/rename",
        serde_json::json!({ "id": team_id, "name": new_name }),
        "application/json",
    )
    .await;
    assert_eq!(rename.status(), StatusCode::OK);
    assert_eq!(
        activity_count(&fixture.pool, team_id, "TeamRenamed").await,
        1
    );
    let delete = send_json(
        &fixture,
        Method::DELETE,
        "/api/v1/teams",
        serde_json::json!({ "ids": [team_id] }),
        "application/json",
    )
    .await;
    assert_eq!(delete.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM teams WHERE id = $1")
            .bind(team_id)
            .fetch_one(&fixture.pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        activity_count(&fixture.pool, team_id, "TeamDeleted").await,
        1
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn team_routes_require_a_human_administrator_and_validate_bounds() {
    let fixture = fixture(true).await;
    let unauthenticated = send_without_principal(&fixture, Method::GET, "/api/v1/teams").await;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
    let invalid = send(
        &fixture,
        Method::GET,
        "/api/v1/teams?Page=0&PageSize=501",
        None,
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    let invalid_team_id = send_json(
        &fixture,
        Method::POST,
        "/api/v1/teams/00000000-0000-0000-0000-000000000000/roles",
        serde_json::json!({ "roleId": Uuid::now_v7() }),
        "application/json",
    )
    .await;
    assert_eq!(invalid_team_id.status(), StatusCode::BAD_REQUEST);
    let missing = send(
        &fixture,
        Method::GET,
        &format!("/api/v1/teams/{}", Uuid::now_v7()),
        None,
    )
    .await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

async fn fixture(custom_access: bool) -> Fixture {
    let guard = TEST_LOCK
        .get_or_init(|| Arc::new(Mutex::new(())))
        .clone()
        .lock_owned()
        .await;
    let database_url = std::env::var("CITADEL_PHASE3_DATABASE_URL")
        .expect("CITADEL_PHASE3_DATABASE_URL is required");
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .unwrap();
    let administrator = seed_administrator(&pool).await;
    let identity_store = Arc::new(PostgresIdentityStore::new(pool.clone()));
    let entitlements = Arc::new(StaticEntitlementService::new(custom_access));
    let clock = Arc::new(SystemClock);
    let identity = Arc::new(IdentityService::new(
        identity_store,
        Arc::new(Argon2PasswordHasher::default()),
        Arc::new(
            JwtSessionTokenCodec::new(
                &[11_u8; 32],
                "teams-http".to_owned(),
                "teams-http".to_owned(),
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
    let store = Arc::new(PostgresTeamStore::new(pool.clone()));
    let teams = Arc::new(TeamReadService::new(store.clone()));
    let mutations = Arc::new(TeamMutationService::new(store, entitlements, clock));
    let app = teams_http::router(TeamsHttpState {
        identity,
        teams,
        mutations: mutations.clone(),
    });
    Fixture {
        app,
        pool,
        administrator,
        mutations,
        _guard: guard,
    }
}

async fn seed_administrator(pool: &PgPool) -> ActorPrincipal {
    let actor_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let name = format!("team-admin-{}", user_id.simple());
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO actors (id, isenabled, type) VALUES ($1, TRUE, 'User')")
        .bind(actor_id)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users (id, actorid, createdat, createdbyactorid, email, name) VALUES ($1, $2, $3, $4, $5, $6)").bind(user_id).bind(actor_id).bind(Utc::now()).bind(SYSTEM_ACTOR_ID).bind(format!("{name}@example.test")).bind(&name).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO actorroles (actorid, roleid) VALUES ($1, $2)")
        .bind(actor_id)
        .bind(ADMIN_ROLE_ID)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    ActorPrincipal {
        subject_id: user_id,
        actor_id: ActorId::new(actor_id),
        name,
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec!["Admin".to_owned()],
    }
}

async fn seed_team(pool: &PgPool, name: &str, enabled: bool) -> (Uuid, Uuid) {
    let actor_id = Uuid::now_v7();
    let team_id = Uuid::now_v7();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO actors (id, isenabled, type) VALUES ($1, $2, 'Team')")
        .bind(actor_id)
        .bind(enabled)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO teams (id, actorid, name) VALUES ($1, $2, $3)")
        .bind(team_id)
        .bind(actor_id)
        .bind(name)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    (team_id, actor_id)
}

async fn seed_user(pool: &PgPool, name: &str) -> (Uuid, Uuid) {
    let actor_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO actors (id, isenabled, type) VALUES ($1, TRUE, 'User')")
        .bind(actor_id)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users (id, actorid, createdbyactorid, email, name) VALUES ($1, $2, $3, $4, $5)").bind(user_id).bind(actor_id).bind(SYSTEM_ACTOR_ID).bind(format!("{name}@example.test")).bind(name).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    (user_id, actor_id)
}

async fn seed_service_account(pool: &PgPool, name: &str, archived: bool) -> (Uuid, Uuid) {
    let actor_id = Uuid::now_v7();
    let id = Uuid::now_v7();
    let now = Utc::now();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO actors (id, isenabled, type) VALUES ($1, TRUE, 'ServiceAccount')")
        .bind(actor_id)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO serviceaccounts (id, actorid, archivedatutc, createdat, createdbyactorid, name, updatedat) VALUES ($1, $2, $3, $4, $5, $6, $4)").bind(id).bind(actor_id).bind(archived.then_some(now)).bind(now).bind(SYSTEM_ACTOR_ID).bind(name).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    (id, actor_id)
}

async fn seed_role(pool: &PgPool, name: &str, role_type: &str) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO roles (id, name, roletype) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(name)
        .bind(role_type)
        .execute(pool)
        .await
        .unwrap();
    id
}

async fn activity_count(pool: &PgPool, id: Uuid, event: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM activityevents WHERE resourceid = $1 AND eventtype = $2",
    )
    .bind(id)
    .bind(event)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn send(fixture: &Fixture, method: Method, uri: &str, body: Option<Value>) -> Response<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    let mut request = builder
        .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
        .unwrap();
    request
        .extensions_mut()
        .insert(fixture.administrator.clone());
    fixture.app.clone().oneshot(request).await.unwrap()
}

async fn send_json(
    fixture: &Fixture,
    method: Method,
    uri: &str,
    body: Value,
    content_type: &str,
) -> Response<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", content_type)
        .body(Body::from(body.to_string()))
        .unwrap();
    request
        .extensions_mut()
        .insert(fixture.administrator.clone());
    fixture.app.clone().oneshot(request).await.unwrap()
}

async fn send_without_principal(fixture: &Fixture, method: Method, uri: &str) -> Response<Body> {
    fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn json(response: Response<Body>) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap()).unwrap()
}
