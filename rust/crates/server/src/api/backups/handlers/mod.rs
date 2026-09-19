use citadel_backups::permissions::*;
use citadel_domain::PermissionPolicy;
mod repositories;
use repositories::*;
mod policies;
use policies::*;
mod runs;
use runs::*;
mod restores;
use crate::api::backups::views::*;
use restores::*;

use crate::api::backups::requests::*;

use crate::capabilities::ResourceCapabilities;

use crate::identity_http::{IdentityHttpResult, identity_result, no_store};

use crate::openapi::router::OpenApiRouterExt;

use crate::request_validation::ApiPath;

use crate::request_validation::ApiQuery;

use crate::request_validation::ValidatedJson;

use axum::extract::{Extension, Path, State};

use axum::http::{HeaderMap, StatusCode};

use axum::response::IntoResponse;

use axum::{Json, Router};

use citadel_backups::{
    BackupError, BackupLog, BackupPolicyConfiguration, BackupRepositoryConfiguration,
    BackupRestoreRequest, BackupService,
};

use citadel_domain::{PermissionLevel, ResourceType};

use citadel_identity::{ActorPrincipal, IdentityError, IdentityService, PermissionGrant};

use serde::{Deserialize, Serialize};

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use uuid::Uuid;

mod streams;

#[derive(Clone)]
pub struct BackupsHttpState {
    pub identity: Arc<IdentityService>,
    pub backups: Arc<BackupService>,
    pub cancellation: CancellationToken,
}

pub fn router(state: BackupsHttpState) -> Router {
    crate::realtime::notify_mutations(
        documented_routes().split_for_parts().0.with_state(state),
        "BackupPolicy",
    )
}

macro_rules! backup_source_preview {
    ($(#[$handler_attr:meta])* $handler:ident,$kind:ident,$resource:ident) => {
        $(#[$handler_attr])*
        async fn $handler(
            State(s): State<BackupsHttpState>,
            p: Option<Extension<ActorPrincipal>>,
            path: Result<Path<Uuid>, axum::extract::rejection::PathRejection>,
            h: HeaderMap,
        ) -> IdentityHttpResult {
            let p = actor(p, &h)?;
            let Path(id) = identity_result(
                path.map_err(|_| IdentityError::Validation("Resource ID must be a UUID.".into())),
                &h,
            )?;
            if id.is_nil() {
                return identity_result(
                    Err(IdentityError::Validation("Resource ID is required.".into())),
                    &h,
                );
            }
            auth(
                &s,
                &p,
                ResourceType::$resource,
                PermissionLevel::Read,
                Some(id),
                &h,
            )
            .await?;
            let cancellation = s.cancellation.child_token();
            let _guard = cancellation.clone().drop_guard();
            let preview = result(
                tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    s.backups.planner().preview(
                        citadel_backups::policies::read_models::BackupPreviewKind::$kind,
                        id,
                        p.actor_id,
                        p.is_administrator(),
                        &cancellation,
                    ),
                )
                .await
                .unwrap_or_else(|_| {
                    Err(BackupError::External(
                        "Backup source preview timed out.".into(),
                    ))
                }),
                &h,
            )?;
            Ok(no_store(Json(BackupSourcePreview::from(preview)).into_response()))
        }
    };
}

backup_source_preview!(
    #[utoipa::path(
    get,
    path = "/api/v1/deployments/{deploymentId}/backup-source-preview",
    operation_id = "getDeploymentBackupSourcePreview",
    tag = "Deployments",
    summary = "Preview Deployment Backup volumes",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/DeploymentBackupSourcePreviewView"), content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("deploymentId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    deployment_source_preview,
    Deployment,
    Deployment
);

backup_source_preview!(
    #[utoipa::path(
    get,
    path = "/api/v1/stacks/{stackId}/backup-source-preview",
    operation_id = "getStackBackupSourcePreview",
    tag = "Stacks",
    summary = "Preview Stack Backup volumes",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/StackBackupSourcePreviewView"), content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("stackId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    stack_source_preview,
    Stack,
    Stack
);

backup_source_preview!(
    #[utoipa::path(
    get,
    path = "/api/v1/swarmServices/{id}/backup-source-preview",
    operation_id = "getSwarmServiceBackupSourcePreview",
    tag = "SwarmServices",
    summary = "Preview Service Backup volumes",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/SwarmServiceBackupSourcePreviewView"), content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    service_source_preview,
    SwarmService,
    SwarmService
);

fn json_uuid(value: &serde_json::Value, key: &str) -> Option<Uuid> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
}

fn logs_text(logs: Vec<BackupLog>) -> String {
    let capacity = logs
        .iter()
        .map(|log| log.message.len().saturating_add(1))
        .sum();
    let mut output = String::with_capacity(capacity);
    for log in logs {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&log.message);
    }
    output
}

