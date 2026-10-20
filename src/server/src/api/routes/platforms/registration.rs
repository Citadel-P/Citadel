use super::*;

#[utoipa::path(
    get,
    path = "/api/v1/platforms/agent/setup",
    operation_id = "getAgentSetup",
    tag = "Platforms",
    summary = "Get regular Agent setup instructions",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::AgentSetupView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn get_agent_setup(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    setup: Option<Extension<Arc<crate::api::resources::platforms::views::AgentSetupView>>>,
    live: Option<Extension<AgentSetupContext>>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(
        principal
            .map(|Extension(actor)| actor)
            .ok_or(ApiError::Unauthenticated),
        &headers,
    )?;
    api_result(
        state
            .identity
            .authorize(
                &principal,
                ResourceType::Platform,
                PermissionLevel::Write,
                None,
            )
            .await,
        &headers,
    )?;
    let Extension(setup) = api_result(
        setup.ok_or_else(|| ApiError::Storage("Agent setup is not configured.".into())),
        &headers,
    )?;
    if let Some(Extension(live)) = live {
        return Ok(no_store(Json(live.view()).into_response()));
    }
    Ok(no_store(Json(setup.as_ref()).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms",
    operation_id = "listPlatforms",
    tag = "Platforms",
    summary = "List authorized Platforms",
    responses(
        (status = 200, description = "Success", body = PlatformsResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("tags" = Option<Vec<String>>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn list_platforms(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    RawQuery(raw_query): RawQuery,
    headers: HeaderMap,
) -> HttpResult {
    let tags = api_result(parse_tag_filters(raw_query.as_deref()), &headers)?;
    let mut platforms = api_result(
        state
            .platforms
            .list_authorized(principal.actor_id, principal.is_administrator(), &tags)
            .await
            .map_err(platform_error)
            .and_then(|value| {
                value
                    .into_iter()
                    .map(PlatformView::try_from)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(ApiError::internal)
            }),
        &headers,
    )?;
    let ids = platforms
        .iter()
        .map(|platform| platform.id)
        .collect::<Vec<_>>();
    let permissions = effective_permissions(&state, &principal, &ids, &headers).await?;
    for platform in &mut platforms {
        platform.capabilities = Some(platform_capabilities(permission_for(
            &permissions,
            platform.id,
            principal.is_administrator(),
        )));
    }
    let global = if principal.is_administrator() {
        EffectivePlatformPermission {
            level_mask: ALL_LEVELS,
            specific_mask: ALL_PLATFORM_SPECIFIC,
        }
    } else {
        state
            .identity
            .global_permission(&principal, ResourceType::Platform)
            .await
            .map_err(|error| crate::api::error::HttpError::from_parts(error, &headers))?
            .map_or_else(EffectivePlatformPermission::default, |permission| {
                EffectivePlatformPermission {
                    level_mask: permission.level as i32,
                    // Project the capability response using the Platform read model.
                    specific_mask: permission.specifics.bits() as i32,
                }
            })
    };
    Ok(no_store(
        Json(PlatformsResponse {
            platforms,
            capabilities: resource_capabilities(global),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/platforms",
    operation_id = "createPlatform",
    tag = "Platforms",
    summary = "Create a Platform",
    request_body = CreatePlatformInput,
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::PlatformView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn create_platform(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    headers: HeaderMap,
    input: Result<Json<CreatePlatformInput>, JsonRejection>,
) -> HttpResult {
    authorize_platform_creation(&state, &principal, &headers).await?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let input: citadel_platforms::CreatePlatformInput = input.into();
    let id = match state
        .registrations
        .create(principal.actor_id, input, &CancellationToken::new())
        .await
    {
        Ok(id) => id,
        Err(PlatformRegistrationError::Runtime(error)) => {
            return Ok(runtime_error_response(error, &headers));
        }
        Err(error) => {
            return api_result(Err(platform_registration_error(error)), &headers);
        }
    };
    let capabilities = authorize_platform(&state, &principal, id, &headers).await?;
    let mut platform = required(
        state
            .platforms
            .get_platform(id)
            .await
            .map_err(platform_error)
            .and_then(|value| {
                value
                    .map(PlatformView::try_from)
                    .transpose()
                    .map_err(ApiError::internal)
            }),
        &headers,
    )?;
    platform.capabilities = Some(capabilities);
    publish_runtime_change(&state, id, "platform", "create", &id.to_string());
    Ok(no_store(Json(platform).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{id}",
    operation_id = "getPlatfom",
    tag = "Platforms",
    summary = "Get a Platform",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::PlatformView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn get_platform(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    let capabilities = authorize_platform(&state, &principal, id, &headers).await?;
    let mut platform = required(
        state
            .platforms
            .get_platform(id)
            .await
            .map_err(platform_error)
            .and_then(|value| {
                value
                    .map(PlatformView::try_from)
                    .transpose()
                    .map_err(ApiError::internal)
            }),
        &headers,
    )?;
    platform.capabilities = Some(capabilities);
    Ok(no_store(Json(platform).into_response()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/platforms/{id}/_metadata",
    operation_id = "updatePlatformMetadata",
    tag = "Platforms",
    summary = "Update Platform metadata",
    request_body(content(
        (PatchPlatformMetadataInput = "application/merge-patch+json"),
        (PatchPlatformMetadataInput = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::PlatformView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn update_platform_metadata(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    input: Result<Json<PatchPlatformMetadataInput>, JsonRejection>,
) -> HttpResult {
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    authorize_platform_level(&state, &principal, id, PermissionLevel::Write, &headers).await?;

    let description = match input.description {
        PatchField::Missing => None,
        PatchField::Null => Some(None),
        PatchField::Value(value) => {
            if value.chars().count() > 600 {
                return api_result(
                    Err(ApiError::Validation(
                        "Description cannot exceed 600 characters.".to_owned(),
                    )),
                    &headers,
                );
            }
            Some(Some(value))
        }
    };
    if let Some(description) = description {
        api_result(
            state
                .platform_metadata
                .update_platform_description(id, description.as_deref())
                .await
                .map_err(platform_metadata_error),
            &headers,
        )?;
    }
    let capabilities = authorize_platform(&state, &principal, id, &headers).await?;
    let mut platform = required(
        state
            .platforms
            .get_platform(id)
            .await
            .map_err(platform_error)
            .and_then(|value| {
                value
                    .map(PlatformView::try_from)
                    .transpose()
                    .map_err(ApiError::internal)
            }),
        &headers,
    )?;
    platform.capabilities = Some(capabilities);
    publish_runtime_change(&state, id, "platform", "update", &id.to_string());
    Ok(no_store(Json(platform).into_response()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/platforms",
    operation_id = "deletePlatforms",
    tag = "Platforms",
    summary = "Delete Platform registrations",
    request_body = DeletePlatformsInput,
    responses(
        (status = 200, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn delete_platforms(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    headers: HeaderMap,
    input: Result<Json<DeletePlatformsInput>, JsonRejection>,
) -> HttpResult {
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let mut input: citadel_platforms::deletion::DeletePlatformsInput = input.into();
    api_result(input.validate().map_err(deletion_error), &headers)?;
    let permissions = effective_permissions(&state, &principal, &input.ids, &headers).await?;
    if input.ids.iter().any(|id| {
        !resource_capabilities(permission_for(
            &permissions,
            *id,
            principal.is_administrator(),
        ))
        .can_execute
    }) {
        return api_result(Err(ApiError::Forbidden), &headers);
    }
    api_result(
        state
            .deletions
            .delete(principal.actor_id, &input.ids, |id| {
                if let Some(realtime) = &state.realtime {
                    realtime.publish_resource_change("Platform", id, "deleted");
                }
            })
            .await
            .map_err(deletion_error),
        &headers,
    )?;
    Ok(no_store(StatusCode::OK.into_response()))
}

pub(super) fn deletion_error(error: PlatformDeletionError) -> ApiError {
    match error {
        PlatformDeletionError::Validation => ApiError::Validation(error.to_string()),
        PlatformDeletionError::NotFound => ApiError::NotFound,
        PlatformDeletionError::InUse => ApiError::Conflict(error.to_string()),
        source @ PlatformDeletionError::Storage(_) => ApiError::internal(source),
    }
}

macro_rules! setup_handler {
    ($(#[$name_attr:meta])* $name:ident,$kind:ident) => {
        $(#[$name_attr])*
        pub(super) async fn $name(
            State(state): State<PlatformsHttpState>,
            Extension(edge): Extension<EdgeHttpContext>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<Uuid>, PathRejection>,
            headers: HeaderMap,
        ) -> HttpResult {
            node_agent_operation(
                state,
                principal,
                path,
                headers,
                Some((
                    SetupKind::$kind,
                    SetupOptions {
                        policy: edge.node_agent_policy,
                        core_url: edge.core_url,
                        image: edge.agent_image,
                        ca_bundle: edge.node_agent_ca_bundle,
                    },
                )),
            )
            .await
        }
    };
}

setup_handler!(
    #[utoipa::path(
    post,
    path = "/api/v1/platforms/{id}/node-agents/install",
    operation_id = "installSwarmNodeAgents",
    tag = "Platforms",
    summary = "Install Docker Swarm node agents",
    responses(
        (status = 200, description = "Success", body = Vec<crate::api::resources::schema_models::platforms::NodeAgentProgressSchema>, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    install_node_agents,
    Install
);

setup_handler!(
    #[utoipa::path(
    post,
    path = "/api/v1/platforms/{id}/node-agents/repair",
    operation_id = "repairSwarmNodeAgents",
    tag = "Platforms",
    summary = "Repair Docker Swarm node-agent coverage",
    responses(
        (status = 200, description = "Success", body = Vec<crate::api::resources::schema_models::platforms::NodeAgentProgressSchema>, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    repair_node_agents,
    Repair
);

setup_handler!(
    #[utoipa::path(
    post,
    path = "/api/v1/platforms/{id}/node-agents/upgrade",
    operation_id = "upgradeSwarmNodeAgents",
    tag = "Platforms",
    summary = "Upgrade Docker Swarm node agents",
    responses(
        (status = 200, description = "Success", body = Vec<crate::api::resources::schema_models::platforms::NodeAgentProgressSchema>, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    upgrade_node_agents,
    Upgrade
);

#[utoipa::path(
    delete,
    path = "/api/v1/platforms/{id}/node-agents",
    operation_id = "removeSwarmNodeAgents",
    tag = "Platforms",
    summary = "Remove Docker Swarm node agents",
    responses(
        (status = 200, description = "Success", body = Vec<crate::api::resources::schema_models::platforms::NodeAgentProgressSchema>, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn remove_node_agents(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    node_agent_operation(state, principal, path, headers, None).await
}

pub(super) async fn node_agent_operation(
    state: PlatformsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    setup: Option<(SetupKind, SetupOptions)>,
) -> HttpResult {
    static OPERATIONS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    let capabilities =
        authorize_platform_level(&state, &principal, id, PermissionLevel::Execute, &headers)
            .await?;
    if !capabilities.can_manage_node_agents {
        return api_result(Err(ApiError::Forbidden), &headers);
    }
    let platform = required(
        state
            .platforms
            .get_platform(id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    if platform.platform_type != citadel_platforms::PlatformKind::DockerSwarm {
        return api_result(
            Err(ApiError::Validation(
                "Node agents require a Docker Swarm platform.".into(),
            )),
            &headers,
        );
    }
    if let Some((_, options)) = &setup {
        api_result(
            options
                .validate()
                .map_err(|e| ApiError::Validation(e.message)),
            &headers,
        )?;
    }
    let permit = match OPERATIONS.try_acquire() {
        Ok(permit) => permit,
        Err(_) => {
            return Ok(runtime_error_response(
                RuntimeCapabilityError::new(
                    RuntimeErrorKind::ResourceExhausted,
                    "Too many node-agent operations are running.",
                    true,
                ),
                &headers,
            ));
        }
    };
    let Some(task) = state.tasks.reserve() else {
        return Ok(runtime_error_response(
            RuntimeCapabilityError::new(
                RuntimeErrorKind::ResourceExhausted,
                "Platform operations are shutting down.",
                false,
            ),
            &headers,
        ));
    };
    let cancellation = state.tasks.cancellation();
    let notify_state = state.clone();
    let store = state.node_agent_store.clone();
    let runtime = state.node_agent_runtime.clone();
    let changed: Arc<dyn Fn(Uuid) + Send + Sync> = Arc::new(move |id| {
        publish_runtime_change(
            &notify_state,
            id,
            "nodeAgentCoverage",
            "update",
            &id.to_string(),
        );
    });
    let guard = cancellation.clone().drop_guard();
    let (sender, mut receiver) = tokio::sync::mpsc::channel(32);
    task.spawn(async move {
        let _permit = permit;
        if let Some((kind, options)) = setup {
            NodeAgentSetupService {
                store,
                runtime,
                changed,
            }
            .run(principal.actor_id, id, kind, options, sender, cancellation)
            .await;
        } else {
            citadel_platforms::node_agents::lifecycle::NodeAgentRemovalService {
                store,
                runtime,
                changed,
            }
            .remove(principal.actor_id, id, sender, cancellation)
            .await;
        }
    });
    let stream = async_stream::stream! {
        let _guard = guard;
        yield Ok::<_,std::convert::Infallible>(bytes::Bytes::from_static(b"["));
        let mut first=true;
        while let Some(item)=receiver.recv().await {
            if !first {yield Ok(bytes::Bytes::from_static(b","));} first=false;
            yield Ok(bytes::Bytes::from(serde_json::to_vec(&item).expect("Node-agent progress serializes")));
        }
        yield Ok(bytes::Bytes::from_static(b"]"));
    };
    let mut response = no_store(axum::body::Body::from_stream(stream).into_response());
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json; charset=utf-8"),
    );
    Ok(response)
}

pub(super) async fn initialize_swarm(
    state: &PlatformsHttpState,
    platform: &citadel_platforms::PlatformDetails,
) -> Result<bool, RuntimeCapabilityError> {
    citadel_platforms::swarm_mutations::initialize_inventory(
        &state.platforms,
        state.projections.as_ref(),
        state.runtime.as_ref(),
        platform,
    )
    .await
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{id}/node-agent-coverage",
    operation_id = "getSwarmNodeAgentCoverage",
    tag = "Platforms",
    summary = "Get Docker Swarm node-agent coverage",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::schema_models::platforms::SwarmNodeAgentCoverageSchema, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn node_coverage(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    let capabilities =
        authorize_platform_level(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let mut platform = required(
        state
            .platforms
            .get_platform(id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    if platform.platform_type != citadel_platforms::PlatformKind::DockerSwarm {
        return api_result(
            Err(ApiError::Validation(
                "Node Agent coverage is available only for Docker Swarm platforms.".into(),
            )),
            &headers,
        );
    }
    let mut coverage = api_result(
        state
            .node_agent_coverage
            .read(&platform)
            .await
            .map_err(ApiError::internal),
        &headers,
    )?;
    if coverage.total_nodes == 0 {
        let initialized = initialize_swarm(&state, &platform).await;
        let changed = match initialized {
            Ok(changed) => changed,
            Err(error) => return Ok(runtime_error_response(error, &headers)),
        };
        if changed {
            publish_runtime_change(&state, id, "platform", "update", &id.to_string());
        }
        platform = required(
            state
                .platforms
                .get_platform(id)
                .await
                .map_err(platform_error),
            &headers,
        )?;
        coverage = api_result(
            state
                .node_agent_coverage
                .read(&platform)
                .await
                .map_err(ApiError::internal),
            &headers,
        )?;
    }
    coverage.can_manage_node_agents = capabilities.can_manage_node_agents;
    Ok(no_store(Json(coverage).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/platforms/{id}/edge/enrollments",
    operation_id = "createEdgeAgentEnrollment",
    tag = "Platforms",
    summary = "Create an Edge Agent enrollment token",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::operation_views::EdgeEnrollmentView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn enroll(
    State(state): State<PlatformsHttpState>,
    Extension(edge): Extension<EdgeHttpContext>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    authorize_platform_level(&state, &principal, id, PermissionLevel::Write, &headers).await?;
    let enrollment = api_result(
        edge.enrollment(EdgeTarget::platform(id), principal.actor_id.value())
            .await,
        &headers,
    )?;
    Ok(no_store(Json(enrollment).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{id}/edge/status",
    operation_id = "getEdgeAgentStatus",
    tag = "Platforms",
    summary = "Get Edge Agent connection status",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::operation_views::EdgeStatusView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn status(
    State(state): State<PlatformsHttpState>,
    Extension(edge): Extension<EdgeHttpContext>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    authorize_platform_level(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let status = api_result(
        edge.store
            .status(&EdgeTarget::platform(id))
            .await
            .map_err(EdgeHttpContext::error),
        &headers,
    )?;
    Ok(no_store(
        Json(crate::api::resources::platforms::operation_views::EdgeStatusView::from(status))
            .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/platforms/{id}/edge/revoke",
    operation_id = "revokeEdgeAgent",
    tag = "Platforms",
    summary = "Revoke an Edge Agent binding",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn revoke(
    State(state): State<PlatformsHttpState>,
    Extension(edge): Extension<EdgeHttpContext>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    authorize_platform_level(&state, &principal, id, PermissionLevel::Write, &headers).await?;
    let target = EdgeTarget::platform(id);
    api_result(
        citadel_platforms::edge_management::revoke(
            edge.store.as_ref(),
            edge.registry.as_ref(),
            &target,
        )
        .await
        .map_err(EdgeHttpContext::error),
        &headers,
    )?;
    publish_runtime_change(&state, id, "platform", "update", &id.to_string());
    Ok(no_store(StatusCode::NO_CONTENT.into_response()))
}

pub(super) static PULL_SLOTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);

#[utoipa::path(
    post,
    path = "/api/v1/platforms/{id}/prune",
    operation_id = "prunePlatform",
    tag = "Platforms",
    summary = "Prune unused Docker resources",
    request_body = PrunePlatformInput,
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::PrunePlatformView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn prune(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    input: Result<Json<PrunePlatformInput>, JsonRejection>,
) -> HttpResult {
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let input: citadel_platforms::prune::PrunePlatformInput = input.into();
    if id.is_nil() {
        return api_result(
            Err(ApiError::Validation("A Platform ID is required.".into())),
            &headers,
        );
    }
    authorize_platform_level(&state, &principal, id, PermissionLevel::Execute, &headers).await?;
    let _permit = api_result(
        PRUNE_SLOTS
            .try_acquire()
            .map_err(|_| ApiError::Conflict("Prune capacity is busy. Retry later.".into())),
        &headers,
    )?;
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let operation = async {
        state
            .runtime
            .pruning(id, &cancel)
            .await?
            .prune(input.resource, &cancel)
            .await
    };
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(300), operation).await;
    // Partial failures can still delete resources. Invalidate inventory on every outcome.
    for resource in ["volume", "network", "image", "platform"] {
        publish_runtime_change(&state, id, resource, "update", &id.to_string());
    }
    match outcome {
        Ok(Ok(value)) => Ok(no_store(Json(value).into_response())),
        Ok(Err(error)) => Ok(runtime_error_response(error, &headers)),
        Err(_) => Ok(runtime_error_response(
            RuntimeCapabilityError::new(
                RuntimeErrorKind::Timeout,
                "Prune timed out; some deletions may have completed.",
                false,
            ),
            &headers,
        )),
    }
}

#[derive(Clone)]
pub struct AgentSetupContext {
    pub signer: Arc<dyn citadel_platforms::management::AgentSigningPort>,
    pub image: String,
    pub requires_tls: bool,
}

impl AgentSetupContext {
    pub fn view(&self) -> crate::api::resources::platforms::views::AgentSetupView {
        crate::api::resources::platforms::views::AgentSetupView::new(
            self.signer.public_key_base64(),
            self.image.clone(),
            self.requires_tls,
        )
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/platforms/agent/setup/rotate-key",
    operation_id = "rotateAgentHubKey",
    tag = "Platforms",
    summary = "Rotate the Agent signing key",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::AgentSetupView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn rotate_key(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    setup: Option<Extension<AgentSetupContext>>,
    headers: HeaderMap,
) -> HttpResult {
    api_result(
        state
            .identity
            .authorize(
                &principal,
                ResourceType::Platform,
                PermissionLevel::Write,
                None,
            )
            .await,
        &headers,
    )?;
    let Extension(setup) = api_result(
        setup.ok_or_else(|| ApiError::Storage("Agent key storage is not configured.".into())),
        &headers,
    )?;
    api_result(
        setup
            .signer
            .rotate()
            .await
            .map_err(|e| ApiError::Storage(e.message)),
        &headers,
    )?;
    Ok(no_store(Json(setup.view()).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/platforms/rename",
    operation_id = "renamePlatform",
    tag = "Platforms",
    summary = "Rename a Platform",
    request_body = RenamePlatformInput,
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::PlatformView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn rename(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    headers: HeaderMap,
    input: Result<Json<RenamePlatformInput>, JsonRejection>,
) -> HttpResult {
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let input: citadel_platforms::management::RenamePlatformInput = input.into();
    update(
        state,
        principal,
        input.id,
        serde_json::json!({"name":input.name}),
        true,
        headers,
    )
    .await
}

#[utoipa::path(
    patch,
    path = "/api/v1/platforms/{id}",
    operation_id = "updatePlatform",
    tag = "Platforms",
    summary = "Update a Platform",
    request_body(content(
        (crate::api::resources::platforms::patch::PlatformPatch = "application/merge-patch+json"),
        (crate::api::resources::platforms::patch::PlatformPatch = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::PlatformView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn patch(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    input: Result<Json<serde_json::Value>, JsonRejection>,
) -> HttpResult {
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    update(state, principal, id, input, false, headers).await
}

pub(super) async fn update(
    state: PlatformsHttpState,
    principal: ActorPrincipal,
    id: Uuid,
    input: serde_json::Value,
    rename: bool,
    headers: HeaderMap,
) -> HttpResult {
    if id.is_nil() {
        return api_result(
            Err(ApiError::Validation("A Platform ID is required.".into())),
            &headers,
        );
    }
    let capabilities =
        authorize_platform_level(&state, &principal, id, PermissionLevel::Write, &headers).await?;
    let current = required(
        state
            .platforms
            .get_platform(id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    // Decode partial fields only after scoped authorization, preserving PATCH omission/null semantics.
    let patch: crate::api::resources::platforms::patch::PlatformPatch = api_result(
        serde_path_to_error::deserialize(input).map_err(|error| {
            crate::api::request_validation::validation_error(format!(
                "Invalid Platform patch: {error}"
            ))
        }),
        &headers,
    )?;
    let input = api_result(
        serde_json::to_value(patch).map_err(ApiError::internal),
        &headers,
    )?;
    let input = api_result(
        patch_input(&current, &input).map_err(platform_registration_error),
        &headers,
    )?;
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    match state
        .management
        .update(&current, &input, principal.actor_id, rename, &cancel)
        .await
    {
        Ok(()) => {}
        Err(PlatformRegistrationError::Runtime(error)) => {
            return Ok(runtime_error_response(error, &headers));
        }
        Err(error) => return api_result(Err(platform_registration_error(error)), &headers),
    }
    let updated = required(
        state
            .platforms
            .get_platform(id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    let mut updated = api_result(
        PlatformView::try_from(updated).map_err(ApiError::internal),
        &headers,
    )?;
    updated.capabilities = Some(capabilities);
    publish_runtime_change(&state, id, "platform", "update", &id.to_string());
    Ok(no_store(Json(updated).into_response()))
}
