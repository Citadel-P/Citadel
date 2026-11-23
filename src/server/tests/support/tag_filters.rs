//! Exercise each catalog through its ordinary HTTP reader using a resource-specific grant.
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use citadel_identity::{ActorPrincipal, AuthenticatedPrincipalType, SYSTEM_ACTOR_ID};
use citadel_primitives::{ActorId, PermissionLevel};
use citadel_tags::TaggableResourceType;
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

pub async fn verify(app: &Router, pool: &sqlx::PgPool, kind: TaggableResourceType, id: Uuid) {
    use TaggableResourceType::*;
    let (endpoint, field) = match kind {
        Registry => ("registries", "registries"),
        GitRepository => ("gitRepositories", "gitRepositories"),
        Script => ("scripts", "scripts"),
        Platform => ("platforms", "platforms"),
        BuildAgentPool => ("buildAgentPools", "pools"),
        Build => ("buildProjects", "projects"),
        Deployment => ("deployments", "deployments"),
        Stack => ("stacks", "stacks"),
        SwarmService => ("swarmServices", "swarmServices"),
        BackupPolicy => ("backupPolicies", "policies"),
    };
    let actor = Uuid::now_v7();
    let reader = ActorPrincipal {
        actor_id: ActorId::new(actor),
        subject_id: actor,
        name: format!("reader-{actor}"),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec![],
    };
    let literal = format!("Ops +%,&_{actor}");
    let uuid_name = Uuid::now_v7().to_string();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO actors(id,type,isenabled) VALUES ($1,'User',true)")
        .bind(actor)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO users(id,actorid,createdat,createdbyactorid,email,name) VALUES ($1,$1,now(),$2,$3,$4)",
    )
    .bind(actor)
    .bind(SYSTEM_ACTOR_ID)
    .bind(format!("{actor}@example.test"))
    .bind(&reader.name)
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO resourceaccesses(id,actorid,resourceid,resourcetype,permissionlevel,specificpermissions) VALUES ($1,$2,$3,$4,$5,0)",
    )
    .bind(Uuid::now_v7())
    .bind(actor)
    .bind(id)
    .bind(kind.permission_type() as i32)
    .bind(PermissionLevel::Read as i32)
    .execute(&mut *tx)
    .await
    .unwrap();
    let mut tags = Vec::new();
    for name in [&literal, &uuid_name] {
        let tag = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO tags(id,name,normalizedname,color,createdbyactorid) VALUES ($1,$2,lower($2),'#445566',$3)",
        )
        .bind(tag)
        .bind(name)
        .bind(SYSTEM_ACTOR_ID)
        .execute(&mut *tx)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO resourcetags(resourcetype,resourceid,tagid,createdbyactorid) VALUES($1,$2,$3,$4)",
        )
        .bind(kind.as_database_str())
        .bind(id)
        .bind(tag)
        .bind(SYSTEM_ACTOR_ID)
        .execute(&mut *tx)
        .await
        .unwrap();
        tags.push(tag);
    }
    tx.commit().await.unwrap();
    let folded = literal.to_uppercase();
    let tag_id = tags[0].to_string();
    for (names, visible) in [
        (vec![], true),
        (vec![literal.as_str(), &uuid_name], true),
        (vec![&folded], true),
        (vec![&uuid_name], true),
        (vec![&literal, "unassigned"], false),
        (vec![&tag_id], false),
    ] {
        let mut params = url::form_urlencoded::Serializer::new(String::new());
        if kind == Stack {
            params.append_pair("catalog", "true");
        }
        for name in names {
            params.append_pair("tags", name);
        }
        let uri = format!("/api/v1/{endpoint}?{}", params.finish());
        let request = Request::builder()
            .uri(&uri)
            .extension(reader.clone())
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{uri}");
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let rows = body[field]
            .as_array()
            .unwrap_or_else(|| panic!("{uri}: {body}"));
        let ids = rows.iter().map(|row| row["id"].clone()).collect::<Vec<_>>();
        assert_eq!(
            ids,
            if visible {
                vec![serde_json::json!(id)]
            } else {
                vec![]
            },
            "{uri}"
        );
    }
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("DELETE FROM tags WHERE id=ANY($1)")
        .bind(&tags)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("DELETE FROM resourceaccesses WHERE actorid=$1")
        .bind(actor)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("DELETE FROM users WHERE id=$1")
        .bind(actor)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("DELETE FROM actors WHERE id=$1")
        .bind(actor)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}
