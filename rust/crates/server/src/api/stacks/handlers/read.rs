use super::*;

#[utoipa::path(
    get,
    path = "/api/v1/stacks",
    operation_id = "listStacks",
    tag = "Stacks",
    summary = "List authorized Stacks",
    responses(
        (status = 200, description = "Success", body = StacksView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("tags" = Option<Vec<String>>, Query), ("platformId" = Option<uuid::Uuid>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn list(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    filter: Result<WorkloadQuery, crate::identity_http::IdentityHttpError>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let filter = filter?;
    let filter = StackFilter {
        tags: filter.tags,
        platform_id: filter.platform_id,
    };
    let capabilities = collection_capabilities(&state.identity, &principal, &headers).await?;
    let value = identity_result(
        state
            .stacks
            .list(principal.actor_id, principal.is_administrator(), &filter)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(
        Json(StacksView {
            stacks: value.into_iter().map(StackView::from).collect(),
            capabilities,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/stacks/{stackId}",
    operation_id = "getStack",
    tag = "Stacks",
    summary = "Get a Stack",
    responses(
        (status = 200, description = "Success", body = StackView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("stackId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn get(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let (principal, id) =
        actor_and_id::<policy::ReadStack>(&state, principal, path, &headers).await?;
    let value = identity_result(
        state
            .stacks
            .get(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(StackView::from(value)).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/stacks/{stackId}/_cfg",
    operation_id = "getStackConfig",
    tag = "Stacks",
    summary = "Get Stack configuration",
    responses(
        (status = 200, description = "Success", body = StackConfigView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("stackId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn get_config(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let (principal, id) =
        actor_and_id::<policy::ReadStack>(&state, principal, path, &headers).await?;
    let value = identity_result(
        state
            .stacks
            .get_config(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(StackConfigView::from(value)).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/stacks/{stackId}/duplicate-draft",
    operation_id = "getStackDuplicateDraft",
    tag = "Stacks",
    summary = "Build a Stack duplicate draft",
    responses(
        (status = 200, description = "Success", body = StackDuplicateDraftView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("stackId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn duplicate_draft(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let (principal, id) =
        actor_and_id::<policy::ReadStackBindings>(&state, principal, path, &headers).await?;
    let value = identity_result(
        state
            .stacks
            .duplicate_draft(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(
        Json(StackDuplicateDraftView::from(value)).into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/stacks/{stackId}/releases",
    operation_id = "listStackReleases",
    tag = "Stacks",
    summary = "List previous healthy Stack releases",
    responses(
        (status = 200, description = "Success", body = StackReleasesView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("stackId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn releases(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let (principal, id) =
        actor_and_id::<policy::ViewStackReleases>(&state, principal, path, &headers).await?;
    let value = identity_result(
        state
            .stacks
            .releases(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(
        Json(StackReleasesView {
            releases: value.into_iter().map(StackReleaseView::from).collect(),
        })
        .into_response(),
    ))
}
