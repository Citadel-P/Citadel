use crate::api::git::repositories::{
    requests::{GitRepositoryPatch, NewGitRepository},
    spec::{GitRepositorySyncMode, RepoCommand},
    views::GitRepositoryView,
};

use crate::capabilities::ResourceCapabilitiesView;

use crate::identity_http::{IdentityHttpResult, identity_result, no_store};

use crate::openapi::router::OpenApiRouterExt;

use crate::request_validation::invalid_json;

use crate::api::catalog_query::{
    DeleteResourcesInput, PatchResourceMetadataInput, RenameResourceInput, parse_catalog_filters,
    principal_and_id,
};

use crate::api::resource_access::{
    authorize_global, authorize_resource, publish_resource_change, require_actor,
};

use axum::Json;

use axum::extract::rejection::{JsonRejection, PathRejection};

use axum::extract::{Extension, Path, RawQuery, State};

use axum::http::{HeaderMap, StatusCode};

use axum::response::IntoResponse;

use citadel_primitives::{PermissionLevel, ResourceType};

use citadel_identity::ActorPrincipal;

use crate::api::metadata_patch::MetadataPatch;

use crate::api::tags::dto::TagSummary;

use serde::Serialize;

use serde_json::Value;

use uuid::Uuid;

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct GitRepositoriesResponse {
    git_repositories: Vec<AuthorizedGitRepositoryView>,
    capabilities: ResourceCapabilitiesView,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct AuthorizedGitRepositoryView {
    #[serde(flatten)]
    repository: GitRepositoryView,
    capabilities: ResourceCapabilitiesView,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct GitRepositoryConfigResponse {
    id: Uuid,
    name: String,
    description: Option<String>,
    url: String,
    default_branch: String,
    git_account_id: Option<Uuid>,
    sync_mode: GitRepositorySyncMode,
    sync_interval_minutes: Option<i32>,
    webhook: Option<Value>,
    on_clone: Option<RepoCommand>,
    on_pull: Option<RepoCommand>,
    tags: Vec<crate::api::tags::dto::TagSummary>,
}

#[utoipa::path(
    get,
    path = "/api/v1/gitRepositories",
    operation_id = "listGitRepositories",
    tag = "GitRepositories",
    summary = "List Git repositories",
    responses(
        (status = 200, description = "Success", body = GitRepositoriesResponse, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("tags" = Option<Vec<String>>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list_git_repositories(
    State(state): State<GitCatalogHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    RawQuery(raw_query): RawQuery,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let filters = identity_result(parse_catalog_filters(raw_query.as_deref(), false), &headers)?;
    let global_capabilities = capabilities(
        &state,
        &principal,
        ResourceType::GitRepository,
        None,
        &headers,
    )
    .await?;
    let authorized_repositories = identity_result(
        state
            .git_repositories
            .list_git_repositories(principal.actor_id, principal.is_administrator())
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    let mut git_repositories = Vec::with_capacity(authorized_repositories.len());
    for repository in authorized_repositories.into_iter().filter(|repository| {
        filters
            .tags
            .iter()
            .all(|id| repository.tags.iter().any(|tag| tag.id == *id))
    }) {
        let row_capabilities = capabilities(
            &state,
            &principal,
            ResourceType::GitRepository,
            Some(repository.id),
            &headers,
        )
        .await?;
        git_repositories.push(AuthorizedGitRepositoryView {
            repository: repository.into(),
            capabilities: row_capabilities,
        });
    }
    Ok(no_store(
        Json(GitRepositoriesResponse {
            git_repositories,
            capabilities: global_capabilities,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/gitRepositories/{id}",
    operation_id = "getGitRepository",
    tag = "GitRepositories",
    summary = "Get a Git repository",
    responses(
        (status = 200, description = "Success", body = AuthorizedGitRepositoryView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_git_repository(
    State(state): State<GitCatalogHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let (principal, id) = principal_and_id(&headers, principal, path)?;
    authorize_resource(
        &state.identity,
        &principal,
        ResourceType::GitRepository,
        id,
        PermissionLevel::Read,
        None,
        &headers,
    )
    .await?;
    let caps = capabilities(
        &state,
        &principal,
        ResourceType::GitRepository,
        Some(id),
        &headers,
    )
    .await?;
    let repository = load_git_repository(&state, id, &headers).await?;
    Ok(no_store(
        Json(AuthorizedGitRepositoryView {
            repository: repository.into(),
            capabilities: caps,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/gitRepositories/{id}/_cfg",
    operation_id = "getGitRepositoryConfig",
    tag = "GitRepositories",
    summary = "Get Git repository configuration",
    responses(
        (status = 200, description = "Success", body = GitRepositoryConfigResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_git_repository_config(
    State(state): State<GitCatalogHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let (principal, id) = principal_and_id(&headers, principal, path)?;
    authorize_resource(
        &state.identity,
        &principal,
        ResourceType::GitRepository,
        id,
        PermissionLevel::Read,
        None,
        &headers,
    )
    .await?;
    let repository = load_git_repository(&state, id, &headers).await?;
    Ok(no_store(
        Json(GitRepositoryConfigResponse {
            id: repository.id,
            name: repository.name,
            description: repository.description,
            url: repository.url,
            default_branch: repository.default_branch,
            git_account_id: repository.git_account_id,
            sync_mode: repository.sync_mode.into(),
            sync_interval_minutes: repository.sync_interval_minutes,
            webhook: repository.webhook,
            on_clone: repository.on_clone.map(|v| v.into()),
            on_pull: repository.on_pull.map(|v| v.into()),
            tags: repository
                .tags
                .into_iter()
                .map(|v| TagSummary {
                    id: v.id,
                    name: v.name,
                    color: v.color,
                })
                .collect(),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/gitRepositories",
    operation_id = "createGitRepository",
    tag = "GitRepositories",
    summary = "Create a Git repository",
    request_body = NewGitRepository,
    responses(
        (status = 200, description = "Success", body = GitRepositoryView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create_git_repository(
    State(state): State<GitCatalogHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<NewGitRepository>, JsonRejection>,
) -> IdentityHttpResult {
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let principal = identity_result(require_actor(principal), &headers)?;
    authorize_global(
        &state.identity,
        &principal,
        ResourceType::GitRepository,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    let input: citadel_git::CreateGitRepository = input.into();
    let service = citadel_git::GitRepositoryService::new(state.git_repositories.clone());
    let repository = identity_result(
        service
            .create(principal.actor_id, input)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state.realtime, "GitRepository", "gitRepositoryChanged");
    Ok(no_store(
        Json(GitRepositoryView::from(repository)).into_response(),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/gitRepositories/{id}",
    operation_id = "updateGitRepository",
    tag = "GitRepositories",
    summary = "Update a Git repository",
    request_body(content(
        (GitRepositoryPatch = "application/merge-patch+json"),
        (GitRepositoryPatch = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = GitRepositoryView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_git_repository(
    State(state): State<GitCatalogHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<GitRepositoryPatch>, JsonRejection>,
) -> IdentityHttpResult {
    let Json(mut input) = identity_result(input.map_err(invalid_json), &headers)?;
    input.name = None;
    input.description = MetadataPatch::Missing;
    input.tag_ids = None;
    mutate_git_repository(
        state,
        principal,
        path,
        headers,
        input,
        citadel_git::GitRepositoryMutationKind::Update,
    )
    .await
}

async fn mutate_git_repository(
    state: GitCatalogHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: GitRepositoryPatch,
    kind: citadel_git::GitRepositoryMutationKind,
) -> IdentityHttpResult {
    let input: citadel_git::GitRepositoryPatch = input.into();
    let (principal, id) = principal_and_id(&headers, principal, path)?;
    authorize_resource(
        &state.identity,
        &principal,
        ResourceType::GitRepository,
        id,
        PermissionLevel::Write,
        None,
        &headers,
    )
    .await?;
    let repository = identity_result(
        state
            .git_repositories
            .update_git_repository(principal.actor_id, id, &input, kind)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state.realtime, "GitRepository", "gitRepositoryChanged");
    Ok(no_store(
        Json(GitRepositoryView::from(repository)).into_response(),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/gitRepositories/{id}/_metadata",
    operation_id = "updateGitRepositoryMetadata",
    tag = "GitRepositories",
    summary = "Update Git repository metadata",
    request_body(content(
        (PatchResourceMetadataInput = "application/merge-patch+json"),
        (PatchResourceMetadataInput = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = GitRepositoryView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_git_repository_metadata(
    State(state): State<GitCatalogHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<PatchResourceMetadataInput>, JsonRejection>,
) -> IdentityHttpResult {
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    mutate_git_repository(
        state,
        principal,
        path,
        headers,
        GitRepositoryPatch {
            description: input.description,
            ..GitRepositoryPatch::default()
        },
        citadel_git::GitRepositoryMutationKind::Metadata,
    )
    .await
}

#[utoipa::path(
    post,
    path = "/api/v1/gitRepositories/rename",
    operation_id = "renameGitRepository",
    tag = "GitRepositories",
    summary = "Rename a Git repository",
    request_body = RenameResourceInput,
    responses(
        (status = 200, description = "Success", body = GitRepositoryView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn rename_git_repository(
    State(state): State<GitCatalogHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<RenameResourceInput>, JsonRejection>,
) -> IdentityHttpResult {
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    mutate_git_repository(
        state,
        principal,
        Ok(Path(input.id)),
        headers,
        GitRepositoryPatch {
            name: Some(input.name),
            ..GitRepositoryPatch::default()
        },
        citadel_git::GitRepositoryMutationKind::Rename,
    )
    .await
}

#[utoipa::path(
    delete,
    path = "/api/v1/gitRepositories",
    operation_id = "deleteGitRepositories",
    tag = "GitRepositories",
    summary = "Delete Git repositories",
    request_body = DeleteResourcesInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn delete_git_repositories(
    State(state): State<GitCatalogHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<DeleteResourcesInput>, JsonRejection>,
) -> IdentityHttpResult {
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let principal = identity_result(require_actor(principal), &headers)?;
    for id in &input.ids {
        authorize_resource(
            &state.identity,
            &principal,
            ResourceType::GitRepository,
            *id,
            PermissionLevel::Execute,
            None,
            &headers,
        )
        .await?;
    }
    identity_result(
        state
            .git_repositories
            .delete_git_repositories(principal.actor_id, &input.ids)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state.realtime, "GitRepository", "gitRepositoryChanged");
    Ok(no_store(StatusCode::NO_CONTENT.into_response()))
}

async fn load_git_repository(
    state: &GitCatalogHttpState,
    id: Uuid,
    headers: &HeaderMap,
) -> IdentityHttpResult<citadel_git::GitRepository> {
    identity_result(
        state
            .git_repositories
            .get_git_repository(id)
            .await
            .map_err(metadata_error),
        headers,
    )
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<GitCatalogHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(list_git_repositories))
        .normalized_routes(utoipa_axum::routes!(create_git_repository))
        .normalized_routes(utoipa_axum::routes!(delete_git_repositories))
        .normalized_routes(utoipa_axum::routes!(get_git_repository))
        .normalized_routes(utoipa_axum::routes!(get_git_repository_config))
        .normalized_routes(utoipa_axum::routes!(update_git_repository))
        .normalized_routes(utoipa_axum::routes!(update_git_repository_metadata))
        .normalized_routes(utoipa_axum::routes!(rename_git_repository))
}

async fn capabilities(
    state: &GitCatalogHttpState,
    principal: &ActorPrincipal,
    _: ResourceType,
    id: Option<Uuid>,
    headers: &HeaderMap,
) -> IdentityHttpResult<ResourceCapabilitiesView> {
    use citadel_git::permissions::*;
    use citadel_primitives::{EffectivePermission, PermissionPolicy, SpecificPermissions};
    let permission = if principal.is_administrator() {
        EffectivePermission::Administrator
    } else {
        let grant = match id {
            Some(id) => {
                state
                    .identity
                    .permission_for_resource(principal, ResourceType::GitRepository, id)
                    .await
            }
            None => {
                state
                    .identity
                    .global_permission(principal, ResourceType::GitRepository)
                    .await
            }
        };
        EffectivePermission::Granted {
            level: identity_result(grant, headers)?
                .map_or(PermissionLevel::None, |grant| grant.level),
            specifics: SpecificPermissions::EMPTY,
        }
    };
    Ok(ResourceCapabilitiesView {
        can_read: permission.allows(ReadGitRepository::REQUIREMENT),
        can_write: permission.allows(WriteGitRepository::REQUIREMENT),
        can_execute: permission.allows(ExecuteGitRepository::REQUIREMENT),
    })
}

#[derive(Clone)]
pub struct GitCatalogHttpState {
    pub git_repositories: std::sync::Arc<dyn citadel_git::GitRepositoryPersistence>,
    pub identity: std::sync::Arc<citadel_identity::IdentityService>,
    pub realtime: Option<crate::realtime::RealtimeHub>,
}
pub fn router(state: GitCatalogHttpState) -> axum::Router {
    documented_routes().split_for_parts().0.with_state(state)
}
pub(crate) fn metadata_error(
    error: citadel_git::GitRepositoryError,
) -> citadel_identity::IdentityError {
    match error {
        citadel_git::GitRepositoryError::Validation(message) => {
            crate::request_validation::validation_error(message)
        }
        citadel_git::GitRepositoryError::NotFound => citadel_identity::IdentityError::NotFound,
        citadel_git::GitRepositoryError::Conflict(message) => {
            citadel_identity::IdentityError::Conflict(message)
        }
        citadel_git::GitRepositoryError::Credential => citadel_identity::IdentityError::Credential,
        citadel_git::GitRepositoryError::Storage(message) => {
            citadel_identity::IdentityError::Storage(message)
        }
    }
}
