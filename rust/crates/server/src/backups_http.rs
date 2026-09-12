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
    BackupError, BackupLog, BackupPolicyInput, BackupRepositoryInput, BackupRestoreRequest,
    BackupService,
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
#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct Repositories {
    repositories: Vec<citadel_backups::BackupRepositoryView>,
    capabilities: ResourceCapabilities,
}
#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct Policies {
    policies: Vec<citadel_backups::BackupPolicyView>,
    capabilities: ResourceCapabilities,
}
#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = server::backups_http::Runs)]
#[serde(rename_all = "camelCase")]
struct Runs {
    runs: Vec<citadel_backups::BackupRunView>,
}
#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct Restores {
    runs: Vec<citadel_backups::BackupRestoreRunView>,
}
#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = server::backups_http::Logs)]
#[serde(rename_all = "camelCase")]
struct Logs {
    run_id: Uuid,
    logs: String,
}
#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = server::backups_http::Events)]
#[serde(rename_all = "camelCase")]
struct Events {
    run_id: Uuid,
    events: Vec<String>,
}
#[derive(Deserialize, Default, utoipa::ToSchema)]
#[schema(as = server::backups_http::QueueInput)]
#[serde(rename_all = "camelCase")]
struct QueueInput {
    trigger: Option<String>,
}
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RunFilter {
    policy_id: Option<Uuid>,
    backup_run_id: Option<Uuid>,
    limit: Option<usize>,
}
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct RestoreInput {
    target_platform_id: Uuid,
    target_volume_name: String,
    overwrite_existing: bool,
    target_docker_node_id: Option<String>,
    source_backup_run_item_id: Option<Uuid>,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct RepositoryLocationInput {
    location: String,
    platform_id: Option<Uuid>,
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
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let permission = identity_result(
        s.identity
            .global_permission(&p, ResourceType::BackupRepository)
            .await,
        &h,
    )?;
    Ok(no_store(
        Json(Repositories {
            repositories: result(
                s.backups
                    .store()
                    .list_repositories(p.actor_id, p.is_administrator())
                    .await,
                &h,
            )?,
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
        (status = 200, description = "Success", body = citadel_backups::BackupRepositoryView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create_repository(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    ValidatedJson(mut i): ValidatedJson<BackupRepositoryInput>,
) -> IdentityHttpResult {
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
        Json(result(
            s.backups.store().create_repository(p.actor_id, &i).await,
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
        (status = 200, description = "Success", body = citadel_backups::BackupRepositoryView, content_type = "application/json"),
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
) -> IdentityHttpResult {
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
    Ok(no_store(
        Json(result(s.backups.store().get_repository(id).await, &h)?).into_response(),
    ))
}
#[utoipa::path(
    patch,
    path = "/api/v1/backupRepositories/{id}",
    operation_id = "updateBackupRepository",
    tag = "BackupRepositories",
    summary = "Update a Backup Repository",
    request_body(content(
        (ref("#/components/schemas/UpdateBackupRepositoryInput") = "application/merge-patch+json"),
        (ref("#/components/schemas/UpdateBackupRepositoryInput") = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = citadel_backups::BackupRepositoryView, content_type = "application/json"),
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
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let Path(id) = identity_result(
        path.map_err(|_| IdentityError::Validation("Repository ID must be a UUID.".into())),
        &h,
    )?;
    if id.is_nil() {
        return identity_result(
            Err(IdentityError::Validation(
                "Repository ID is required.".into(),
            )),
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
    let Json(patch) = identity_result(body.map_err(crate::request_validation::invalid_json), &h)?;
    if patch.get("spec").is_some_and(|v| !v.is_null()) {
        let current = result(s.backups.store().get_repository(id).await, &h)?;
        let input = result(
            citadel_backups::repository_patch::apply(&current, &patch),
            &h,
        )?;
        authorize_repository_dependencies(&s, &p, &input, &h).await?;
    }
    Ok(no_store(
        Json(result(
            s.backups.store().update_repository(id, &patch).await,
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
) -> IdentityHttpResult {
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
        (status = 200, description = "Success", body = citadel_backups::BackupRepositoryValidationView, content_type = "application/json"),
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
) -> IdentityHttpResult {
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
) -> IdentityHttpResult {
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
) -> IdentityHttpResult {
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
) -> IdentityHttpResult {
    repository_operation(s, p, id, h, input, "Prune").await
}
async fn repository_operation(
    s: BackupsHttpState,
    p: Option<Extension<ActorPrincipal>>,
    id: Uuid,
    h: HeaderMap,
    input: RepositoryLocationInput,
    op: &str,
) -> IdentityHttpResult {
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
                &input.location,
                input.platform_id,
                &operation_cancellation,
            )
            .await,
        &h,
    )?;
    if op == "Validate" {
        return Ok(no_store(Json(operation.validation).into_response()));
    }
    if let Some(message) = operation.error_message {
        return result(Err(BackupError::External(message)), &h);
    }
    Ok(StatusCode::NO_CONTENT.into_response())
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
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let permission = identity_result(
        s.identity
            .global_permission(&p, ResourceType::BackupPolicy)
            .await,
        &h,
    )?;
    Ok(no_store(
        Json(Policies {
            policies: result(
                s.backups
                    .store()
                    .list_policies(p.actor_id, p.is_administrator())
                    .await,
                &h,
            )?,
            capabilities: permission.into(),
        })
        .into_response(),
    ))
}
#[utoipa::path(
    get,
    path = "/api/v1/backupPolicies/platform-summaries",
    operation_id = "getPlatformBackupSummaries",
    tag = "BackupPolicies",
    summary = "Get Platform Backup summaries",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/PlatformBackupSummariesView"), content_type = "application/json"),
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
) -> IdentityHttpResult {
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
            let id = identity_result(
                Uuid::parse_str(&value)
                    .map_err(|_| IdentityError::Validation("Platform IDs must be UUIDs.".into())),
                &h,
            )?;
            if !id.is_nil() {
                ids.push(id);
            }
            if ids.len() > 256 {
                return identity_result(
                    Err(IdentityError::Validation(
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
        Json(serde_json::json!({"platforms":platforms})).into_response(),
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
        (status = 200, description = "Success", body = citadel_backups::BackupPolicyView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create_policy(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    ValidatedJson(mut i): ValidatedJson<BackupPolicyInput>,
) -> IdentityHttpResult {
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
    identity_result(
        s.identity
            .ensure_run_as_allowed(
                &p,
                citadel_domain::ActorId::new(
                    i.run_as_actor_id.expect("validation sets the run-as Actor"),
                ),
            )
            .await,
        &h,
    )?;
    Ok(no_store(
        Json(result(
            s.backups.store().create_policy(p.actor_id, &i).await,
            &h,
        )?)
        .into_response(),
    ))
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
                        citadel_backups::source_preview::BackupPreviewKind::$kind,
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
            Ok(no_store(Json(preview).into_response()))
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

#[utoipa::path(
    get,
    path = "/api/v1/backupPolicies/{id}",
    operation_id = "getBackupPolicy",
    tag = "BackupPolicies",
    summary = "Get a Backup Policy",
    responses(
        (status = 200, description = "Success", body = citadel_backups::BackupPolicyView, content_type = "application/json"),
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
) -> IdentityHttpResult {
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
    Ok(no_store(
        Json(result(s.backups.store().get_policy(id).await, &h)?).into_response(),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/backupPolicies/{id}",
    operation_id = "updateBackupPolicy",
    tag = "BackupPolicies",
    summary = "Update a Backup Policy",
    request_body(content(
        (ref("#/components/schemas/UpdateBackupPolicyInput") = "application/merge-patch+json"),
        (ref("#/components/schemas/UpdateBackupPolicyInput") = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = citadel_backups::BackupPolicyView, content_type = "application/json"),
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
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let Path(id) = identity_result(
        path.map_err(|_| IdentityError::Validation("Policy ID must be a UUID.".into())),
        &h,
    )?;
    if id.is_nil() {
        return identity_result(
            Err(IdentityError::Validation("Policy ID is required.".into())),
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
    let Json(patch) = identity_result(body.map_err(crate::request_validation::invalid_json), &h)?;
    let current = result(s.backups.store().get_policy(id).await, &h)?;
    let input = result(citadel_backups::policy_update::merge(&current, &patch), &h)?;
    if citadel_backups::policy_update::changes_paid_trigger(&current, &input) {
        result(s.backups.ensure_automated_operations().await, &h)?;
    }
    if citadel_backups::policy_update::changes_execution(&patch) {
        identity_result(
            s.identity
                .ensure_run_as_allowed(
                    &p,
                    citadel_domain::ActorId::new(
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
        Json(result(
            s.backups
                .store()
                .update_policy(p.actor_id, id, current.row_version, &input)
                .await,
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
    request_body = citadel_backups::policy_metadata::RenameBackupPolicyInput,
    responses(
        (status = 200, description = "Success", body = citadel_backups::BackupPolicyView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn rename_policy(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    body: Result<
        Json<citadel_backups::policy_metadata::RenameBackupPolicyInput>,
        axum::extract::rejection::JsonRejection,
    >,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let Json(mut input) =
        identity_result(body.map_err(crate::request_validation::invalid_json), &h)?;
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
        Json(result(
            s.backups.store().rename_policy(p.actor_id, &input).await,
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
        (ref("#/components/schemas/PatchResourceMetadata") = "application/merge-patch+json"),
        (ref("#/components/schemas/PatchResourceMetadata") = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = citadel_backups::BackupPolicyView, content_type = "application/json"),
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
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let Path(id) = identity_result(
        path.map_err(|_| IdentityError::Validation("Policy ID must be a UUID.".into())),
        &h,
    )?;
    if id.is_nil() {
        return identity_result(
            Err(IdentityError::Validation("Policy ID is required.".into())),
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
    let Json(patch) = identity_result(body.map_err(crate::request_validation::invalid_json), &h)?;
    let description = result(
        citadel_backups::policy_metadata::description_patch(&patch),
        &h,
    )?;
    Ok(no_store(
        Json(result(
            s.backups
                .store()
                .update_policy_description(p.actor_id, id, description)
                .await,
            &h,
        )?)
        .into_response(),
    ))
}

async fn authorize_repository_dependencies(
    state: &BackupsHttpState,
    principal: &ActorPrincipal,
    input: &BackupRepositoryInput,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    auth(
        state,
        principal,
        ResourceType::Binding,
        PermissionLevel::Read,
        None,
        headers,
    )
    .await?;
    if let Some(platform_id) = json_uuid(&input.spec, "platformId") {
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

async fn authorize_policy_dependencies(
    state: &BackupsHttpState,
    principal: &ActorPrincipal,
    input: &BackupPolicyInput,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    auth(
        state,
        principal,
        ResourceType::BackupRepository,
        PermissionLevel::Read,
        Some(input.backup_repository_id),
        headers,
    )
    .await?;
    let resource = match input
        .source
        .get("$type")
        .and_then(serde_json::Value::as_str)
    {
        Some("DockerVolume") => {
            json_uuid(&input.source, "platformId").map(|id| (ResourceType::Platform, id))
        }
        Some("Stack") => json_uuid(&input.source, "stackId").map(|id| (ResourceType::Stack, id)),
        Some("Deployment") => {
            json_uuid(&input.source, "deploymentId").map(|id| (ResourceType::Deployment, id))
        }
        Some("SwarmService") => {
            json_uuid(&input.source, "swarmServiceId").map(|id| (ResourceType::SwarmService, id))
        }
        _ => None,
    };
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

fn json_uuid(value: &serde_json::Value, key: &str) -> Option<Uuid> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
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
) -> IdentityHttpResult {
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
    post,
    path = "/api/v1/backupPolicies/{id}/runs",
    operation_id = "queueBackupRun",
    tag = "BackupPolicies",
    summary = "Queue a Backup Run",
    request_body = Option<QueueInput>,
    responses(
        (status = 200, description = "Success", body = citadel_backups::BackupRunView, content_type = "application/json"),
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
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let input = input.map(|ValidatedJson(v)| v).unwrap_or_default();
    let run = enqueue_backup(&s, &p, id, input, &h).await?;
    Ok(no_store(Json(run).into_response()))
}
async fn enqueue_backup(
    s: &BackupsHttpState,
    p: &ActorPrincipal,
    id: Uuid,
    input: QueueInput,
    h: &HeaderMap,
) -> IdentityHttpResult<citadel_backups::BackupRunView> {
    auth(
        s,
        p,
        ResourceType::BackupPolicy,
        PermissionLevel::Execute,
        Some(id),
        h,
    )
    .await?;
    let trigger = input.trigger.unwrap_or_else(|| "Manual".into());
    if !matches!(trigger.as_str(), "Manual" | "Schedule" | "Webhook") {
        return identity_result(
            Err(IdentityError::Validation(
                "Backup trigger is invalid.".into(),
            )),
            h,
        );
    }
    if trigger != "Manual" {
        result(s.backups.ensure_automated_operations().await, h)?;
    }
    let run = result(
        s.backups
            .store()
            .enqueue_backup(p.actor_id, id, &trigger)
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
) -> IdentityHttpResult {
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
            runs: result(
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
        (status = 200, description = "Success", body = citadel_backups::BackupRunView, content_type = "application/json"),
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
) -> IdentityHttpResult {
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
    Ok(no_store(Json(run).into_response()))
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
) -> IdentityHttpResult {
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
    // .NET exposes an empty events collection. Execution output remains in /logs.
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
) -> IdentityHttpResult {
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
) -> IdentityHttpResult {
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
) -> IdentityHttpResult {
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
    path = "/api/v1/backupRuns/{id}/restoreVolume",
    operation_id = "restoreBackupVolume",
    tag = "BackupRuns",
    summary = "Queue a Volume restore",
    request_body = RestoreInput,
    responses(
        (status = 200, description = "Success", body = citadel_backups::BackupRestoreRunView, content_type = "application/json"),
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
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    Ok(no_store(
        Json(enqueue_restore(&s, &p, id, i, &h).await?).into_response(),
    ))
}
async fn enqueue_restore(
    s: &BackupsHttpState,
    p: &ActorPrincipal,
    id: Uuid,
    i: RestoreInput,
    h: &HeaderMap,
) -> IdentityHttpResult<citadel_backups::BackupRestoreRunView> {
    let source = result(s.backups.store().get_run(id).await, h)?;
    let permission = auth(
        s,
        p,
        ResourceType::BackupPolicy,
        PermissionLevel::Execute,
        Some(source.backup_policy_id),
        h,
    )
    .await?;
    identity_result(
        permission
            .has_specific(citadel_domain::SpecificPermission::Restore)
            .then_some(())
            .ok_or(IdentityError::Forbidden),
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
) -> IdentityHttpResult {
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
            runs: result(
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
        (status = 200, description = "Success", body = citadel_backups::BackupRestoreRunView, content_type = "application/json"),
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
) -> IdentityHttpResult {
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
    Ok(no_store(Json(restore).into_response()))
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
) -> IdentityHttpResult {
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
) -> IdentityHttpResult {
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
fn validate_repository_location(
    input: &RepositoryLocationInput,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    let valid = match input.location.as_str() {
        "Core" => input.platform_id.is_none(),
        "Platform" => input.platform_id.is_some_and(|id| !id.is_nil()),
        _ => false,
    };
    identity_result(
        if valid {
            Ok(())
        } else {
            Err(IdentityError::Validation(
                "Backup execution location is invalid.".to_owned(),
            ))
        },
        headers,
    )
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
            .filter(|grant| grant.level.grants(l))
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
