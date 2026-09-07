use axum::extract::rejection::{JsonRejection, PathRejection};
use axum::extract::{Extension, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::{Json, Router};
use citadel_contracts::http::routes;
use citadel_domain::{PermissionLevel, ResourceType, SpecificPermission};
use citadel_identity::ActorPrincipal;
use citadel_platforms::ResourceCapabilitiesView;
use citadel_resources::{
    BindingValidation, ExternalSecretInput, ExternalSecretPatch, NewResourceBinding,
    ResourceBindingInput, ResourceBindingScope, ResourceBindingsView, SecretDefinitionView,
    SecretProviderInput, SecretProviderPatch, SecretProviderView, validate_binding,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    ResourcesHttpState, authorize_global, authorize_resource, capabilities, invalid_json,
    metadata_error, publish_resource_change, require_actor,
};
use crate::contract_router::ContractRouterExt;
use crate::identity_http::{IdentityHttpResult, identity_result, no_store};

pub(super) fn router(state: ResourcesHttpState) -> Router {
    Router::new()
        .contract_route(routes::GET_GLOBAL_RESOURCE_BINDINGS, get_global_bindings)
        .contract_route(
            routes::CREATE_GLOBAL_RESOURCE_BINDING,
            create_global_binding,
        )
        .contract_route(
            routes::UPDATE_GLOBAL_RESOURCE_BINDING,
            update_global_binding,
        )
        .contract_route(
            routes::DELETE_GLOBAL_RESOURCE_BINDING,
            delete_global_binding,
        )
        .contract_route(routes::GET_RESOURCE_BINDINGS, get_resource_bindings)
        .contract_route(routes::CREATE_RESOURCE_BINDING, create_resource_binding)
        .contract_route(routes::UPDATE_RESOURCE_BINDING, update_resource_binding)
        .contract_route(routes::DELETE_RESOURCE_BINDING, delete_resource_binding)
        .contract_route(routes::LIST_SECRET_DEFINITIONS, list_secrets)
        .contract_route(routes::CREATE_INTERNAL_SECRET, create_internal_secret)
        .contract_route(routes::CREATE_EXTERNAL_SECRET, create_external_secret)
        .contract_route(routes::UPDATE_EXTERNAL_SECRET, update_external_secret)
        .contract_route(routes::DELETE_SECRET_DEFINITION, delete_secret)
        .contract_route(routes::LIST_SECRET_PROVIDERS, list_secret_providers)
        .contract_route(
            routes::CREATE_VAULT_KV2_SECRET_PROVIDER,
            create_secret_provider,
        )
        .contract_route(
            routes::UPDATE_VAULT_KV2_SECRET_PROVIDER,
            update_secret_provider,
        )
        .contract_route(routes::DELETE_SECRET_PROVIDER, delete_secret_provider)
        .contract_route(
            routes::TEST_VAULT_KV2_SECRET_PROVIDER_CONNECTION,
            test_secret_provider,
        )
        .contract_route(routes::TEST_EXTERNAL_SECRET, test_external_secret)
        .with_state(state)
}

async fn test_secret_provider(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<citadel_resources::TestSecretProviderInput>, JsonRejection>,
) -> IdentityHttpResult {
    global_authorization(&state, principal, PermissionLevel::Write, &headers).await?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let result = identity_result(
        state
            .resources
            .test_secret_provider(input)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    Ok(no_store(Json(result).into_response()))
}

async fn test_external_secret(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<citadel_resources::TestExternalSecretInput>, JsonRejection>,
) -> IdentityHttpResult {
    global_authorization(&state, principal, PermissionLevel::Write, &headers).await?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let result = identity_result(
        state
            .resources
            .test_external_secret(input)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    Ok(no_store(Json(result).into_response()))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GlobalBindingsResponse {
    #[serde(flatten)]
    bindings: ResourceBindingsView,
    capabilities: ResourceCapabilitiesView,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SecretDefinitionsResponse {
    secrets: Vec<SecretDefinitionView>,
    capabilities: ResourceCapabilitiesView,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SecretProvidersResponse {
    providers: Vec<SecretProviderView>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InternalSecretInput {
    name: String,
    value: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SecretScopeQuery {
    scope: Option<ResourceBindingScope>,
    resource_id: Option<Uuid>,
    target_resource_type: Option<ResourceType>,
    target_resource_id: Option<Uuid>,
}

async fn get_global_bindings(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal =
        global_authorization(&state, principal, PermissionLevel::Read, &headers).await?;
    let bindings = load_bindings(&state, ResourceBindingScope::Global, None, &headers).await?;
    let caps = capabilities(&state, &principal, ResourceType::Binding, None, &headers).await?;
    Ok(no_store(
        Json(GlobalBindingsResponse {
            bindings,
            capabilities: caps,
        })
        .into_response(),
    ))
}

async fn create_global_binding(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<NewResourceBinding>, JsonRejection>,
) -> IdentityHttpResult {
    let Json(mut input) = identity_result(input.map_err(invalid_json), &headers)?;
    global_authorization(&state, principal, PermissionLevel::Write, &headers).await?;
    input.scope = Some(ResourceBindingScope::Global);
    input.resource_id = None;
    validate_new_binding(&input, ResourceBindingScope::Global, None, &headers)?;
    let bindings = identity_result(
        state
            .resources
            .store()
            .create_binding(&input)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state, "Binding", "resourceBindingChanged");
    Ok(no_store(Json(bindings).into_response()))
}

async fn update_global_binding(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<ResourceBindingInput>, JsonRejection>,
) -> IdentityHttpResult {
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    global_authorization(&state, principal, PermissionLevel::Write, &headers).await?;
    validate_update_binding(&input, ResourceBindingScope::Global, None, &headers)?;
    let bindings = identity_result(
        state
            .resources
            .store()
            .update_binding(&input, ResourceBindingScope::Global, None)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state, "Binding", "resourceBindingChanged");
    Ok(no_store(Json(bindings).into_response()))
}

async fn delete_global_binding(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let Path(id) = path_value(path, &headers)?;
    binding_resource_authorization(&state, principal, id, PermissionLevel::Write, &headers).await?;
    let bindings = identity_result(
        state
            .resources
            .store()
            .delete_binding(id, ResourceBindingScope::Global, None)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state, "Binding", "resourceBindingChanged");
    Ok(no_store(Json(bindings).into_response()))
}

async fn get_resource_bindings(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(ResourceBindingScope, Uuid)>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let (principal, scope, id) =
        scoped_authorization(&state, principal, path, PermissionLevel::Read, &headers).await?;
    let _ = principal;
    let bindings = load_bindings(&state, scope, Some(id), &headers).await?;
    Ok(no_store(Json(bindings).into_response()))
}

async fn create_resource_binding(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(ResourceBindingScope, Uuid)>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<NewResourceBinding>, JsonRejection>,
) -> IdentityHttpResult {
    let Json(mut input) = identity_result(input.map_err(invalid_json), &headers)?;
    let (_, scope, id) =
        scoped_authorization(&state, principal, path, PermissionLevel::Write, &headers).await?;
    input.scope = Some(scope);
    input.resource_id = Some(id);
    validate_new_binding(&input, scope, Some(id), &headers)?;
    let bindings = identity_result(
        state
            .resources
            .store()
            .create_binding(&input)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state, "Binding", "resourceBindingChanged");
    Ok(no_store(Json(bindings).into_response()))
}

async fn update_resource_binding(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(ResourceBindingScope, Uuid)>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<ResourceBindingInput>, JsonRejection>,
) -> IdentityHttpResult {
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let (_, scope, id) =
        scoped_authorization(&state, principal, path, PermissionLevel::Write, &headers).await?;
    validate_update_binding(&input, scope, Some(id), &headers)?;
    let bindings = identity_result(
        state
            .resources
            .store()
            .update_binding(&input, scope, Some(id))
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state, "Binding", "resourceBindingChanged");
    Ok(no_store(Json(bindings).into_response()))
}

async fn delete_resource_binding(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(ResourceBindingScope, Uuid, Uuid)>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let Path((scope, resource_id, id)) = identity_result(
        path.map_err(|error| citadel_identity::IdentityError::Validation(error.to_string())),
        &headers,
    )?;
    let principal = identity_result(require_actor(principal), &headers)?;
    authorize_scope(
        &state,
        &principal,
        scope,
        resource_id,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    let bindings = identity_result(
        state
            .resources
            .store()
            .delete_binding(id, scope, Some(resource_id))
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state, "Binding", "resourceBindingChanged");
    Ok(no_store(Json(bindings).into_response()))
}

async fn list_secrets(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    query: Query<SecretScopeQuery>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let Query(query) = query;
    let principal =
        secret_scope_authorization(&state, principal, &query, PermissionLevel::Read, &headers)
            .await?;
    let secrets = identity_result(
        state
            .resources
            .store()
            .list_secret_definitions(query.scope, query.resource_id)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    let caps = capabilities(&state, &principal, ResourceType::Binding, None, &headers).await?;
    Ok(no_store(
        Json(SecretDefinitionsResponse {
            secrets,
            capabilities: caps,
        })
        .into_response(),
    ))
}

async fn create_internal_secret(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    query: Query<SecretScopeQuery>,
    headers: HeaderMap,
    input: Result<Json<InternalSecretInput>, JsonRejection>,
) -> IdentityHttpResult {
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let Query(query) = query;
    secret_scope_authorization(&state, principal, &query, PermissionLevel::Write, &headers).await?;
    let secret = identity_result(
        state
            .resources
            .create_internal_secret(&input.name, &input.value)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state, "Binding", "secretDefinitionChanged");
    Ok(no_store(Json(secret).into_response()))
}

async fn create_external_secret(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<ExternalSecretInput>, JsonRejection>,
) -> IdentityHttpResult {
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    global_authorization(&state, principal, PermissionLevel::Write, &headers).await?;
    identity_result(input.validate().map_err(metadata_error), &headers)?;
    let secret = identity_result(
        state
            .resources
            .store()
            .create_external_secret(&input)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state, "Binding", "secretDefinitionChanged");
    Ok(no_store(Json(secret).into_response()))
}

async fn update_external_secret(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<ExternalSecretPatch>, JsonRejection>,
) -> IdentityHttpResult {
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let Path(id) = path_value(path, &headers)?;
    binding_resource_authorization(&state, principal, id, PermissionLevel::Write, &headers).await?;
    let secret = identity_result(
        state
            .resources
            .store()
            .update_external_secret(id, &input)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state, "Binding", "secretDefinitionChanged");
    Ok(no_store(Json(secret).into_response()))
}

async fn delete_secret(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let Path(id) = path_value(path, &headers)?;
    binding_resource_authorization(&state, principal, id, PermissionLevel::Write, &headers).await?;
    identity_result(
        state
            .resources
            .store()
            .delete_secret_definition(id)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state, "Binding", "secretDefinitionChanged");
    Ok(no_store(StatusCode::NO_CONTENT.into_response()))
}

async fn list_secret_providers(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    global_authorization(&state, principal, PermissionLevel::Read, &headers).await?;
    let providers = identity_result(
        state
            .resources
            .store()
            .list_secret_providers()
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    Ok(no_store(
        Json(SecretProvidersResponse { providers }).into_response(),
    ))
}

async fn create_secret_provider(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<SecretProviderInput>, JsonRejection>,
) -> IdentityHttpResult {
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    global_authorization(&state, principal, PermissionLevel::Write, &headers).await?;
    let provider = identity_result(
        state
            .resources
            .create_secret_provider(input)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state, "Binding", "secretProviderChanged");
    Ok(no_store(Json(provider).into_response()))
}

async fn update_secret_provider(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<SecretProviderPatch>, JsonRejection>,
) -> IdentityHttpResult {
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let Path(id) = path_value(path, &headers)?;
    binding_resource_authorization(&state, principal, id, PermissionLevel::Write, &headers).await?;
    let provider = identity_result(
        state
            .resources
            .update_secret_provider(id, input)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state, "Binding", "secretProviderChanged");
    Ok(no_store(Json(provider).into_response()))
}

async fn delete_secret_provider(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let Path(id) = path_value(path, &headers)?;
    binding_resource_authorization(&state, principal, id, PermissionLevel::Write, &headers).await?;
    identity_result(
        state
            .resources
            .store()
            .delete_secret_provider(id)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state, "Binding", "secretProviderChanged");
    Ok(no_store(StatusCode::NO_CONTENT.into_response()))
}

async fn global_authorization(
    state: &ResourcesHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> IdentityHttpResult<ActorPrincipal> {
    let principal = identity_result(require_actor(principal), headers)?;
    authorize_global(state, &principal, ResourceType::Binding, level, headers).await?;
    Ok(principal)
}

async fn binding_resource_authorization(
    state: &ResourcesHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    id: Uuid,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> IdentityHttpResult<ActorPrincipal> {
    let principal = identity_result(require_actor(principal), headers)?;
    authorize_resource(
        state,
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
    state: &ResourcesHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(ResourceBindingScope, Uuid)>, PathRejection>,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> IdentityHttpResult<(ActorPrincipal, ResourceBindingScope, Uuid)> {
    let principal = identity_result(require_actor(principal), headers)?;
    let Path((scope, id)) = identity_result(
        path.map_err(|error| citadel_identity::IdentityError::Validation(error.to_string())),
        headers,
    )?;
    authorize_scope(state, &principal, scope, id, level, headers).await?;
    Ok((principal, scope, id))
}

async fn secret_scope_authorization(
    state: &ResourcesHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    query: &SecretScopeQuery,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> IdentityHttpResult<ActorPrincipal> {
    let principal = identity_result(require_actor(principal), headers)?;
    if let Some(target_type) = query.target_resource_type {
        if query.scope.is_some() || query.resource_id.is_some() {
            return Err(crate::identity_http::IdentityHttpError::from_parts(
                citadel_identity::IdentityError::Validation(
                    "scope/resourceId cannot be combined with targetResourceType.".to_owned(),
                ),
                headers,
            ));
        }
        if !matches!(
            target_type,
            ResourceType::Build | ResourceType::BackupRepository
        ) {
            return Err(crate::identity_http::IdentityHttpError::from_parts(
                citadel_identity::IdentityError::Validation(format!(
                    "Secret definitions cannot be requested for resource type '{target_type:?}'."
                )),
                headers,
            ));
        }
        if let Some(resource_id) = query.target_resource_id {
            authorize_resource(
                state,
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
                state,
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
        return Err(crate::identity_http::IdentityHttpError::from_parts(
            citadel_identity::IdentityError::Validation(
                "targetResourceType must be provided when targetResourceId is provided.".to_owned(),
            ),
            headers,
        ));
    }
    match (query.scope, query.resource_id) {
        (None, None) => {
            authorize_global(state, &principal, ResourceType::Binding, level, headers).await?
        }
        (Some(scope), Some(id)) => {
            authorize_scope(state, &principal, scope, id, level, headers).await?
        }
        _ => {
            return Err(crate::identity_http::IdentityHttpError::from_parts(
                citadel_identity::IdentityError::Validation(
                    "scope and resourceId must be provided together.".to_owned(),
                ),
                headers,
            ));
        }
    }
    Ok(principal)
}

async fn authorize_scope(
    state: &ResourcesHttpState,
    principal: &ActorPrincipal,
    scope: ResourceBindingScope,
    resource_id: Uuid,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    let resource_type = match scope {
        ResourceBindingScope::Stack => ResourceType::Stack,
        ResourceBindingScope::Deployment => ResourceType::Deployment,
        ResourceBindingScope::SwarmService => ResourceType::SwarmService,
        ResourceBindingScope::Global => {
            return Err(crate::identity_http::IdentityHttpError::from_parts(
                citadel_identity::IdentityError::Validation(
                    "Global resource bindings do not target a resource.".to_owned(),
                ),
                headers,
            ));
        }
    };
    authorize_resource(
        state,
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
    input: &NewResourceBinding,
    scope: ResourceBindingScope,
    resource_id: Option<Uuid>,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    identity_result(
        validate_binding(BindingValidation {
            name: &input.name,
            kind: input.kind,
            scope,
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
    input: &ResourceBindingInput,
    scope: ResourceBindingScope,
    resource_id: Option<Uuid>,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    identity_result(
        validate_binding(BindingValidation {
            name: &input.name,
            kind: input.kind,
            scope,
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
    state: &ResourcesHttpState,
    scope: ResourceBindingScope,
    resource_id: Option<Uuid>,
    headers: &HeaderMap,
) -> IdentityHttpResult<ResourceBindingsView> {
    identity_result(
        state
            .resources
            .store()
            .get_bindings(scope, resource_id)
            .await
            .map_err(metadata_error),
        headers,
    )
}

fn path_value<T>(
    path: Result<Path<T>, PathRejection>,
    headers: &HeaderMap,
) -> IdentityHttpResult<Path<T>> {
    identity_result(
        path.map_err(|error| citadel_identity::IdentityError::Validation(error.to_string())),
        headers,
    )
}
