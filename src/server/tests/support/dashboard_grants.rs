use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use citadel_identity::ActorPrincipal;
use citadel_primitives::{PermissionLevel, ResourceType};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

pub async fn verify(
    app: &Router,
    pool: &PgPool,
    admin: &ActorPrincipal,
    path: &str,
    grantee: Uuid,
) {
    let shared = Uuid::now_v7();
    let restricted = Uuid::now_v7();
    let private = Uuid::now_v7();
    sqlx::query(
        "WITH private_owner AS (
            INSERT INTO actors(id,type,isenabled) VALUES ($5,'User',true) RETURNING id
         )
         INSERT INTO dashboards(id,owneractorid,name,visibility,configuration) VALUES
            ($1,$4,'Operations','InstanceShared',$6), ($2,$4,'On call','Restricted',$6),
            ($3,(SELECT id FROM private_owner),'Personal notes','Private',$6)",
    )
    .bind(shared)
    .bind(restricted)
    .bind(private)
    .bind(admin.actor_id.value())
    .bind(Uuid::now_v7())
    .bind(json!({"schemaVersion":1,"rangeMinutes":60,"tags":[],"widgets":[]}))
    .execute(pool)
    .await
    .unwrap();

    let grants = json!({"resourceAccesses": [
        {"resourceType":"Dashboard","resourceId":shared,"permissionLevel":"Read"},
        {"resourceType":"Dashboard","resourceId":restricted,"permissionLevel":"Write"}
    ]});
    let save = Request::patch(path)
        .header("content-type", "application/merge-patch+json")
        .extension(admin.clone())
        .body(Body::from(grants.to_string()))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(save).await.unwrap().status(),
        StatusCode::OK
    );

    // A stale grant must never reveal a private board's title in the administrator's access form.
    sqlx::query("INSERT INTO resourceaccesses(id,actorid,resourcetype,resourceid,permissionlevel,specificpermissions)
        VALUES($1,$2,$3,$4,$5,0)")
        .bind(Uuid::now_v7()).bind(grantee).bind(ResourceType::Dashboard as i32)
        .bind(private).bind(PermissionLevel::Read as i32).execute(pool).await.unwrap();

    let reopen = Request::get(path)
        .extension(admin.clone())
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(reopen).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
            .unwrap();
    let saved = body["resourceAccesses"].as_array().unwrap();
    assert_eq!(saved.len(), 3);
    for (id, name, level) in [
        (shared, Some("Operations"), "Read"),
        (restricted, Some("On call"), "Write"),
        (private, None, "Read"),
    ] {
        let grant = saved
            .iter()
            .find(|grant| grant["resourceId"] == json!(id))
            .unwrap();
        assert_eq!(grant["resourceType"], "Dashboard");
        assert_eq!(grant.get("resourceName"), Some(&json!(name)));
        assert_eq!(grant["permissionLevel"], level);
    }
}
