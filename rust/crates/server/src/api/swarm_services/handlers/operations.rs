use super::*;

#[utoipa::path(
    post,
    path = "/api/v1/swarmServices/{id}/check-updates",
    operation_id = "checkSwarmServiceUpdates",
    tag = "SwarmServices",
    summary = "Check the applied Service image for updates",
    responses(
        (status = 200, description = "Success", body = ManagedSwarmServiceView, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn check_updates(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize::<policy::WriteSwarmService>(&state, &principal, id, &headers).await?;
    let cancel = tokio_util::sync::CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let value = identity_result(
        state
            .services
            .check_updates(
                principal.actor_id,
                principal.is_administrator(),
                id,
                &cancel,
            )
            .await
            .map_err(|error| match error {
                SwarmServiceError::Runtime(message) => IdentityError::External(message),
                other => service_error(other),
            }),
        &headers,
    )?;
    Ok(no_store(
        Json(ManagedSwarmServiceView::from(value)).into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/swarmServices/{id}/apply",
    operation_id = "applySwarmService",
    tag = "SwarmServices",
    summary = "Apply a managed Docker Swarm Service and stream progress",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/SwarmServiceProgressItems"), content_type = "application/json"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn apply(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize::<policy::ApplySwarmService>(&state, &principal, id, &headers).await?;
    Ok(progress_response(state.services.apply(
        principal.actor_id,
        principal.is_administrator(),
        id,
    )))
}

#[utoipa::path(
    post,
    path = "/api/v1/swarmServices/{id}/scale",
    operation_id = "scaleSwarmService",
    tag = "SwarmServices",
    summary = "Scale a managed Docker Swarm Service and stream progress",
    request_body = ScaleSwarmServiceInput,
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/SwarmServiceProgressItems"), content_type = "application/json"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn scale(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<ScaleSwarmServiceInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize::<policy::ScaleSwarmService>(&state, &principal, id, &headers).await?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    Ok(progress_response(state.services.scale(
        principal.actor_id,
        principal.is_administrator(),
        id,
        input.replicas,
    )))
}

#[utoipa::path(
    post,
    path = "/api/v1/swarmServices/{id}/force-update",
    operation_id = "forceUpdateSwarmService",
    tag = "SwarmServices",
    summary = "Force a managed Docker Swarm Service task update and stream progress",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/SwarmServiceProgressItems"), content_type = "application/json"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn force_update(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize::<policy::ForceUpdateSwarmService>(&state, &principal, id, &headers).await?;
    Ok(progress_response(state.services.force_update(
        principal.actor_id,
        principal.is_administrator(),
        id,
    )))
}
