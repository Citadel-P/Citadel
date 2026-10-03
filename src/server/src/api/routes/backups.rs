//! Backups HTTP routes, authorization and local request handling.
use crate::api::resources::backups::patch::typed_patch;
use crate::{
    api::{
        error::{ApiError, HttpResult, api_result, no_store},
        resources::backups::{
            patch::*,
            requests::{QueueInput, RepositoryLocationInput, RestoreInput, RunFilter, *},
            spec::BackupExecutionLocation,
            views::{Events, Logs, Policies, Repositories, Restores, Runs, *},
        },
    },
    openapi::router::OpenApiRouterExt,
    request_validation::{ApiPath, ApiQuery, ValidatedJson},
};

use axum::{
    Json, Router,
    body::Body,
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

use citadel_backups::{
    BackupError, BackupLog, BackupPolicyConfiguration, BackupRepositoryConfiguration,
    BackupRestoreRequest, BackupService,
    permissions::*,
    runs::progress::{BackupProgressItem, is_terminal},
};

use citadel_identity::{ActorPrincipal, IdentityService, PermissionGrant};

use citadel_primitives::{PermissionLevel, PermissionPolicy, ResourceType};

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use uuid::Uuid;

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
        .normalized_routes(utoipa_axum::routes!(run_backup))
        .normalized_routes(utoipa_axum::routes!(list_runs))
        .normalized_routes(utoipa_axum::routes!(get_run))
        .normalized_routes(utoipa_axum::routes!(get_backup_logs))
        .normalized_routes(utoipa_axum::routes!(get_backup_events))
        .normalized_routes(utoipa_axum::routes!(cancel_backup))
        .normalized_routes(utoipa_axum::routes!(queue_restore))
        .normalized_routes(utoipa_axum::routes!(run_restore))
        .normalized_routes(utoipa_axum::routes!(list_restores))
        .normalized_routes(utoipa_axum::routes!(get_restore))
        .normalized_routes(utoipa_axum::routes!(get_restore_logs))
        .normalized_routes(utoipa_axum::routes!(get_restore_events))
        .normalized_routes(utoipa_axum::routes!(cancel_restore))
}

