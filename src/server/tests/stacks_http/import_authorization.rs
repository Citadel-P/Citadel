use super::*;
use citadel_identity::{RolePermissionDetails, RoleRepository};
use citadel_primitives::{PermissionLevel, ResourceType};
use std::sync::atomic::AtomicUsize;

#[derive(Default)]
pub(super) struct Source(AtomicUsize);
impl citadel_stacks::StackSourceMaterializerPort for Source {
    fn materialize<'a>(
        &'a self,
        _: &'a StackOperationClaim,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackApplySource, StackError>> {
        Box::pin(async move {
            self.0.fetch_add(1, Ordering::Relaxed);
            Err(StackError::NotFound)
        })
    }
}

async fn actor(
    pool: &sqlx::PgPool,
    admin: &ActorPrincipal,
    write: bool,
    platform: Option<Uuid>,
    service_account: bool,
) -> ActorPrincipal {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,$2)")
        .bind(id)
        .bind(if service_account {
            "ServiceAccount"
        } else {
            "User"
        })
        .execute(pool)
        .await
        .unwrap();
    if write {
        let role = Uuid::now_v7();
        sqlx::query("INSERT INTO roles(id,name,roletype) VALUES($1,$2,'Custom')")
            .bind(role)
            .bind(format!("import-{role}"))
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO actorroles(actorid,roleid) VALUES($1,$2)")
            .bind(id)
            .bind(role)
            .execute(pool)
            .await
            .unwrap();
        citadel_adapters::persistence::postgres::identity::roles::repository::PostgresRoleRepository::new(pool.clone())
            .patch_permissions(role, Some(&[RolePermissionDetails {
                resource_type: ResourceType::Stack, permission_level: PermissionLevel::Write,
                specific_permissions: vec![],
            }]), admin.actor_id, chrono::Utc::now(), true).await.unwrap();
    }
    if let Some(platform) = platform {
        sqlx::query("INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,1,$3,$4,0)")
            .bind(Uuid::now_v7()).bind(id).bind(platform).bind(ResourceType::Platform as i32)
            .execute(pool).await.unwrap();
    }
    ActorPrincipal {
        subject_id: id,
        actor_id: ActorId::new(id),
        name: "import boundary".into(),
        principal_type: if service_account {
            AuthenticatedPrincipalType::ServiceAccount
        } else {
            AuthenticatedPrincipalType::User
        },
        credential_id: None,
        roles: vec![],
    }
}

pub(super) async fn verify(
    app: &Router,
    pool: &sqlx::PgPool,
    admin: &ActorPrincipal,
    runtime: &CompletingStackRuntime,
    source: &Source,
    other_platform: Uuid,
    platform: Uuid,
) {
    let project = "authorization-source";
    *runtime.import_claim.lock().unwrap() = Some(StackImportClaim {
        orphaned_owner_id: None,
        platform_id: platform,
        platform_name: "fixture".into(),
        project_name: project.into(),
        import_kind: StackImportKind::SwarmStack,
        runtime_fingerprint: "authorization-fingerprint".into(),
        service_names: vec!["api".into()],
        container_ids: vec![],
        container_names: vec![],
        services: vec![ComposeProjectRuntimeService {
            name: "api".into(),
            image: Some("nginx:alpine".into()),
            container_count: 1,
            states: vec!["running:1".into()],
        }],
    });
    let path = format!("/api/v1/platforms/{platform}/unmanaged-compose-projects/{project}");
    let body = json!({"name":"authorization-import", "stackSource":"WebEditor",
        "spec":{"$type":"WebEditor","composeFile":"services:\n  api:\n    image: nginx:alpine\n", "updateBehavior":"Disabled", "projectName":project}});
    let writer = actor(pool, admin, true, None, false).await;
    let other_reader = actor(pool, admin, true, Some(other_platform), false).await;
    let reader = actor(pool, admin, false, Some(platform), false).await;
    let allowed = actor(pool, admin, true, Some(platform), false).await;
    let service = actor(pool, admin, true, Some(platform), true).await;
    let mut git = body.clone();
    git["stackSource"] = json!("Git");
    git["spec"] = json!({"$type":"Git","gitRepoId":Uuid::now_v7(), "branch":"main", "composePaths":["compose.yml"], "updateBehavior":"Disabled"});
    for (principal, input) in [
        (&writer, &body),
        (&other_reader, &body),
        (&reader, &body),
        (&allowed, &git),
        (&service, &git),
    ] {
        for kind in [None, Some("SwarmStack")] {
            let mut input = input.clone();
            if let Some(kind) = kind {
                input["importKind"] = json!(kind);
            }
            for endpoint in ["import-draft", "import"] {
                let mut input = input.clone();
                if endpoint == "import" {
                    input["previewFingerprint"] = json!("stale");
                }
                let reads = runtime.import_reads.load(Ordering::Relaxed);
                let materials = source.0.load(Ordering::Relaxed);
                let response = request(
                    app,
                    Method::POST,
                    &format!("{path}/{endpoint}"),
                    Some(principal.clone()),
                    Some(input.clone()),
                )
                .await;
                assert_eq!(
                    response.status(),
                    StatusCode::FORBIDDEN,
                    "{endpoint} kind={kind:?}"
                );
                assert_eq!(
                    runtime.import_reads.load(Ordering::Relaxed),
                    reads,
                    "denial must precede runtime reads"
                );
                assert_eq!(
                    source.0.load(Ordering::Relaxed),
                    materials,
                    "denial must precede materialization"
                );
            }
        }
    }
    for principal in [&reader, &allowed, &service, admin] {
        assert_eq!(
            request(app, Method::GET, &path, Some(principal.clone()), None)
                .await
                .status(),
            StatusCode::OK
        );
    }
    for principal in [&allowed, &service, admin] {
        for kind in [None, Some("SwarmStack")] {
            let mut input = body.clone();
            if let Some(kind) = kind {
                input["importKind"] = json!(kind);
            }
            assert_eq!(
                request(
                    app,
                    Method::POST,
                    &format!("{path}/import-draft"),
                    Some(principal.clone()),
                    Some(input)
                )
                .await
                .status(),
                StatusCode::OK
            );
        }
    }
    for kind in [None, Some("SwarmStack")] {
        let mut stale = body.clone();
        stale["previewFingerprint"] = json!("stale");
        if let Some(kind) = kind {
            stale["importKind"] = json!(kind);
        }
        assert_eq!(
            request(
                app,
                Method::POST,
                &format!("{path}/import"),
                Some(admin.clone()),
                Some(stale)
            )
            .await
            .status(),
            StatusCode::CONFLICT
        );
    }
    *runtime.import_claim.lock().unwrap() = None;
}
