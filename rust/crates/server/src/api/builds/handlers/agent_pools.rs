use super::*;

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct Pools {
    pub(super) pools: Vec<AuthorizedPool>,
    pub(super) capabilities: ResourceCapabilitiesView,
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct AuthorizedPool {
    #[serde(flatten)]
    pub(super) pool: BuildAgentPoolView,
    pub(super) capabilities: ResourceCapabilitiesView,
}

#[utoipa::path(
    get,
    path = "/api/v1/buildAgentPools",
    operation_id = "listBuildAgentPools",
    tag = "BuildAgentPools",
    summary = "List Build Agent Pools",
    responses(
        (status = 200, description = "Success", body = Pools, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("tags" = Option<Vec<String>>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn list_pools(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    let tags = identity_result(
        crate::api::tags::handlers::parse_filters(query.as_deref()),
        &headers,
    )?;
    let mut build_agent_pools = identity_result(
        state
            .builds
            .store()
            .list_pools(principal.actor_id, principal.is_administrator())
            .await
            .map_err(map_error),
        &headers,
    )?;
    build_agent_pools.retain(|pool| crate::api::tags::handlers::matches_filters(&pool.tags, &tags));
    let pools = identity_result(
        authorized_pools(state.builds.store().as_ref(), &principal, build_agent_pools)
            .await
            .map_err(map_error),
        &headers,
    )?;
    let capabilities = pool_permissions(&state, &principal, None, &headers).await?;
    Ok(no_store(
        Json(Pools {
            pools,
            capabilities,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/buildAgentPools",
    operation_id = "createBuildAgentPool",
    tag = "BuildAgentPools",
    summary = "Create a Build Agent Pool",
    request_body = BuildAgentPoolInput,
    responses(
        (status = 200, description = "Success", body = BuildAgentPoolView, content_type = "application/json"),
        crate::openapi::errors::CreateErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn create_pool(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(input): ValidatedJson<BuildAgentPoolInput>,
) -> IdentityHttpResult {
    let mut input: citadel_builds::BuildAgentPoolConfiguration = input.into();
    let principal = actor(principal, &headers)?;
    authorize_global_for(
        &state,
        &principal,
        ResourceType::BuildAgentPool,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    identity_result(input.validate().map_err(map_error), &headers)?;
    let pool = identity_result(
        state
            .builds
            .store()
            .create_pool(principal.actor_id, &input)
            .await
            .map_err(map_error),
        &headers,
    )?;
    pool_response(&state, &principal, pool, &headers).await
}

#[utoipa::path(
    get,
    path = "/api/v1/buildAgentPools/{id}",
    operation_id = "getBuildAgentPool",
    tag = "BuildAgentPools",
    summary = "Get a Build Agent Pool",
    responses(
        (status = 200, description = "Success", body = BuildAgentPoolView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn get_pool(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize_for(
        &state,
        &principal,
        ResourceType::BuildAgentPool,
        id,
        PermissionLevel::Read,
        &headers,
    )
    .await?;
    let pool = identity_result(
        state.builds.store().get_pool(id).await.map_err(map_error),
        &headers,
    )?;
    pool_response(&state, &principal, pool, &headers).await
}

#[utoipa::path(
    post,
    path = "/api/v1/buildAgentPools/{id}/test",
    operation_id = "testBuildAgentPool",
    tag = "BuildAgentPools",
    summary = "Test a Build Agent Pool",
    responses(
        (status = 200, description = "Success", body = BuildAgentPoolView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn test_pool(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize_for(
        &state,
        &principal,
        ResourceType::BuildAgentPool,
        id,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    let pool = identity_result(
        state
            .builds
            .test_pool(id, principal.actor_id)
            .await
            .map_err(map_error),
        &headers,
    )?;
    pool_response(&state, &principal, pool, &headers).await
}

#[utoipa::path(
    patch,
    path = "/api/v1/buildAgentPools/{id}",
    operation_id = "updateBuildAgentPool",
    tag = "BuildAgentPools",
    summary = "Update a Build Agent Pool",
    request_body(content(
        (ref("#/components/schemas/UpdateBuildAgentPoolInput") = "application/merge-patch+json"),
        (ref("#/components/schemas/UpdateBuildAgentPoolInput") = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = BuildAgentPoolView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn update_pool(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    ValidatedJson(patch): ValidatedJson<serde_json::Value>,
) -> IdentityHttpResult {
    save_pool(&state, principal, id, patch, None, false, &headers).await
}

#[utoipa::path(
    patch,
    path = "/api/v1/buildAgentPools/{id}/_metadata",
    operation_id = "updateBuildAgentPoolMetadata",
    tag = "BuildAgentPools",
    summary = "Update Build Agent Pool metadata",
    request_body(content(
        (ref("#/components/schemas/PatchResourceMetadata") = "application/merge-patch+json"),
        (ref("#/components/schemas/PatchResourceMetadata") = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = BuildAgentPoolView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn update_pool_metadata(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    ValidatedJson(patch): ValidatedJson<serde_json::Value>,
) -> IdentityHttpResult {
    save_pool(&state, principal, id, patch, None, true, &headers).await
}

#[derive(Deserialize, utoipa::ToSchema)]
pub(super) struct RenamePool {
    pub(super) id: Uuid,
    pub(super) name: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/buildAgentPools/rename",
    operation_id = "renameBuildAgentPool",
    tag = "BuildAgentPools",
    summary = "Rename a Build Agent Pool",
    request_body = RenamePool,
    responses(
        (status = 200, description = "Success", body = BuildAgentPoolView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn rename_pool(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(input): ValidatedJson<RenamePool>,
) -> IdentityHttpResult {
    save_pool(
        &state,
        principal,
        input.id,
        serde_json::json!({}),
        Some(input.name),
        true,
        &headers,
    )
    .await
}

pub(super) async fn save_pool(
    state: &BuildsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    id: Uuid,
    patch: serde_json::Value,
    name: Option<String>,
    metadata_only: bool,
    headers: &HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, headers)?;
    authorize_for(
        state,
        &principal,
        ResourceType::BuildAgentPool,
        id,
        PermissionLevel::Write,
        headers,
    )
    .await?;
    let current = identity_result(
        state.builds.store().get_pool(id).await.map_err(map_error),
        headers,
    )?;
    if current.archived_at.is_some() {
        return identity_result(Err(IdentityError::NotFound), headers);
    }
    let mut input = identity_result(
        current.apply_patch(patch, metadata_only).map_err(map_error),
        headers,
    )?;
    if let Some(name) = name {
        input.name = name;
    }
    identity_result(input.validate().map_err(map_error), headers)?;
    let pool = identity_result(
        state
            .builds
            .store()
            .update_pool(&current, &input, principal.actor_id)
            .await
            .map_err(map_error),
        headers,
    )?;
    pool_response(state, &principal, pool, headers).await
}

pub(crate) async fn authorized_pools(
    store: &dyn citadel_builds::BuildRepository,
    principal: &ActorPrincipal,
    values: Vec<citadel_builds::BuildAgentPool>,
) -> Result<Vec<AuthorizedPool>, BuildError> {
    let permissions = if principal.is_administrator() || values.is_empty() {
        Default::default()
    } else {
        let ids = values.iter().map(|pool| pool.id).collect::<Vec<_>>();
        store.pool_permissions(principal.actor_id, &ids).await?
    };
    Ok(values
        .into_iter()
        .map(|pool| AuthorizedPool {
            capabilities: pool_capabilities(if principal.is_administrator() {
                EffectivePermission::Administrator
            } else {
                granted(
                    permissions
                        .get(&pool.id)
                        .copied()
                        .unwrap_or(PermissionLevel::None),
                )
            }),
            pool: pool.into(),
        })
        .collect())
}

pub(super) async fn pool_permissions(
    state: &BuildsHttpState,
    principal: &ActorPrincipal,
    id: Option<Uuid>,
    headers: &HeaderMap,
) -> IdentityHttpResult<ResourceCapabilitiesView> {
    if principal.is_administrator() {
        return Ok(pool_capabilities(EffectivePermission::Administrator));
    }
    let permission = match id {
        Some(id) => {
            state
                .identity
                .permission_for_resource(principal, ResourceType::BuildAgentPool, id)
                .await
        }
        None => {
            state
                .identity
                .global_permission(principal, ResourceType::BuildAgentPool)
                .await
        }
    };
    Ok(pool_capabilities(granted(
        identity_result(permission, headers)?.map_or(PermissionLevel::None, |grant| grant.level),
    )))
}

pub(super) async fn pool_response(
    state: &BuildsHttpState,
    principal: &ActorPrincipal,
    pool: citadel_builds::BuildAgentPool,
    headers: &HeaderMap,
) -> IdentityHttpResult {
    let capabilities = pool_permissions(state, principal, Some(pool.id), headers).await?;
    Ok(no_store(
        Json(AuthorizedPool {
            pool: pool.into(),
            capabilities,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/buildAgentPools/{id}",
    operation_id = "archiveBuildAgentPool",
    tag = "BuildAgentPools",
    summary = "Archive a Build Agent Pool",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn archive_pool(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize_for(
        &state,
        &principal,
        ResourceType::BuildAgentPool,
        id,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    identity_result(
        state
            .builds
            .store()
            .archive_pool(id, principal.actor_id)
            .await
            .map_err(map_error),
        &headers,
    )?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(
    post,
    path = "/api/v1/buildAgentPools/{id}/edge/enrollments",
    operation_id = "createBuildAgentPoolEdgeEnrollment",
    tag = "BuildAgentPools",
    summary = "Create build pool Edge Agent enrollment",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/EdgeAgentEnrollmentView"), content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn enroll_pool(
    State(state): State<BuildsHttpState>,
    Extension(edge): Extension<crate::platforms_http::EdgeHttpContext>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize_for(
        &state,
        &principal,
        ResourceType::BuildAgentPool,
        id,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    edge.enrollment(
        citadel_adapters::edge::EdgeTarget::build_pool(id),
        principal.actor_id.value(),
        &headers,
    )
    .await
}

#[utoipa::path(
    get,
    path = "/api/v1/buildAgentPools/{id}/edge/status",
    operation_id = "getBuildAgentPoolEdgeStatus",
    tag = "BuildAgentPools",
    summary = "Get build pool Edge Agent status",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/EdgeAgentStatusView"), content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn pool_edge_status(
    State(state): State<BuildsHttpState>,
    Extension(edge): Extension<crate::platforms_http::EdgeHttpContext>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize_for(
        &state,
        &principal,
        ResourceType::BuildAgentPool,
        id,
        PermissionLevel::Read,
        &headers,
    )
    .await?;
    let status = identity_result(
        edge.store
            .status(&citadel_adapters::edge::EdgeTarget::build_pool(id))
            .await
            .map_err(crate::platforms_http::EdgeHttpContext::error),
        &headers,
    )?;
    Ok(no_store(Json(status).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/buildAgentPools/{id}/edge/revoke",
    operation_id = "revokeBuildAgentPoolEdgeAgent",
    tag = "BuildAgentPools",
    summary = "Revoke build pool Edge Agent",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn revoke_pool_edge(
    State(state): State<BuildsHttpState>,
    Extension(edge): Extension<crate::platforms_http::EdgeHttpContext>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize_for(
        &state,
        &principal,
        ResourceType::BuildAgentPool,
        id,
        PermissionLevel::Execute,
        &headers,
    )
    .await?;
    let target = citadel_adapters::edge::EdgeTarget::build_pool(id);
    identity_result(
        edge.store
            .status(&target)
            .await
            .map_err(crate::platforms_http::EdgeHttpContext::error),
        &headers,
    )?;
    identity_result(
        edge.store
            .revoke(&target)
            .await
            .map_err(crate::platforms_http::EdgeHttpContext::error),
        &headers,
    )?;
    edge.registry.disconnect(&target);
    Ok(no_store(StatusCode::NO_CONTENT.into_response()))
}
