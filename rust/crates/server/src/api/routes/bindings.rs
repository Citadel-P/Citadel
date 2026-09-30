use crate::{
    api::{
        error::{ApiError, HttpResult, api_result, no_store},
        resource_access::{
            authorize_global, authorize_resource, capabilities, publish_resource_change,
            require_actor,
        },
        resources::bindings::{
            requests::{
                ExternalSecretInput, ExternalSecretPatch, InternalSecretInput, NewResourceBinding,
                ResourceBindingInput, SecretProviderInput, SecretProviderPatch, SecretScopeQuery,
            },
            spec::ResourceBindingScope,
            views::{
                GlobalBindingsResponse, ResourceBindingsView, SecretDefinitionsResponse,
                SecretProvidersResponse,
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
        Extension, Path, Query, State,
        rejection::{JsonRejection, PathRejection},
    },
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

use citadel_bindings::{BindingValidation, validate_binding};

use citadel_identity::{ActorPrincipal, IdentityService};

use citadel_primitives::{PermissionLevel, ResourceType, SpecificPermission};

use std::sync::Arc;

use uuid::Uuid;

#[derive(Clone)]
pub struct BindingsHttpState {
    pub identity: Arc<IdentityService>,
    pub secrets: Arc<citadel_bindings::SecretService>,
    pub realtime: Option<RealtimeHub>,
}

pub(crate) fn metadata_error(error: citadel_bindings::BindingError) -> ApiError {
    match error {
        citadel_bindings::BindingError::Validation(message) => {
            crate::request_validation::validation_error(message)
        }
        citadel_bindings::BindingError::NotFound => ApiError::NotFound,
        citadel_bindings::BindingError::Conflict(message) => ApiError::Conflict(message),
        citadel_bindings::BindingError::Credential => ApiError::Credential,
        source @ citadel_bindings::BindingError::Storage(_) => ApiError::internal(source),
    }
}

pub fn router(state: BindingsHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<BindingsHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(get_global_bindings))
        .normalized_routes(utoipa_axum::routes!(create_global_binding))
        .normalized_routes(utoipa_axum::routes!(update_global_binding))
        .normalized_routes(utoipa_axum::routes!(delete_global_binding))
        .normalized_routes(utoipa_axum::routes!(get_resource_bindings))
        .normalized_routes(utoipa_axum::routes!(create_resource_binding))
        .normalized_routes(utoipa_axum::routes!(update_resource_binding))
        .normalized_routes(utoipa_axum::routes!(delete_resource_binding))
        .normalized_routes(utoipa_axum::routes!(list_secrets))
        .normalized_routes(utoipa_axum::routes!(create_internal_secret))
        .normalized_routes(utoipa_axum::routes!(create_external_secret))
        .normalized_routes(utoipa_axum::routes!(update_external_secret))
        .normalized_routes(utoipa_axum::routes!(delete_secret))
        .normalized_routes(utoipa_axum::routes!(list_secret_providers))
        .normalized_routes(utoipa_axum::routes!(create_secret_provider))
        .normalized_routes(utoipa_axum::routes!(update_secret_provider))
        .normalized_routes(utoipa_axum::routes!(delete_secret_provider))
        .normalized_routes(utoipa_axum::routes!(test_secret_provider))
        .normalized_routes(utoipa_axum::routes!(test_external_secret))
}

#[utoipa::path(
    post,
    path = "/api/v1/resourceBindings/secret-providers/vault-kv2/test",
    operation_id = "testVaultKvV2SecretProviderConnection",
    tag = "ResourceBindings",
    summary = "Test a Vault-compatible KV v2 Secret provider connection",
    request_body = crate::api::resources::bindings::requests::TestSecretProviderInput,
    responses(
        (status = 200, description = "Success", body = crate::api::resources::schema_models::bindings::SecretTestResultSchema, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn test_secret_provider(
    State(state): State<BindingsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<
        Json<crate::api::resources::bindings::requests::TestSecretProviderInput>,
        JsonRejection,
    >,
) -> HttpResult {
    global_authorization(&state, principal, PermissionLevel::Write, &headers).await?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let result = api_result(
        state
            .secrets
            .test_secret_provider(input.into())
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    Ok(no_store(Json(result).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/resourceBindings/secrets/external/test",
    operation_id = "testExternalSecret",
    tag = "ResourceBindings",
    summary = "Test an external Secret reference",
    request_body = crate::api::resources::bindings::requests::TestExternalSecretInput,
    responses(
        (status = 200, description = "Success", body = crate::api::resources::schema_models::bindings::SecretTestResultSchema, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn test_external_secret(
    State(state): State<BindingsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<
        Json<crate::api::resources::bindings::requests::TestExternalSecretInput>,
        JsonRejection,
    >,
) -> HttpResult {
    global_authorization(&state, principal, PermissionLevel::Write, &headers).await?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let result = api_result(
        state
            .secrets
            .test_external_secret(input.into())
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    Ok(no_store(Json(result).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/resourceBindings/global",
    operation_id = "getGlobalResourceBindings",
    tag = "ResourceBindings",
    summary = "Get global resource bindings",
    responses(
        (status = 200, description = "Success", body = GlobalBindingsResponse, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_global_bindings(
    State(state): State<BindingsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> HttpResult {
    let principal =
        global_authorization(&state, principal, PermissionLevel::Read, &headers).await?;
    let bindings = load_bindings(&state, ResourceBindingScope::Global, None, &headers).await?;
    let caps = capabilities(
        &state.identity,
        &principal,
        ResourceType::Binding,
        None,
        &headers,
    )
    .await?;
    Ok(no_store(
        Json(GlobalBindingsResponse {
            bindings,
            capabilities: caps,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/resourceBindings/global",
    operation_id = "createGlobalResourceBinding",
    tag = "ResourceBindings",
    summary = "Create a global resource binding",
    request_body = NewResourceBinding,
    responses(
        (status = 200, description = "Success", body = crate::api::resources::bindings::views::ResourceBindingsView, content_type = "application/json"),
        crate::openapi::errors::CreateErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create_global_binding(
    State(state): State<BindingsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<NewResourceBinding>, JsonRejection>,
) -> HttpResult {
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let mut input: citadel_bindings::NewResourceBinding = input.into();
    global_authorization(&state, principal, PermissionLevel::Write, &headers).await?;
    input.scope = Some(citadel_bindings::ResourceBindingScope::Global);
    input.resource_id = None;
    validate_new_binding(&input, ResourceBindingScope::Global, None, &headers)?;
    let bindings = api_result(
        state
            .secrets
            .store()
            .create_binding(&input)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state.realtime, "Binding", "resourceBindingChanged");
    Ok(no_store(
        Json(crate::api::resources::bindings::views::ResourceBindingsView::from(bindings))
            .into_response(),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/resourceBindings/global",
    operation_id = "updateGlobalResourceBinding",
    tag = "ResourceBindings",
    summary = "Update a global resource binding",
    request_body = ResourceBindingInput,
    responses(
        (status = 200, description = "Success", body = crate::api::resources::bindings::views::ResourceBindingsView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_global_binding(
    State(state): State<BindingsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<ResourceBindingInput>, JsonRejection>,
) -> HttpResult {
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let input: citadel_bindings::ResourceBindingInput = input.into();
    global_authorization(&state, principal, PermissionLevel::Write, &headers).await?;
    validate_update_binding(&input, ResourceBindingScope::Global, None, &headers)?;
    let bindings = api_result(
        state
            .secrets
            .store()
            .update_binding(&input, citadel_bindings::ResourceBindingScope::Global, None)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state.realtime, "Binding", "resourceBindingChanged");
    Ok(no_store(
        Json(crate::api::resources::bindings::views::ResourceBindingsView::from(bindings))
            .into_response(),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/resourceBindings/global/{id}",
    operation_id = "deleteGlobalResourceBinding",
    tag = "ResourceBindings",
    summary = "Delete a global resource binding",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::bindings::views::ResourceBindingsView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn delete_global_binding(
    State(state): State<BindingsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let Path(id) = path_value(path, &headers)?;
    binding_resource_authorization(&state, principal, id, PermissionLevel::Write, &headers).await?;
    let bindings = api_result(
        state
            .secrets
            .store()
            .delete_binding(id, citadel_bindings::ResourceBindingScope::Global, None)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state.realtime, "Binding", "resourceBindingChanged");
    Ok(no_store(
        Json(crate::api::resources::bindings::views::ResourceBindingsView::from(bindings))
            .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/resourceBindings/{scope}/{resourceId}",
    operation_id = "getResourceBindings",
    tag = "ResourceBindings",
    summary = "Get resource bindings",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::bindings::views::ResourceBindingsView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("scope" = String, Path), ("resourceId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_resource_bindings(
    State(state): State<BindingsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(ResourceBindingScope, Uuid)>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let (principal, scope, id) =
        scoped_authorization(&state, principal, path, PermissionLevel::Read, &headers).await?;
    let _ = principal;
    let bindings = load_bindings(&state, scope, Some(id), &headers).await?;
    Ok(no_store(Json(bindings).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/resourceBindings/{scope}/{resourceId}",
    operation_id = "createResourceBinding",
    tag = "ResourceBindings",
    summary = "Create a resource binding",
    request_body = NewResourceBinding,
    responses(
        (status = 200, description = "Success", body = crate::api::resources::bindings::views::ResourceBindingsView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("scope" = String, Path), ("resourceId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create_resource_binding(
    State(state): State<BindingsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(ResourceBindingScope, Uuid)>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<NewResourceBinding>, JsonRejection>,
) -> HttpResult {
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let mut input: citadel_bindings::NewResourceBinding = input.into();
    let (_, scope, id) =
        scoped_authorization(&state, principal, path, PermissionLevel::Write, &headers).await?;
    input.scope = Some(scope.into());
    input.resource_id = Some(id);
    validate_new_binding(&input, scope, Some(id), &headers)?;
    let bindings = api_result(
        state
            .secrets
            .store()
            .create_binding(&input)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state.realtime, "Binding", "resourceBindingChanged");
    Ok(no_store(
        Json(crate::api::resources::bindings::views::ResourceBindingsView::from(bindings))
            .into_response(),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/resourceBindings/{scope}/{resourceId}",
    operation_id = "updateResourceBinding",
    tag = "ResourceBindings",
    summary = "Update a resource binding",
    request_body = ResourceBindingInput,
    responses(
        (status = 200, description = "Success", body = crate::api::resources::bindings::views::ResourceBindingsView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("scope" = String, Path), ("resourceId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_resource_binding(
    State(state): State<BindingsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(ResourceBindingScope, Uuid)>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<ResourceBindingInput>, JsonRejection>,
) -> HttpResult {
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let input: citadel_bindings::ResourceBindingInput = input.into();
    let (_, scope, id) =
        scoped_authorization(&state, principal, path, PermissionLevel::Write, &headers).await?;
    validate_update_binding(&input, scope, Some(id), &headers)?;
    let bindings = api_result(
        state
            .secrets
            .store()
            .update_binding(&input, scope.into(), Some(id))
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state.realtime, "Binding", "resourceBindingChanged");
    Ok(no_store(
        Json(crate::api::resources::bindings::views::ResourceBindingsView::from(bindings))
            .into_response(),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/resourceBindings/{scope}/{resourceId}/{id}",
    operation_id = "deleteResourceBinding",
    tag = "ResourceBindings",
    summary = "Delete a resource binding",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::bindings::views::ResourceBindingsView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("scope" = String, Path), ("resourceId" = uuid::Uuid, Path), ("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn delete_resource_binding(
    State(state): State<BindingsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(ResourceBindingScope, Uuid, Uuid)>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let Path((scope, resource_id, id)) = api_result(
        path.map_err(|error| crate::api::error::ApiError::Validation(error.to_string())),
        &headers,
    )?;
    let principal = api_result(require_actor(principal), &headers)?;
    authorize_scope(
        &state,
        &principal,
        scope,
        resource_id,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    let bindings = api_result(
        state
            .secrets
            .store()
            .delete_binding(id, scope.into(), Some(resource_id))
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state.realtime, "Binding", "resourceBindingChanged");
    Ok(no_store(
        Json(crate::api::resources::bindings::views::ResourceBindingsView::from(bindings))
            .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/resourceBindings/secrets",
    operation_id = "listSecretDefinitions",
    tag = "ResourceBindings",
    summary = "List Secret definitions",
    responses(
        (status = 200, description = "Success", body = SecretDefinitionsResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("scope" = Option<crate::api::resources::bindings::spec::ResourceBindingScope>, Query), ("resourceId" = Option<uuid::Uuid>, Query), ("targetResourceType" = Option<crate::api::resources::vocabulary::ResourceTypeSchema>, Query), ("targetResourceId" = Option<uuid::Uuid>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list_secrets(
    State(state): State<BindingsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    query: Query<SecretScopeQuery>,
    headers: HeaderMap,
) -> HttpResult {
    let Query(query) = query;
    let principal =
        secret_scope_authorization(&state, principal, &query, PermissionLevel::Read, &headers)
            .await?;
    let secrets = api_result(
        state
            .secrets
            .store()
            .list_secret_definitions(query.scope.map(Into::into), query.resource_id)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    let caps = capabilities(
        &state.identity,
        &principal,
        ResourceType::Binding,
        None,
        &headers,
    )
    .await?;
    Ok(no_store(
        Json(SecretDefinitionsResponse {
            secrets: secrets.into_iter().map(Into::into).collect(),
            capabilities: caps,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/resourceBindings/secrets",
    operation_id = "createInternalSecret",
    tag = "ResourceBindings",
    summary = "Create an internal encrypted Secret",
    request_body = InternalSecretInput,
    responses(
        (status = 200, description = "Success", body = crate::api::resources::bindings::views::SecretDefinitionView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("scope" = Option<String>, Query), ("resourceId" = Option<uuid::Uuid>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create_internal_secret(
    State(state): State<BindingsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    query: Query<SecretScopeQuery>,
    headers: HeaderMap,
    input: Result<Json<InternalSecretInput>, JsonRejection>,
) -> HttpResult {
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let Query(query) = query;
    secret_scope_authorization(&state, principal, &query, PermissionLevel::Write, &headers).await?;
    let secret = api_result(
        state
            .secrets
            .create_internal_secret(&input.name, &input.value)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state.realtime, "Binding", "secretDefinitionChanged");
    Ok(no_store(
        Json(crate::api::resources::bindings::views::SecretDefinitionView::from(secret))
            .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/resourceBindings/secrets/external",
    operation_id = "createExternalSecret",
    tag = "ResourceBindings",
    summary = "Create an external Secret definition",
    request_body = ExternalSecretInput,
    responses(
        (status = 200, description = "Success", body = crate::api::resources::bindings::views::SecretDefinitionView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create_external_secret(
    State(state): State<BindingsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<ExternalSecretInput>, JsonRejection>,
) -> HttpResult {
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let input: citadel_bindings::ExternalSecretInput = input.into();
    global_authorization(&state, principal, PermissionLevel::Write, &headers).await?;
    api_result(input.validate().map_err(metadata_error), &headers)?;
    let secret = api_result(
        state
            .secrets
            .store()
            .create_external_secret(&input)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state.realtime, "Binding", "secretDefinitionChanged");
    Ok(no_store(
        Json(crate::api::resources::bindings::views::SecretDefinitionView::from(secret))
            .into_response(),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/resourceBindings/secrets/external/{id}",
    operation_id = "updateExternalSecret",
    tag = "ResourceBindings",
    summary = "Update an external Secret definition",
    request_body(content(
        (ExternalSecretPatch = "application/merge-patch+json"),
        (ExternalSecretPatch = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = crate::api::resources::bindings::views::SecretDefinitionView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_external_secret(
    State(state): State<BindingsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<ExternalSecretPatch>, JsonRejection>,
) -> HttpResult {
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let input: citadel_bindings::ExternalSecretPatch = input.into();
    let Path(id) = path_value(path, &headers)?;
    binding_resource_authorization(&state, principal, id, PermissionLevel::Write, &headers).await?;
    let secret = api_result(
        state
            .secrets
            .store()
            .update_external_secret(id, &input)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state.realtime, "Binding", "secretDefinitionChanged");
    Ok(no_store(
        Json(crate::api::resources::bindings::views::SecretDefinitionView::from(secret))
            .into_response(),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/resourceBindings/secrets/{id}",
    operation_id = "deleteSecretDefinition",
    tag = "ResourceBindings",
    summary = "Delete an unused Secret definition",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn delete_secret(
    State(state): State<BindingsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let Path(id) = path_value(path, &headers)?;
    binding_resource_authorization(&state, principal, id, PermissionLevel::Write, &headers).await?;
    api_result(
        state
            .secrets
            .store()
            .delete_secret_definition(id)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state.realtime, "Binding", "secretDefinitionChanged");
    Ok(no_store(StatusCode::NO_CONTENT.into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/resourceBindings/secret-providers",
    operation_id = "listSecretProviders",
    tag = "ResourceBindings",
    summary = "List Secret providers",
    responses(
        (status = 200, description = "Success", body = SecretProvidersResponse, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list_secret_providers(
    State(state): State<BindingsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> HttpResult {
    global_authorization(&state, principal, PermissionLevel::Read, &headers).await?;
    let providers = api_result(
        state
            .secrets
            .store()
            .list_secret_providers()
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    Ok(no_store(
        Json(SecretProvidersResponse {
            providers: providers.into_iter().map(Into::into).collect(),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/resourceBindings/secret-providers/vault-kv2",
    operation_id = "createVaultKvV2SecretProvider",
    tag = "ResourceBindings",
    summary = "Create a Vault-compatible KV v2 Secret provider",
    request_body = SecretProviderInput,
    responses(
        (status = 200, description = "Success", body = crate::api::resources::bindings::views::SecretProviderView, content_type = "application/json"),
        crate::openapi::errors::CreateErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create_secret_provider(
    State(state): State<BindingsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<SecretProviderInput>, JsonRejection>,
) -> HttpResult {
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let input: citadel_bindings::SecretProviderInput = input.into();
    global_authorization(&state, principal, PermissionLevel::Write, &headers).await?;
    let provider = api_result(
        state
            .secrets
            .create_secret_provider(input)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state.realtime, "Binding", "secretProviderChanged");
    Ok(no_store(
        Json(crate::api::resources::bindings::views::SecretProviderView::from(provider))
            .into_response(),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/resourceBindings/secret-providers/vault-kv2/{id}",
    operation_id = "updateVaultKvV2SecretProvider",
    tag = "ResourceBindings",
    summary = "Update a Vault-compatible KV v2 Secret provider",
    request_body(content(
        (SecretProviderPatch = "application/merge-patch+json"),
        (SecretProviderPatch = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = crate::api::resources::bindings::views::SecretProviderView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_secret_provider(
    State(state): State<BindingsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<SecretProviderPatch>, JsonRejection>,
) -> HttpResult {
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let input: citadel_bindings::SecretProviderPatch = input.into();
    let Path(id) = path_value(path, &headers)?;
    binding_resource_authorization(&state, principal, id, PermissionLevel::Write, &headers).await?;
    let provider = api_result(
        state
            .secrets
            .update_secret_provider(id, input)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state.realtime, "Binding", "secretProviderChanged");
    Ok(no_store(
        Json(crate::api::resources::bindings::views::SecretProviderView::from(provider))
            .into_response(),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/resourceBindings/secret-providers/{id}",
    operation_id = "deleteSecretProvider",
    tag = "ResourceBindings",
    summary = "Delete a Secret provider",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn delete_secret_provider(
    State(state): State<BindingsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let Path(id) = path_value(path, &headers)?;
    binding_resource_authorization(&state, principal, id, PermissionLevel::Write, &headers).await?;
    api_result(
        state
            .secrets
            .store()
            .delete_secret_provider(id)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state.realtime, "Binding", "secretProviderChanged");
    Ok(no_store(StatusCode::NO_CONTENT.into_response()))
}

async fn global_authorization(
    state: &BindingsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> HttpResult<ActorPrincipal> {
    let principal = api_result(require_actor(principal), headers)?;
    authorize_global(
        &state.identity,
        &principal,
        ResourceType::Binding,
        level,
        headers,
    )
    .await?;
    Ok(principal)
}

async fn binding_resource_authorization(
    state: &BindingsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    id: Uuid,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> HttpResult<ActorPrincipal> {
    let principal = api_result(require_actor(principal), headers)?;
    authorize_resource(
        &state.identity,
        &principal,
        ResourceType::Binding,
        id,
        level,
        None,
        headers,
    )
    .await?;
    Ok(principal)
}

async fn scoped_authorization(
    state: &BindingsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(ResourceBindingScope, Uuid)>, PathRejection>,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> HttpResult<(ActorPrincipal, ResourceBindingScope, Uuid)> {
    let principal = api_result(require_actor(principal), headers)?;
    let Path((scope, id)) = api_result(
        path.map_err(|error| crate::api::error::ApiError::Validation(error.to_string())),
        headers,
    )?;
    authorize_scope(state, &principal, scope, id, level, headers).await?;
    Ok((principal, scope, id))
}

async fn secret_scope_authorization(
    state: &BindingsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    query: &SecretScopeQuery,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> HttpResult<ActorPrincipal> {
    let principal = api_result(require_actor(principal), headers)?;
    if let Some(target_type) = query.target_resource_type {
        if query.scope.is_some() || query.resource_id.is_some() {
            return Err(crate::api::error::HttpError::from_parts(
                crate::api::error::ApiError::Validation(
                    "scope/resourceId cannot be combined with targetResourceType.".to_owned(),
                ),
                headers,
            ));
        }
        if !matches!(
            target_type,
            ResourceType::Build | ResourceType::BackupRepository
        ) {
            return Err(crate::api::error::HttpError::from_parts(
                crate::api::error::ApiError::Validation(format!(
                    "Secret definitions cannot be requested for resource type '{target_type:?}'."
                )),
                headers,
            ));
        }
        if let Some(resource_id) = query.target_resource_id {
            authorize_resource(
                &state.identity,
                &principal,
                target_type,
                resource_id,
                PermissionLevel::Write,
                None,
                headers,
            )
            .await?;
        } else {
            authorize_global(
                &state.identity,
                &principal,
                target_type,
                PermissionLevel::Write,
                headers,
            )
            .await?;
        }
        return Ok(principal);
    }
    if query.target_resource_id.is_some() {
        return Err(crate::api::error::HttpError::from_parts(
            crate::api::error::ApiError::Validation(
                "targetResourceType must be provided when targetResourceId is provided.".to_owned(),
            ),
            headers,
        ));
    }
    match (query.scope, query.resource_id) {
        (None, None) => {
            authorize_global(
                &state.identity,
                &principal,
                ResourceType::Binding,
                level,
                headers,
            )
            .await?
        }
        (Some(scope), Some(id)) => {
            authorize_scope(state, &principal, scope, id, level, headers).await?
        }
        _ => {
            return Err(crate::api::error::HttpError::from_parts(
                crate::api::error::ApiError::Validation(
                    "scope and resourceId must be provided together.".to_owned(),
                ),
                headers,
            ));
        }
    }
    Ok(principal)
}

async fn authorize_scope(
    state: &BindingsHttpState,
    principal: &ActorPrincipal,
    scope: ResourceBindingScope,
    resource_id: Uuid,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> HttpResult<()> {
    let resource_type = match scope {
        ResourceBindingScope::Stack => ResourceType::Stack,
        ResourceBindingScope::Deployment => ResourceType::Deployment,
        ResourceBindingScope::SwarmService => ResourceType::SwarmService,
        ResourceBindingScope::Global => {
            return Err(crate::api::error::HttpError::from_parts(
                crate::api::error::ApiError::Validation(
                    "Global resource bindings do not target a resource.".to_owned(),
                ),
                headers,
            ));
        }
    };
    authorize_resource(
        &state.identity,
        principal,
        resource_type,
        resource_id,
        level,
        Some(SpecificPermission::ResourceBindings),
        headers,
    )
    .await
}

fn validate_new_binding(
    input: &citadel_bindings::NewResourceBinding,
    scope: ResourceBindingScope,
    resource_id: Option<Uuid>,
    headers: &HeaderMap,
) -> HttpResult<()> {
    api_result(
        validate_binding(BindingValidation {
            name: &input.name,
            kind: input.kind,
            scope: scope.into(),
            resource_id,
            value: input.value.as_deref(),
            secret_id: input.secret_id,
            delivery: input.secret_delivery_mode,
            target_path: input.target_path.as_deref(),
        })
        .map_err(metadata_error),
        headers,
    )
}

fn validate_update_binding(
    input: &citadel_bindings::ResourceBindingInput,
    scope: ResourceBindingScope,
    resource_id: Option<Uuid>,
    headers: &HeaderMap,
) -> HttpResult<()> {
    api_result(
        validate_binding(BindingValidation {
            name: &input.name,
            kind: input.kind,
            scope: scope.into(),
            resource_id,
            value: input.value.as_deref(),
            secret_id: input.secret_id,
            delivery: input.secret_delivery_mode,
            target_path: input.target_path.as_deref(),
        })
        .map_err(metadata_error),
        headers,
    )
}

async fn load_bindings(
    state: &BindingsHttpState,
    scope: ResourceBindingScope,
    resource_id: Option<Uuid>,
    headers: &HeaderMap,
) -> HttpResult<ResourceBindingsView> {
    api_result(
        state
            .secrets
            .store()
            .get_bindings(scope.into(), resource_id)
            .await
            .map(ResourceBindingsView::from)
            .map_err(metadata_error),
        headers,
    )
}

fn path_value<T>(path: Result<Path<T>, PathRejection>, headers: &HeaderMap) -> HttpResult<Path<T>> {
    api_result(
        path.map_err(|error| crate::api::error::ApiError::Validation(error.to_string())),
        headers,
    )
}
