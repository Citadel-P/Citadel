use super::*;

#[utoipa::path(
    get,
    path = "/api/v1/swarmServices/{id}/duplicate-draft",
    operation_id = "getSwarmServiceDuplicateDraft",
    tag = "SwarmServices",
    summary = "Prepare a managed Service duplicate",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/SwarmServiceDuplicateDraftView"), content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn duplicate_draft(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize::<policy::ReadSwarmService>(&state, &principal, id, &headers).await?;
    let draft = identity_result(
        state
            .services
            .duplicate_draft(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(no_store(Json(serde_json::json!({
        "draft": {"name":draft.name,"platformId":draft.platform_id,"description":draft.description,
            "spec":SwarmServiceSpec::from(draft.spec),"tagIds":draft.tag_ids,"duplicateSource":{
                "resourceType":"SwarmService","resourceId":id,"resourceName":draft.source_name}},
        "warnings":draft.warnings
    })).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/swarmServices",
    operation_id = "listManagedSwarmServices",
    tag = "SwarmServices",
    summary = "List managed Docker Swarm Services",
    responses(
        (status = 200, description = "Success", body = ManagedSwarmServicesView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("tags" = Option<Vec<String>>, Query), ("platformId" = Option<uuid::Uuid>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn list(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    filter: Result<WorkloadQuery, crate::identity_http::IdentityHttpError>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let filter = filter?;
    let filter = SwarmServiceFilter {
        tags: filter.tags,
        platform_id: filter.platform_id,
    };
    let capabilities = collection_capabilities(&state.identity, &principal, &headers).await?;
    let value = identity_result(
        state
            .services
            .list(principal.actor_id, principal.is_administrator(), &filter)
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(no_store(
        Json(ManagedSwarmServicesView {
            swarm_services: value
                .into_iter()
                .map(ManagedSwarmServiceView::from)
                .collect(),
            capabilities,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/swarmServices/{id}",
    operation_id = "getManagedSwarmService",
    tag = "SwarmServices",
    summary = "Get a managed Docker Swarm Service",
    responses(
        (status = 200, description = "Success", body = ManagedSwarmServiceView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn get(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize::<policy::ReadSwarmService>(&state, &principal, id, &headers).await?;
    let value = identity_result(
        state
            .services
            .get(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(no_store(
        Json(ManagedSwarmServiceView::from(value)).into_response(),
    ))
}
