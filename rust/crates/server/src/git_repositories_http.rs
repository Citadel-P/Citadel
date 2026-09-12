use crate::request_validation::ApiQuery;
use std::sync::Arc;

use axum::extract::rejection::PathRejection;
use axum::extract::{Extension, Path, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::{Json, Router};
use citadel_domain::{PermissionLevel, ResourceType};
use citadel_git::{
    GitRepositoryExecutionError, GitRepositoryExecutionService, GitRepositoryRefView, RemoteBranch,
};
use citadel_identity::{ActorPrincipal, IdentityError, IdentityService};
use citadel_resources::{GitRepositoryView, ResourceMetadataService};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::identity_http::{IdentityHttpResult, identity_result, no_store};
use crate::openapi::router::OpenApiRouterExt;
use crate::realtime::RealtimeHub;

#[derive(Clone)]
pub struct GitRepositoriesHttpState {
    pub identity: Arc<IdentityService>,
    pub resources: Arc<ResourceMetadataService>,
    pub execution: Arc<GitRepositoryExecutionService>,
    pub realtime: Option<RealtimeHub>,
    pub cancellation: CancellationToken,
}

pub fn router(state: GitRepositoriesHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct FilesQuery {
    commit_sha: Option<String>,
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompareQuery {
    base_commit_sha: String,
    head_commit_sha: String,
}

#[derive(Debug, Deserialize, Default)]
struct BranchQuery {
    branch: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct GitRepositoryRefsResponse {
    refs: Vec<GitRepositoryRefView>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct GitRepositoryBranchesResponse {
    branches: Vec<GitRepositoryBranchResponse>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct GitRepositoryBranchResponse {
    branch: String,
    commit_sha: String,
}

impl From<RemoteBranch> for GitRepositoryBranchResponse {
    fn from(value: RemoteBranch) -> Self {
        Self {
            branch: value.name,
            commit_sha: value.commit,
        }
    }
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
) -> IdentityHttpResult {
    let (principal, id) = principal_and_id(principal, path, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let refs = identity_result(
        state.execution.list_refs(id).await.map_err(execution_error),
        &headers,
    )?;
    Ok(no_store(
        Json(GitRepositoryRefsResponse { refs }).into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/gitRepositories/{id}/files",
    operation_id = "listGitRepositoryDirectory",
    tag = "GitRepositories",
    summary = "List files in an immutable Git tree",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/GitRepositoryDirectoryListingView"), content_type = "application/json"),
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
) -> IdentityHttpResult {
    let (principal, id) = principal_and_id(principal, path, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let listing = identity_result(
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
    Ok(no_store(Json(listing).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/gitRepositories/{id}/files/content",
    operation_id = "getGitRepositoryFileContent",
    tag = "GitRepositories",
    summary = "Read a bounded immutable Git file",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/GitRepositoryFileContentView"), content_type = "application/json"),
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
) -> IdentityHttpResult {
    let (principal, id) = principal_and_id(principal, path, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let path = query.path.as_deref().ok_or_else(|| {
        crate::identity_http::IdentityHttpError::from_parts(
            IdentityError::Validation("Repository file path is required.".to_owned()),
            &headers,
        )
    })?;
    let file = identity_result(
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
    Ok(no_store(Json(file).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/gitRepositories/{id}/compare",
    operation_id = "compareGitRepositoryCommits",
    tag = "GitRepositories",
    summary = "Compare immutable Git commits",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/GitCommitComparisonView"), content_type = "application/json"),
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
) -> IdentityHttpResult {
    let (principal, id) = principal_and_id(principal, path, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let comparison = identity_result(
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
    Ok(no_store(Json(comparison).into_response()))
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
) -> IdentityHttpResult {
    let (principal, id) = principal_and_id(principal, path, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let branches = identity_result(
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
        (status = 200, description = "Success", body = ref("#/components/schemas/GitRepositoryComposeDiscovery"), content_type = "application/json"),
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
) -> IdentityHttpResult {
    let (principal, id) = principal_and_id(principal, path, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let projects = identity_result(
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
    Ok(no_store(Json(projects).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/gitRepositories/{id}/sync",
    operation_id = "syncGitRepository",
    tag = "GitRepositories",
    summary = "Queue Git repository synchronization",
    responses(
        (status = 200, description = "Success", body = citadel_resources::GitRepositoryView, content_type = "application/json"),
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
) -> IdentityHttpResult {
    let (principal, id) = principal_and_id(principal, path, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Execute, &headers).await?;
    identity_result(
        state
            .execution
            .request_sync(principal.actor_id, id, query.branch.as_deref())
            .await
            .map_err(execution_error),
        &headers,
    )?;
    let repository: GitRepositoryView = identity_result(
        state
            .resources
            .store()
            .get_git_repository(id)
            .await
            .map_err(|error| IdentityError::Storage(error.to_string())),
        &headers,
    )?;
    if let Some(realtime) = &state.realtime {
        realtime.publish_resource_change("GitRepository", id, "gitRepositoryChanged");
    }
    Ok(no_store(Json(repository).into_response()))
}

fn principal_and_id(
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: &HeaderMap,
) -> IdentityHttpResult<(ActorPrincipal, Uuid)> {
    let principal = identity_result(
        principal
            .map(|Extension(value)| value)
            .ok_or(IdentityError::Unauthenticated),
        headers,
    )?;
    let Path(id) = identity_result(
        path.map_err(|error| IdentityError::Validation(error.to_string())),
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
) -> IdentityHttpResult<()> {
    identity_result(
        state
            .identity
            .authorize_resource(principal, ResourceType::GitRepository, id, level, None)
            .await,
        headers,
    )?;
    Ok(())
}

fn execution_error(error: GitRepositoryExecutionError) -> IdentityError {
    match error {
        GitRepositoryExecutionError::Validation(message) => {
            crate::request_validation::validation_error(message)
        }
        GitRepositoryExecutionError::NotFound => IdentityError::NotFound,
        GitRepositoryExecutionError::NotSynchronized => {
            IdentityError::Conflict("Git repository has not been synchronized yet.".to_owned())
        }
        GitRepositoryExecutionError::Conflict => {
            IdentityError::Conflict("Git repository operation is already running.".to_owned())
        }
        GitRepositoryExecutionError::Git(_) => IdentityError::External(
            "The Git repository operation failed. Verify the remote, revision, and credentials."
                .to_owned(),
        ),
        GitRepositoryExecutionError::Credential => IdentityError::Credential,
        GitRepositoryExecutionError::Authentication => IdentityError::Unauthenticated,
        GitRepositoryExecutionError::Storage(message) => IdentityError::Storage(message),
    }
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
