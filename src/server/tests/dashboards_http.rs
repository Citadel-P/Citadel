use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use citadel_adapters::persistence::postgres::{
    dashboards::PostgresDashboardRepository,
    identity::authentication::store::StaticEntitlementService,
};
use citadel_identity::{ActorPrincipal, AuthenticatedPrincipalType};
use citadel_primitives::{ActorId, PermissionLevel, ResourceType};
use citadel_server::api::routes::dashboards::{DashboardsHttpState, router};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

async fn actor(pool: &sqlx::PgPool, admin: bool) -> ActorPrincipal {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,type,isenabled) VALUES($1,'User',true)")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    ActorPrincipal {
        actor_id: ActorId::new(id),
        subject_id: id,
        name: format!("reader-{id}"),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: if admin { vec!["Admin".into()] } else { vec![] },
    }
}

async fn send(
    app: &Router,
    who: &ActorPrincipal,
    method: Method,
    path: &str,
    body: Value,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(format!("/api/v1/{path}"))
                .header("content-type", "application/json")
                .extension(who.clone())
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn call(
    app: &Router,
    who: &ActorPrincipal,
    method: Method,
    path: &str,
    body: Value,
    status: StatusCode,
) -> Value {
    let response = send(app, who, method, path, body).await;
    let actual = response.status();
    let bytes = to_bytes(response.into_body(), 200_000).await.unwrap();
    assert_eq!(
        actual,
        status,
        "{path}: {}",
        String::from_utf8_lossy(&bytes)
    );
    if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

#[tokio::test]
#[ignore = "requires a disposable CITADEL_DASHBOARD_DATABASE_URL"]
async fn dashboard_visibility_revisions_and_defaults_follow_the_viewer() {
    let db = std::env::var("CITADEL_DASHBOARD_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&db)
        .await
        .unwrap();
    let pool = sqlx::PgPool::connect(&db).await.unwrap();
    let repository = Arc::new(PostgresDashboardRepository::new(pool.clone()));
    let app = |team| {
        router(DashboardsHttpState {
            dashboards: repository.clone(),
            entitlements: Arc::new(StaticEntitlementService::new(team)),
        })
    };
    let (community, team) = (app(false), app(true));
    let owner = actor(&pool, false).await;
    let admin = actor(&pool, true).await;
    let other = actor(&pool, false).await;
    for (actor, role) in [(&other, "Viewer"), (&owner, "Operator")] {
        let grant = sqlx::query(
            "INSERT INTO actorroles(actorid, roleid) SELECT $1,id FROM roles WHERE name=$2",
        )
        .bind(actor.actor_id.value())
        .bind(role)
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(grant.rows_affected(), 1);
    }
    let private = Uuid::now_v7();
    let shared = Uuid::now_v7();
    let restricted = Uuid::now_v7();
    let config = json!({"schemaVersion":1,"rangeMinutes":60,"tags":[],"widgets":[]});
    let board = |id, visibility| json!({"id":id,"name":"System","visibility":visibility,"configuration":config});
    let private_path = format!("dashboards/{private}");
    let restricted_path = format!("dashboards/{restricted}");
    let saved = call(
        &community,
        &owner,
        Method::POST,
        "dashboards",
        board(private, "Private"),
        StatusCode::OK,
    )
    .await;
    assert_eq!(saved["summary"]["revision"], 1);
    assert_eq!(
        call(
            &community,
            &owner,
            Method::POST,
            "dashboards",
            board(private, "Private"),
            StatusCode::OK
        )
        .await,
        saved
    );
    for viewer in [&other, &admin] {
        call(
            &community,
            viewer,
            Method::GET,
            &private_path,
            Value::Null,
            StatusCode::NOT_FOUND,
        )
        .await;
    }
    call(
        &community,
        &other,
        Method::POST,
        "dashboards",
        board(shared, "InstanceShared"),
        StatusCode::FORBIDDEN,
    )
    .await;
    call(
        &community,
        &admin,
        Method::POST,
        "dashboards",
        board(shared, "InstanceShared"),
        StatusCode::OK,
    )
    .await;
    let denied = call(
        &community,
        &admin,
        Method::POST,
        "dashboards",
        board(restricted, "Restricted"),
        StatusCode::FORBIDDEN,
    )
    .await;
    assert_eq!(denied["capability"], "CustomAccessControl");
    call(
        &team,
        &admin,
        Method::POST,
        "dashboards",
        board(restricted, "Restricted"),
        StatusCode::OK,
    )
    .await;
    let picker = call(
        &community,
        &other,
        Method::GET,
        "dashboards",
        Value::Null,
        StatusCode::OK,
    )
    .await;
    assert_eq!(picker["dashboards"].as_array().unwrap().len(), 1);
    assert_eq!(picker["dashboards"][0]["id"], json!(shared));
    assert!(picker["dashboards"][0].get("configuration").is_none());
    call(
        &community,
        &other,
        Method::GET,
        &restricted_path,
        Value::Null,
        StatusCode::NOT_FOUND,
    )
    .await;
    sqlx::query("INSERT INTO resourceaccesses(id,actorid,resourceid,resourcetype,permissionlevel,specificpermissions) VALUES ($1,$2,$3,$4,$5,0)")
        .bind(Uuid::now_v7()).bind(other.actor_id.value()).bind(restricted)
        .bind(ResourceType::Dashboard as i32).bind(PermissionLevel::Write as i32).execute(&pool).await.unwrap();
    call(
        &community,
        &owner,
        Method::GET,
        &restricted_path,
        Value::Null,
        StatusCode::NOT_FOUND,
    )
    .await;
    let mut patch = json!({"name":"Updated","revision":1,"configuration":config});
    patch["visibility"] = json!("InstanceShared");
    call(
        &community,
        &other,
        Method::PUT,
        &restricted_path,
        patch.clone(),
        StatusCode::BAD_REQUEST,
    )
    .await;
    patch.as_object_mut().unwrap().remove("visibility");
    let updated = call(
        &community,
        &other,
        Method::PUT,
        &restricted_path,
        patch.clone(),
        StatusCode::OK,
    )
    .await;
    assert_eq!(updated["summary"]["revision"], 2);
    call(
        &community,
        &admin,
        Method::PUT,
        &restricted_path,
        patch,
        StatusCode::CONFLICT,
    )
    .await;
    let concurrent = |name| json!({"name":name,"revision":2,"configuration":config});
    let (first, second) = tokio::join!(
        send(
            &community,
            &admin,
            Method::PUT,
            &restricted_path,
            concurrent("Admin change")
        ),
        send(
            &community,
            &other,
            Method::PUT,
            &restricted_path,
            concurrent("Writer change")
        ),
    );
    let mut statuses = [first.status(), second.status()];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::OK, StatusCode::CONFLICT]);
    // An older client may still read a newer definition, but saving must preserve it.
    sqlx::query("UPDATE dashboards SET configuration=jsonb_set(configuration,'{schemaVersion}','2') WHERE id=$1")
        .bind(restricted).execute(&pool).await.unwrap();
    call(
        &community,
        &other,
        Method::PUT,
        &restricted_path,
        json!({"name":"Old editor","revision":3,"configuration":config}),
        StatusCode::BAD_REQUEST,
    )
    .await;
    let newer = call(
        &community,
        &other,
        Method::GET,
        &restricted_path,
        Value::Null,
        StatusCode::OK,
    )
    .await;
    assert_eq!(newer["configuration"]["schemaVersion"], 2);
    assert_eq!(newer["summary"]["revision"], 3);
    call(
        &community,
        &other,
        Method::DELETE,
        &format!("{restricted_path}?revision=2"),
        Value::Null,
        StatusCode::FORBIDDEN,
    )
    .await;
    call(
        &community,
        &admin,
        Method::PUT,
        "instance/dashboard-default",
        json!({"dashboardId":shared}),
        StatusCode::NO_CONTENT,
    )
    .await;
    call(
        &community,
        &owner,
        Method::PUT,
        "instance/dashboard-default",
        json!({"dashboardId":private}),
        StatusCode::FORBIDDEN,
    )
    .await;
    call(
        &community,
        &owner,
        Method::PUT,
        "profile/dashboard-default",
        json!({"dashboardId":private}),
        StatusCode::NO_CONTENT,
    )
    .await;
    let default = |id| json!({"dashboardId":id});
    assert_eq!(
        call(
            &community,
            &owner,
            Method::GET,
            "profile/dashboard-default",
            Value::Null,
            StatusCode::OK
        )
        .await,
        default(private)
    );
    call(
        &community,
        &owner,
        Method::DELETE,
        &format!("{private_path}?revision=1"),
        Value::Null,
        StatusCode::NO_CONTENT,
    )
    .await;
    assert_eq!(
        call(
            &community,
            &owner,
            Method::GET,
            "profile/dashboard-default",
            Value::Null,
            StatusCode::OK
        )
        .await,
        default(shared)
    );
    call(
        &community,
        &other,
        Method::PUT,
        "profile/dashboard-default",
        default(restricted),
        StatusCode::NO_CONTENT,
    )
    .await;
    sqlx::query("DELETE FROM resourceaccesses WHERE actorid=$1 AND resourceid=$2")
        .bind(other.actor_id.value())
        .bind(restricted)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        call(
            &community,
            &other,
            Method::GET,
            "profile/dashboard-default",
            Value::Null,
            StatusCode::OK
        )
        .await,
        default(shared)
    );
}
