use axum::extract::rejection::{JsonRejection, PathRejection};
use axum::extract::{Extension, Path, RawQuery, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::{Json, Router};
use citadel_contracts::http::routes;
use citadel_domain::{PermissionLevel, ResourceType};
use citadel_identity::ActorPrincipal;
use citadel_platforms::ResourceCapabilitiesView;
use citadel_resources::{
    CatalogMutationKind, GitRepositoryPatch, GitRepositoryView, MetadataPatch, NewGitRepository,
    NewRegistry, RegistryPatch, RegistryView,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::{
    ResourcesHttpState, authorize_global, authorize_resource, capabilities, invalid_json,
    metadata_error, publish_resource_change, require_actor,
};
use crate::contract_router::ContractRouterExt;
use crate::identity_http::{IdentityHttpError, IdentityHttpResult, identity_result, no_store};

pub(super) fn router(state: ResourcesHttpState) -> Router {
    Router::new()
        .contract_route(routes::LIST_REGISTRIES, list_registries)
        .contract_route(routes::CREATE_REGISTRY, create_registry)
        .contract_route(routes::DELETE_REGISTRIES, delete_registries)
        .contract_route(routes::GET_REGISTRY, get_registry)
        .contract_route(routes::GET_REGISTRY_CONFIG, get_registry_config)
        .contract_route(routes::UPDATE_REGISTRY, update_registry)
        .contract_route(routes::UPDATE_REGISTRY_METADATA, update_registry_metadata)
        .contract_route(routes::RENAME_REGISTRY, rename_registry)
        .contract_route(routes::LIST_GIT_REPOSITORIES, list_git_repositories)
        .contract_route(routes::CREATE_GIT_REPOSITORY, create_git_repository)
        .contract_route(routes::DELETE_GIT_REPOSITORIES, delete_git_repositories)
        .contract_route(routes::GET_GIT_REPOSITORY, get_git_repository)
        .contract_route(routes::GET_GIT_REPOSITORY_CONFIG, get_git_repository_config)
        .contract_route(routes::UPDATE_GIT_REPOSITORY, update_git_repository)
        .contract_route(
            routes::UPDATE_GIT_REPOSITORY_METADATA,
            update_git_repository_metadata,
        )
        .contract_route(routes::RENAME_GIT_REPOSITORY, rename_git_repository)
        .with_state(state)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RegistriesResponse {
    registries: Vec<AuthorizedRegistryView>,
    capabilities: ResourceCapabilitiesView,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthorizedRegistryView {
    #[serde(flatten)]
    registry: RegistryView,
    capabilities: ResourceCapabilitiesView,
    is_default: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RegistryConfigResponse {
    id: Uuid,
    name: String,
    registry_host: String,
    status: citadel_resources::RegistryStatus,
    description: String,
    configuration: Value,
    tags: Vec<citadel_resources::TagSummary>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GitRepositoriesResponse {
    git_repositories: Vec<AuthorizedGitRepositoryView>,
    capabilities: ResourceCapabilitiesView,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthorizedGitRepositoryView {
    #[serde(flatten)]
    repository: GitRepositoryView,
    capabilities: ResourceCapabilitiesView,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GitRepositoryConfigResponse {
    id: Uuid,
    name: String,
    description: Option<String>,
    url: String,
    default_branch: String,
    git_account_id: Option<Uuid>,
    sync_mode: citadel_resources::GitRepositorySyncMode,
    sync_interval_minutes: Option<i32>,
    webhook: Option<Value>,
    on_clone: Option<citadel_resources::RepoCommand>,
    on_pull: Option<citadel_resources::RepoCommand>,
    tags: Vec<citadel_resources::TagSummary>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteResourcesInput {
    ids: Vec<Uuid>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenameResourceInput {
    id: Uuid,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PatchResourceMetadataInput {
    #[serde(default)]
    description: MetadataPatch<String>,
    #[serde(default, rename = "tags")]
    _tags: Option<Vec<String>>,
}

#[derive(Default)]
struct CatalogFilters {
    include_disabled: bool,
    tags: Vec<Uuid>,
}

async fn list_registries(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    RawQuery(raw_query): RawQuery,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let filters = identity_result(parse_catalog_filters(raw_query.as_deref(), true), &headers)?;
    let global_capabilities =
        capabilities(&state, &principal, ResourceType::Registry, None, &headers).await?;
    let authorized_registries = identity_result(
        state
            .resources
            .store()
            .list_registries(principal.actor_id, principal.is_administrator())
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    let mut registries = Vec::with_capacity(authorized_registries.len());
    for registry in authorized_registries.into_iter().filter(|registry| {
        (filters.include_disabled || registry.status != citadel_resources::RegistryStatus::Disabled)
            && has_all_tags(&registry.tags, &filters.tags)
    }) {
        let row_capabilities = capabilities(
            &state,
            &principal,
            ResourceType::Registry,
            Some(registry.id),
            &headers,
        )
        .await?;
        registries.push(AuthorizedRegistryView {
            is_default: registry.id == Uuid::from_u128(0x100),
            registry,
            capabilities: row_capabilities,
        });
    }
    Ok(no_store(
        Json(RegistriesResponse {
            registries,
            capabilities: global_capabilities,
        })
        .into_response(),
    ))
}

async fn get_registry(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let (principal, id) = principal_and_id(&headers, principal, path)?;
    authorize_resource(
        &state,
        &principal,
        ResourceType::Registry,
        id,
        PermissionLevel::Read,
        None,
        &headers,
    )
    .await?;
    let caps = capabilities(
        &state,
        &principal,
        ResourceType::Registry,
        Some(id),
        &headers,
    )
    .await?;
    let registry = load_registry(&state, id, &headers).await?;
    Ok(no_store(
        Json(AuthorizedRegistryView {
            is_default: id == Uuid::from_u128(0x100),
            registry,
            capabilities: caps,
        })
        .into_response(),
    ))
}

async fn get_registry_config(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let (principal, id) = principal_and_id(&headers, principal, path)?;
    authorize_resource(
        &state,
        &principal,
        ResourceType::Registry,
        id,
        PermissionLevel::Read,
        None,
        &headers,
    )
    .await?;
    let value = load_registry(&state, id, &headers).await?;
    Ok(no_store(
        Json(RegistryConfigResponse {
            id: value.id,
            name: value.name,
            registry_host: value.registry_host,
            status: value.status,
            description: value.description.unwrap_or_default(),
            configuration: value.configuration,
            tags: value.tags,
        })
        .into_response(),
    ))
}

async fn create_registry(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<NewRegistry>, JsonRejection>,
) -> IdentityHttpResult {
    let Json(mut input) = identity_result(input.map_err(invalid_json), &headers)?;
    let principal = identity_result(require_actor(principal), &headers)?;
    authorize_global(
        &state,
        &principal,
        ResourceType::Registry,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    identity_result(input.validate().map_err(metadata_error), &headers)?;
    let registry = identity_result(
        state
            .resources
            .store()
            .create_registry(principal.actor_id, &input)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state, "Registry", "registryChanged");
    Ok(no_store(Json(registry).into_response()))
}

async fn update_registry(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<RegistryPatch>, JsonRejection>,
) -> IdentityHttpResult {
    let Json(mut input) = identity_result(input.map_err(invalid_json), &headers)?;
    input.name = None;
    input.description = MetadataPatch::Missing;
    input.tag_ids = None;
    mutate_registry(
        state,
        principal,
        path,
        headers,
        input,
        CatalogMutationKind::Update,
    )
    .await
}

async fn mutate_registry(
    state: ResourcesHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: RegistryPatch,
    kind: CatalogMutationKind,
) -> IdentityHttpResult {
    let (principal, id) = principal_and_id(&headers, principal, path)?;
    authorize_resource(
        &state,
        &principal,
        ResourceType::Registry,
        id,
        PermissionLevel::Write,
        None,
        &headers,
    )
    .await?;
    prevent_default_registry(id, &headers)?;
    let registry = identity_result(
        state
            .resources
            .store()
            .update_registry(principal.actor_id, id, &input, kind)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state, "Registry", "registryChanged");
    Ok(no_store(Json(registry).into_response()))
}

async fn update_registry_metadata(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<PatchResourceMetadataInput>, JsonRejection>,
) -> IdentityHttpResult {
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    mutate_registry(
        state,
        principal,
        path,
        headers,
        RegistryPatch {
            description: input.description,
            ..RegistryPatch::default()
        },
        CatalogMutationKind::Metadata,
    )
    .await
}

async fn rename_registry(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<RenameResourceInput>, JsonRejection>,
) -> IdentityHttpResult {
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    mutate_registry(
        state,
        principal,
        Ok(Path(input.id)),
        headers,
        RegistryPatch {
            name: Some(input.name),
            ..RegistryPatch::default()
        },
        CatalogMutationKind::Rename,
    )
    .await
}

async fn delete_registries(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<DeleteResourcesInput>, JsonRejection>,
) -> IdentityHttpResult {
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let principal = identity_result(require_actor(principal), &headers)?;
    for id in &input.ids {
        authorize_resource(
            &state,
            &principal,
            ResourceType::Registry,
            *id,
            PermissionLevel::Execute,
            None,
            &headers,
        )
        .await?;
    }
    if input.ids.contains(&Uuid::from_u128(0x100)) {
        return Err(IdentityHttpError::from_parts(
            citadel_identity::IdentityError::Conflict(
                "The default Registry cannot be deleted.".to_owned(),
            ),
            &headers,
        ));
    }
    identity_result(
        state
            .resources
            .store()
            .delete_registries(principal.actor_id, &input.ids)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state, "Registry", "registryChanged");
    Ok(no_store(StatusCode::NO_CONTENT.into_response()))
}

async fn list_git_repositories(
    State(state): State<ResourcesHttpState>,
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
            .resources
            .store()
            .list_git_repositories(principal.actor_id, principal.is_administrator())
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    let mut git_repositories = Vec::with_capacity(authorized_repositories.len());
    for repository in authorized_repositories
        .into_iter()
        .filter(|repository| has_all_tags(&repository.tags, &filters.tags))
    {
        let row_capabilities = capabilities(
            &state,
            &principal,
            ResourceType::GitRepository,
            Some(repository.id),
            &headers,
        )
        .await?;
        git_repositories.push(AuthorizedGitRepositoryView {
            repository,
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

async fn get_git_repository(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let (principal, id) = principal_and_id(&headers, principal, path)?;
    authorize_resource(
        &state,
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
            repository,
            capabilities: caps,
        })
        .into_response(),
    ))
}

async fn get_git_repository_config(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let (principal, id) = principal_and_id(&headers, principal, path)?;
    authorize_resource(
        &state,
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
            sync_mode: repository.sync_mode,
            sync_interval_minutes: repository.sync_interval_minutes,
            webhook: repository.webhook,
            on_clone: repository.on_clone,
            on_pull: repository.on_pull,
            tags: repository.tags,
        })
        .into_response(),
    ))
}

async fn create_git_repository(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<NewGitRepository>, JsonRejection>,
) -> IdentityHttpResult {
    let Json(mut input) = identity_result(input.map_err(invalid_json), &headers)?;
    let principal = identity_result(require_actor(principal), &headers)?;
    authorize_global(
        &state,
        &principal,
        ResourceType::GitRepository,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    identity_result(input.validate().map_err(metadata_error), &headers)?;
    let repository = identity_result(
        state
            .resources
            .store()
            .create_git_repository(principal.actor_id, &input)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state, "GitRepository", "gitRepositoryChanged");
    Ok(no_store(Json(repository).into_response()))
}

async fn update_git_repository(
    State(state): State<ResourcesHttpState>,
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
        CatalogMutationKind::Update,
    )
    .await
}

async fn mutate_git_repository(
    state: ResourcesHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: GitRepositoryPatch,
    kind: CatalogMutationKind,
) -> IdentityHttpResult {
    let (principal, id) = principal_and_id(&headers, principal, path)?;
    authorize_resource(
        &state,
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
            .resources
            .store()
            .update_git_repository(principal.actor_id, id, &input, kind)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state, "GitRepository", "gitRepositoryChanged");
    Ok(no_store(Json(repository).into_response()))
}

async fn update_git_repository_metadata(
    State(state): State<ResourcesHttpState>,
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
        CatalogMutationKind::Metadata,
    )
    .await
}

async fn rename_git_repository(
    State(state): State<ResourcesHttpState>,
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
        CatalogMutationKind::Rename,
    )
    .await
}

async fn delete_git_repositories(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<DeleteResourcesInput>, JsonRejection>,
) -> IdentityHttpResult {
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let principal = identity_result(require_actor(principal), &headers)?;
    for id in &input.ids {
        authorize_resource(
            &state,
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
            .resources
            .store()
            .delete_git_repositories(principal.actor_id, &input.ids)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state, "GitRepository", "gitRepositoryChanged");
    Ok(no_store(StatusCode::NO_CONTENT.into_response()))
}

fn principal_and_id(
    headers: &HeaderMap,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
) -> IdentityHttpResult<(ActorPrincipal, Uuid)> {
    let principal = identity_result(require_actor(principal), headers)?;
    let Path(id) = identity_result(
        path.map_err(|error| citadel_identity::IdentityError::Validation(error.to_string())),
        headers,
    )?;
    Ok((principal, id))
}

async fn load_registry(
    state: &ResourcesHttpState,
    id: Uuid,
    headers: &HeaderMap,
) -> IdentityHttpResult<RegistryView> {
    identity_result(
        state
            .resources
            .store()
            .get_registry(id)
            .await
            .map_err(metadata_error),
        headers,
    )
}

async fn load_git_repository(
    state: &ResourcesHttpState,
    id: Uuid,
    headers: &HeaderMap,
) -> IdentityHttpResult<GitRepositoryView> {
    identity_result(
        state
            .resources
            .store()
            .get_git_repository(id)
            .await
            .map_err(metadata_error),
        headers,
    )
}

fn prevent_default_registry(id: Uuid, headers: &HeaderMap) -> IdentityHttpResult<()> {
    if id == Uuid::from_u128(0x100) {
        return Err(IdentityHttpError::from_parts(
            citadel_identity::IdentityError::Conflict(
                "The default Registry cannot be changed.".to_owned(),
            ),
            headers,
        ));
    }
    Ok(())
}

fn parse_catalog_filters(
    query: Option<&str>,
    allow_include_disabled: bool,
) -> Result<CatalogFilters, citadel_identity::IdentityError> {
    let mut filters = CatalogFilters::default();
    for (key, value) in url::form_urlencoded::parse(query.unwrap_or_default().as_bytes()) {
        if key == "includeDisabled" && allow_include_disabled {
            filters.include_disabled = value.parse::<bool>().map_err(|_| {
                citadel_identity::IdentityError::Validation(
                    "includeDisabled must be true or false.".to_owned(),
                )
            })?;
        } else if key == "tags" {
            if filters.tags.len() >= 100 {
                return Err(citadel_identity::IdentityError::Validation(
                    "At most 100 Tags may be filtered at once.".to_owned(),
                ));
            }
            let tag = Uuid::parse_str(&value).map_err(|_| {
                citadel_identity::IdentityError::Validation("A Tag ID is invalid.".to_owned())
            })?;
            if !filters.tags.contains(&tag) {
                filters.tags.push(tag);
            }
        }
    }
    Ok(filters)
}

fn has_all_tags(tags: &[citadel_resources::TagSummary], required: &[Uuid]) -> bool {
    required
        .iter()
        .all(|required| tags.iter().any(|tag| tag.id == *required))
}
