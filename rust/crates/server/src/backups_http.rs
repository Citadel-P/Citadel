use crate::capabilities::ResourceCapabilities;
use crate::contract_router::ContractRouterExt;
use crate::identity_http::{IdentityHttpResult, identity_result, no_store};
use axum::extract::{Extension, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::{Json, Router};
use citadel_backups::{
    BackupError, BackupLog, BackupPolicyInput, BackupRepositoryInput, BackupRestoreRequest,
    BackupService,
};
use citadel_contracts::http::routes;
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
        Router::new()
            .contract_route(routes::LIST_BACKUP_REPOSITORIES, list_repositories)
            .contract_route(routes::CREATE_BACKUP_REPOSITORY, create_repository)
            .contract_route(routes::GET_BACKUP_REPOSITORY, get_repository)
            .contract_route(routes::UPDATE_BACKUP_REPOSITORY, update_repository)
            .contract_route(routes::ARCHIVE_BACKUP_REPOSITORY, archive_repository)
            .contract_route(routes::VALIDATE_BACKUP_REPOSITORY, validate_repository)
            .contract_route(routes::INITIALIZE_BACKUP_REPOSITORY, initialize_repository)
            .contract_route(routes::CHECK_BACKUP_REPOSITORY, check_repository)
            .contract_route(routes::PRUNE_BACKUP_REPOSITORY, prune_repository)
            .contract_route(routes::LIST_BACKUP_POLICIES, list_policies)
            .contract_route(routes::GET_PLATFORM_BACKUP_SUMMARIES, platform_summaries)
            .contract_route(
                routes::GET_DEPLOYMENT_BACKUP_SOURCE_PREVIEW,
                deployment_source_preview,
            )
            .contract_route(
                routes::GET_STACK_BACKUP_SOURCE_PREVIEW,
                stack_source_preview,
            )
            .contract_route(
                routes::GET_SWARM_SERVICE_BACKUP_SOURCE_PREVIEW,
                service_source_preview,
            )
            .contract_route(routes::CREATE_BACKUP_POLICY, create_policy)
            .contract_route(routes::GET_BACKUP_POLICY, get_policy)
            .contract_route(routes::UPDATE_BACKUP_POLICY, update_policy)
            .contract_route(routes::RENAME_BACKUP_POLICY, rename_policy)
            .contract_route(
                routes::UPDATE_BACKUP_POLICY_METADATA,
                update_policy_metadata,
            )
            .contract_route(routes::ARCHIVE_BACKUP_POLICY, archive_policy)
            .contract_route(routes::QUEUE_BACKUP_RUN, queue_backup)
            .contract_route(routes::RUN_BACKUP_POLICY, streams::run_backup)
            .contract_route(routes::LIST_BACKUP_RUNS, list_runs)
            .contract_route(routes::GET_BACKUP_RUN, get_run)
            .contract_route(routes::GET_BACKUP_LOGS, get_backup_logs)
            .contract_route(routes::GET_BACKUP_RUN_EVENTS, get_backup_events)
            .contract_route(routes::CANCEL_BACKUP_RUN, cancel_backup)
            .contract_route(routes::QUEUE_BACKUP_RESTORE, queue_restore)
            .contract_route(routes::RUN_BACKUP_RESTORE_VOLUME, streams::run_restore)
            .contract_route(routes::LIST_BACKUP_RESTORES, list_restores)
            .contract_route(routes::GET_BACKUP_RESTORE, get_restore)
            .contract_route(routes::GET_BACKUP_RESTORE_LOGS, get_restore_logs)
            .contract_route(routes::GET_BACKUP_RESTORE_RUN_EVENTS, get_restore_events)
            .contract_route(routes::CANCEL_BACKUP_RESTORE, cancel_restore)
            .with_state(state),
        "BackupPolicy",
    )
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Repositories {
    repositories: Vec<citadel_backups::BackupRepositoryView>,
    capabilities: ResourceCapabilities,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Policies {
    policies: Vec<citadel_backups::BackupPolicyView>,
    capabilities: ResourceCapabilities,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Runs {
    runs: Vec<citadel_backups::BackupRunView>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Restores {
    runs: Vec<citadel_backups::BackupRestoreRunView>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Logs {
    run_id: Uuid,
    logs: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Events {
    run_id: Uuid,
    events: Vec<String>,
}
#[derive(Deserialize, Default)]
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
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestoreInput {
    target_platform_id: Uuid,
    target_volume_name: String,
    overwrite_existing: bool,
    target_docker_node_id: Option<String>,
    source_backup_run_item_id: Option<Uuid>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RepositoryLocationInput {
    location: String,
    platform_id: Option<Uuid>,
}

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
async fn create_repository(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    Json(mut i): Json<BackupRepositoryInput>,
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
async fn get_repository(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
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
    let Json(patch) = identity_result(
        body.map_err(|_| IdentityError::Validation("The request body is invalid.".into())),
        &h,
    )?;
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

async fn archive_repository(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
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
async fn validate_repository(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    h: HeaderMap,
    Json(input): Json<RepositoryLocationInput>,
) -> IdentityHttpResult {
    repository_operation(s, p, id, h, input, "Validate").await
}
async fn initialize_repository(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    h: HeaderMap,
    Json(input): Json<RepositoryLocationInput>,
) -> IdentityHttpResult {
    repository_operation(s, p, id, h, input, "Initialize").await
}
async fn check_repository(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    h: HeaderMap,
    Json(input): Json<RepositoryLocationInput>,
) -> IdentityHttpResult {
    repository_operation(s, p, id, h, input, "Check").await
}
async fn prune_repository(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    h: HeaderMap,
    Json(input): Json<RepositoryLocationInput>,
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

async fn create_policy(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    Json(mut i): Json<BackupPolicyInput>,
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
    ($handler:ident,$kind:ident,$resource:ident) => {
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
backup_source_preview!(deployment_source_preview, Deployment, Deployment);
backup_source_preview!(stack_source_preview, Stack, Stack);
backup_source_preview!(service_source_preview, SwarmService, SwarmService);

async fn get_policy(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
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
    let Json(patch) = identity_result(
        body.map_err(|_| IdentityError::Validation("The request body is invalid.".into())),
        &h,
    )?;
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
    let Json(mut input) = identity_result(
        body.map_err(|_| IdentityError::Validation("The request body is invalid.".into())),
        &h,
    )?;
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
    let Json(patch) = identity_result(
        body.map_err(|_| IdentityError::Validation("The request body is invalid.".into())),
        &h,
    )?;
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
async fn archive_policy(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
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
async fn queue_backup(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    h: HeaderMap,
    input: Option<Json<QueueInput>>,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let input = input.map(|Json(v)| v).unwrap_or_default();
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
async fn list_runs(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Query(f): Query<RunFilter>,
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
async fn get_run(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
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
async fn get_backup_events(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
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

async fn get_restore_events(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
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

async fn get_backup_logs(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
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
async fn cancel_backup(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
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
async fn queue_restore(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    h: HeaderMap,
    Json(i): Json<RestoreInput>,
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
async fn list_restores(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Query(f): Query<RunFilter>,
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
async fn get_restore(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
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
async fn get_restore_logs(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
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
async fn cancel_restore(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
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
            BackupError::Validation(m) => IdentityError::Validation(m),
            BackupError::NotFound => IdentityError::NotFound,
            BackupError::Conflict(m) => IdentityError::Conflict(m),
            BackupError::Storage(m) => IdentityError::Storage(m),
            BackupError::External(m) => IdentityError::External(m),
        }),
        h,
    )
}
