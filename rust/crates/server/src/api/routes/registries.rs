use crate::{
    api::{
        catalog_query::{
            DeleteResourcesInput, PatchResourceMetadataInput, RenameResourceInput,
            parse_catalog_filters, principal_and_id,
        },
        error::{ApiError, HttpError, HttpResult, api_result, no_store},
        resource_access::{
            authorize_global, authorize_resource, capabilities, publish_resource_change,
            require_actor,
        },
        resources::registries::{
            requests::{NewRegistry, RegistryPatch},
            views::{
                AuthorizedRegistryView, RegistriesResponse, RegistryConfigResponse, RegistryView,
            },
        },
    },
    openapi::router::OpenApiRouterExt,
    realtime::RealtimeHub,
    request_validation::invalid_json,
};

use axum::{
    Json, Router,
    extract::{
        Extension, Path, RawQuery, State,
        rejection::{JsonRejection, PathRejection},
    },
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

use citadel_identity::{ActorPrincipal, IdentityService};

use citadel_primitives::{PermissionLevel, ResourceType};

use citadel_primitives::PatchField;
use citadel_registries::RegistryMutationKind;

use std::sync::Arc;

use uuid::Uuid;

#[derive(Clone)]
pub struct RegistriesHttpState {
    pub identity: Arc<IdentityService>,
    pub registries: Arc<dyn citadel_registries::RegistryRepository>,
    pub registry_connections: Arc<dyn citadel_registries::RegistryConnectionChecker>,
    pub realtime: Option<RealtimeHub>,
}

pub(crate) fn metadata_error(error: citadel_registries::RegistryError) -> ApiError {
    match error {
        citadel_registries::RegistryError::Validation(message) => {
            crate::request_validation::validation_error(message)
        }
        citadel_registries::RegistryError::NotFound => ApiError::NotFound,
        citadel_registries::RegistryError::Conflict(message) => ApiError::Conflict(message),
        source @ citadel_registries::RegistryError::Storage(_) => ApiError::internal(source),
    }
}

use crate::api::resources::registries::examples;

pub fn router(state: RegistriesHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<RegistriesHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(list_registries))
        .normalized_routes(utoipa_axum::routes!(create_registry))
        .normalized_routes(utoipa_axum::routes!(delete_registries))
        .normalized_routes(utoipa_axum::routes!(get_registry))
        .normalized_routes(utoipa_axum::routes!(get_registry_config))
        .normalized_routes(utoipa_axum::routes!(update_registry))
        .normalized_routes(utoipa_axum::routes!(update_registry_metadata))
        .normalized_routes(utoipa_axum::routes!(rename_registry))
}