macro_rules! backup_source_preview {
    ($(#[$handler_attr:meta])* $handler:ident,$kind:ident,$resource:ident) => {
        $(#[$handler_attr])*
        async fn $handler(
            State(s): State<BackupsHttpState>,
            p: Option<Extension<ActorPrincipal>>,
            path: Result<Path<Uuid>, axum::extract::rejection::PathRejection>,
            h: HeaderMap,
        ) -> HttpResult {
            let p = actor(p, &h)?;
            let Path(id) = api_result(
                path.map_err(|_| ApiError::Validation("Resource ID must be a UUID.".into())),
                &h,
            )?;
            if id.is_nil() {
                return api_result(
                    Err(ApiError::Validation("Resource ID is required.".into())),
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
            Ok(no_store(Json(api_result(BackupSourcePreview::try_from(preview).map_err(ApiError::internal), &h)?).into_response()))
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
        (status = 200, description = "Success", body = BackupSourcePreview, content_type = "application/json"),
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
        (status = 200, description = "Success", body = BackupSourcePreview, content_type = "application/json"),
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
        (status = 200, description = "Success", body = BackupSourcePreview, content_type = "application/json"),
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

fn actor(v: Option<Extension<ActorPrincipal>>, h: &HeaderMap) -> HttpResult<ActorPrincipal> {
    api_result(v.map(|Extension(v)| v).ok_or(ApiError::Unauthenticated), h)
}

async fn auth(
    s: &BackupsHttpState,
    p: &ActorPrincipal,
    t: ResourceType,
    l: PermissionLevel,
    id: Option<Uuid>,
    h: &HeaderMap,
) -> HttpResult<PermissionGrant> {
    let permission = match id {
        Some(id) => s.identity.permission_for_resource(p, t, id).await,
        None => s.identity.global_permission(p, t).await,
    };
    let permission = api_result(permission, h)?;
    api_result(
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
                citadel_primitives::EffectivePermission::Granted {
                    level: grant.level,
                    specifics: citadel_primitives::SpecificPermissions::EMPTY,
                }
                .allows(requirement)
            })
            .ok_or(ApiError::Forbidden),
        h,
    )
}

fn result<T>(r: Result<T, BackupError>, h: &HeaderMap) -> HttpResult<T> {
    api_result(
        r.map_err(|e| match e {
            BackupError::LicenseRequired => ApiError::LicenseRequired("automated-operations"),
            BackupError::Validation(m) => crate::request_validation::validation_error(m),
            BackupError::NotFound => ApiError::NotFound,
            BackupError::Conflict(m) => ApiError::Conflict(m),
            source @ BackupError::Storage(_) => ApiError::internal(source),
            BackupError::External(m) => ApiError::External(m),
        }),
        h,
    )
}

#[utoipa::path(
    get,
    path = "/api/v1/backupPolicies",
    operation_id = "listBackupPolicies",
    tag = "BackupPolicies",
    summary = "List Backup Policies",
    responses(
        (status = 200, description = "Success", body = Policies, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("tags" = Option<Vec<String>>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list_policies(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
) -> HttpResult {
    let p = actor(p, &h)?;
    let records = result(
        s.backups
            .store()
            .list_policies(p.actor_id, p.is_administrator())
            .await,
        &h,
    )?;
    let ids = records.iter().map(|item| item.id).collect::<Vec<_>>();
    let permissions = api_result(
        s.identity
            .permissions_for_resources(&p, ResourceType::BackupPolicy, &ids)
            .await,
        &h,
    )?;
    let items = records
        .into_iter()
        .map(|item| {
            let id = item.id;
            let mut view = api_result(
                BackupPolicyView::try_from(item).map_err(ApiError::internal),
                &h,
            )?;
            view.capabilities = Some(crate::api::resource_access::capabilities_from_permission(
                permissions.get(&id).copied().flatten(),
            ));
            Ok::<_, crate::api::error::HttpError>(view)
        })
        .collect::<Result<_, _>>()?;
    let permission = api_result(
        s.identity
            .global_permission(&p, ResourceType::BackupPolicy)
            .await,
        &h,
    )?;
    Ok(no_store(
        Json(Policies {
            policies: items,
            capabilities: permission.into(),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/backupPolicies",
    operation_id = "createBackupPolicy",
    tag = "BackupPolicies",
    summary = "Create a Backup Policy",
    request_body = BackupPolicyInput,
    responses(
        (status = 200, description = "Success", body = BackupPolicyView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create_policy(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    ValidatedJson(i): ValidatedJson<BackupPolicyInput>,
) -> HttpResult {
    let mut i: citadel_backups::BackupPolicyConfiguration = i.into();
    let p = actor(p, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Write,
        None,
        &h,
    )
    .await?;
    result(i.validate(p.actor_id), &h)?;
    authorize_policy_dependencies(&s, &p, &i, &h).await?;
    api_result(
        s.identity
            .ensure_run_as_allowed(
                &p,
                citadel_primitives::ActorId::new(
                    i.run_as_actor_id.expect("validation sets the run-as Actor"),
                ),
            )
            .await,
        &h,
    )?;
    Ok(no_store(
        Json(api_result(
            BackupPolicyView::try_from(result(
                s.backups.store().create_policy(p.actor_id, &i).await,
                &h,
            )?)
            .map_err(ApiError::internal),
            &h,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/backupPolicies/{id}",
    operation_id = "getBackupPolicy",
    tag = "BackupPolicies",
    summary = "Get a Backup Policy",
    responses(
        (status = 200, description = "Success", body = BackupPolicyView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_policy(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> HttpResult {
    let p = actor(p, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Read,
        Some(id),
        &h,
    )
    .await?;
    let mut view = api_result(
        BackupPolicyView::try_from(result(s.backups.store().get_policy(id).await, &h)?)
            .map_err(ApiError::internal),
        &h,
    )?;
    let permission = api_result(
        s.identity
            .permission_for_resource(&p, ResourceType::BackupPolicy, id)
            .await,
        &h,
    )?;
    view.capabilities = Some(crate::api::resource_access::capabilities_from_permission(
        permission,
    ));
    Ok(no_store(Json(view).into_response()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/backupPolicies/{id}",
    operation_id = "updateBackupPolicy",
    tag = "BackupPolicies",
    summary = "Update a Backup Policy",
    request_body(content(
        (UpdateBackupPolicyInput = "application/merge-patch+json"),
        (UpdateBackupPolicyInput = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = BackupPolicyView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_policy(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, axum::extract::rejection::PathRejection>,
    h: HeaderMap,
    body: Result<Json<serde_json::Value>, axum::extract::rejection::JsonRejection>,
) -> HttpResult {
    let p = actor(p, &h)?;
    let Path(id) = api_result(
        path.map_err(|_| ApiError::Validation("Policy ID must be a UUID.".into())),
        &h,
    )?;
    if id.is_nil() {
        return api_result(
            Err(ApiError::Validation("Policy ID is required.".into())),
            &h,
        );
    }
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Write,
        Some(id),
        &h,
    )
    .await?;
    let Json(patch) = api_result(body.map_err(crate::request_validation::invalid_json), &h)?;
    let patch = api_result(typed_patch::<UpdateBackupPolicyInput>(patch), &h)?;
    let current = result(s.backups.store().get_policy(id).await, &h)?;
    let input = result(
        citadel_backups::policies::patch::merge(&current, &patch),
        &h,
    )?;
    if citadel_backups::policies::patch::changes_paid_trigger(&current, &input) {
        result(s.backups.ensure_automated_operations().await, &h)?;
    }
    if citadel_backups::policies::patch::changes_execution(&patch) {
        api_result(
            s.identity
                .ensure_run_as_allowed(
                    &p,
                    citadel_primitives::ActorId::new(
                        input.run_as_actor_id.expect("validated run-as Actor"),
                    ),
                )
                .await,
            &h,
        )?;
    }
    if patch.get("source").is_some_and(|v| !v.is_null())
        || patch
            .get("backupRepositoryId")
            .is_some_and(|v| !v.is_null())
    {
        authorize_policy_dependencies(&s, &p, &input, &h).await?;
        let repository = result(
            s.backups
                .store()
                .get_repository(input.backup_repository_id)
                .await,
            &h,
        )?;
        let cancellation = s.cancellation.child_token();
        let _guard = cancellation.clone().drop_guard();
        result(
            tokio::time::timeout(
                std::time::Duration::from_secs(30),
                s.backups
                    .planner()
                    .validate_source(&input.source, &repository, &cancellation),
            )
            .await
            .unwrap_or_else(|_| {
                Err(BackupError::External(
                    "Backup source validation timed out.".into(),
                ))
            }),
            &h,
        )?;
    }
    Ok(no_store(
        Json(api_result(
            BackupPolicyView::try_from(result(
                s.backups
                    .store()
                    .update_policy(p.actor_id, id, current.row_version, &input)
                    .await,
                &h,
            )?)
            .map_err(ApiError::internal),
            &h,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/backupPolicies/rename",
    operation_id = "renameBackupPolicy",
    tag = "BackupPolicies",
    summary = "Rename a Backup Policy",
    request_body = RenameBackupPolicyInput,
    responses(
        (status = 200, description = "Success", body = BackupPolicyView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn rename_policy(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    body: Result<Json<RenameBackupPolicyInput>, axum::extract::rejection::JsonRejection>,
) -> HttpResult {
    let p = actor(p, &h)?;
    let Json(input) = api_result(body.map_err(crate::request_validation::invalid_json), &h)?;
    let mut input: citadel_backups::policies::metadata::RenameBackupPolicyInput = input.into();
    result(input.validate(), &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Write,
        Some(input.id),
        &h,
    )
    .await?;
    Ok(no_store(
        Json(api_result(
            BackupPolicyView::try_from(result(
                s.backups.store().rename_policy(p.actor_id, &input).await,
                &h,
            )?)
            .map_err(ApiError::internal),
            &h,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/backupPolicies/{id}/_metadata",
    operation_id = "updateBackupPolicyMetadata",
    tag = "BackupPolicies",
    summary = "Update Backup Policy metadata",
    request_body(content(
        (BackupMetadataPatch = "application/merge-patch+json"),
        (BackupMetadataPatch = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = BackupPolicyView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_policy_metadata(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, axum::extract::rejection::PathRejection>,
    h: HeaderMap,
    body: Result<Json<serde_json::Value>, axum::extract::rejection::JsonRejection>,
) -> HttpResult {
    let p = actor(p, &h)?;
    let Path(id) = api_result(
        path.map_err(|_| ApiError::Validation("Policy ID must be a UUID.".into())),
        &h,
    )?;
    if id.is_nil() {
        return api_result(
            Err(ApiError::Validation("Policy ID is required.".into())),
            &h,
        );
    }
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Write,
        Some(id),
        &h,
    )
    .await?;
    let Json(patch) = api_result(body.map_err(crate::request_validation::invalid_json), &h)?;
    let patch = api_result(typed_patch::<BackupMetadataPatch>(patch), &h)?;
    let description = result(
        citadel_backups::policies::metadata::description_patch(&patch),
        &h,
    )?;
    Ok(no_store(
        Json(api_result(
            BackupPolicyView::try_from(result(
                s.backups
                    .store()
                    .update_policy_description(p.actor_id, id, description)
                    .await,
                &h,
            )?)
            .map_err(ApiError::internal),
            &h,
        )?)
        .into_response(),
    ))
}

async fn authorize_policy_dependencies(
    state: &BackupsHttpState,
    principal: &ActorPrincipal,
    input: &BackupPolicyConfiguration,
    headers: &HeaderMap,
) -> HttpResult<()> {
    auth(
        state,
        principal,
        ResourceType::BackupRepository,
        PermissionLevel::Read,
        Some(input.backup_repository_id),
        headers,
    )
    .await?;
    let resource = input.source.resource();
    if let Some((resource_type, resource_id)) = resource {
        auth(
            state,
            principal,
            resource_type,
            PermissionLevel::Read,
            Some(resource_id),
            headers,
        )
        .await?;
    }
    Ok(())
}

#[utoipa::path(
    delete,
    path = "/api/v1/backupPolicies/{id}",
    operation_id = "archiveBackupPolicy",
    tag = "BackupPolicies",
    summary = "Archive a Backup Policy",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn archive_policy(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> HttpResult {
    let p = actor(p, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Write,
        Some(id),
        &h,
    )
    .await?;
    result(s.backups.store().archive_policy(id).await, &h)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(
    get,
    path = "/api/v1/backupRepositories",
    operation_id = "listBackupRepositories",
    tag = "BackupRepositories",
    summary = "List Backup Repositories",
    responses(
        (status = 200, description = "Success", body = Repositories, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list_repositories(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
) -> HttpResult {
    let p = actor(p, &h)?;
    let records = result(
        s.backups
            .store()
            .list_repositories(p.actor_id, p.is_administrator())
            .await,
        &h,
    )?;
    let ids = records.iter().map(|item| item.id).collect::<Vec<_>>();
    let permissions = api_result(
        s.identity
            .permissions_for_resources(&p, ResourceType::BackupRepository, &ids)
            .await,
        &h,
    )?;
    let items = records
        .into_iter()
        .map(|item| {
            let id = item.id;
            let mut view = api_result(
                BackupRepositoryView::try_from(item).map_err(ApiError::internal),
                &h,
            )?;
            view.capabilities = Some(crate::api::resource_access::capabilities_from_permission(
                permissions.get(&id).copied().flatten(),
            ));
            Ok::<_, crate::api::error::HttpError>(view)
        })
        .collect::<Result<_, _>>()?;
    let permission = api_result(
        s.identity
            .global_permission(&p, ResourceType::BackupRepository)
            .await,
        &h,
    )?;
    Ok(no_store(
        Json(Repositories {
            repositories: items,
            capabilities: permission.into(),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/backupRepositories",
    operation_id = "createBackupRepository",
    tag = "BackupRepositories",
    summary = "Create a Backup Repository",
    request_body = BackupRepositoryInput,
    responses(
        (status = 200, description = "Success", body = BackupRepositoryView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create_repository(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    ValidatedJson(i): ValidatedJson<BackupRepositoryInput>,
) -> HttpResult {
    let mut i: citadel_backups::BackupRepositoryConfiguration = i.into();
    let p = actor(p, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupRepository,
        PermissionLevel::Write,
        None,
        &h,
    )
    .await?;
    result(i.validate(), &h)?;
    authorize_repository_dependencies(&s, &p, &i, &h).await?;
    Ok(no_store(
        Json(api_result(
            BackupRepositoryView::try_from(result(
                s.backups.store().create_repository(p.actor_id, &i).await,
                &h,
            )?)
            .map_err(ApiError::internal),
            &h,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/backupRepositories/{id}",
    operation_id = "getBackupRepository",
    tag = "BackupRepositories",
    summary = "Get a Backup Repository",
    responses(
        (status = 200, description = "Success", body = BackupRepositoryView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_repository(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> HttpResult {
    let p = actor(p, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupRepository,
        PermissionLevel::Read,
        Some(id),
        &h,
    )
    .await?;
    let mut view = api_result(
        BackupRepositoryView::try_from(result(s.backups.store().get_repository(id).await, &h)?)
            .map_err(ApiError::internal),
        &h,
    )?;
    let permission = api_result(
        s.identity
            .permission_for_resource(&p, ResourceType::BackupRepository, id)
            .await,
        &h,
    )?;
    view.capabilities = Some(crate::api::resource_access::capabilities_from_permission(
        permission,
    ));
    Ok(no_store(Json(view).into_response()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/backupRepositories/{id}",
    operation_id = "updateBackupRepository",
    tag = "BackupRepositories",
    summary = "Update a Backup Repository",
    request_body(content(
        (UpdateBackupRepositoryInput = "application/merge-patch+json"),
        (UpdateBackupRepositoryInput = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = BackupRepositoryView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_repository(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, axum::extract::rejection::PathRejection>,
    h: HeaderMap,
    body: Result<Json<serde_json::Value>, axum::extract::rejection::JsonRejection>,
) -> HttpResult {
    let p = actor(p, &h)?;
    let Path(id) = api_result(
        path.map_err(|_| ApiError::Validation("Repository ID must be a UUID.".into())),
        &h,
    )?;
    if id.is_nil() {
        return api_result(
            Err(ApiError::Validation("Repository ID is required.".into())),
            &h,
        );
    }
    auth(
        &s,
        &p,
        ResourceType::BackupRepository,
        PermissionLevel::Write,
        Some(id),
        &h,
    )
    .await?;
    let Json(patch) = api_result(body.map_err(crate::request_validation::invalid_json), &h)?;
    let patch = api_result(typed_patch::<UpdateBackupRepositoryInput>(patch), &h)?;
    if patch.get("spec").is_some_and(|v| !v.is_null()) {
        let current = result(s.backups.store().get_repository(id).await, &h)?;
        let input = result(
            citadel_backups::repositories::patch::apply(&current, &patch),
            &h,
        )?;
        authorize_repository_dependencies(&s, &p, &input, &h).await?;
    }
    Ok(no_store(
        Json(api_result(
            BackupRepositoryView::try_from(result(
                s.backups.store().update_repository(id, &patch).await,
                &h,
            )?)
            .map_err(ApiError::internal),
            &h,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/backupRepositories/{id}",
    operation_id = "archiveBackupRepository",
    tag = "BackupRepositories",
    summary = "Archive a Backup Repository",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn archive_repository(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> HttpResult {
    let p = actor(p, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupRepository,
        PermissionLevel::Write,
        Some(id),
        &h,
    )
    .await?;
    result(s.backups.store().archive_repository(id).await, &h)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(
    post,
    path = "/api/v1/backupRepositories/{id}/validate",
    operation_id = "validateBackupRepository",
    tag = "BackupRepositories",
    summary = "Validate a Backup Repository",
    request_body = RepositoryLocationInput,
    responses(
        (status = 200, description = "Success", body = BackupRepositoryValidationView, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn validate_repository(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
    ValidatedJson(input): ValidatedJson<RepositoryLocationInput>,
) -> HttpResult {
    repository_operation(s, p, id, h, input, "Validate").await
}

#[utoipa::path(
    post,
    path = "/api/v1/backupRepositories/{id}/initialize",
    operation_id = "initializeBackupRepository",
    tag = "BackupRepositories",
    summary = "Initialize a Backup Repository",
    request_body = RepositoryLocationInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn initialize_repository(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
    ValidatedJson(input): ValidatedJson<RepositoryLocationInput>,
) -> HttpResult {
    repository_operation(s, p, id, h, input, "Initialize").await
}

#[utoipa::path(
    post,
    path = "/api/v1/backupRepositories/{id}/check",
    operation_id = "checkBackupRepository",
    tag = "BackupRepositories",
    summary = "Check a Backup Repository",
    request_body = RepositoryLocationInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn check_repository(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
    ValidatedJson(input): ValidatedJson<RepositoryLocationInput>,
) -> HttpResult {
    repository_operation(s, p, id, h, input, "Check").await
}

#[utoipa::path(
    post,
    path = "/api/v1/backupRepositories/{id}/prune",
    operation_id = "pruneBackupRepository",
    tag = "BackupRepositories",
    summary = "Prune a Backup Repository",
    request_body = RepositoryLocationInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn prune_repository(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
    ValidatedJson(input): ValidatedJson<RepositoryLocationInput>,
) -> HttpResult {
    repository_operation(s, p, id, h, input, "Prune").await
}

async fn repository_operation(
    s: BackupsHttpState,
    p: Option<Extension<ActorPrincipal>>,
    id: Uuid,
    h: HeaderMap,
    input: RepositoryLocationInput,
    op: &str,
) -> HttpResult {
    let p = actor(p, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupRepository,
        PermissionLevel::Execute,
        Some(id),
        &h,
    )
    .await?;
    validate_repository_location(&input, &h)?;
    if let Some(platform_id) = input.platform_id {
        auth(
            &s,
            &p,
            ResourceType::Platform,
            PermissionLevel::Execute,
            Some(platform_id),
            &h,
        )
        .await?;
    }
    let operation_cancellation = s.cancellation.child_token();
    let operation = result(
        s.backups
            .repository_operation(
                id,
                op,
                match input.location {
                    BackupExecutionLocation::Core => "Core",
                    BackupExecutionLocation::Platform => "Platform",
                },
                input.platform_id,
                &operation_cancellation,
            )
            .await,
        &h,
    )?;
    if op == "Validate" {
        return Ok(no_store(
            Json(api_result(
                BackupRepositoryValidationView::try_from(operation.validation)
                    .map_err(ApiError::internal),
                &h,
            )?)
            .into_response(),
        ));
    }
    if let Some(message) = operation.error_message {
        return result(Err(BackupError::External(message)), &h);
    }
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn authorize_repository_dependencies(
    state: &BackupsHttpState,
    principal: &ActorPrincipal,
    input: &BackupRepositoryConfiguration,
    headers: &HeaderMap,
) -> HttpResult<()> {
    auth(
        state,
        principal,
        ResourceType::Binding,
        PermissionLevel::Read,
        None,
        headers,
    )
    .await?;
    if let Some(platform_id) = input.spec.platform_id() {
        auth(
            state,
            principal,
            ResourceType::Platform,
            PermissionLevel::Read,
            Some(platform_id),
            headers,
        )
        .await?;
    }
    Ok(())
}

fn validate_repository_location(
    input: &RepositoryLocationInput,
    headers: &HeaderMap,
) -> HttpResult<()> {
    let valid = match input.location {
        BackupExecutionLocation::Core => input.platform_id.is_none(),
        BackupExecutionLocation::Platform => input.platform_id.is_some_and(|id| !id.is_nil()),
    };
    api_result(
        if valid {
            Ok(())
        } else {
            Err(ApiError::Validation(
                "Backup execution location is invalid.".to_owned(),
            ))
        },
        headers,
    )
}

#[utoipa::path(
    get,
    path = "/api/v1/backupRestoreRuns/{id}/events",
    operation_id = "getBackupRestoreRunEvents",
    tag = "BackupRestoreRuns",
    summary = "Get Backup Restore Run events",
    responses(
        (status = 200, description = "Success", body = Events, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_restore_events(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> HttpResult {
    let p = actor(p, &h)?;
    let restore = result(s.backups.store().get_restore(id).await, &h)?;
    let source = result(s.backups.store().get_run(restore.backup_run_id).await, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Read,
        Some(source.backup_policy_id),
        &h,
    )
    .await?;
    Ok(no_store(
        Json(Events {
            run_id: id,
            events: Vec::new(),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/backupRuns/{id}/restoreVolume",
    operation_id = "restoreBackupVolume",
    tag = "BackupRuns",
    summary = "Queue a Volume restore",
    request_body = RestoreInput,
    responses(
        (status = 200, description = "Success", body = BackupRestoreRunView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn queue_restore(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
    ValidatedJson(i): ValidatedJson<RestoreInput>,
) -> HttpResult {
    let p = actor(p, &h)?;
    Ok(no_store(
        Json(api_result(
            BackupRestoreRunView::try_from(enqueue_restore(&s, &p, id, i, &h).await?)
                .map_err(ApiError::internal),
            &h,
        )?)
        .into_response(),
    ))
}

async fn enqueue_restore(
    s: &BackupsHttpState,
    p: &ActorPrincipal,
    id: Uuid,
    i: RestoreInput,
    h: &HeaderMap,
) -> HttpResult<citadel_backups::BackupRestoreRun> {
    let source = result(s.backups.store().get_run(id).await, h)?;
    api_result(
        s.identity
            .require_resource::<citadel_backups::permissions::RestoreBackupPolicy>(
                p,
                source.backup_policy_id,
            )
            .await,
        h,
    )?;
    auth(
        s,
        p,
        ResourceType::Platform,
        PermissionLevel::Write,
        Some(i.target_platform_id),
        h,
    )
    .await?;
    let run = result(
        s.backups
            .store()
            .enqueue_restore(BackupRestoreRequest {
                actor: p.actor_id,
                backup_run_id: id,
                target_platform_id: i.target_platform_id,
                target_volume_name: i.target_volume_name,
                overwrite_existing: i.overwrite_existing,
                target_docker_node_id: i.target_docker_node_id,
                source_backup_run_item_id: i.source_backup_run_item_id,
            })
            .await,
        h,
    )?;
    Ok(run)
}

#[utoipa::path(
    get,
    path = "/api/v1/backupRestoreRuns",
    operation_id = "listBackupRestoreRuns",
    tag = "BackupRestoreRuns",
    summary = "List Backup Restore Runs",
    responses(
        (status = 200, description = "Success", body = Restores, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("backupRunId" = Option<uuid::Uuid>, Query), ("policyId" = Option<uuid::Uuid>, Query), ("limit" = Option<i32>, Query, minimum = 1, maximum = 100, extensions(("x-citadel-default" = json!(50))))),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list_restores(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiQuery(f): ApiQuery<RunFilter>,
    h: HeaderMap,
) -> HttpResult {
    let p = actor(p, &h)?;
    if let Some(id) = f.policy_id {
        auth(
            &s,
            &p,
            ResourceType::BackupPolicy,
            PermissionLevel::Read,
            Some(id),
            &h,
        )
        .await?;
    }
    Ok(no_store(
        Json(Restores {
            runs: api_result(
                result(
                    s.backups
                        .store()
                        .list_restores(
                            p.actor_id,
                            p.is_administrator(),
                            f.backup_run_id,
                            f.policy_id,
                            f.limit.unwrap_or(50),
                        )
                        .await,
                    &h,
                )?
                .into_iter()
                .map(BackupRestoreRunView::try_from)
                .collect::<Result<_, _>>()
                .map_err(ApiError::internal),
                &h,
            )?,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/backupRestoreRuns/{id}",
    operation_id = "getBackupRestoreRun",
    tag = "BackupRestoreRuns",
    summary = "Get a Backup Restore Run",
    responses(
        (status = 200, description = "Success", body = BackupRestoreRunView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_restore(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> HttpResult {
    let p = actor(p, &h)?;
    let restore = result(s.backups.store().get_restore(id).await, &h)?;
    let source = result(s.backups.store().get_run(restore.backup_run_id).await, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Read,
        Some(source.backup_policy_id),
        &h,
    )
    .await?;
    Ok(no_store(
        Json(api_result(
            BackupRestoreRunView::try_from(restore).map_err(ApiError::internal),
            &h,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/backupRestoreRuns/{id}/logs",
    operation_id = "getBackupRestoreRunLogs",
    tag = "BackupRestoreRuns",
    summary = "Get Backup Restore Run logs",
    responses(
        (status = 200, description = "Success", body = Logs, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_restore_logs(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> HttpResult {
    let p = actor(p, &h)?;
    let restore = result(s.backups.store().get_restore(id).await, &h)?;
    let source = result(s.backups.store().get_run(restore.backup_run_id).await, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Read,
        Some(source.backup_policy_id),
        &h,
    )
    .await?;
    Ok(no_store(
        Json(Logs {
            run_id: id,
            logs: logs_text(result(s.backups.store().restore_logs(id).await, &h)?),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/backupRestoreRuns/{id}/cancel",
    operation_id = "cancelBackupRestoreRun",
    tag = "BackupRestoreRuns",
    summary = "Cancel a Backup Restore Run",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn cancel_restore(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> HttpResult {
    let p = actor(p, &h)?;
    let restore = result(s.backups.store().get_restore(id).await, &h)?;
    let source = result(s.backups.store().get_run(restore.backup_run_id).await, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Execute,
        Some(source.backup_policy_id),
        &h,
    )
    .await?;
    result(s.backups.cancel_restore(id).await, &h)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(
    get,
    path = "/api/v1/backupPolicies/platform-summaries",
    operation_id = "getPlatformBackupSummaries",
    tag = "BackupPolicies",
    summary = "Get Platform Backup summaries",
    responses(
        (status = 200, description = "Success", body = PlatformBackupSummaries, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("platformIds" = Vec<uuid::Uuid>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn platform_summaries(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    axum::extract::RawQuery(query): axum::extract::RawQuery,
    h: HeaderMap,
) -> HttpResult {
    let p = actor(p, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Read,
        None,
        &h,
    )
    .await?;
    let mut ids = Vec::new();
    for (key, value) in url::form_urlencoded::parse(query.as_deref().unwrap_or_default().as_bytes())
    {
        if key == "platformIds" {
            let id = api_result(
                Uuid::parse_str(&value)
                    .map_err(|_| ApiError::Validation("Platform IDs must be UUIDs.".into())),
                &h,
            )?;
            if !id.is_nil() {
                ids.push(id);
            }
            if ids.len() > 256 {
                return api_result(
                    Err(ApiError::Validation(
                        "At most 256 Platform IDs can be requested.".into(),
                    )),
                    &h,
                );
            }
        }
    }
    ids.sort_unstable();
    ids.dedup();
    let platforms = result(
        s.backups
            .store()
            .platform_summaries(p.actor_id, p.is_administrator(), &ids)
            .await,
        &h,
    )?;
    Ok(no_store(
        Json(PlatformBackupSummaries {
            platforms: api_result(
                platforms
                    .into_iter()
                    .map(PlatformBackupSummary::try_from)
                    .collect::<Result<_, _>>()
                    .map_err(ApiError::internal),
                &h,
            )?,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/backupPolicies/{id}/runs",
    operation_id = "queueBackupRun",
    tag = "BackupPolicies",
    summary = "Queue a Backup Run",
    request_body = Option<QueueInput>,
    responses(
        (status = 200, description = "Success", body = BackupRunView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn queue_backup(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
    input: Option<ValidatedJson<QueueInput>>,
) -> HttpResult {
    let p = actor(p, &h)?;
    let input = input.map(|ValidatedJson(v)| v).unwrap_or_default();
    let run = enqueue_backup(&s, &p, id, input, &h).await?;
    Ok(no_store(
        Json(api_result(
            BackupRunView::try_from(run).map_err(ApiError::internal),
            &h,
        )?)
        .into_response(),
    ))
}

async fn enqueue_backup(
    s: &BackupsHttpState,
    p: &ActorPrincipal,
    id: Uuid,
    input: QueueInput,
    h: &HeaderMap,
) -> HttpResult<citadel_backups::BackupRun> {
    auth(
        s,
        p,
        ResourceType::BackupPolicy,
        PermissionLevel::Execute,
        Some(id),
        h,
    )
    .await?;
    let trigger = input.trigger.unwrap_or_default().as_str();
    if trigger != "Manual" {
        result(s.backups.ensure_automated_operations().await, h)?;
    }
    let run = result(
        s.backups
            .store()
            .enqueue_backup(p.actor_id, id, trigger)
            .await,
        h,
    )?;
    Ok(run)
}

#[utoipa::path(
    get,
    path = "/api/v1/backupRuns",
    operation_id = "listBackupRuns",
    tag = "BackupRuns",
    summary = "List Backup Runs",
    responses(
        (status = 200, description = "Success", body = Runs, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("policyId" = Option<uuid::Uuid>, Query), ("limit" = Option<i32>, Query, minimum = 1, maximum = 100, extensions(("x-citadel-default" = json!(50))))),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list_runs(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiQuery(f): ApiQuery<RunFilter>,
    h: HeaderMap,
) -> HttpResult {
    let p = actor(p, &h)?;
    if let Some(id) = f.policy_id {
        auth(
            &s,
            &p,
            ResourceType::BackupPolicy,
            PermissionLevel::Read,
            Some(id),
            &h,
        )
        .await?;
    }
    Ok(no_store(
        Json(Runs {
            runs: api_result(
                result(
                    s.backups
                        .store()
                        .list_runs(
                            p.actor_id,
                            p.is_administrator(),
                            f.policy_id,
                            f.limit.unwrap_or(50),
                        )
                        .await,
                    &h,
                )?
                .into_iter()
                .map(BackupRunView::try_from)
                .collect::<Result<_, _>>()
                .map_err(ApiError::internal),
                &h,
            )?,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/backupRuns/{id}",
    operation_id = "getBackupRun",
    tag = "BackupRuns",
    summary = "Get a Backup Run",
    responses(
        (status = 200, description = "Success", body = BackupRunView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_run(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> HttpResult {
    let p = actor(p, &h)?;
    let run = result(s.backups.store().get_run(id).await, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Read,
        Some(run.backup_policy_id),
        &h,
    )
    .await?;
    Ok(no_store(
        Json(api_result(
            BackupRunView::try_from(run).map_err(ApiError::internal),
            &h,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/backupRuns/{id}/events",
    operation_id = "getBackupRunEvents",
    tag = "BackupRuns",
    summary = "Get Backup Run events",
    responses(
        (status = 200, description = "Success", body = Events, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_backup_events(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> HttpResult {
    let p = actor(p, &h)?;
    let run = result(s.backups.store().get_run(id).await, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Read,
        Some(run.backup_policy_id),
        &h,
    )
    .await?;
    // Expose an empty events collection. Execution output remains in /logs.
    Ok(no_store(
        Json(Events {
            run_id: id,
            events: Vec::new(),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/backupRuns/{id}/logs",
    operation_id = "getBackupRunLogs",
    tag = "BackupRuns",
    summary = "Get Backup Run logs",
    responses(
        (status = 200, description = "Success", body = Logs, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_backup_logs(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> HttpResult {
    let p = actor(p, &h)?;
    let run = result(s.backups.store().get_run(id).await, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Read,
        Some(run.backup_policy_id),
        &h,
    )
    .await?;
    Ok(no_store(
        Json(Logs {
            run_id: id,
            logs: logs_text(result(s.backups.store().backup_logs(id).await, &h)?),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/backupRuns/{id}/cancel",
    operation_id = "cancelBackupRun",
    tag = "BackupRuns",
    summary = "Cancel a Backup Run",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn cancel_backup(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> HttpResult {
    let p = actor(p, &h)?;
    let run = result(s.backups.store().get_run(id).await, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Execute,
        Some(run.backup_policy_id),
        &h,
    )
    .await?;
    result(s.backups.cancel_backup(id).await, &h)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(
    post,
    path = "/api/v1/backupPolicies/{id}/run",
    operation_id = "runBackupPolicy",
    tag = "BackupPolicies",
    summary = "Run a Backup Policy with progress",
    request_body = QueueInput,
    responses(
        (status = 200, description = "Success", body = Vec<BackupRunStreamItem>, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn run_backup(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, axum::extract::rejection::PathRejection>,
    h: HeaderMap,
    body: Result<Json<QueueInput>, axum::extract::rejection::JsonRejection>,
) -> HttpResult {
    let p = actor(p, &h)?;
    let Path(id) = api_result(
        path.map_err(|_| ApiError::Validation("Policy ID must be a UUID.".into())),
        &h,
    )?;
    let Json(input) = api_result(body.map_err(crate::request_validation::invalid_json), &h)?;
    let progress = s.backups.subscribe_progress(); // Subscribe before enqueue: fast workers must not lose completion.
    let run = enqueue_backup(&s, &p, id, input, &h).await?;
    Ok(response(
        s,
        p,
        id,
        run.id,
        false,
        progress,
        format!("Backup run queued for \"{}\".", run.policy_name_snapshot),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/backupRuns/{id}/restoreVolume/run",
    operation_id = "runBackupRestoreVolume",
    tag = "BackupRuns",
    summary = "Restore a Backup Volume with progress",
    request_body = RestoreInput,
    responses(
        (status = 200, description = "Success", body = Vec<BackupRestoreRunStreamItem>, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn run_restore(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, axum::extract::rejection::PathRejection>,
    h: HeaderMap,
    body: Result<Json<RestoreInput>, axum::extract::rejection::JsonRejection>,
) -> HttpResult {
    let p = actor(p, &h)?;
    let Path(id) = api_result(
        path.map_err(|_| ApiError::Validation("Backup Run ID must be a UUID.".into())),
        &h,
    )?;
    let Json(input) = api_result(body.map_err(crate::request_validation::invalid_json), &h)?;
    let progress = s.backups.subscribe_progress();
    let run = enqueue_restore(&s, &p, id, input, &h).await?;
    let source = result(s.backups.store().get_run(id).await, &h)?;
    Ok(response(
        s,
        p,
        source.backup_policy_id,
        run.id,
        true,
        progress,
        "Volume restore queued.".into(),
    ))
}

fn response(
    state: BackupsHttpState,
    principal: ActorPrincipal,
    policy_id: Uuid,
    id: Uuid,
    restore: bool,
    mut progress: tokio::sync::broadcast::Receiver<Arc<BackupProgressItem>>,
    queued: String,
) -> axum::response::Response {
    // The durable worker owns execution. Dropping this HTTP body only releases this
    // bounded subscription; explicit Cancel remains the authority to cancel a run.
    let stream = async_stream::stream! {
        yield Ok::<_,serde_json::Error>(bytes::Bytes::from_static(b"["));
        yield encode(BackupProgressItem::message(id,"Queued",queued),restore,false);
        let mut refresh=tokio::time::interval(std::time::Duration::from_secs(15));
        let mut received_output=false;
        refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            let update=tokio::select! {
                _=state.cancellation.cancelled()=>break,
                _=refresh.tick()=>None,
                update=progress.recv()=>match update {
                    Ok(item) if item.run_id==id=>Some(item),
                    Ok(_)=>continue,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_))=>{received_output=false;None},
                    Err(tokio::sync::broadcast::error::RecvError::Closed)=>break,
                }
            };
            let authorized=state.identity.permission_for_resource(&principal,ResourceType::BackupPolicy,policy_id).await;
            if !authorized.ok().flatten().is_some_and(|p|p.allows(if restore { citadel_backups::permissions::RestoreBackupPolicy::REQUIREMENT } else { citadel_backups::permissions::ExecuteBackupPolicy::REQUIREMENT })) {break;}
            if let Some(item)=update && !is_terminal(item.status.as_deref().unwrap_or("Running")) {
                received_output |= item.status.is_none();
                yield encode((*item).clone(),restore,true);
                continue;
            }
            // Recheck persisted state after lag/restart/completion. We never start a
            // second execution, and emit terminal success only after commit succeeds.
            let current=if restore {
                state.backups.store().get_restore(id).await.map(|r|(r.status.to_string(),r.error_message,r.exit_code))
            } else {state.backups.store().get_run(id).await.map(|r|(r.status.to_string(),r.error_message,r.exit_code))};
            let (status,error,exit)=match current {
                Ok(value)=>value,
                Err(_)=>{yield encode(BackupProgressItem::message(id,"Interrupted","Run status is unavailable. Reopen the run to inspect its persisted status."),restore,true);break;}
            };
            if is_terminal(&status) {
                let logs=if restore{state.backups.store().restore_logs(id).await}else{state.backups.store().backup_logs(id).await};
                if !received_output && let Ok(logs)=logs {for log in logs {yield encode(BackupProgressItem{run_id:id,status:None,message:Some(log.message),stream:Some(log.stream),exit_code:None},restore,true);}}
                yield encode(BackupProgressItem{run_id:id,status:Some(status),message:error,stream:None,exit_code:exit},restore,true);
                break;
            }
        }
        yield Ok(bytes::Bytes::from_static(b"]"));
    };
    let mut response = no_store(Body::from_stream(stream).into_response());
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json; charset=utf-8"),
    );
    response
}

fn encode(
    item: BackupProgressItem,
    restore: bool,
    comma: bool,
) -> Result<bytes::Bytes, serde_json::Error> {
    let mut bytes = Vec::new();
    if comma {
        bytes.push(b',');
    }
    if restore {
        serde_json::to_writer(
            &mut bytes,
            &BackupRestoreRunStreamItem {
                restore_run_id: item.run_id,
                status: item
                    .status
                    .map(|v| serde_json::from_value(v.into()))
                    .transpose()?,
                message: item.message,
                stream: item.stream,
                exit_code: item.exit_code,
            },
        )?;
    } else {
        serde_json::to_writer(
            &mut bytes,
            &BackupRunStreamItem {
                run_id: item.run_id,
                status: item
                    .status
                    .map(|v| serde_json::from_value(v.into()))
                    .transpose()?,
                message: item.message,
                stream: item.stream,
                exit_code: item.exit_code,
            },
        )?;
    }
    Ok(bytes.into())
}
