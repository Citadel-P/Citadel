//! Git repositories HTTP routes, authorization and local request handling.
use crate::{
    api::{
        catalog_query::{
            DeleteResourcesInput, PatchResourceMetadataInput, RenameResourceInput,
            parse_catalog_filters, principal_and_id,
        },
        error::{ApiError, HttpResult, api_result, no_store},
        resource_access::{
            authorize_global, authorize_resource, publish_resource_change, require_actor,
        },
        resources::{
            capabilities::ResourceCapabilitiesView,
            git_repositories::{
                requests::{
                    BranchQuery, CompareQuery, FilesQuery, GitRepositoryPatch, NewGitRepository,
                },
                views::{
                    AuthorizedGitRepositoryView, GitRepositoriesResponse,
                    GitRepositoryBranchesResponse, GitRepositoryConfigResponse,
                    GitRepositoryRefsResponse, GitRepositoryView, *,
                },
            },
        },
    },
    openapi::router::OpenApiRouterExt,
    realtime::RealtimeHub,
    request_validation::{ApiQuery, invalid_json},
};
use citadel_primitives::PatchField;

use axum::{
    Json, Router,
    extract::{
        Extension, Path, RawQuery, State,
        rejection::{JsonRejection, PathRejection},
    },
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

use citadel_git::{GitRepositoryExecutionError, GitRepositoryExecutionService};

use citadel_identity::{ActorPrincipal, IdentityService};

use citadel_primitives::{PermissionLevel, ResourceType};

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use uuid::Uuid;

#[derive(Clone)]
pub struct GitRepositoriesHttpState {
    pub identity: Arc<IdentityService>,
    pub repository: Arc<dyn citadel_git::GitRepositoryPersistence>,
    pub execution: Arc<GitRepositoryExecutionService>,
    pub realtime: Option<RealtimeHub>,
    pub cancellation: CancellationToken,
}

pub fn router(state: GitRepositoriesHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<GitRepositoriesHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(refs))
        .normalized_routes(utoipa_axum::routes!(files))
        .normalized_routes(utoipa_axum::routes!(file_content))
        .normalized_routes(utoipa_axum::routes!(compare))
        .normalized_routes(utoipa_axum::routes!(branches))
        .normalized_routes(utoipa_axum::routes!(compose_projects))
        .normalized_routes(utoipa_axum::routes!(sync))
}