#[utoipa::path(
    get,
    path = "/api/v1/registries",
    operation_id = "listRegistries",
    tag = "Registries",
    summary = "List Registries",
    responses(
        (status = 200, description = "Success", body = RegistriesResponse, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("includeDisabled" = Option<bool>, Query, extensions(("x-citadel-default" = json!(false)))), ("tags" = Option<Vec<String>>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list_registries(
    State(state): State<RegistriesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    RawQuery(raw_query): RawQuery,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let filters = api_result(parse_catalog_filters(raw_query.as_deref(), true), &headers)?;
    let global_capabilities = capabilities(
        &state.identity,
        &principal,
        ResourceType::Registry,
        None,
        &headers,
    )
    .await?;
    let authorized_registries = api_result(
        state
            .registries
            .list_registries(principal.actor_id, principal.is_administrator())
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    // SQL has already selected the authorized IDs. Resolve their capability
    // metadata with one batch of ACL misses, including denied entries.
    let permission_ids = authorized_registries
        .iter()
        .map(|resource| resource.id)
        .collect::<Vec<_>>();
    let row_permissions = api_result(
        state
            .identity
            .permissions_for_resources(&principal, ResourceType::Registry, &permission_ids)
            .await,
        &headers,
    )?;
    let mut registries = Vec::with_capacity(authorized_registries.len());
    for registry in authorized_registries.into_iter().filter(|registry| {
        (filters.include_disabled
            || registry.status != citadel_registries::RegistryStatus::Disabled)
            && has_all_tags(&registry.tags, &filters.tags)
    }) {
        let row_capabilities = crate::api::resource_access::capabilities_from_permission(
            row_permissions.get(&registry.id).copied().flatten(),
        );
        registries.push(AuthorizedRegistryView {
            is_default: registry.id == Uuid::from_u128(0x100),
            registry: api_result(
                RegistryView::try_from(registry).map_err(ApiError::internal),
                &headers,
            )?,
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

#[utoipa::path(
    get,
    path = "/api/v1/registries/{id}",
    operation_id = "getRegistry",
    tag = "Registries",
    summary = "Get a Registry",
    responses(
        (status = 200, description = "Success", body = AuthorizedRegistryView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_registry(
    State(state): State<RegistriesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let (principal, id) = principal_and_id(&headers, principal, path)?;
    authorize_resource(
        &state.identity,
        &principal,
        ResourceType::Registry,
        id,
        PermissionLevel::Read,
        None,
        &headers,
    )
    .await?;
    let caps = capabilities(
        &state.identity,
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

#[utoipa::path(
    get,
    path = "/api/v1/registries/{id}/_cfg",
    operation_id = "getRegistryConfig",
    tag = "Registries",
    summary = "Get Registry configuration",
    responses(
        (status = 200, description = "Success", body = RegistryConfigResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_registry_config(
    State(state): State<RegistriesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let (principal, id) = principal_and_id(&headers, principal, path)?;
    authorize_resource(
        &state.identity,
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
            configuration: api_result(
                serde_json::from_value(value.configuration).map_err(ApiError::internal),
                &headers,
            )?,
            tags: value.tags,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/registries",
    operation_id = "createRegistry",
    tag = "Registries",
    summary = "Create a Registry",
    request_body(content = NewRegistry, examples(
            ("Azure" = (value = json!(examples::azure()))),
            ("AWS" = (value = json!(examples::aws()))),
            ("Gitlab" = (value = json!(examples::gitlab()))),
            ("DockerHub" = (value = json!(examples::dockerhub()))),
            ("GitHub" = (value = json!(examples::github()))),
            ("Custom" = (value = json!(examples::custom())))
    )),
    responses(
        (status = 200, description = "Success", body = crate::api::resources::registries::views::RegistryView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create_registry(
    State(state): State<RegistriesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<NewRegistry>, JsonRejection>,
) -> HttpResult {
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let mut input: citadel_registries::NewRegistry = input.into();
    let principal = api_result(require_actor(principal), &headers)?;
    authorize_global(
        &state.identity,
        &principal,
        ResourceType::Registry,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    let registry = api_result(
        citadel_registries::create_registry(
            state.registries.as_ref(),
            state.registry_connections.as_ref(),
            principal.actor_id,
            &mut input,
        )
        .await
        .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state.realtime, "Registry", "registryChanged");
    Ok(no_store(
        Json(api_result(
            RegistryView::try_from(registry).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/registries/{id}",
    operation_id = "updateRegistry",
    tag = "Registries",
    summary = "Update a Registry",
    request_body(content(
        (RegistryPatch = "application/json", examples(
            ("Azure" = (value = json!(examples::azure()))),
            ("AWS" = (value = json!(examples::aws()))),
            ("Gitlab" = (value = json!(examples::gitlab()))),
            ("DockerHub" = (value = json!(examples::dockerhub()))),
            ("GitHub" = (value = json!(examples::github()))),
            ("Custom" = (value = json!(examples::custom())))
        )),
        (RegistryPatch = "application/merge-patch+json", examples(
            ("Azure" = (value = json!(examples::azure()))),
            ("AWS" = (value = json!(examples::aws()))),
            ("Gitlab" = (value = json!(examples::gitlab()))),
            ("DockerHub" = (value = json!(examples::dockerhub()))),
            ("GitHub" = (value = json!(examples::github()))),
            ("Custom" = (value = json!(examples::custom())))
        ))
    )),

    responses(
        (status = 200, description = "Success", body = crate::api::resources::registries::views::RegistryView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_registry(
    State(state): State<RegistriesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<RegistryPatch>, JsonRejection>,
) -> HttpResult {
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let mut input: citadel_registries::RegistryPatch = input.into();
    input.name = None;
    input.description = PatchField::Missing;
    input.tag_ids = None;
    mutate_registry(
        state,
        principal,
        path,
        headers,
        input,
        RegistryMutationKind::Update,
    )
    .await
}

async fn mutate_registry(
    state: RegistriesHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: citadel_registries::RegistryPatch,
    kind: RegistryMutationKind,
) -> HttpResult {
    let (principal, id) = principal_and_id(&headers, principal, path)?;
    authorize_resource(
        &state.identity,
        &principal,
        ResourceType::Registry,
        id,
        PermissionLevel::Write,
        None,
        &headers,
    )
    .await?;
    prevent_default_registry(id, &headers)?;
    let registry = api_result(
        citadel_registries::update_registry(
            state.registries.as_ref(),
            state.registry_connections.as_ref(),
            principal.actor_id,
            id,
            &input,
            kind,
        )
        .await
        .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state.realtime, "Registry", "registryChanged");
    Ok(no_store(
        Json(api_result(
            RegistryView::try_from(registry).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/registries/{id}/_metadata",
    operation_id = "updateRegistryMetadata",
    tag = "Registries",
    summary = "Update Registry metadata",
    request_body(content(
        (PatchResourceMetadataInput = "application/merge-patch+json"),
        (PatchResourceMetadataInput = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = crate::api::resources::registries::views::RegistryView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_registry_metadata(
    State(state): State<RegistriesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<PatchResourceMetadataInput>, JsonRejection>,
) -> HttpResult {
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    mutate_registry(
        state,
        principal,
        path,
        headers,
        citadel_registries::RegistryPatch {
            description: input.description,
            ..citadel_registries::RegistryPatch::default()
        },
        RegistryMutationKind::Metadata,
    )
    .await
}

#[utoipa::path(
    post,
    path = "/api/v1/registries/rename",
    operation_id = "renameRegistry",
    tag = "Registries",
    summary = "Rename a Registry",
    request_body = RenameResourceInput,
    responses(
        (status = 200, description = "Success", body = crate::api::resources::registries::views::RegistryView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn rename_registry(
    State(state): State<RegistriesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<RenameResourceInput>, JsonRejection>,
) -> HttpResult {
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    mutate_registry(
        state,
        principal,
        Ok(Path(input.id)),
        headers,
        citadel_registries::RegistryPatch {
            name: Some(input.name),
            ..citadel_registries::RegistryPatch::default()
        },
        RegistryMutationKind::Rename,
    )
    .await
}

#[utoipa::path(
    delete,
    path = "/api/v1/registries",
    operation_id = "deleteRegistries",
    tag = "Registries",
    summary = "Delete Registries",
    request_body = DeleteResourcesInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn delete_registries(
    State(state): State<RegistriesHttpState>,
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
            ResourceType::Registry,
            *id,
            PermissionLevel::Execute,
            None,
            &headers,
        )
        .await?;
    }
    if input.ids.contains(&Uuid::from_u128(0x100)) {
        return Err(HttpError::from_parts(
            crate::api::error::ApiError::Conflict(
                "The default Registry cannot be deleted.".to_owned(),
            ),
            &headers,
        ));
    }
    api_result(
        state
            .registries
            .delete_registries(principal.actor_id, &input.ids)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state.realtime, "Registry", "registryChanged");
    Ok(no_store(StatusCode::NO_CONTENT.into_response()))
}

async fn load_registry(
    state: &RegistriesHttpState,
    id: Uuid,
    headers: &HeaderMap,
) -> HttpResult<RegistryView> {
    api_result(
        state
            .registries
            .get_registry(id)
            .await
            .map_err(metadata_error)
            .and_then(|value| RegistryView::try_from(value).map_err(ApiError::internal)),
        headers,
    )
}

fn prevent_default_registry(id: Uuid, headers: &HeaderMap) -> HttpResult<()> {
    if id == Uuid::from_u128(0x100) {
        return Err(HttpError::from_parts(
            crate::api::error::ApiError::Conflict(
                "The default Registry cannot be changed.".to_owned(),
            ),
            headers,
        ));
    }
    Ok(())
}

fn has_all_tags(tags: &[citadel_tags::TagSummary], required: &[Uuid]) -> bool {
    required
        .iter()
        .all(|required| tags.iter().any(|tag| tag.id == *required))
}