fn actor(
    v: Option<Extension<ActorPrincipal>>,
    h: &HeaderMap,
) -> IdentityHttpResult<ActorPrincipal> {
    identity_result(
        v.map(|Extension(v)| v)
            .ok_or(IdentityError::Unauthenticated),
        h,
    )
}

async fn auth(
    s: &BackupsHttpState,
    p: &ActorPrincipal,
    t: ResourceType,
    l: PermissionLevel,
    id: Option<Uuid>,
    h: &HeaderMap,
) -> IdentityHttpResult<PermissionGrant> {
    let permission = match id {
        Some(id) => s.identity.permission_for_resource(p, t, id).await,
        None => s.identity.global_permission(p, t).await,
    };
    let permission = identity_result(permission, h)?;
    identity_result(
        permission
            .filter(|grant| {
                let requirement = match (t, l) {
                    (ResourceType::BackupRepository, PermissionLevel::Read) => {
                        ReadBackupRepository::REQUIREMENT
                    }
                    (ResourceType::BackupRepository, PermissionLevel::Write) => {
                        WriteBackupRepository::REQUIREMENT
                    }
                    (ResourceType::BackupRepository, PermissionLevel::Execute) => {
                        ExecuteBackupRepository::REQUIREMENT
                    }
                    (ResourceType::BackupPolicy, PermissionLevel::Read) => {
                        ReadBackupPolicy::REQUIREMENT
                    }
                    (ResourceType::BackupPolicy, PermissionLevel::Write) => {
                        WriteBackupPolicy::REQUIREMENT
                    }
                    (ResourceType::BackupPolicy, PermissionLevel::Execute) => {
                        ExecuteBackupPolicy::REQUIREMENT
                    }
                    _ => return grant.level.grants(l),
                };
                citadel_domain::EffectivePermission::Granted {
                    level: grant.level,
                    specifics: citadel_domain::SpecificPermissions::EMPTY,
                }
                .allows(requirement)
            })
            .ok_or(IdentityError::Forbidden),
        h,
    )
}

fn result<T>(r: Result<T, BackupError>, h: &HeaderMap) -> IdentityHttpResult<T> {
    identity_result(
        r.map_err(|e| match e {
            BackupError::LicenseRequired => IdentityError::LicenseRequired("automated-operations"),
            BackupError::Validation(m) => crate::request_validation::validation_error(m),
            BackupError::NotFound => IdentityError::NotFound,
            BackupError::Conflict(m) => IdentityError::Conflict(m),
            BackupError::Storage(m) => IdentityError::Storage(m),
            BackupError::External(m) => IdentityError::External(m),
        }),
        h,
    )
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<BackupsHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(list_repositories))
        .normalized_routes(utoipa_axum::routes!(create_repository))
        .normalized_routes(utoipa_axum::routes!(get_repository))
        .normalized_routes(utoipa_axum::routes!(update_repository))
        .normalized_routes(utoipa_axum::routes!(archive_repository))
        .normalized_routes(utoipa_axum::routes!(validate_repository))
        .normalized_routes(utoipa_axum::routes!(initialize_repository))
        .normalized_routes(utoipa_axum::routes!(check_repository))
        .normalized_routes(utoipa_axum::routes!(prune_repository))
        .normalized_routes(utoipa_axum::routes!(list_policies))
        .normalized_routes(utoipa_axum::routes!(platform_summaries))
        .normalized_routes(utoipa_axum::routes!(deployment_source_preview))
        .normalized_routes(utoipa_axum::routes!(stack_source_preview))
        .normalized_routes(utoipa_axum::routes!(service_source_preview))
        .normalized_routes(utoipa_axum::routes!(create_policy))
        .normalized_routes(utoipa_axum::routes!(get_policy))
        .normalized_routes(utoipa_axum::routes!(update_policy))
        .normalized_routes(utoipa_axum::routes!(rename_policy))
        .normalized_routes(utoipa_axum::routes!(update_policy_metadata))
        .normalized_routes(utoipa_axum::routes!(archive_policy))
        .normalized_routes(utoipa_axum::routes!(queue_backup))
        .normalized_routes(utoipa_axum::routes!(streams::run_backup))
        .normalized_routes(utoipa_axum::routes!(list_runs))
        .normalized_routes(utoipa_axum::routes!(get_run))
        .normalized_routes(utoipa_axum::routes!(get_backup_logs))
        .normalized_routes(utoipa_axum::routes!(get_backup_events))
        .normalized_routes(utoipa_axum::routes!(cancel_backup))
        .normalized_routes(utoipa_axum::routes!(queue_restore))
        .normalized_routes(utoipa_axum::routes!(streams::run_restore))
        .normalized_routes(utoipa_axum::routes!(list_restores))
        .normalized_routes(utoipa_axum::routes!(get_restore))
        .normalized_routes(utoipa_axum::routes!(get_restore_logs))
        .normalized_routes(utoipa_axum::routes!(get_restore_events))
        .normalized_routes(utoipa_axum::routes!(cancel_restore))
}