pub(crate) fn documented_catalog_routes() -> utoipa_axum::router::OpenApiRouter<GitCatalogHttpState>
{
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

#[utoipa::path(
    get,
    path = "/api/v1/gitRepositories/{id}/refs",
    operation_id = "getGitRepositoryRefs",
    tag = "GitRepositories",
    summary = "Get synchronized Git repository references",
    responses(
        (status = 200, description = "Success", body = GitRepositoryRefsResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn refs(
    State(state): State<GitRepositoriesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let (principal, id) = execution_principal_and_id(principal, path, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let refs = api_result(
        state.execution.list_refs(id).await.map_err(execution_error),
        &headers,
    )?;
    Ok(no_store(
        Json(GitRepositoryRefsResponse {
            refs: api_result(
                refs.into_iter()
                    .map(GitRepositoryRefView::try_from)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(ApiError::internal),
                &headers,
            )?,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/gitRepositories/{id}/files",
    operation_id = "listGitRepositoryDirectory",
    tag = "GitRepositories",
    summary = "List files in an immutable Git tree",
    responses(
        (status = 200, description = "Success", body = GitDirectoryListing, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = uuid::Uuid, Path), ("commitSha" = Option<String>, Query), ("path" = Option<String>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn files(
    State(state): State<GitRepositoriesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    ApiQuery(query): ApiQuery<FilesQuery>,
    headers: HeaderMap,
) -> HttpResult {
    let (principal, id) = execution_principal_and_id(principal, path, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let listing = api_result(
        state
            .execution
            .list_directory(
                id,
                query.commit_sha.as_deref(),
                query.path.as_deref().unwrap_or_default(),
                &state.cancellation.child_token(),
            )
            .await
            .map_err(execution_error),
        &headers,
    )?;
    Ok(no_store(
        Json(GitDirectoryListing::from(listing)).into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/gitRepositories/{id}/files/content",
    operation_id = "getGitRepositoryFileContent",
    tag = "GitRepositories",
    summary = "Read a bounded immutable Git file",
    responses(
        (status = 200, description = "Success", body = GitFileContent, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = uuid::Uuid, Path), ("commitSha" = Option<String>, Query), ("path" = String, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn file_content(
    State(state): State<GitRepositoriesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    ApiQuery(query): ApiQuery<FilesQuery>,
    headers: HeaderMap,
) -> HttpResult {
    let (principal, id) = execution_principal_and_id(principal, path, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let path = query.path.as_deref().ok_or_else(|| {
        crate::api::error::HttpError::from_parts(
            ApiError::Validation("Repository file path is required.".to_owned()),
            &headers,
        )
    })?;
    let file = api_result(
        state
            .execution
            .read_file(
                id,
                query.commit_sha.as_deref(),
                path,
                &state.cancellation.child_token(),
            )
            .await
            .map_err(execution_error),
        &headers,
    )?;
    Ok(no_store(Json(GitFileContent::from(file)).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/gitRepositories/{id}/compare",
    operation_id = "compareGitRepositoryCommits",
    tag = "GitRepositories",
    summary = "Compare immutable Git commits",
    responses(
        (status = 200, description = "Success", body = GitCommitComparison, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = uuid::Uuid, Path), ("baseCommitSha" = String, Query), ("headCommitSha" = String, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn compare(
    State(state): State<GitRepositoriesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    ApiQuery(query): ApiQuery<CompareQuery>,
    headers: HeaderMap,
) -> HttpResult {
    let (principal, id) = execution_principal_and_id(principal, path, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let comparison = api_result(
        state
            .execution
            .compare(
                id,
                &query.base_commit_sha,
                &query.head_commit_sha,
                &state.cancellation.child_token(),
            )
            .await
            .map_err(execution_error),
        &headers,
    )?;
    Ok(no_store(
        Json(GitCommitComparison::from(comparison)).into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/gitRepositories/{id}/branches",
    operation_id = "discoverGitRepositoryBranches",
    tag = "GitRepositories",
    summary = "Discover remote Git branches",
    responses(
        (status = 200, description = "Success", body = GitRepositoryBranchesResponse, content_type = "application/json"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn branches(
    State(state): State<GitRepositoriesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let (principal, id) = execution_principal_and_id(principal, path, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let branches = api_result(
        state
            .execution
            .discover_branches(id, &state.cancellation.child_token())
            .await
            .map_err(execution_error),
        &headers,
    )?;
    Ok(no_store(
        Json(GitRepositoryBranchesResponse {
            branches: branches.into_iter().map(Into::into).collect(),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/gitRepositories/{id}/compose-projects",
    operation_id = "discoverGitRepositoryComposeProjects",
    tag = "GitRepositories",
    summary = "Discover Compose projects in a Git repository",
    responses(
        (status = 200, description = "Success", body = GitComposeDiscovery, content_type = "application/json"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("id" = uuid::Uuid, Path), ("branch" = Option<String>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn compose_projects(
    State(state): State<GitRepositoriesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    ApiQuery(query): ApiQuery<BranchQuery>,
    headers: HeaderMap,
) -> HttpResult {
    let (principal, id) = execution_principal_and_id(principal, path, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let projects = api_result(
        state
            .execution
            .discover_compose_projects(
                id,
                query.branch.as_deref(),
                &state.cancellation.child_token(),
            )
            .await
            .map_err(execution_error),
        &headers,
    )?;
    Ok(no_store(
        Json(GitComposeDiscovery::from(projects)).into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/gitRepositories/{id}/sync",
    operation_id = "syncGitRepository",
    tag = "GitRepositories",
    summary = "Queue Git repository synchronization",
    responses(
        (status = 200, description = "Success", body = GitRepositoryView, content_type = "application/json"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    params(("id" = uuid::Uuid, Path), ("branch" = Option<String>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn sync(
    State(state): State<GitRepositoriesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    ApiQuery(query): ApiQuery<BranchQuery>,
    headers: HeaderMap,
) -> HttpResult {
    let (principal, id) = execution_principal_and_id(principal, path, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Execute, &headers).await?;
    api_result(
        state
            .execution
            .request_sync(principal.actor_id, id, query.branch.as_deref())
            .await
            .map_err(execution_error),
        &headers,
    )?;
    let repository: citadel_git::GitRepository = api_result(
        state
            .repository
            .get_git_repository(id)
            .await
            .map_err(ApiError::internal),
        &headers,
    )?;
    if let Some(realtime) = &state.realtime {
        realtime.publish_resource_change("GitRepository", id, "gitRepositoryChanged");
    }
    Ok(no_store(
        Json(api_result(
            GitRepositoryView::try_from(repository).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
    ))
}

fn execution_principal_and_id(
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: &HeaderMap,
) -> HttpResult<(ActorPrincipal, Uuid)> {
    let principal = api_result(
        principal
            .map(|Extension(value)| value)
            .ok_or(ApiError::Unauthenticated),
        headers,
    )?;
    let Path(id) = api_result(
        path.map_err(|error| ApiError::Validation(error.to_string())),
        headers,
    )?;
    Ok((principal, id))
}

async fn authorize(
    state: &GitRepositoriesHttpState,
    principal: &ActorPrincipal,
    id: Uuid,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> HttpResult<()> {
    api_result(
        match (ResourceType::GitRepository, level) {
            (ResourceType::GitRepository, PermissionLevel::Read) => {
                state
                    .identity
                    .require_resource::<citadel_git::permissions::ReadGitRepository>(principal, id)
                    .await
            }
            (ResourceType::GitRepository, PermissionLevel::Write) => {
                state
                    .identity
                    .require_resource::<citadel_git::permissions::WriteGitRepository>(principal, id)
                    .await
            }
            (ResourceType::GitRepository, PermissionLevel::Execute) => {
                state
                    .identity
                    .require_resource::<citadel_git::permissions::ExecuteGitRepository>(
                        principal, id,
                    )
                    .await
            }
            _ => {
                state
                    .identity
                    .authorize_resource(principal, ResourceType::GitRepository, id, level, None)
                    .await
            }
        },
        headers,
    )
}

fn execution_error(error: GitRepositoryExecutionError) -> ApiError {
    match error {
        GitRepositoryExecutionError::Validation(message) => {
            crate::request_validation::validation_error(message)
        }
        GitRepositoryExecutionError::NotFound => ApiError::NotFound,
        GitRepositoryExecutionError::NotSynchronized => {
            ApiError::Conflict("Git repository has not been synchronized yet.".to_owned())
        }
        GitRepositoryExecutionError::Conflict => {
            ApiError::Conflict("Git repository operation is already running.".to_owned())
        }
        GitRepositoryExecutionError::Git(_) => ApiError::External(
            "The Git repository operation failed. Verify the remote, revision, and credentials."
                .to_owned(),
        ),
        GitRepositoryExecutionError::Credential => ApiError::Credential,
        GitRepositoryExecutionError::Authentication => ApiError::Unauthenticated,
        source @ GitRepositoryExecutionError::Storage(_) => ApiError::internal(source),
    }
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
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let filters = api_result(parse_catalog_filters(raw_query.as_deref(), false), &headers)?;
    let global_capabilities = capabilities(
        &state,
        &principal,
        ResourceType::GitRepository,
        None,
        &headers,
    )
    .await?;
    let authorized_repositories = api_result(
        state
            .git_repositories
            .list_git_repositories(principal.actor_id, principal.is_administrator())
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    // SQL has already selected the authorized IDs. Resolve their capability
    // metadata with one batch of ACL misses, including denied entries.
    let permission_ids = authorized_repositories
        .iter()
        .map(|resource| resource.id)
        .collect::<Vec<_>>();
    let row_permissions = if principal.is_administrator() {
        std::collections::BTreeMap::new()
    } else {
        api_result(
            state
                .identity
                .permissions_for_resources(&principal, ResourceType::GitRepository, &permission_ids)
                .await,
            &headers,
        )?
    };
    let mut git_repositories = Vec::with_capacity(authorized_repositories.len());
    for repository in authorized_repositories.into_iter().filter(|repository| {
        filters
            .tags
            .iter()
            .all(|id| repository.tags.iter().any(|tag| tag.id == *id))
    }) {
        let row_capabilities = git_capabilities(
            &principal,
            row_permissions.get(&repository.id).copied().flatten(),
        );
        git_repositories.push(AuthorizedGitRepositoryView {
            repository: api_result(
                GitRepositoryView::try_from(repository).map_err(ApiError::internal),
                &headers,
            )?,
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
) -> HttpResult {
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
            repository: api_result(
                GitRepositoryView::try_from(repository).map_err(ApiError::internal),
                &headers,
            )?,
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
) -> HttpResult {
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
    let details = load_git_repository(&state, id, &headers).await?;
    let repository = details;
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
            tags: repository.tags.into_iter().map(Into::into).collect(),
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
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    authorize_global(
        &state.identity,
        &principal,
        ResourceType::GitRepository,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let input: citadel_git::CreateGitRepository = input.into();
    let service = citadel_git::GitRepositoryService::new(state.git_repositories.clone());
    let repository = api_result(
        service
            .create(principal.actor_id, input)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state.realtime, "GitRepository", "gitRepositoryChanged");
    Ok(no_store(
        Json(api_result(
            GitRepositoryView::try_from(repository).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
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
) -> HttpResult {
    let input = input.map_err(invalid_json).map(|Json(mut input)| {
        input.name = None;
        input.description = PatchField::Missing;
        input.tag_ids = None;
        input
    });
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
    input: Result<GitRepositoryPatch, ApiError>,
    kind: citadel_git::GitRepositoryMutationKind,
) -> HttpResult {
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
    let input: citadel_git::GitRepositoryPatch = api_result(input, &headers)?.into();
    let repository = api_result(
        state
            .git_repositories
            .update_git_repository(principal.actor_id, id, &input, kind)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state.realtime, "GitRepository", "gitRepositoryChanged");
    Ok(no_store(
        Json(api_result(
            GitRepositoryView::try_from(repository).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
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
) -> HttpResult {
    mutate_git_repository(
        state,
        principal,
        path,
        headers,
        input
            .map_err(invalid_json)
            .map(|Json(input)| GitRepositoryPatch {
                description: input.description,
                ..GitRepositoryPatch::default()
            }),
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
) -> HttpResult {
    let principal = Some(Extension(api_result(require_actor(principal), &headers)?));
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    mutate_git_repository(
        state,
        principal,
        Ok(Path(input.id)),
        headers,
        Ok(GitRepositoryPatch {
            name: Some(input.name),
            ..GitRepositoryPatch::default()
        }),
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
) -> HttpResult {
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let principal = api_result(require_actor(principal), &headers)?;
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
    api_result(
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
) -> HttpResult<citadel_git::GitRepository> {
    api_result(
        state
            .git_repositories
            .get_git_repository(id)
            .await
            .map_err(metadata_error),
        headers,
    )
}

async fn capabilities(
    state: &GitCatalogHttpState,
    principal: &ActorPrincipal,
    _: ResourceType,
    id: Option<Uuid>,
    headers: &HeaderMap,
) -> HttpResult<ResourceCapabilitiesView> {
    let grant = if principal.is_administrator() {
        None
    } else {
        api_result(
            match id {
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
            },
            headers,
        )?
    };
    Ok(git_capabilities(principal, grant))
}

fn git_capabilities(
    principal: &ActorPrincipal,
    grant: Option<citadel_identity::PermissionGrant>,
) -> ResourceCapabilitiesView {
    use citadel_git::permissions::*;
    use citadel_primitives::{EffectivePermission, PermissionPolicy, SpecificPermissions};
    let permission = if principal.is_administrator() {
        EffectivePermission::Administrator
    } else {
        EffectivePermission::Granted {
            level: grant.map_or(PermissionLevel::None, |grant| grant.level),
            specifics: SpecificPermissions::EMPTY,
        }
    };
    ResourceCapabilitiesView {
        can_read: permission.allows(ReadGitRepository::REQUIREMENT),
        can_write: permission.allows(WriteGitRepository::REQUIREMENT),
        can_execute: permission.allows(ExecuteGitRepository::REQUIREMENT),
    }
}

#[derive(Clone)]
pub struct GitCatalogHttpState {
    pub git_repositories: std::sync::Arc<dyn citadel_git::GitRepositoryPersistence>,
    pub identity: std::sync::Arc<citadel_identity::IdentityService>,
    pub realtime: Option<crate::realtime::RealtimeHub>,
}

pub fn catalog_router(state: GitCatalogHttpState) -> axum::Router {
    documented_catalog_routes()
        .split_for_parts()
        .0
        .with_state(state)
}

pub(crate) fn metadata_error(
    error: citadel_git::GitRepositoryError,
) -> crate::api::error::ApiError {
    match error {
        citadel_git::GitRepositoryError::Validation(message) => {
            crate::request_validation::validation_error(message)
        }
        citadel_git::GitRepositoryError::NotFound => crate::api::error::ApiError::NotFound,
        citadel_git::GitRepositoryError::Conflict(message) => {
            crate::api::error::ApiError::Conflict(message)
        }
        citadel_git::GitRepositoryError::Credential => crate::api::error::ApiError::Credential,
        source @ citadel_git::GitRepositoryError::Storage(_) => {
            crate::api::error::ApiError::internal(source)
        }
    }
}
