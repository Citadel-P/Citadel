use super::*;

macro_rules! swarm_list_handler {
    ($(#[$name_attr:meta])* $name:ident, $method:ident, $item:ty) => {
        $(#[$name_attr])*
        pub(super) async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<Uuid>, PathRejection>,
            headers: HeaderMap,
        ) -> HttpResult {
            let principal = api_result(require_actor(principal), &headers)?;
            let Path(platform_id) = api_result(path.map_err(invalid_path), &headers)?;
            let capabilities =
                authorize_platform(&state, &principal, platform_id, &headers).await?;
            let mut items: Vec<$item> = api_result(
                state
                    .platforms
                    .$method(platform_id)
                    .await.map(|items| items.into_iter().map(<$item>::from).collect::<Vec<_>>())
                    .map_err(platform_error),
                &headers,
            )?;
            for item in &mut items {
                item.capabilities = Some(capabilities);
            }
            Ok(no_store(
                Json(SwarmItemsResponse {
                    items,
                    capabilities,
                })
                .into_response(),
            ))
        }
    };
}

macro_rules! swarm_get_handler {
    ($(#[$name_attr:meta])* $name:ident, $method:ident, $item:ident) => {
        $(#[$name_attr])*
        pub(super) async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<(Uuid, String)>, PathRejection>,
            headers: HeaderMap,
        ) -> HttpResult {
            let principal = api_result(require_actor(principal), &headers)?;
            let Path((platform_id, resource_id)) =
                api_result(path.map_err(invalid_path), &headers)?;
            api_result(validate_docker_resource_id(&resource_id), &headers)?;
            let capabilities =
                authorize_platform(&state, &principal, platform_id, &headers).await?;
            let mut item = required(
                state
                    .platforms
                    .$method(platform_id, &resource_id)
                    .await.map(|item| item.map(<$item>::from))
                    .map_err(platform_error),
                &headers,
            )?;
            item.capabilities = Some(capabilities);
            Ok(no_store(Json(item).into_response()))
        }
    };
}

swarm_list_handler!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/nodes",
    operation_id = "listSwarmNodes",
    tag = "Platforms",
    summary = "List Swarm Nodes",
    responses(
        (status = 200, description = "Success", body = SwarmItemsResponse<SwarmNodeView>, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    list_swarm_nodes,
    list_swarm_nodes,
    SwarmNodeView
);

swarm_get_handler!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/nodes/{nodeId}",
    operation_id = "getSwarmNode",
    tag = "Platforms",
    summary = "Get a Swarm Node",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::SwarmNodeView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("nodeId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    get_swarm_node,
    get_swarm_node,
    SwarmNodeView
);

swarm_list_handler!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/services",
    operation_id = "listSwarmServices",
    tag = "Platforms",
    summary = "List Swarm Services",
    responses(
        (status = 200, description = "Success", body = SwarmItemsResponse<SwarmServiceView>, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    list_swarm_services,
    list_swarm_services,
    SwarmServiceView
);

swarm_get_handler!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/services/{resourceId}",
    operation_id = "getSwarmService",
    tag = "Platforms",
    summary = "Get a Swarm Service",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::SwarmServiceView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    get_swarm_service,
    get_swarm_service,
    SwarmServiceView
);

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/tasks",
    operation_id = "listSwarmTasks",
    tag = "Platforms",
    summary = "List Swarm Tasks",
    responses(
        (status = 200, description = "Success", body = SwarmItemsResponse<SwarmTaskView>, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("limit" = Option<i32>, Query, minimum = 1, maximum = 200, extensions(("x-citadel-default" = json!(50)))), ("serviceId" = Option<String>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn list_swarm_tasks(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath(platform_id): ApiPath<Uuid>,
    filters: Result<Query<SwarmTaskFilters>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let Query(filters) = api_result(filters.map_err(invalid_query), &headers)?;
    api_result(validate_swarm_task_filters(&filters), &headers)?;
    let capabilities = authorize_platform(&state, &principal, platform_id, &headers).await?;
    let mut items = api_result(
        state
            .platforms
            .list_swarm_tasks(platform_id, filters.service_id.as_deref(), filters.limit)
            .await
            .map(|value| {
                value
                    .into_iter()
                    .map(crate::api::resources::platforms::views::SwarmTaskView::from)
                    .collect::<Vec<_>>()
            })
            .map_err(platform_error),
        &headers,
    )?;
    for item in &mut items {
        item.capabilities = Some(capabilities);
    }
    Ok(no_store(
        Json(SwarmItemsResponse {
            items,
            capabilities,
        })
        .into_response(),
    ))
}

swarm_get_handler!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/tasks/{resourceId}",
    operation_id = "getSwarmTask",
    tag = "Platforms",
    summary = "Get a Swarm Task",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::SwarmTaskView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    get_swarm_task,
    get_swarm_task,
    SwarmTaskView
);

swarm_list_handler!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/networks",
    operation_id = "listSwarmNetworks",
    tag = "Platforms",
    summary = "List Swarm Networks",
    responses(
        (status = 200, description = "Success", body = SwarmItemsResponse<SwarmNetworkView>, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    list_swarm_networks,
    list_swarm_networks,
    SwarmNetworkView
);

swarm_get_handler!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/networks/{resourceId}",
    operation_id = "getSwarmNetwork",
    tag = "Platforms",
    summary = "Get a Swarm Network",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::SwarmNetworkView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    get_swarm_network,
    get_swarm_network,
    SwarmNetworkView
);

swarm_list_handler!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/configs",
    operation_id = "listSwarmConfigs",
    tag = "Platforms",
    summary = "List Swarm Configs",
    responses(
        (status = 200, description = "Success", body = SwarmItemsResponse<SwarmConfigView>, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    list_swarm_configs,
    list_swarm_configs,
    SwarmConfigView
);

swarm_get_handler!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/configs/{resourceId}",
    operation_id = "getSwarmConfig",
    tag = "Platforms",
    summary = "Get a Swarm Config",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::SwarmConfigView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    get_swarm_config,
    get_swarm_config,
    SwarmConfigView
);

swarm_list_handler!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/secrets",
    operation_id = "listSwarmSecrets",
    tag = "Platforms",
    summary = "List Swarm Secrets",
    responses(
        (status = 200, description = "Success", body = SwarmItemsResponse<SwarmSecretView>, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    list_swarm_secrets,
    list_swarm_secrets,
    SwarmSecretView
);

swarm_get_handler!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/secrets/{resourceId}",
    operation_id = "getSwarmSecret",
    tag = "Platforms",
    summary = "Get a Swarm Secret",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::SwarmSecretView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    get_swarm_secret,
    get_swarm_secret,
    SwarmSecretView
);

pub(super) fn validation(value: Result<(), &'static str>) -> Result<(), ApiError> {
    value.map_err(|message| ApiError::Validation(message.into()))
}

pub(super) async fn context(
    state: &PlatformsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    platform: Uuid,
    level: PermissionLevel,
    inspect: bool,
    headers: &HeaderMap,
) -> HttpResult<citadel_platforms::PlatformDetails> {
    let principal = api_result(require_actor(principal), headers)?;
    if platform.is_nil() {
        return api_result(
            Err(ApiError::Validation("Platform id is required.".into())),
            headers,
        );
    }
    authorize_platform_level(state, &principal, platform, level, headers).await?;
    if inspect && !principal.is_administrator() {
        api_result(
            state
                .identity
                .authorize_resource(
                    &principal,
                    ResourceType::Platform,
                    platform,
                    PermissionLevel::Read,
                    Some(SpecificPermission::Inspect),
                )
                .await,
            headers,
        )?;
    }
    swarm_platform(state, platform, headers).await?;
    required(
        state
            .platforms
            .get_platform(platform)
            .await
            .map_err(platform_error),
        headers,
    )
}

pub(super) async fn execute_swarm(
    state: &PlatformsHttpState,
    platform: &citadel_platforms::PlatformDetails,
    operation: citadel_platforms::swarm_mutations::SwarmOperation,
    headers: &HeaderMap,
) -> HttpResult {
    let changed = |kind: &str, action: &str, id: &str| {
        publish_runtime_change(state, platform.id, kind, action, id)
    };
    let service = citadel_platforms::swarm_mutations::SwarmOperations {
        reads: &state.platforms,
        services: state.services.as_ref(),
        runtime: state.runtime.as_ref(),
        projections: state.projections.as_ref(),
        changed: &changed,
    };
    match service
        .execute(platform, operation, &state.tasks.cancellation())
        .await
    {
        Ok(()) => Ok(no_store(StatusCode::NO_CONTENT.into_response())),
        Err(citadel_platforms::swarm_mutations::SwarmCompletionError::Runtime(error)) => {
            Ok(runtime_error_response(error, headers))
        }
        Err(citadel_platforms::swarm_mutations::SwarmCompletionError::Projection(error)) => {
            api_result(Err(ApiError::internal(error)), headers)
        }
    }
}

#[utoipa::path(
    patch,
    path = "/api/v1/platforms/{platformId}/swarm/nodes/{nodeId}",
    operation_id = "updateSwarmNode",
    tag = "Platforms",
    summary = "updateSwarmNode",
    request_body = UpdateSwarmNodeInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("nodeId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn update_node(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
    body: Result<Json<UpdateSwarmNodeInput>, JsonRejection>,
) -> HttpResult {
    let Path((pid, id)) = api_result(path.map_err(invalid_path), &headers)?;
    let platform = context(
        &state,
        principal,
        pid,
        PermissionLevel::Write,
        false,
        &headers,
    )
    .await?;
    let Json(input) = api_result(body.map_err(invalid_json), &headers)?;
    let input: citadel_platforms::swarm_mutations::UpdateSwarmNodeInput = input.into();
    execute_swarm(
        &state,
        &platform,
        citadel_platforms::swarm_mutations::SwarmOperation::UpdateNode { id, input },
        &headers,
    )
    .await
}

#[utoipa::path(
    patch,
    path = "/api/v1/platforms/{platformId}/swarm/nodes/availability",
    operation_id = "updateSwarmNodesAvailability",
    tag = "Platforms",
    summary = "updateSwarmNodesAvailability",
    request_body = UpdateSwarmNodesAvailabilityInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn update_availability(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    body: Result<Json<UpdateSwarmNodesAvailabilityInput>, JsonRejection>,
) -> HttpResult {
    let Path(pid) = api_result(path.map_err(invalid_path), &headers)?;
    let platform = context(
        &state,
        principal,
        pid,
        PermissionLevel::Write,
        false,
        &headers,
    )
    .await?;
    let Json(input) = api_result(body.map_err(invalid_json), &headers)?;
    let input: citadel_platforms::swarm_mutations::UpdateSwarmNodesAvailabilityInput = input.into();
    execute_swarm(
        &state,
        &platform,
        citadel_platforms::swarm_mutations::SwarmOperation::UpdateAvailability(input),
        &headers,
    )
    .await
}

#[utoipa::path(
    post,
    path = "/api/v1/platforms/{platformId}/swarm/services/{resourceId}/restart",
    operation_id = "restartSwarmService",
    tag = "Platforms",
    summary = "restartSwarmService",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn restart_service(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let Path((pid, id)) = api_result(path.map_err(invalid_path), &headers)?;
    let platform = context(
        &state,
        principal,
        pid,
        PermissionLevel::Execute,
        false,
        &headers,
    )
    .await?;
    execute_swarm(
        &state,
        &platform,
        citadel_platforms::swarm_mutations::SwarmOperation::RestartService(id),
        &headers,
    )
    .await
}

macro_rules! material_create {
    ($(#[$name_attr:meta])* $name:ident,$secret:expr) => {
        $(#[$name_attr])*
        pub(super) async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<Uuid>, PathRejection>,
            headers: HeaderMap,
            body: Result<Json<CreateSwarmMaterialInput>, JsonRejection>,
        ) -> HttpResult {
            create_material(state, principal, path, headers, body, $secret).await
        }
    };
}

material_create!(
    #[utoipa::path(
    post,
    path = "/api/v1/platforms/{platformId}/swarm/secrets",
    operation_id = "createSwarmSecret",
    tag = "Platforms",
    summary = "createSwarmSecret",
    request_body = crate::api::resources::schema_models::platforms::CreateSwarmMaterialInputSchema,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    create_secret,
    true
);

material_create!(
    #[utoipa::path(
    post,
    path = "/api/v1/platforms/{platformId}/swarm/configs",
    operation_id = "createSwarmConfig",
    tag = "Platforms",
    summary = "createSwarmConfig",
    request_body = crate::api::resources::schema_models::platforms::CreateSwarmMaterialInputSchema,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    create_config,
    false
);

pub(super) async fn create_material(
    state: PlatformsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    body: Result<Json<CreateSwarmMaterialInput>, JsonRejection>,
    secret: bool,
) -> HttpResult {
    let Path(pid) = api_result(path.map_err(invalid_path), &headers)?;
    let platform = context(
        &state,
        principal,
        pid,
        PermissionLevel::Write,
        false,
        &headers,
    )
    .await?;
    let Json(input) = api_result(body.map_err(invalid_json), &headers)?;
    execute_swarm(
        &state,
        &platform,
        citadel_platforms::swarm_mutations::SwarmOperation::CreateMaterial { secret, input },
        &headers,
    )
    .await
}

macro_rules! material_labels {
    ($(#[$name_attr:meta])* $name:ident,$secret:expr) => {
        $(#[$name_attr])*
        pub(super) async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<(Uuid, String)>, PathRejection>,
            headers: HeaderMap,
            body: Result<Json<UpdateSwarmResourceLabelsInput>, JsonRejection>,
        ) -> HttpResult {
            update_labels(state, principal, path, headers, body, $secret).await
        }
    };
}

material_labels!(
    #[utoipa::path(
    patch,
    path = "/api/v1/platforms/{platformId}/swarm/secrets/{resourceId}/labels",
    operation_id = "updateSwarmSecretLabels",
    tag = "Platforms",
    summary = "updateSwarmSecretLabels",
    request_body = crate::api::resources::platforms::requests::UpdateSwarmResourceLabelsInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    update_secret_labels,
    true
);

material_labels!(
    #[utoipa::path(
    patch,
    path = "/api/v1/platforms/{platformId}/swarm/configs/{resourceId}/labels",
    operation_id = "updateSwarmConfigLabels",
    tag = "Platforms",
    summary = "updateSwarmConfigLabels",
    request_body = crate::api::resources::platforms::requests::UpdateSwarmResourceLabelsInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    update_config_labels,
    false
);

pub(super) async fn update_labels(
    state: PlatformsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
    body: Result<Json<UpdateSwarmResourceLabelsInput>, JsonRejection>,
    secret: bool,
) -> HttpResult {
    let Path((pid, id)) = api_result(path.map_err(invalid_path), &headers)?;
    let platform = context(
        &state,
        principal,
        pid,
        PermissionLevel::Write,
        false,
        &headers,
    )
    .await?;
    let Json(input) = api_result(body.map_err(invalid_json), &headers)?;
    let input: citadel_platforms::swarm_mutations::UpdateSwarmResourceLabelsInput = input.into();
    execute_swarm(
        &state,
        &platform,
        citadel_platforms::swarm_mutations::SwarmOperation::UpdateLabels { secret, id, input },
        &headers,
    )
    .await
}

macro_rules! delete_resources {
    ($(#[$name_attr:meta])* $name:ident,$kind:expr) => {
        $(#[$name_attr])*
        pub(super) async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<Uuid>, PathRejection>,
            headers: HeaderMap,
            body: Result<Json<DeleteSwarmResourcesInput>, JsonRejection>,
        ) -> HttpResult {
            delete_swarm_resources(state, principal, path, headers, body, $kind).await
        }
    };
}

delete_resources!(
    #[utoipa::path(
    delete,
    path = "/api/v1/platforms/{platformId}/swarm/services",
    operation_id = "deleteSwarmInventoryServices",
    tag = "Platforms",
    summary = "deleteSwarmInventoryServices",
    request_body = crate::api::resources::platforms::requests::DeleteSwarmResourcesInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    delete_services,
    SwarmResourceKind::Service
);

delete_resources!(
    #[utoipa::path(
    delete,
    path = "/api/v1/platforms/{platformId}/swarm/secrets",
    operation_id = "deleteSwarmSecrets",
    tag = "Platforms",
    summary = "deleteSwarmSecrets",
    request_body = crate::api::resources::platforms::requests::DeleteSwarmResourcesInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    delete_secrets,
    SwarmResourceKind::Secret
);

delete_resources!(
    #[utoipa::path(
    delete,
    path = "/api/v1/platforms/{platformId}/swarm/configs",
    operation_id = "deleteSwarmConfigs",
    tag = "Platforms",
    summary = "deleteSwarmConfigs",
    request_body = crate::api::resources::platforms::requests::DeleteSwarmResourcesInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    delete_configs,
    SwarmResourceKind::Config
);

pub(super) async fn delete_swarm_resources(
    state: PlatformsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    body: Result<Json<DeleteSwarmResourcesInput>, JsonRejection>,
    kind: SwarmResourceKind,
) -> HttpResult {
    let Path(pid) = api_result(path.map_err(invalid_path), &headers)?;
    let platform = context(
        &state,
        principal,
        pid,
        if kind == SwarmResourceKind::Service {
            PermissionLevel::Execute
        } else {
            PermissionLevel::Write
        },
        false,
        &headers,
    )
    .await?;
    let Json(input) = api_result(body.map_err(invalid_json), &headers)?;
    let input: citadel_platforms::swarm_mutations::DeleteSwarmResourcesInput = input.into();
    execute_swarm(
        &state,
        &platform,
        citadel_platforms::swarm_mutations::SwarmOperation::Delete { kind, input },
        &headers,
    )
    .await
}

pub(crate) fn inspect_service_view(
    value: citadel_platforms::swarm_mutations::SwarmServiceInspection,
) -> crate::api::resources::platforms::operation_views::ServiceInspectionView {
    crate::api::resources::platforms::operation_views::ServiceInspectionView {
        id: value.id,
        version_index: value.version_index,
        name: value.name,
        mode: value.mode,
        image: value.image,
        running_task_count: value.running_task_count,
        desired_task_count: value.desired_task_count,
        update_state: value.update_state,
        update_message: value.update_message,
        ports: value.ports,
        network_ids: value.network_ids,
        secret_ids: value.secret_ids,
        config_ids: value.config_ids,
        labels: value.labels,
        created_at: value.created_at,
        updated_at: value.updated_at,
    }
}

macro_rules! reader {
    ($(#[$name_attr:meta])* $name:ident,$kind:expr) => {
        $(#[$name_attr])*
        pub(super) async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<(Uuid, String)>, PathRejection>,
            headers: HeaderMap,
        ) -> HttpResult {
            read_swarm_resource(state, principal, path, headers, $kind).await
        }
    };
}

reader!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/nodes/{nodeId}/inspect",
    operation_id = "inspectSwarmNode",
    tag = "Platforms",
    summary = "inspectSwarmNode",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::operation_views::NodeInspectionView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("nodeId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    inspect_node,
    "node"
);

reader!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/services/{resourceId}/inspect",
    operation_id = "inspectSwarmService",
    tag = "Platforms",
    summary = "inspectSwarmService",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::operation_views::ServiceInspectionView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    inspect_service,
    "service"
);

reader!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/configs/{resourceId}/content",
    operation_id = "getSwarmConfigData",
    tag = "Platforms",
    summary = "getSwarmConfigData",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::operation_views::ConfigContentView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    config_data,
    "config"
);

pub(super) async fn read_swarm_resource(
    state: PlatformsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
    kind: &str,
) -> HttpResult {
    let Path((pid, id)) = api_result(path.map_err(invalid_path), &headers)?;
    let platform = context(
        &state,
        principal,
        pid,
        PermissionLevel::Read,
        true,
        &headers,
    )
    .await?;
    api_result(validation(resource_id(&id)), &headers)?;
    match kind {
        "node" => {
            required(
                state
                    .platforms
                    .get_swarm_node(pid, &id)
                    .await
                    .map(|value| {
                        value.map(crate::api::resources::platforms::views::SwarmNodeView::from)
                    })
                    .map_err(platform_error),
                &headers,
            )?;
        }
        "service" => {
            required(
                state
                    .platforms
                    .get_swarm_service(pid, &id)
                    .await
                    .map(|value| {
                        value.map(crate::api::resources::platforms::views::SwarmServiceView::from)
                    })
                    .map_err(platform_error),
                &headers,
            )?;
        }
        _ => {
            required(
                state
                    .platforms
                    .get_swarm_config(pid, &id)
                    .await
                    .map(|value| {
                        value.map(crate::api::resources::platforms::views::SwarmConfigView::from)
                    })
                    .map_err(platform_error),
                &headers,
            )?;
        }
    }
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let runtime = api_result(
        state
            .runtime
            .swarm(pid, &cancel)
            .await
            .map_err(ApiError::from),
        &headers,
    )?;
    let result = citadel_platforms::swarm_mutations::bounded(async {
        let client = &runtime;
        if kind == "config" {
            manager_identity(runtime.as_ref(), &platform, &cancel).await?;
        }
        match kind {
            "service" => {
                let service = client.inspect_service(&id, &cancel).await?;
                if service.id != id {
                    return Err(RuntimeCapabilityError::new(
                        RuntimeErrorKind::Remote,
                        "Docker returned a different Service.",
                        false,
                    ));
                }
                Ok(json!(inspect_service_view(service)))
            }
            "node" => {
                let (node, running, desired) = client.inspect_node(&id, &cancel).await?;
                if node.id != id {
                    return Err(RuntimeCapabilityError::new(
                        RuntimeErrorKind::Remote,
                        "Docker returned a different Node.",
                        false,
                    ));
                }
                Ok(json!(
                    crate::api::resources::platforms::operation_views::NodeInspectionView {
                        id: node.id,
                        version_index: node.version_index,
                        hostname: node.hostname,
                        role: node.role,
                        is_leader: node.is_leader,
                        reachability: node.reachability,
                        status: node.status,
                        status_message: node.status_message,
                        availability: node.availability,
                        engine_version: node.engine_version,
                        operating_system: node.operating_system,
                        architecture: node.architecture,
                        address: node.address,
                        labels: node.labels,
                        running_task_count: running,
                        desired_task_count: desired,
                        created_at: node.created_at,
                        updated_at: node.updated_at
                    }
                ))
            }
            _ => Ok(json!(
                crate::api::resources::platforms::operation_views::ConfigContentView {
                    content: client.config_data(&id, &cancel).await?
                }
            )),
        }
    })
    .await;
    match result {
        Ok(value) => Ok(no_store(Json(value).into_response())),
        Err(error) => Ok(runtime_error_response(error, &headers)),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm",
    operation_id = "getSwarmOverview",
    tag = "Platforms",
    summary = "Get Swarm inventory health and quorum",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::swarm_views::SwarmOverviewView, content_type = "application/json"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn get(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    if id.is_nil() {
        return api_result(
            Err(ApiError::Validation("Invalid Platform id.".into())),
            &headers,
        );
    }
    let capabilities = authorize_platform(&state, &principal, id, &headers).await?;
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
                "Platform must be Docker Swarm.".into(),
            )),
            &headers,
        );
    }
    let mut summary = api_result(
        state
            .platforms
            .swarm_summary(id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    if summary.node_count == 0 && platform.status == citadel_primitives::PlatformStatus::Online {
        match initialize_swarm(&state, &platform).await {
            Ok(true) => publish_runtime_change(&state, id, "platform", "update", &id.to_string()),
            Ok(false) => {}
            Err(error) => return Ok(runtime_error_response(error, &headers)),
        }
        platform = required(
            state
                .platforms
                .get_platform(id)
                .await
                .map_err(platform_error),
            &headers,
        )?;
        summary = api_result(
            state
                .platforms
                .swarm_summary(id)
                .await
                .map_err(platform_error),
            &headers,
        )?;
    }
    Ok(no_store(
        Json(crate::api::resources::platforms::swarm_views::for_platform(
            summary,
            &platform,
            capabilities,
        ))
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/platforms/{platformId}/swarm/refresh",
    operation_id = "refreshSwarmInventory",
    tag = "Platforms",
    summary = "Refresh Swarm inventory from the manager",
    responses(
        (status = 204, description = "Inventory refreshed"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn refresh_swarm_inventory(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    // Refresh only observes Docker state; it does not modify Swarm resources.
    let platform = context(
        &state,
        principal,
        id,
        PermissionLevel::Read,
        false,
        &headers,
    )
    .await?;
    match citadel_platforms::swarm_mutations::refresh_inventory(
        state.projections.as_ref(),
        state.runtime.as_ref(),
        &platform,
        |kind| publish_runtime_change(&state, id, kind, "update", ""),
    )
    .await
    {
        Ok(()) => Ok(no_store(StatusCode::NO_CONTENT.into_response())),
        Err(error) => Ok(runtime_error_response(error, &headers)),
    }
}

macro_rules! task_reader {
    ($(#[$name_attr:meta])* $name:ident, $permission:ident) => {
        $(#[$name_attr])*
        pub(super) async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<(Uuid, String)>, PathRejection>,
            headers: HeaderMap,
        ) -> HttpResult {
            read_task_runtime(
                state,
                principal,
                path,
                headers,
                SpecificPermission::$permission,
            )
            .await
        }
    };
}

task_reader!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/tasks/{resourceId}/inspect",
    operation_id = "inspectSwarmTask",
    tag = "Platforms",
    summary = "Inspect the current Task container",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::descriptor_views::ContainerInspectionView, content_type = "application/json"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    inspect_swarm_task,
    Inspect
);

task_reader!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/tasks/{resourceId}/terminal",
    operation_id = "getSwarmTaskTerminalTarget",
    tag = "Platforms",
    summary = "Resolve the current Task terminal target",
    responses(
        (status = 200, description = "Success", body = TaskTerminalView, content_type = "application/json"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    terminal,
    Terminal
);

pub(super) async fn read_task_runtime(
    state: PlatformsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
    specific: SpecificPermission,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path((platform, id)) = api_result(path.map_err(invalid_path), &headers)?;
    api_result(validate_docker_resource_id(&id), &headers)?;
    if platform.is_nil() || id.len() > 255 {
        return api_result(
            Err(ApiError::Validation("Invalid Task identity.".into())),
            &headers,
        );
    }
    if !principal.is_administrator() {
        api_result(
            state
                .identity
                .authorize_resource(
                    &principal,
                    ResourceType::Platform,
                    platform,
                    PermissionLevel::Read,
                    Some(specific),
                )
                .await,
            &headers,
        )?;
    }
    swarm_platform(&state, platform, &headers).await?;
    let projection = required(
        state
            .platforms
            .get_swarm_task(platform, &id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    // Bound both manager inspection and the node-local read with one deadline.
    let result = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let live = state
            .runtime
            .tasks(platform, &cancel)
            .await?
            .inspect_task(&id, &cancel)
            .await?;
        let docker_id = citadel_platforms::validate_running_task(&projection, &live)?;
        if specific == SpecificPermission::Terminal {
            return Ok(Json(TaskTerminalView {
                docker_container_id: docker_id.into(),
            })
            .into_response());
        }
        // Use the existing redacted Container inspection contract, not raw Task JSON.
        let inspection = state
            .runtime
            .container_inspection(platform, Some(&live.node_id), &cancel)
            .await?
            .inspection(docker_id, &cancel)
            .await?;
        let inspection = serde_json::from_value::<
            crate::api::resources::platforms::descriptor_views::ContainerInspectionView,
        >(inspection)
        .map_err(|_| {
            RuntimeCapabilityError::new(
                RuntimeErrorKind::Conflict,
                "Invalid Container inspection response.",
                false,
            )
        })?;
        Ok(Json(inspection).into_response())
    })
    .await
    .unwrap_or_else(|_| {
        Err(RuntimeCapabilityError::new(
            RuntimeErrorKind::Timeout,
            "Task runtime read timed out.",
            false,
        ))
    });
    match result {
        Ok(response) => Ok(no_store(response)),
        Err(error) => Ok(runtime_error_response(error, &headers)),
    }
}
