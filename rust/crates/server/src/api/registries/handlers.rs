mod registry_examples;

use axum::extract::rejection::{JsonRejection, PathRejection};

use axum::extract::{Extension, Path, RawQuery, State};

use axum::http::{HeaderMap, StatusCode};

use axum::response::IntoResponse;

use axum::{Json, Router};

use citadel_primitives::{PermissionLevel, ResourceType};

use citadel_identity::ActorPrincipal;

use crate::platforms_http::views::ResourceCapabilitiesView;

use citadel_registries::MetadataPatch;
use citadel_registries::RegistryMutationKind;

use crate::api::registries::dto::{NewRegistry, RegistryPatch, RegistryView};

use serde::Serialize;

use serde_json::Value;

use uuid::Uuid;

use super::{
    RegistriesHttpState, authorize_global, authorize_resource, capabilities, invalid_json,
    metadata_error, publish_resource_change, require_actor,
};

use crate::identity_http::{IdentityHttpError, IdentityHttpResult, identity_result, no_store};

use crate::openapi::router::OpenApiRouterExt;

pub fn router(state: RegistriesHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct RegistriesResponse {
    registries: Vec<AuthorizedRegistryView>,
    capabilities: ResourceCapabilitiesView,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct AuthorizedRegistryView {
    #[serde(flatten)]
    registry: RegistryView,
    capabilities: ResourceCapabilitiesView,
    is_default: bool,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct RegistryConfigResponse {
    id: Uuid,
    name: String,
    registry_host: String,
    status: crate::api::registries::dto::RegistryStatus,
    description: String,
    configuration: Value,
    tags: Vec<crate::api::tags::dto::TagSummary>,
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
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let filters = identity_result(parse_catalog_filters(raw_query.as_deref(), true), &headers)?;
    let global_capabilities = capabilities(
        &state.identity,
        &principal,
        ResourceType::Registry,
        None,
        &headers,
    )
    .await?;
    let authorized_registries = identity_result(
        state
            .registries
            .list_registries(principal.actor_id, principal.is_administrator())
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    let mut registries = Vec::with_capacity(authorized_registries.len());
    for registry in authorized_registries.into_iter().filter(|registry| {
        (filters.include_disabled
            || registry.status != citadel_registries::RegistryStatus::Disabled)
            && has_all_tags(&registry.tags, &filters.tags)
    }) {
        let row_capabilities = capabilities(
            &state.identity,
            &principal,
            ResourceType::Registry,
            Some(registry.id),
            &headers,
        )
        .await?;
        registries.push(AuthorizedRegistryView {
            is_default: registry.id == Uuid::from_u128(0x100),
            registry: registry.into(),
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
) -> IdentityHttpResult {
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
) -> IdentityHttpResult {
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
            configuration: value.configuration,
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
            ("Azure" = (value = json!(registry_examples::azure()))),
            ("AWS" = (value = json!(registry_examples::aws()))),
            ("Gitlab" = (value = json!(registry_examples::gitlab()))),
            ("DockerHub" = (value = json!(registry_examples::dockerhub()))),
            ("GitHub" = (value = json!(registry_examples::github()))),
            ("Custom" = (value = json!(registry_examples::custom())))
    )),
    responses(
        (status = 200, description = "Success", body = crate::api::registries::dto::RegistryView, content_type = "application/json"),
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
) -> IdentityHttpResult {
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let mut input: citadel_registries::NewRegistry = input.into();
    let principal = identity_result(require_actor(principal), &headers)?;
    authorize_global(
        &state.identity,
        &principal,
        ResourceType::Registry,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    identity_result(input.validate().map_err(metadata_error), &headers)?;
    let registry = identity_result(
        state
            .registries
            .create_registry(principal.actor_id, &input)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state.realtime, "Registry", "registryChanged");
    Ok(no_store(
        Json(crate::api::registries::dto::RegistryView::from(registry)).into_response(),
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
            ("Azure" = (value = json!(registry_examples::azure()))),
            ("AWS" = (value = json!(registry_examples::aws()))),
            ("Gitlab" = (value = json!(registry_examples::gitlab()))),
            ("DockerHub" = (value = json!(registry_examples::dockerhub()))),
            ("GitHub" = (value = json!(registry_examples::github()))),
            ("Custom" = (value = json!(registry_examples::custom())))
        )),
        (RegistryPatch = "application/merge-patch+json", examples(
            ("Azure" = (value = json!(registry_examples::azure()))),
            ("AWS" = (value = json!(registry_examples::aws()))),
            ("Gitlab" = (value = json!(registry_examples::gitlab()))),
            ("DockerHub" = (value = json!(registry_examples::dockerhub()))),
            ("GitHub" = (value = json!(registry_examples::github()))),
            ("Custom" = (value = json!(registry_examples::custom())))
        ))
    )),

    responses(
        (status = 200, description = "Success", body = crate::api::registries::dto::RegistryView, content_type = "application/json"),
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
) -> IdentityHttpResult {
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let mut input: citadel_registries::RegistryPatch = input.into();
    input.name = None;
    input.description = MetadataPatch::Missing;
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
) -> IdentityHttpResult {
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
    let registry = identity_result(
        state
            .registries
            .update_registry(principal.actor_id, id, &input, kind)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state.realtime, "Registry", "registryChanged");
    Ok(no_store(
        Json(crate::api::registries::dto::RegistryView::from(registry)).into_response(),
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
        (status = 200, description = "Success", body = crate::api::registries::dto::RegistryView, content_type = "application/json"),
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
) -> IdentityHttpResult {
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    mutate_registry(
        state,
        principal,
        path,
        headers,
        citadel_registries::RegistryPatch {
            description: input.description.into(),
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
        (status = 200, description = "Success", body = crate::api::registries::dto::RegistryView, content_type = "application/json"),
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
) -> IdentityHttpResult {
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
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
) -> IdentityHttpResult {
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let principal = identity_result(require_actor(principal), &headers)?;
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
        return Err(IdentityHttpError::from_parts(
            citadel_identity::IdentityError::Conflict(
                "The default Registry cannot be deleted.".to_owned(),
            ),
            &headers,
        ));
    }
    identity_result(
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
) -> IdentityHttpResult<RegistryView> {
    identity_result(
        state
            .registries
            .get_registry(id)
            .await
            .map(RegistryView::from)
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

fn has_all_tags(tags: &[citadel_tags::TagSummary], required: &[Uuid]) -> bool {
    required
        .iter()
        .all(|required| tags.iter().any(|tag| tag.id == *required))
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
use crate::api::catalog_query::{
    DeleteResourcesInput, PatchResourceMetadataInput, RenameResourceInput, parse_catalog_filters,
    principal_and_id,
};
