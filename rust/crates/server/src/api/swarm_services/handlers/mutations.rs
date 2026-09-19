use super::*;

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct ServiceMetadataInput {
    #[serde(deserialize_with = "deserialize_description")]
    description: Option<String>,
}

pub(super) fn deserialize_description<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    serde::Deserialize::deserialize(deserializer)
}

#[utoipa::path(
    patch,
    path = "/api/v1/swarmServices/{id}/_metadata",
    operation_id = "updateSwarmServiceMetadata",
    tag = "SwarmServices",
    summary = "Update managed Service metadata",
    request_body(content(
        (ServiceMetadataInput = "application/merge-patch+json"),
        (ServiceMetadataInput = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = ManagedSwarmServiceView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn update_metadata(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<ServiceMetadataInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize::<policy::WriteSwarmService>(&state, &principal, id, &headers).await?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let value = identity_result(
        state
            .services
            .update_description(
                principal.actor_id,
                principal.is_administrator(),
                id,
                input.description.as_deref(),
            )
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(no_store(
        Json(ManagedSwarmServiceView::from(value)).into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/swarmServices",
    operation_id = "createSwarmService",
    tag = "SwarmServices",
    summary = "Create a managed Docker Swarm Service",
    request_body = CreateSwarmServiceInput,
    responses(
        (status = 200, description = "Success", body = ManagedSwarmServiceView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn create(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<CreateSwarmServiceInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    identity_result(
        state
            .identity
            .require_scope::<policy::CreateSwarmService>(&principal)
            .await,
        &headers,
    )?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let value = identity_result(
        state
            .services
            .create(
                principal.actor_id,
                principal.is_administrator(),
                input.into(),
            )
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(no_store(
        Json(ManagedSwarmServiceView::from(value)).into_response(),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/swarmServices/{id}",
    operation_id = "updateSwarmService",
    tag = "SwarmServices",
    summary = "Update managed Docker Swarm Service configuration",
    request_body = UpdateSwarmServiceInput,
    responses(
        (status = 200, description = "Success", body = ManagedSwarmServiceView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn update(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<UpdateSwarmServiceInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize::<policy::WriteSwarmService>(&state, &principal, id, &headers).await?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let value = identity_result(
        state
            .services
            .update(
                principal.actor_id,
                principal.is_administrator(),
                id,
                input.into(),
            )
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(no_store(
        Json(ManagedSwarmServiceView::from(value)).into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/swarmServices/rename",
    operation_id = "renameSwarmService",
    tag = "SwarmServices",
    summary = "Rename a managed Docker Swarm Service",
    request_body = RenameSwarmServiceInput,
    responses(
        (status = 200, description = "Success", body = ManagedSwarmServiceView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn rename(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<RenameSwarmServiceInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    authorize::<policy::WriteSwarmService>(&state, &principal, input.id, &headers).await?;
    let value = identity_result(
        state
            .services
            .rename(
                principal.actor_id,
                principal.is_administrator(),
                input.into(),
            )
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(no_store(
        Json(ManagedSwarmServiceView::from(value)).into_response(),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/swarmServices",
    operation_id = "deleteSwarmServices",
    tag = "SwarmServices",
    summary = "Delete managed Docker Swarm Services",
    request_body = Vec<Uuid>,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn delete(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<Vec<Uuid>>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Json(ids) = identity_result(input.map_err(invalid_json), &headers)?;
    identity_result(
        state
            .services
            .delete(principal.actor_id, principal.is_administrator(), &ids)
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
