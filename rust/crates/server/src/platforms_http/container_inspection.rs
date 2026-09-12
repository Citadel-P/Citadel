use super::*;
use citadel_adapters::statistics_read_store::PostgresStatisticsReadStore;
use citadel_platforms::{StatisticsReadStore, containers::ContainerInspectionPort};

#[utoipa::path(
    get,
    path = "/api/v1/swarmServices/{id}/inspect",
    operation_id = "inspectManagedSwarmService",
    summary = "Inspect the deployed managed Service",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/SwarmServiceInspectView"), content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn inspect_managed_service(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    use citadel_swarm_services::SwarmServiceStore;
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    if !principal.is_administrator() {
        identity_result(
            state
                .identity
                .authorize_resource(
                    &principal,
                    ResourceType::SwarmService,
                    id,
                    PermissionLevel::Read,
                    Some(SpecificPermission::Inspect),
                )
                .await,
            &headers,
        )?;
    }
    let store =
        citadel_adapters::swarm_service_store::PostgresSwarmServiceStore::new(state.pool.clone());
    let service = identity_result(
        store
            .get_authorized(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(crate::swarm_services_http::service_error),
        &headers,
    )?;
    let docker_id = identity_result(
        service
            .docker_service_id
            .ok_or_else(|| IdentityError::Conflict("The Service has not been applied.".into())),
        &headers,
    )?;
    authorize_platform(&state, &principal, service.platform_id, &headers).await?;
    logs::swarm_platform(&state, service.platform_id, &headers).await?;
    let runtime = match runtime_for(&state, service.platform_id).await {
        Ok(runtime) => runtime,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let cancel = tokio_util::sync::CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let client = swarm_inventory::client(&runtime);
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        client.inspect_service(&docker_id, &cancel),
    )
    .await;
    let service = match result {
        Ok(Ok(service)) if service.id == docker_id => service,
        Ok(Err(error)) => return Ok(runtime_error_response(error, &headers)),
        _ => {
            return identity_result(
                Err(IdentityError::External(
                    "Service inspection failed or timed out.".into(),
                )),
                &headers,
            );
        }
    };
    Ok(no_store(
        Json(swarm_inventory::inspect_service_view(service)).into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/stacks/{stackId}/data",
    operation_id = "getContainersData",
    summary = "Get Stack runtime containers",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ContainersDataView"), content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("stackId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn stack_data(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(stack_id) = identity_result(path.map_err(invalid_path), &headers)?;
    if stack_id.is_nil() {
        return identity_result(
            Err(IdentityError::Validation("Invalid Stack id".into())),
            &headers,
        );
    }
    if !principal.is_administrator() {
        identity_result(
            state
                .identity
                .authorize_resource(
                    &principal,
                    ResourceType::Stack,
                    stack_id,
                    PermissionLevel::Read,
                    None,
                )
                .await,
            &headers,
        )?;
    }
    let containers = identity_result(
        state
            .platforms
            .list_stack_containers(stack_id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    Ok(no_store(
        Json(serde_json::json!({"containers":containers.iter()
        .map(ContainerView::runtime_data).collect::<Vec<_>>() }))
        .into_response(),
    ))
}

#[derive(Clone, Copy)]
enum ReadKind {
    Inspect,
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
        ) -> IdentityHttpResult {
            read_container(State(state), principal, path, headers, ReadKind::$kind).await
        }
    };
}
container_reader!(
    #[utoipa::path(
    get,
    path = "/api/v1/containers/{id}/inspect",
    operation_id = "inspectContainer",
    summary = "Inspect a Container with sensitive environment values redacted",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ContainerInspectView"), content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    inspect,
    Inspect
);
container_reader!(
    #[utoipa::path(
    get,
    path = "/api/v1/containers/{id}/info",
    operation_id = "getContainerInfo",
    summary = "getContainerInfo",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ContainerInfoView"), content_type = "application/json"),
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
    summary = "getContainerData",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ContainerDataView"), content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    data,
    Data
);

async fn read_container(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<String>, PathRejection>,
    headers: HeaderMap,
    kind: ReadKind,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(reference) = identity_result(path.map_err(invalid_path), &headers)?;
    if !valid_container_reference(&reference) {
        return identity_result(
            Err(IdentityError::Validation(
                "Must be a valid container id".into(),
            )),
            &headers,
        );
    }
    let store = PostgresStatisticsReadStore::new(state.pool.clone());
    let target = match store.find_container(&reference).await {
        Ok(target) => required(Ok(target), &headers)?,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let capabilities = authorize_platform(&state, &principal, target.platform_id, &headers).await?;
    if matches!(kind, ReadKind::Inspect) && !capabilities.can_inspect {
        return identity_result(Err(IdentityError::Forbidden), &headers);
    }
    let container = required(
        state
            .platforms
            .get_container(target.id)
            .await
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
        let mut data = container.runtime_data();
        data["capabilities"] = serde_json::json!(capabilities);
        data["containerStat"] = serde_json::Value::Null;
        return Ok(no_store(Json(data).into_response()));
    }
    inspect_target(&state, container, &headers, kind, Some(capabilities)).await
}

macro_rules! deployment_reader {
    ($(#[$name_attr:meta])* $name:ident, $kind:ident) => {
        $(#[$name_attr])*
        pub(super) async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<Uuid>, PathRejection>,
            headers: HeaderMap,
        ) -> IdentityHttpResult {
            read_deployment(state, principal, path, headers, ReadKind::$kind).await
        }
    };
}
deployment_reader!(
    #[utoipa::path(
    get,
    path = "/api/v1/deployments/{id}/inspect",
    operation_id = "inspectDeployment",
    summary = "inspectDeployment",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ContainerInspectView"), content_type = "application/json"),
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
    summary = "getDeploymentContainerInfo",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ContainerInfoView"), content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    deployment_info,
    Info
);

async fn read_deployment(
    state: PlatformsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    kind: ReadKind,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    if id.is_nil() {
        return identity_result(
            Err(IdentityError::Validation("Invalid Deployment id".into())),
            &headers,
        );
    }
    identity_result(
        state
            .identity
            .authorize_resource(
                &principal,
                ResourceType::Deployment,
                id,
                PermissionLevel::Read,
                matches!(kind, ReadKind::Inspect).then_some(SpecificPermission::Inspect),
            )
            .await,
        &headers,
    )?;
    let ids: Vec<Uuid> = identity_result(sqlx::query_scalar("SELECT c.id FROM containers c JOIN deployments d ON d.id=c.deploymentid AND d.platformid=c.platformid WHERE d.id=$1 LIMIT 2")
        .bind(id).fetch_all(&state.pool).await.map_err(|error|IdentityError::Storage(error.to_string())), &headers)?;
    let container_id = match ids.as_slice() {
        [id] => *id,
        [] => return identity_result(Err(IdentityError::NotFound), &headers),
        _ => return identity_result(
            Err(IdentityError::Conflict(
                "Deployment container identity is ambiguous. Refresh inventory before inspecting."
                    .into(),
            )),
            &headers,
        ),
    };
    let container = required(
        state
            .platforms
            .get_container(container_id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    if container.deployment_id != Some(id) {
        return identity_result(Err(IdentityError::NotFound), &headers);
    }
    inspect_target(&state, container, &headers, kind, None).await
}

#[utoipa::path(
    get,
    path = "/api/v1/stacks/{stackId}/containers/{containerId}/inspect",
    operation_id = "inspectStackContainer",
    summary = "Inspect a Stack Container with sensitive environment values redacted",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ContainerInspectView"), content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("stackId" = uuid::Uuid, Path), ("containerId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn inspect_stack(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path((stack_id, reference)) = identity_result(path.map_err(invalid_path), &headers)?;
    if stack_id.is_nil() || !valid_container_reference(&reference) {
        return identity_result(
            Err(IdentityError::Validation(
                "Invalid Stack or container id".into(),
            )),
            &headers,
        );
    }
    if !principal.is_administrator() {
        identity_result(
            state
                .identity
                .authorize_resource(
                    &principal,
                    ResourceType::Stack,
                    stack_id,
                    PermissionLevel::Read,
                    Some(SpecificPermission::Inspect),
                )
                .await,
            &headers,
        )?;
    }
    // Resolve within this Stack, not globally: Docker IDs can occur on multiple
    // Platforms. A container from another Stack must never satisfy this route.
    let id: Option<Uuid> = identity_result(
        sqlx::query_scalar(
            "SELECT id FROM containers WHERE stackid=$1 AND (id=$2 OR dockercontainerid=$3)",
        )
        .bind(stack_id)
        .bind(Uuid::parse_str(&reference).ok())
        .bind(&reference)
        .fetch_optional(&state.pool)
        .await
        .map_err(|error| IdentityError::Storage(error.to_string())),
        &headers,
    )?;
    let id = required(Ok(id), &headers)?;
    let container = required(
        state
            .platforms
            .get_container(id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    if container.stack_id != Some(stack_id) {
        return identity_result(Err(IdentityError::NotFound), &headers);
    }
    inspect_target(&state, container, &headers, ReadKind::Inspect, None).await
}

async fn inspect_target(
    state: &PlatformsHttpState,
    container: ContainerView,
    headers: &HeaderMap,
    kind: ReadKind,
    capabilities: Option<PlatformCapabilitiesView>,
) -> IdentityHttpResult {
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
            .map_err(platform_error),
        headers,
    )?;
    if platform.status != "Online" {
        return Ok(conflict_response(
            "Platform is disconnected or unavailable.".into(),
            headers,
        ));
    }
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let result = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let runtime = runtime_for_node(
            state,
            container.platform_id,
            container.docker_node_id.as_deref(),
        )
        .await?;
        match runtime {
            RuntimeRef::Local(runtime) => {
                runtime.inspection(&container.container_id, &cancel).await
            }
            RuntimeRef::Agent(runtime) => {
                runtime.inspection(&container.container_id, &cancel).await
            }
            RuntimeRef::Edge(runtime) => runtime.inspection(&container.container_id, &cancel).await,
        }
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
            let value = if matches!(kind, ReadKind::Info) {
                summary(&container, &platform, inspection, capabilities)
            } else {
                inspection
            };
            Ok(no_store(Json(value).into_response()))
        }
        Err(error) => Ok(runtime_error_response(error, headers)),
    }
}

fn summary(
    container: &ContainerView,
    platform: &PlatformView,
    inspection: serde_json::Value,
    capabilities: Option<PlatformCapabilitiesView>,
) -> serde_json::Value {
    use serde_json::{Value, json};
    let volumes: Vec<_> = inspection["mounts"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|mount| mount["name"].as_str())
        .collect();
    let networks: serde_json::Map<String, Value> = inspection
        .pointer("/networkSettings/networks")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .map(|(name, settings)| {
            let id = settings["networkID"]
                .as_str()
                .filter(|id| !id.is_empty())
                .unwrap_or(name);
            (name.clone(), Value::String(id.into()))
        })
        .collect();
    json!({
        "name":inspection["name"].as_str().unwrap_or_default(),
        "containerId":container.container_id,"platformId":container.platform_id,
        "platformName":if capabilities.is_some() { platform.name.as_str() } else { "" },
        "startedAt":inspection.pointer("/state/startedAt").and_then(Value::as_str).unwrap_or_default(),
        "finishedAt":inspection.pointer("/state/finishedAt").and_then(Value::as_str).unwrap_or_default(),
        "volumes":volumes,"networks":networks,
        "ports":inspection.pointer("/hostConfig/portBindings").filter(|v|v.is_object()).cloned().unwrap_or_else(||json!({})),
        "state":inspection.pointer("/state/status").and_then(Value::as_str).unwrap_or("Unknown"),
        "imageView":container.image_view,"deploymentView":container.deployment_view,"capabilities":capabilities,
    })
}
