use super::*;

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{id}/containers",
    operation_id = "listContainers",
    tag = "Platforms",
    summary = "List Platform Containers",
    responses(
        (status = 200, description = "Success", body = ContainersResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn list_containers(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath(platform_id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    let capabilities = authorize_platform(&state, &principal, platform_id, &headers).await?;
    let mut containers = api_result(
        state
            .platforms
            .list_containers(platform_id)
            .await
            .map(|value| {
                value
                    .into_iter()
                    .map(crate::api::resources::platforms::views::ContainerView::from)
                    .collect::<Vec<_>>()
            })
            .map_err(platform_error),
        &headers,
    )?;
    if let Some(service) = &state.backup_coverage {
        containers = service.enrich_containers(&principal, containers).await;
    }
    api_result(
        crate::api::resources::platforms::container_views::enrich_container_owner_names(
            &state.platforms,
            &principal,
            &mut containers,
        )
        .await
        .map_err(platform_error),
        &headers,
    )?;
    for container in &mut containers {
        container.capabilities = Some(capabilities);
    }
    Ok(no_store(
        Json(ContainersResponse {
            containers,
            capabilities,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/containers/{id}",
    operation_id = "getContainer",
    tag = "Containers",
    summary = "Get a Container",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::ContainerView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn get_container(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath(reference): ApiPath<String>,
    headers: HeaderMap,
) -> HttpResult {
    if !valid_container_reference(&reference) {
        return api_result(
            Err(ApiError::Validation("Must be a valid container id".into())),
            &headers,
        );
    }
    let id = match Uuid::parse_str(&reference) {
        Ok(id) => id,
        Err(_) => {
            let store = &state.statistics;
            match store.find_container(&reference).await {
                Ok(target) => required(Ok(target), &headers)?.id,
                Err(error) => return Ok(runtime_error_response(error, &headers)),
            }
        }
    };
    let mut container = required(
        state
            .platforms
            .get_container(id)
            .await
            .map(|value| value.map(crate::api::resources::platforms::views::ContainerView::from))
            .map_err(platform_error),
        &headers,
    )?;
    container.capabilities =
        Some(authorize_platform(&state, &principal, container.platform_id, &headers).await?);
    Ok(no_store(Json(container).into_response()))
}

#[utoipa::path(
    get, path = "/api/v1/containers/{id}/health-history", operation_id = "getContainerHealthHistory",
    tag = "Containers", summary = "Get bounded Container health and lifecycle transitions",
    responses((status = 200, description = "Up to 200 transitions within seven days; history may have gaps", body = Vec<crate::api::resources::platforms::health_views::ContainerHealthTransitionView>), crate::openapi::errors::ResourceErrors),
    params(("id" = Uuid, Path)), security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn container_health_history(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    let container = required(
        state
            .platforms
            .get_container(id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    authorize_platform(&state, &principal, container.platform_id, &headers).await?;
    let history = api_result(
        state
            .platforms
            .container_health_history(id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    Ok(no_store(Json(history.into_iter().map(crate::api::resources::platforms::health_views::ContainerHealthTransitionView::from).collect::<Vec<_>>()).into_response()))
}

pub(super) fn valid_container_reference(reference: &str) -> bool {
    match Uuid::parse_str(reference) {
        Ok(id) => !id.is_nil(),
        Err(_) => {
            (12..=64).contains(&reference.len()) && reference.bytes().all(|b| b.is_ascii_hexdigit())
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/swarmServices/{id}/inspect",
    operation_id = "inspectManagedSwarmService",
    tag = "SwarmServices",
    summary = "Inspect the deployed managed Service",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::operation_views::ServiceInspectionView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn inspect_managed_service(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    if !principal.is_administrator() {
        api_result(
            state
                .identity
                .require_resource::<citadel_swarm_services::permissions::InspectSwarmService>(
                    &principal, id,
                )
                .await,
            &headers,
        )?;
    }
    let store = &state.services;
    let service = api_result(
        store
            .get_authorized(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(crate::api::routes::swarm_services::service_error),
        &headers,
    )?;
    let docker_id = api_result(
        service
            .docker_service_id
            .as_ref()
            .ok_or_else(|| ApiError::Conflict("The Service has not been applied.".into())),
        &headers,
    )?;
    authorize_platform(&state, &principal, service.platform_id, &headers).await?;
    swarm_platform(&state, service.platform_id, &headers).await?;
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let runtime = match state.runtime.swarm(service.platform_id, &cancel).await {
        Ok(runtime) => runtime,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let client = &runtime;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        client.inspect_service(docker_id, &cancel),
    )
    .await;
    let service = match result {
        Ok(Ok(service)) if service.id == *docker_id => service,
        Ok(Err(error)) => return Ok(runtime_error_response(error, &headers)),
        _ => {
            return api_result(
                Err(ApiError::External(
                    "Service inspection failed or timed out.".into(),
                )),
                &headers,
            );
        }
    };
    Ok(no_store(
        Json(inspect_service_view(service)).into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/stacks/{stackId}/data",
    operation_id = "getContainersData",
    tag = "Stacks",
    summary = "Get Stack runtime containers",
    responses(
        (status = 200, description = "Success", body = ContainerRuntimeListView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("stackId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn stack_data(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath(stack_id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    if stack_id.is_nil() {
        return api_result(
            Err(ApiError::Validation("Invalid Stack id".into())),
            &headers,
        );
    }
    if !principal.is_administrator() {
        api_result(
            state
                .identity
                .require_resource::<citadel_stacks::permissions::ReadStack>(&principal, stack_id)
                .await,
            &headers,
        )?;
    }
    let containers = api_result(
        state
            .platforms
            .list_stack_containers(stack_id)
            .await
            .map(|value| {
                value
                    .into_iter()
                    .map(crate::api::resources::platforms::views::ContainerView::from)
                    .collect::<Vec<_>>()
            })
            .map_err(platform_error),
        &headers,
    )?;
    Ok(no_store(
        Json(ContainerRuntimeListView {
            containers: api_result(
                containers
                    .iter()
                    .map(ContainerView::runtime_data)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(ApiError::internal),
                &headers,
            )?,
        })
        .into_response(),
    ))
}

#[derive(Clone, Copy)]
pub(super) enum ReadKind {
    Inspect,
    Networking,
    Info,
    Data,
}

macro_rules! container_reader {
    ($(#[$name_attr:meta])* $name:ident, $kind:ident) => {
        $(#[$name_attr])*
        pub(super) async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<String>, PathRejection>,
            headers: HeaderMap,
        ) -> HttpResult {
            read_container(State(state), principal, path, headers, ReadKind::$kind).await
        }
    };
}

container_reader!(
    #[utoipa::path(
    get,
    path = "/api/v1/containers/{id}/inspect",
    operation_id = "inspectContainer",
    tag = "Containers",
    summary = "Inspect a Container with sensitive environment values redacted",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::descriptor_views::ContainerInspectionView, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    inspect_container,
    Inspect
);

container_reader!(
    #[utoipa::path(
    get,
    path = "/api/v1/containers/{id}/info",
    operation_id = "getContainerInfo",
    tag = "Containers",
    summary = "getContainerInfo",
    responses(
        (status = 200, description = "Success", body = ContainerSummaryView, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    info,
    Info
);

container_reader!(
    #[utoipa::path(
    get,
    path = "/api/v1/containers/{id}/data",
    operation_id = "getContainerData",
    tag = "Containers",
    summary = "getContainerData",
    responses(
        (status = 200, description = "Success", body = ContainerRuntimeView, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    data,
    Data
);

pub(super) async fn read_container(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<String>, PathRejection>,
    headers: HeaderMap,
    kind: ReadKind,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(reference) = api_result(path.map_err(invalid_path), &headers)?;
    if !valid_container_reference(&reference) {
        return api_result(
            Err(ApiError::Validation("Must be a valid container id".into())),
            &headers,
        );
    }
    let store = &state.statistics;
    let target = match store.find_container(&reference).await {
        Ok(target) => required(Ok(target), &headers)?,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let capabilities = authorize_platform(&state, &principal, target.platform_id, &headers).await?;
    // Networking is a sanitized read; inspect_target also checks workload visibility.
    if matches!(kind, ReadKind::Inspect) && !capabilities.can_inspect {
        return api_result(Err(ApiError::Forbidden), &headers);
    }
    let container = required(
        state
            .platforms
            .get_container(target.id)
            .await
            .map(|value| value.map(crate::api::resources::platforms::views::ContainerView::from))
            .map_err(platform_error),
        &headers,
    )?;
    if container.platform_id != target.platform_id || container.container_id != target.docker_id {
        return Ok(conflict_response(
            "Container identity changed. Refresh inventory before inspecting.".into(),
            &headers,
        ));
    }
    if matches!(kind, ReadKind::Data) {
        let mut data = api_result(
            container.runtime_data().map_err(ApiError::internal),
            &headers,
        )?;
        data.capabilities = Some(capabilities);
        data.container_stat = None;
        if let Some(service) = &state.backup_coverage {
            data.backup_coverage = Some(
                service
                    .container(
                        &principal,
                        container.platform_id,
                        container.docker_node_id.as_deref(),
                        &container.container_id,
                    )
                    .await,
            );
        }
        return Ok(no_store(Json(data).into_response()));
    }
    inspect_target(
        &state,
        &principal,
        container,
        &headers,
        kind,
        Some(capabilities),
    )
    .await
}

macro_rules! deployment_reader {
    ($(#[$name_attr:meta])* $name:ident, $kind:ident) => {
        $(#[$name_attr])*
        pub(super) async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<Uuid>, PathRejection>,
            headers: HeaderMap,
        ) -> HttpResult {
            read_deployment(state, principal, path, headers, ReadKind::$kind).await
        }
    };
}

deployment_reader!(
    #[utoipa::path(
    get,
    path = "/api/v1/deployments/{id}/inspect",
    operation_id = "inspectDeployment",
    tag = "Deployments",
    summary = "inspectDeployment",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::descriptor_views::ContainerInspectionView, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    inspect_deployment,
    Inspect
);

deployment_reader!(
    #[utoipa::path(
    get,
    path = "/api/v1/deployments/{id}/info",
    operation_id = "getDeploymentContainerInfo",
    tag = "Deployments",
    summary = "getDeploymentContainerInfo",
    responses(
        (status = 200, description = "Success", body = ContainerSummaryView, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    deployment_info,
    Info
);

pub(super) async fn read_deployment(
    state: PlatformsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    kind: ReadKind,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    if id.is_nil() {
        return api_result(
            Err(ApiError::Validation("Invalid Deployment id".into())),
            &headers,
        );
    }
    use citadel_deployments::permissions::{InspectDeployment, ReadDeployment};
    let authorization = if matches!(kind, ReadKind::Inspect) {
        state
            .identity
            .require_resource::<InspectDeployment>(&principal, id)
            .await
    } else {
        state
            .identity
            .require_resource::<ReadDeployment>(&principal, id)
            .await
    };
    api_result(authorization, &headers)?;
    let container = api_result(
        state
            .platforms
            .deployment_container(id)
            .await
            .map(ContainerView::from)
            .map_err(platform_error),
        &headers,
    )?;
    // Deployment access is sufficient for diagnostics. Include optional host
    // capabilities only when the actor may also read that Platform.
    let capabilities = if matches!(kind, ReadKind::Info) {
        let permissions =
            effective_permissions(&state, &principal, &[container.platform_id], &headers).await?;
        let capabilities = platform_capabilities(permission_for(
            &permissions,
            container.platform_id,
            principal.is_administrator(),
        ));
        capabilities.can_read.then_some(capabilities)
    } else {
        None
    };
    inspect_target(&state, &principal, container, &headers, kind, capabilities).await
}

#[utoipa::path(
    get,
    path = "/api/v1/stacks/{stackId}/containers/{containerId}/inspect",
    operation_id = "inspectStackContainer",
    tag = "Stacks",
    summary = "Inspect a Stack Container with sensitive environment values redacted",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::descriptor_views::ContainerInspectionView, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("stackId" = uuid::Uuid, Path), ("containerId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn inspect_stack(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath((stack_id, reference)): ApiPath<(Uuid, String)>,
    headers: HeaderMap,
) -> HttpResult {
    if stack_id.is_nil() || !valid_container_reference(&reference) {
        return api_result(
            Err(ApiError::Validation("Invalid Stack or container id".into())),
            &headers,
        );
    }
    if !principal.is_administrator() {
        api_result(
            state
                .identity
                .require_resource::<citadel_stacks::permissions::InspectStack>(&principal, stack_id)
                .await,
            &headers,
        )?;
    }
    let container = api_result(
        state
            .platforms
            .stack_container(stack_id, &reference)
            .await
            .map(ContainerView::from)
            .map_err(platform_error),
        &headers,
    )?;
    inspect_target(
        &state,
        &principal,
        container,
        &headers,
        ReadKind::Inspect,
        None,
    )
    .await
}

pub(super) async fn inspect_target(
    state: &PlatformsHttpState,
    principal: &ActorPrincipal,
    container: ContainerView,
    headers: &HeaderMap,
    kind: ReadKind,
    capabilities: Option<PlatformCapabilitiesView>,
) -> HttpResult {
    let network_container = if matches!(kind, ReadKind::Networking) {
        let visible = api_result(
            state
                .platforms
                .network_containers(
                    principal.actor_id,
                    principal.is_administrator(),
                    container.platform_id,
                    Some(container.id),
                )
                .await
                .map_err(platform_error),
            headers,
        )?;
        let Some(visible) = visible.into_iter().next() else {
            return api_result(Err(ApiError::Forbidden), headers);
        };
        Some(visible)
    } else {
        None
    };
    if container.projection_stale_since.is_some() {
        return Ok(conflict_response(
            "Container inventory is stale. Refresh before inspecting.".into(),
            headers,
        ));
    }
    let platform = required(
        state
            .platforms
            .get_platform(container.platform_id)
            .await
            .map_err(platform_error)
            .and_then(|value| {
                value
                    .map(PlatformView::try_from)
                    .transpose()
                    .map_err(ApiError::internal)
            }),
        headers,
    )?;
    if platform.status != citadel_primitives::PlatformStatus::Online {
        return Ok(conflict_response(
            "Platform is disconnected or unavailable.".into(),
            headers,
        ));
    }
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let result = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        state
            .runtime
            .container_inspection(
                container.platform_id,
                container.docker_node_id.as_deref(),
                &cancel,
            )
            .await?
            .inspection(&container.container_id, &cancel)
            .await
    })
    .await
    .unwrap_or_else(|_| {
        Err(RuntimeCapabilityError::new(
            RuntimeErrorKind::Timeout,
            "Container inspection timed out.",
            false,
        ))
    });
    match result {
        Ok(inspection) => {
            if let Some(network_container) = network_container {
                return Ok(no_store(Json(crate::api::resources::platforms::networking_views::ContainerNetworkingView::from_inspection(&inspection, network_container)).into_response()));
            }
            if matches!(kind, ReadKind::Info) {
                let mut view = api_result(
                    ContainerSummaryView::from_inspection(
                        &container,
                        &platform.name,
                        &inspection,
                        capabilities,
                    )
                    .map_err(ApiError::internal),
                    headers,
                )?;
                if let Some(service) = &state.backup_coverage {
                    view.backup_coverage = Some(
                        service
                            .inspected(
                                principal,
                                container.platform_id,
                                container.docker_node_id.as_deref(),
                                &inspection,
                            )
                            .await,
                    );
                }
                Ok(no_store(Json(view).into_response()))
            } else {
                let inspection = api_result(
                    serde_json::from_value::<
                        crate::api::resources::platforms::descriptor_views::ContainerInspectionView,
                    >(inspection)
                    .map_err(ApiError::internal),
                    headers,
                )?;
                Ok(no_store(Json(inspection).into_response()))
            }
        }
        Err(error) => Ok(runtime_error_response(error, headers)),
    }
}

macro_rules! action_handler {
    ($(#[$name_attr:meta])* $name:ident, $action:ident) => {
        $(#[$name_attr])*
        pub(super) async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            headers: HeaderMap,
            body: Result<Json<Vec<String>>, JsonRejection>,
        ) -> HttpResult {
            let principal = api_result(require_actor(principal), &headers)?;
            let Json(ids) = api_result(body.map_err(invalid_json), &headers)?;
            execute(state, principal, ids, ContainerAction::$action, &headers).await
        }
    };
}

action_handler!(
    #[utoipa::path(
    patch,
    path = "/api/v1/containers/start",
    operation_id = "startContainers",
    tag = "Containers",
    summary = "Start Containers",
    request_body = Vec<uuid::Uuid>,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    start,
    Start
);

action_handler!(
    #[utoipa::path(
    patch,
    path = "/api/v1/containers/stop",
    operation_id = "stopContainers",
    tag = "Containers",
    summary = "Stop Containers",
    request_body = Vec<uuid::Uuid>,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    stop,
    Stop
);

action_handler!(
    #[utoipa::path(
    patch,
    path = "/api/v1/containers/restart",
    operation_id = "restartContainers",
    tag = "Containers",
    summary = "Restart Containers",
    request_body = Vec<uuid::Uuid>,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    restart,
    Restart
);

action_handler!(
    #[utoipa::path(
    patch,
    path = "/api/v1/containers/pause",
    operation_id = "pauseContainers",
    tag = "Containers",
    summary = "Pause Containers",
    request_body = Vec<uuid::Uuid>,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    pause,
    Pause
);

action_handler!(
    #[utoipa::path(
    patch,
    path = "/api/v1/containers/unpause",
    operation_id = "unpauseContainers",
    tag = "Containers",
    summary = "Unpause Containers",
    request_body = Vec<uuid::Uuid>,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    unpause,
    Unpause
);

macro_rules! deployment_action_handler {
    ($(#[$name_attr:meta])* $name:ident, $action:ident) => {
        $(#[$name_attr])*
        pub(super) async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            headers: HeaderMap,
            body: Result<Json<Vec<Uuid>>, JsonRejection>,
        ) -> HttpResult {
            let principal = api_result(require_actor(principal), &headers)?;
            let Json(ids) = api_result(body.map_err(invalid_json), &headers)?;
            match state
                .containers
                .execute_deployments(
                    principal.actor_id,
                    principal.is_administrator(),
                    ids,
                    ContainerAction::$action,
                )
                .await
            {
                Ok(()) => Ok(no_store(StatusCode::NO_CONTENT.into_response())),
                Err(error) => Ok(runtime_error_response(error, &headers)),
            }
        }
    };
}

deployment_action_handler!(
    #[utoipa::path(
    post,
    path = "/api/v1/deployments/start",
    operation_id = "startDeployments",
    tag = "Deployments",
    summary = "Start Deployments",
    request_body = Vec<uuid::Uuid>,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    start_deployments,
    Start
);

deployment_action_handler!(
    #[utoipa::path(
    post,
    path = "/api/v1/deployments/stop",
    operation_id = "stopDeployments",
    tag = "Deployments",
    summary = "Stop Deployments",
    request_body = Vec<uuid::Uuid>,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    stop_deployments,
    Stop
);

deployment_action_handler!(
    #[utoipa::path(
    post,
    path = "/api/v1/deployments/restart",
    operation_id = "restartDeployments",
    tag = "Deployments",
    summary = "Restart Deployments",
    request_body = Vec<uuid::Uuid>,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    restart_deployments,
    Restart
);

deployment_action_handler!(
    #[utoipa::path(
    post,
    path = "/api/v1/deployments/pause",
    operation_id = "pauseDeployments",
    tag = "Deployments",
    summary = "Pause Deployments",
    request_body = Vec<uuid::Uuid>,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    pause_deployments,
    Pause
);

deployment_action_handler!(
    #[utoipa::path(
    post,
    path = "/api/v1/deployments/resume",
    operation_id = "resumeDeployments",
    tag = "Deployments",
    summary = "Resume Deployments",
    request_body = Vec<uuid::Uuid>,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    resume_deployments,
    Unpause
);

#[utoipa::path(
    delete,
    path = "/api/v1/containers",
    operation_id = "deleteContainers",
    tag = "Containers",
    summary = "Delete Containers",
    request_body = DeleteInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn delete_containers(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    headers: HeaderMap,
    ValidatedJson(input): ValidatedJson<DeleteInput>,
) -> HttpResult {
    execute(
        state,
        principal,
        input.container_ids,
        ContainerAction::Delete(input.options.into()),
        &headers,
    )
    .await
}

pub(super) async fn execute(
    state: PlatformsHttpState,
    principal: ActorPrincipal,
    ids: Vec<String>,
    action: ContainerAction,
    headers: &HeaderMap,
) -> HttpResult {
    match state
        .containers
        .execute(
            principal.actor_id,
            principal.is_administrator(),
            ids,
            action,
        )
        .await
    {
        Ok(()) => Ok(no_store(StatusCode::NO_CONTENT.into_response())),
        Err(error) => Ok(runtime_error_response(error, headers)),
    }
}

container_reader!(
    #[utoipa::path(get, path="/api/v1/containers/{id}/networking", operation_id="getContainerNetworking", tag="Containers",
    summary="Read observed Container networking without its configuration",
    responses((status=200, body=crate::api::resources::platforms::networking_views::ContainerNetworkingView), crate::openapi::errors::ExternalResourceErrors),
    params(("id"=String, Path)), security(("Bearer"=[])),
    extensions(("x-citadel-principal"=json!("actor")), ("x-citadel-public"=json!(true)), ("x-citadel-setup-exempt"=json!(false))))]
    container_networking,
    Networking
);
