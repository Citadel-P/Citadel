use super::*;

pub(super) async fn verify(app: &Router, pool: &sqlx::PgPool, admin: &ActorPrincipal, id: Uuid) {
    let actor = Uuid::now_v7();
    let grant = Uuid::now_v7();
    let user = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'User')")
        .bind(actor)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,1,$3,$4,$5)")
        .bind(grant).bind(actor).bind(id).bind(citadel_primitives::ResourceType::SwarmService as i32)
        .bind(citadel_primitives::SpecificPermission::Apply as i32).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO users(id,actorid,createdbyactorid,email,name) VALUES($1,$2,$3,$4,'Swarm operation reader')")
        .bind(user).bind(actor).bind(SYSTEM_ACTOR_ID).bind(format!("{user}@example.test"))
        .execute(pool).await.unwrap();
    let mut reader = admin.clone();
    reader.subject_id = user;
    reader.actor_id = ActorId::new(actor);
    reader.roles.clear();
    let denied = request(
        app,
        Method::POST,
        &format!("/api/v1/swarmServices/{id}/scale"),
        Some(reader.clone()),
        Some(json!({"replicas":3})),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    for (operation, kind, body) in [
        ("apply", "Apply", None),
        ("force-update", "ForceUpdate", None),
        ("scale", "Scale", Some(json!({"replicas":3}))),
    ] {
        if operation == "scale" {
            // Use the application writer: direct SQL leaves the earlier denial cached.
            use citadel_identity::{UserPatchMutation, UserRepository, UserResourceAccessInput};
            use citadel_primitives::{PermissionLevel, ResourceType, SpecificPermission};
            citadel_adapters::persistence::postgres::identity::users::repository::PostgresUserRepository::new(pool.clone())
                .patch(user, &UserPatchMutation {
                    resource_accesses: Some(vec![UserResourceAccessInput {
                        resource_type: ResourceType::SwarmService, resource_id: id,
                        permission_level: PermissionLevel::Write,
                        specific_permissions: vec![SpecificPermission::Apply],
                    }]),
                    ..Default::default()
                }, admin.actor_id, chrono::Utc::now(), true).await.unwrap();
        }
        let response = request(
            app,
            Method::POST,
            &format!("/api/v1/swarmServices/{id}/{operation}"),
            Some(reader.clone()),
            body,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let progress = response_json(response).await;
        assert_eq!(
            progress.as_array().unwrap().last().unwrap()["isCompleted"],
            true,
            "{progress}"
        );
        let state = response_json(
            request(
                app,
                Method::GET,
                &format!("/api/v1/swarmServices/{id}"),
                Some(reader.clone()),
                None,
            )
            .await,
        )
        .await;
        assert_eq!(state["currentOperation"]["kind"], kind);
        assert_eq!(state["controlState"], "Idle");
        assert_eq!(state["capabilities"]["canApply"], true);
        if operation == "scale" {
            assert_eq!(state["spec"]["replicas"], 3);
        }
    }
    sqlx::query("DELETE FROM resourceaccesses WHERE actorid=$1")
        .bind(actor)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM activityevents WHERE createdbyactorid=$1")
        .bind(actor)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM users WHERE id=$1")
        .bind(user)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM actors WHERE id=$1")
        .bind(actor)
        .execute(pool)
        .await
        .unwrap();
}
