use super::*;

pub(super) async fn verify(app: &Router, pool: &sqlx::PgPool, admin: &ActorPrincipal) {
    assert_capabilities(app, admin, true, true, true).await;

    let actor_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'User')")
        .bind(actor_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users(id,actorid,createdbyactorid,email,name) VALUES($1,$2,$3,$4,'Stack capabilities')")
        .bind(user_id).bind(actor_id).bind(SYSTEM_ACTOR_ID)
        .bind(format!("{user_id}@example.test")).execute(pool).await.unwrap();
    let reader = ActorPrincipal {
        subject_id: user_id,
        actor_id: ActorId::new(actor_id),
        name: "Stack capabilities".into(),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec![],
    };
    assert_capabilities(app, &reader, false, false, false).await;

    // A per-resource grant must not grant collection-level create permission.
    sqlx::query("INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,4,$3,$4,0)")
        .bind(Uuid::now_v7()).bind(actor_id).bind(Uuid::now_v7())
        .bind(citadel_primitives::ResourceType::Stack as i32).execute(pool).await.unwrap();
    assert_capabilities(app, &reader, false, false, false).await;

    let role_id = Uuid::now_v7();
    sqlx::query("INSERT INTO roles(id,name,roletype) VALUES($1,$2,'Custom')")
        .bind(role_id)
        .bind(format!("stack-capabilities-{role_id}"))
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO actorroles(actorid,roleid) VALUES($1,$2)")
        .bind(actor_id)
        .bind(role_id)
        .execute(pool)
        .await
        .unwrap();
    use citadel_identity::{RolePermissionDetails, RoleRepository};
    use citadel_primitives::PermissionLevel;
    let roles = citadel_adapters::persistence::postgres::identity::roles::repository::PostgresRoleRepository::new(pool.clone());
    for (level, write, execute) in [
        (PermissionLevel::Read, false, false),
        (PermissionLevel::Write, true, false),
        (PermissionLevel::Execute, true, true),
    ] {
        // Exercise the real ACL mutation path so committed changes invalidate the authorization cache.
        roles
            .patch_permissions(
                role_id,
                Some(&[RolePermissionDetails {
                    resource_type: citadel_primitives::ResourceType::Stack,
                    permission_level: level,
                    specific_permissions: Vec::new(),
                }]),
                admin.actor_id,
                chrono::Utc::now(),
                true,
            )
            .await
            .unwrap();
        assert_capabilities(app, &reader, true, write, execute).await;
    }
}

async fn assert_capabilities(
    app: &Router,
    principal: &ActorPrincipal,
    read: bool,
    write: bool,
    execute: bool,
) {
    let response = response_json(
        request(
            app,
            Method::GET,
            "/api/v1/stacks",
            Some(principal.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(
        response["capabilities"],
        json!({
            "canRead":read,"canWrite":write,"canExecute":execute
        }),
        "Stack collection capabilities must use the frontend/.NET contract"
    );
}

pub(super) async fn verify_releases(
    app: &Router,
    pool: &sqlx::PgPool,
    admin: &ActorPrincipal,
    id: Uuid,
) {
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
    assert_eq!(
        detail["capabilities"]["canViewReleases"], true,
        "administrators must be able to open Releases"
    );
    for (level, specific, allowed) in [
        (1, 0, false),
        (1, 32, false),
        (1, 64, true),
        (2, 64, true),
        (4, 96, true),
    ] {
        // Each case starts with a fresh actor before its first authorization read.
        let actor = Uuid::now_v7();
        sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'User')")
            .bind(actor)
            .execute(pool)
            .await
            .unwrap();
        let reader = ActorPrincipal {
            subject_id: actor,
            actor_id: ActorId::new(actor),
            name: "release-reader".into(),
            principal_type: AuthenticatedPrincipalType::User,
            credential_id: None,
            roles: vec![],
        };
        let access = Uuid::now_v7();
        sqlx::query("INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,1,$3,2,0)").bind(access).bind(actor).bind(id).execute(pool).await.unwrap();

        sqlx::query(
            "UPDATE resourceaccesses SET specificpermissions=$2,permissionlevel=$3 WHERE id=$1",
        )
        .bind(access)
        .bind(specific)
        .bind(level)
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
        assert_eq!(detail["capabilities"]["canViewReleases"], allowed);
        let response = request(
            app,
            Method::GET,
            &format!("/api/v1/stacks/{id}/releases"),
            Some(reader.clone()),
            None,
        )
        .await;
        assert_eq!(
            response.status(),
            if allowed {
                StatusCode::OK
            } else {
                StatusCode::FORBIDDEN
            }
        );
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

pub(super) async fn verify_runtime(
    app: &Router,
    pool: &sqlx::PgPool,
    admin: &ActorPrincipal,
    id: Uuid,
) {
    let flags = ["canViewLogs", "canInspect", "canOpenTerminal", "canPull"];
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
    for flag in flags {
        assert_eq!(
            detail["capabilities"][flag], true,
            "administrator capability {flag}"
        );
    }
    for (level, specific, enabled) in [
        (1, 0, [false, false, false, false]),
        (1, 1, [true, false, false, false]),
        (1, 2, [false, true, false, false]),
        (1, 16, [false, false, true, false]),
        (1, 8, [false, false, false, true]),
        (1, 96, [false, false, false, false]),
        (2, 127, [true, true, true, true]),
        (4, 127, [true, true, true, true]),
    ] {
        // Each case starts with a fresh actor before its first authorization read.
        let actor = Uuid::now_v7();
        sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'User')")
            .bind(actor)
            .execute(pool)
            .await
            .unwrap();
        let reader = ActorPrincipal {
            subject_id: actor,
            actor_id: ActorId::new(actor),
            name: "runtime-reader".into(),
            principal_type: AuthenticatedPrincipalType::User,
            credential_id: None,
            roles: vec![],
        };
        let access = Uuid::now_v7();
        sqlx::query("INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,1,$3,2,0)").bind(access).bind(actor).bind(id).execute(pool).await.unwrap();

        sqlx::query(
            "UPDATE resourceaccesses SET specificpermissions=$2,permissionlevel=$3 WHERE id=$1",
        )
        .bind(access)
        .bind(specific)
        .bind(level)
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
        assert_eq!(detail["capabilities"]["canRead"], true);
        assert_eq!(detail["capabilities"]["canWrite"], level >= 2);
        for (flag, allowed) in flags.into_iter().zip(enabled) {
            assert_eq!(
                detail["capabilities"][flag], allowed,
                "level={level}, specific={specific}, {flag}"
            );
        }
        let list = response_json(
            request(
                app,
                Method::GET,
                "/api/v1/stacks",
                Some(reader.clone()),
                None,
            )
            .await,
        )
        .await;
        let listed = list["stacks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|stack| stack["id"] == id.to_string())
            .unwrap();
        assert_eq!(listed["capabilities"], detail["capabilities"]);
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
