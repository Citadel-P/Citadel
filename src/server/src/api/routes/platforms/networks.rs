use super::*;

#[utoipa::path(
    get,
    path = "/api/v1/networks/{platformId}",
    operation_id = "listNetworks",
    tag = "Networks",
    summary = "List Platform Networks",
    responses(
        (status = 200, description = "Success", body = NetworksResponse, content_type = "application/json"),
        crate::openapi::errors::CreateErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("Dangling" = Option<bool>, Query), ("Driver" = Option<String>, Query), ("Id" = Option<String>, Query), ("Name" = Option<String>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn list_networks(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath(platform_id): ApiPath<Uuid>,
    ApiQuery(filters): ApiQuery<NetworkFilters>,
    headers: HeaderMap,
) -> HttpResult {
    validate_filter_lengths(
        [
            filters.driver.as_deref(),
            filters.id.as_deref(),
            filters.name.as_deref(),
        ],
        &headers,
    )?;
    let platform_capabilities =
        authorize_platform(&state, &principal, platform_id, &headers).await?;
    let capabilities = network_capabilities(platform_capabilities);
    let cancellation = CancellationToken::new();
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let values = async {
        state
            .runtime
            .inventory(platform_id, &cancellation)
            .await?
            .list_networks(&cancellation)
            .await
    }
    .await;
    let values = match values {
        Ok(values) => values,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let mut networks: Vec<_> = api_result(
        values
            .into_iter()
            .filter(|network| network_matches(network, &filters))
            .map(|network| map_network(network, capabilities))
            .collect::<Result<_, _>>()
            .map_err(ApiError::internal),
        &headers,
    )?;
    let node_networks = api_result(
        state
            .platforms
            .list_node_networks(platform_id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    networks.extend(api_result(
        node_networks
            .into_iter()
            .filter(|network| network_matches(&network.resource, &filters))
            .map(|network| map_node_network(network, capabilities))
            .collect::<Result<Vec<_>, _>>()
            .map_err(ApiError::internal),
        &headers,
    )?);
    api_result(
        crate::api::resources::platforms::network_views::enrich_networks(
            &state.platforms,
            &principal,
            platform_id,
            &mut networks,
        )
        .await
        .map_err(platform_error),
        &headers,
    )?;
    Ok(no_store(
        Json(NetworksResponse {
            networks,
            capabilities: ResourceCapabilitiesView {
                can_read: platform_capabilities.can_read,
                can_write: platform_capabilities.can_write,
                can_execute: platform_capabilities.can_execute,
            },
        })
        .into_response(),
    ))
}

/// Called only after LookupStore has checked source/context Platform access.
pub(crate) async fn lookup_platform_resources(
    state: &PlatformsHttpState,
    platform_id: Uuid,
    kind: citadel_discovery::LookupResourceType,
    headers: &HeaderMap,
) -> HttpResult {
    let cancellation = CancellationToken::new();
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let names = async {
        let runtime = state.runtime.inventory(platform_id, &cancellation).await?;
        if kind == citadel_discovery::LookupResourceType::Volume {
            runtime
                .list_volumes(&cancellation)
                .await
                .map(|values| values.into_iter().map(|v| v.name).collect::<Vec<_>>())
        } else {
            runtime
                .list_networks(&cancellation)
                .await
                .map(|values| values.into_iter().map(|v| v.name).collect::<Vec<_>>())
        }
    }
    .await;
    let mut names = match names {
        Ok(names) => names,
        Err(error) => return Ok(runtime_error_response(error, headers)),
    };
    names.sort();
    let rows: Vec<_> = names
        .into_iter()
        .map(|name| citadel_discovery::LookupResourceInfo {
            id: Uuid::nil(),
            name,
            group: None,
        })
        .collect();
    Ok(no_store(Json(rows).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/networks/{platformId}/{networkId}",
    operation_id = "inspectNetwork",
    tag = "Networks",
    summary = "Inspect a Platform Network",
    responses(
        (status = 200, description = "Success", body = NetworkView, content_type = "application/json"),
        crate::openapi::errors::CreateErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("networkId" = String, Path), ("dockerNodeId" = Option<String>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn get_network(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath((platform_id, network_id)): ApiPath<(Uuid, String)>,
    selector: Result<Query<DockerNodeSelector>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let Query(selector) = api_result(selector.map_err(invalid_query), &headers)?;
    api_result(validate_docker_resource_id(&network_id), &headers)?;
    if let Some(node_id) = selector.docker_node_id.as_deref() {
        api_result(validate_docker_resource_id(node_id), &headers)?;
    }
    let platform_capabilities =
        authorize_platform(&state, &principal, platform_id, &headers).await?;
    let capabilities = network_capabilities(platform_capabilities);
    let cancellation = CancellationToken::new();
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let value = async {
        state
            .runtime
            .networks(
                platform_id,
                selector.docker_node_id.as_deref(),
                &cancellation,
            )
            .await?
            .inspect_network(&network_id, &cancellation)
            .await
    }
    .await;
    let mut network = match value {
        Ok(network) => api_result(
            map_network(network, capabilities).map_err(ApiError::internal),
            &headers,
        )?,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    network.docker_node_id = selector.docker_node_id;
    api_result(
        crate::api::resources::platforms::network_views::enrich_networks(
            &state.platforms,
            &principal,
            platform_id,
            std::slice::from_mut(&mut network),
        )
        .await
        .map_err(platform_error),
        &headers,
    )?;
    Ok(no_store(Json(network).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/networks",
    operation_id = "createNetwork",
    tag = "Networks",
    summary = "Create a Network",
    request_body = CreateNetworkInput,
    responses(
        (status = 200, description = "Success", body = CreatedNetworkView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn create_network(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    headers: HeaderMap,
    input: Result<Json<CreateNetworkInput>, JsonRejection>,
) -> HttpResult {
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    api_result(validate_network_input(&input.network), &headers)?;
    authorize_platform_level(
        &state,
        &principal,
        input.platform_id,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    if input.network.scope.eq_ignore_ascii_case("swarm")
        && !api_result(platform_is_swarm(&state, input.platform_id).await, &headers)?
    {
        return api_result(
            Err(ApiError::Validation(
                "Swarm-scoped overlay networks require a Docker Swarm platform.".to_owned(),
            )),
            &headers,
        );
    }
    let cancellation = CancellationToken::new();
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let result = async {
        state
            .runtime
            .networks(input.platform_id, None, &cancellation)
            .await?
            .create_network(&input.network.into(), &cancellation)
            .await
    }
    .await;
    let created = match result {
        Ok(created) => created,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    publish_runtime_change(&state, input.platform_id, "network", "create", &created.id);
    Ok(no_store(
        Json(CreatedNetworkView { id: created.id }).into_response(),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/networks",
    operation_id = "deleteNetworks",
    tag = "Networks",
    summary = "Delete Networks",
    request_body = DeleteNetworksInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn delete_networks(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    headers: HeaderMap,
    input: Result<Json<DeleteNetworksInput>, JsonRejection>,
) -> HttpResult {
    let Json(mut input) = api_result(input.map_err(invalid_json), &headers)?;
    api_result(
        validate_resource_ids(&mut input.ids, 100, "Network"),
        &headers,
    )?;
    authorize_platform_level(
        &state,
        &principal,
        input.platform_id,
        PermissionLevel::Execute,
        &headers,
    )
    .await?;
    let is_swarm = api_result(platform_is_swarm(&state, input.platform_id).await, &headers)?;
    let cancellation = CancellationToken::new();
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let runtime = match state
        .runtime
        .networks(input.platform_id, None, &cancellation)
        .await
    {
        Ok(runtime) => runtime,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    match citadel_platforms::resource_mutations::delete_networks(
        &state.platforms,
        runtime.as_ref(),
        input.platform_id,
        is_swarm,
        &input.ids,
        &cancellation,
        |id| publish_runtime_change(&state, input.platform_id, "network", "remove", id),
    )
    .await
    {
        Ok(()) => Ok(no_store(StatusCode::NO_CONTENT.into_response())),
        Err(citadel_platforms::resource_mutations::NetworkDeletionError::Read(error)) => {
            api_result(Err(platform_error(error)), &headers)
        }
        Err(citadel_platforms::resource_mutations::NetworkDeletionError::Runtime(error)) => {
            Ok(runtime_error_response(error, &headers))
        }
    }
}

#[utoipa::path(get, path="/api/v1/platforms/{id}/network-containers", operation_id="listNetworkContainers", tag="Platforms",
    summary="List authorized workload identities for the network explorer",
    responses((status=200, description="Success", body=crate::api::resources::platforms::networking_views::NetworkContainersView), crate::openapi::errors::ResourceErrors),
    params(("id"=Uuid, Path)), security(("Bearer"=[])),
    extensions(("x-citadel-principal"=json!("actor")), ("x-citadel-public"=json!(true)), ("x-citadel-setup-exempt"=json!(false))))]
pub(super) async fn network_containers(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    authorize_platform(&state, &principal, id, &headers).await?;
    let mut containers = api_result(
        state
            .platforms
            .network_containers(principal.actor_id, principal.is_administrator(), id, None)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    let truncated = containers.len() > 500;
    containers.truncate(500);
    Ok(no_store(
        Json(
            crate::api::resources::platforms::networking_views::NetworkContainersView {
                containers,
                truncated,
            },
        )
        .into_response(),
    ))
}
