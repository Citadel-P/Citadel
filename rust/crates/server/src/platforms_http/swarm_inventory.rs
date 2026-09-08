use super::*;
use citadel_adapters::swarm_inventory::SwarmInventoryClient;
use citadel_contracts::citadel::swarm::v1::SwarmServiceMessage;
use citadel_platforms::swarm_mutations::*;
use serde_json::{Value, json};
use std::time::Duration;

pub(crate) fn client<'a>(runtime: &'a RuntimeRef<'_>) -> SwarmInventoryClient<'a> {
    match runtime {
        RuntimeRef::Local(r) => SwarmInventoryClient::Local(r),
        RuntimeRef::Agent(r) => SwarmInventoryClient::Agent(r),
        RuntimeRef::Edge(r) => SwarmInventoryClient::Edge(r),
    }
}
fn validation(value: Result<(), &'static str>) -> Result<(), IdentityError> {
    value.map_err(|message| IdentityError::Validation(message.into()))
}
fn conflict(message: &str) -> IdentityError {
    IdentityError::Conflict(message.into())
}
async fn context(
    state: &PlatformsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    platform: Uuid,
    level: PermissionLevel,
    inspect: bool,
    headers: &HeaderMap,
) -> IdentityHttpResult<PlatformView> {
    let principal = identity_result(require_actor(principal), headers)?;
    if platform.is_nil() {
        return identity_result(
            Err(IdentityError::Validation("Platform id is required.".into())),
            headers,
        );
    }
    authorize_platform_level(state, &principal, platform, level, headers).await?;
    if inspect && !principal.is_administrator() {
        identity_result(
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
    logs::swarm_platform(state, platform, headers).await?;
    required(
        state
            .platforms
            .get_platform(platform)
            .await
            .map_err(platform_error),
        headers,
    )
}
async fn manager_identity(
    runtime: &RuntimeRef<'_>,
    platform: &PlatformView,
    cancel: &CancellationToken,
) -> Result<(), RuntimeCapabilityError> {
    use citadel_platforms::PlatformRuntimePort;
    let info = match runtime {
        RuntimeRef::Local(r) => r.get_info(cancel).await,
        RuntimeRef::Agent(r) => r.get_info(cancel).await,
        RuntimeRef::Edge(r) => r.get_info(cancel).await,
    }?;
    if !manager_matches(
        &info,
        platform.cluster_id.as_deref(),
        &platform.platform_descriptor,
    ) {
        return Err(RuntimeCapabilityError::new(
            RuntimeErrorKind::Conflict,
            "The connected Docker manager no longer belongs to this Swarm platform.",
            false,
        ));
    }
    Ok(())
}
async fn refresh(
    state: &PlatformsHttpState,
    platform: &PlatformView,
) -> Result<(), RuntimeCapabilityError> {
    let runtime = runtime_for(state, platform.id).await?;
    let port: &dyn PlatformInventoryPort = match &runtime {
        RuntimeRef::Local(r) => *r,
        RuntimeRef::Agent(r) => r,
        RuntimeRef::Edge(r) => r,
    };
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let snapshot = tokio::time::timeout(
        Duration::from_secs(30),
        citadel_platforms::jobs::collect_inventory(
            port,
            &citadel_platforms::jobs::InventoryCollectionTarget {
                platform_id: platform.id,
                platform_type: platform.platform_type.clone(),
            },
            &cancel,
        ),
    )
    .await
    .map_err(|_| {
        RuntimeCapabilityError::new(
            RuntimeErrorKind::Timeout,
            "Swarm inventory refresh timed out.",
            false,
        )
    })??;
    citadel_adapters::swarm_inventory_store::refresh(&state.pool, &snapshot).await?;
    for kind in ["node", "service", "task", "secret", "config", "platform"] {
        publish_runtime_change(state, platform.id, kind, "update", "");
    }
    Ok(())
}
async fn finish(
    state: &PlatformsHttpState,
    platform: &PlatformView,
    result: Result<(), RuntimeCapabilityError>,
    headers: &HeaderMap,
) -> IdentityHttpResult {
    // A refresh is attempted after all dispatched outcomes, including ambiguous
    // transport errors. Never report a successful refresh as a successful write.
    let refreshed = refresh(state, platform).await;
    match result.and(refreshed) {
        Ok(()) => Ok(no_store(StatusCode::NO_CONTENT.into_response())),
        Err(error) => Ok(runtime_error_response(error, headers)),
    }
}
async fn bounded<T>(
    future: impl std::future::Future<Output = Result<T, RuntimeCapabilityError>>,
) -> Result<T, RuntimeCapabilityError> {
    tokio::time::timeout(Duration::from_secs(30), future)
        .await
        .unwrap_or_else(|_| {
            Err(RuntimeCapabilityError::new(
                RuntimeErrorKind::Timeout,
                "Swarm operation timed out; inventory will be reconciled.",
                false,
            ))
        })
}

pub(super) async fn update_node(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
    body: Result<Json<UpdateSwarmNodeInput>, JsonRejection>,
) -> IdentityHttpResult {
    let Path((pid, id)) = identity_result(path.map_err(invalid_path), &headers)?;
    let platform = context(
        &state,
        principal,
        pid,
        PermissionLevel::Write,
        false,
        &headers,
    )
    .await?;
    let Json(input) = identity_result(body.map_err(invalid_json), &headers)?;
    identity_result(validation(resource_id(&id).and(input.validate())), &headers)?;
    let node = required(
        state
            .platforms
            .get_swarm_node(pid, &id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    identity_result(check_node(&node, input.version_index), &headers)?;
    let runtime = runtime_for(&state, pid).await.map_err(|e| {
        crate::identity_http::IdentityHttpError::from_parts(conflict(&e.to_string()), &headers)
    })?;
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let result = bounded(async {
        manager_identity(&runtime, &platform, &cancel).await?;
        client(&runtime).update_node(&id, &input, &cancel).await
    })
    .await;
    finish(&state, &platform, result, &headers).await
}
fn check_node(node: &SwarmNodeView, version: i64) -> Result<(), IdentityError> {
    if node.is_stale {
        Err(conflict(
            "Node inventory is stale. Refresh before making changes.",
        ))
    } else if node.version_index != version {
        Err(conflict("Node changed. Reload it before saving."))
    } else {
        Ok(())
    }
}
pub(super) async fn update_availability(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    body: Result<Json<UpdateSwarmNodesAvailabilityInput>, JsonRejection>,
) -> IdentityHttpResult {
    let Path(pid) = identity_result(path.map_err(invalid_path), &headers)?;
    let platform = context(
        &state,
        principal,
        pid,
        PermissionLevel::Write,
        false,
        &headers,
    )
    .await?;
    let Json(input) = identity_result(body.map_err(invalid_json), &headers)?;
    identity_result(validation(input.validate()), &headers)?;
    let mut updates = Vec::with_capacity(input.nodes.len());
    for target in input.nodes {
        let node = required(
            state
                .platforms
                .get_swarm_node(pid, &target.node_id)
                .await
                .map_err(platform_error),
            &headers,
        )?;
        identity_result(check_node(&node, target.version_index), &headers)?;
        if !node.availability.eq_ignore_ascii_case(&input.availability) {
            updates.push((
                node.id,
                UpdateSwarmNodeInput {
                    version_index: target.version_index,
                    availability: input.availability.clone(),
                    labels: node.labels,
                },
            ));
        }
    }
    if updates.is_empty() {
        return Ok(no_store(StatusCode::NO_CONTENT.into_response()));
    }
    let runtime = identity_result(
        runtime_for(&state, pid)
            .await
            .map_err(|e| conflict(&e.to_string())),
        &headers,
    )?;
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let result = bounded(async {
        manager_identity(&runtime, &platform, &cancel).await?;
        for (index, (id, input)) in updates.iter().enumerate() {
            if let Err(error) = client(&runtime).update_node(id, input, &cancel).await {
                return Err(partial(error, index, updates.len(), "Updated"));
            }
        }
        Ok(())
    })
    .await;
    finish(&state, &platform, result, &headers).await
}
fn partial(
    error: RuntimeCapabilityError,
    completed: usize,
    total: usize,
    verb: &str,
) -> RuntimeCapabilityError {
    if completed == 0 {
        error
    } else {
        RuntimeCapabilityError::new(
            RuntimeErrorKind::Conflict,
            format!(
                "{verb} {completed} of {total} resources before Docker rejected the operation: {error}"
            ),
            false,
        )
    }
}
async fn service_guard(
    state: &PlatformsHttpState,
    pid: Uuid,
    id: &str,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    let service = required(
        state
            .platforms
            .get_swarm_service(pid, id)
            .await
            .map_err(platform_error),
        headers,
    )?;
    if service.is_stale {
        return identity_result(Err(conflict("Service inventory is stale.")), headers);
    }
    let managed = identity_result(
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM swarmservices WHERE platformid=$1 AND dockerserviceid=$2)",
        )
        .bind(pid)
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| IdentityError::Storage(e.to_string())),
        headers,
    )?;
    if managed || service.ownership == "System" {
        return identity_result(
            Err(conflict(
                "This Service must be changed through its Citadel owner.",
            )),
            headers,
        );
    }
    Ok(())
}
pub(super) async fn restart_service(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let Path((pid, id)) = identity_result(path.map_err(invalid_path), &headers)?;
    let platform = context(
        &state,
        principal,
        pid,
        PermissionLevel::Execute,
        false,
        &headers,
    )
    .await?;
    identity_result(validation(resource_id(&id)), &headers)?;
    service_guard(&state, pid, &id, &headers).await?;
    let runtime = identity_result(
        runtime_for(&state, pid)
            .await
            .map_err(|e| conflict(&e.to_string())),
        &headers,
    )?;
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let result = bounded(async {
        manager_identity(&runtime, &platform, &cancel).await?;
        client(&runtime).restart_service(&id, &cancel).await
    })
    .await;
    finish(&state, &platform, result, &headers).await
}

macro_rules! material_create {
    ($name:ident,$secret:expr) => {
        pub(super) async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<Uuid>, PathRejection>,
            headers: HeaderMap,
            body: Result<Json<CreateSwarmMaterialInput>, JsonRejection>,
        ) -> IdentityHttpResult {
            create_material(state, principal, path, headers, body, $secret).await
        }
    };
}
material_create!(create_secret, true);
material_create!(create_config, false);
async fn create_material(
    state: PlatformsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    body: Result<Json<CreateSwarmMaterialInput>, JsonRejection>,
    secret: bool,
) -> IdentityHttpResult {
    let Path(pid) = identity_result(path.map_err(invalid_path), &headers)?;
    let platform = context(
        &state,
        principal,
        pid,
        PermissionLevel::Write,
        false,
        &headers,
    )
    .await?;
    let Json(input) = identity_result(body.map_err(invalid_json), &headers)?;
    identity_result(validation(input.validate(secret)), &headers)?;
    let runtime = identity_result(
        runtime_for(&state, pid)
            .await
            .map_err(|e| conflict(&e.to_string())),
        &headers,
    )?;
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let result = bounded(async {
        manager_identity(&runtime, &platform, &cancel).await?;
        client(&runtime)
            .create_material(secret, &input, &cancel)
            .await
    })
    .await;
    finish(&state, &platform, result, &headers).await
}
macro_rules! material_labels {
    ($name:ident,$secret:expr) => {
        pub(super) async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<(Uuid, String)>, PathRejection>,
            headers: HeaderMap,
            body: Result<Json<UpdateSwarmResourceLabelsInput>, JsonRejection>,
        ) -> IdentityHttpResult {
            update_labels(state, principal, path, headers, body, $secret).await
        }
    };
}
material_labels!(update_secret_labels, true);
material_labels!(update_config_labels, false);
async fn material_guard(
    state: &PlatformsHttpState,
    pid: Uuid,
    id: &str,
    secret: bool,
    deleting: bool,
    version: Option<i64>,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    let (stale, in_use, current) = if secret {
        let r = required(
            state
                .platforms
                .get_swarm_secret(pid, id)
                .await
                .map_err(platform_error),
            headers,
        )?;
        (r.is_stale, r.in_use, r.version_index)
    } else {
        let r = required(
            state
                .platforms
                .get_swarm_config(pid, id)
                .await
                .map_err(platform_error),
            headers,
        )?;
        (r.is_stale, r.in_use, r.version_index)
    };
    if stale || (deleting && in_use) || version.is_some_and(|v| v != current) {
        return identity_result(
            Err(conflict(if stale {
                "Resource inventory is stale."
            } else if in_use && deleting {
                "Resource is in use and cannot be deleted."
            } else {
                "Resource changed. Reload it before saving."
            })),
            headers,
        );
    }
    Ok(())
}
async fn update_labels(
    state: PlatformsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
    body: Result<Json<UpdateSwarmResourceLabelsInput>, JsonRejection>,
    secret: bool,
) -> IdentityHttpResult {
    let Path((pid, id)) = identity_result(path.map_err(invalid_path), &headers)?;
    let platform = context(
        &state,
        principal,
        pid,
        PermissionLevel::Write,
        false,
        &headers,
    )
    .await?;
    let Json(input) = identity_result(body.map_err(invalid_json), &headers)?;
    identity_result(validation(resource_id(&id).and(input.validate())), &headers)?;
    material_guard(
        &state,
        pid,
        &id,
        secret,
        false,
        Some(input.version_index),
        &headers,
    )
    .await?;
    let runtime = identity_result(
        runtime_for(&state, pid)
            .await
            .map_err(|e| conflict(&e.to_string())),
        &headers,
    )?;
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let result = bounded(async {
        manager_identity(&runtime, &platform, &cancel).await?;
        client(&runtime)
            .update_labels(secret, &id, &input, &cancel)
            .await
    })
    .await;
    finish(&state, &platform, result, &headers).await
}
macro_rules! delete_resources {
    ($name:ident,$kind:expr) => {
        pub(super) async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<Uuid>, PathRejection>,
            headers: HeaderMap,
            body: Result<Json<DeleteSwarmResourcesInput>, JsonRejection>,
        ) -> IdentityHttpResult {
            delete(state, principal, path, headers, body, $kind).await
        }
    };
}
delete_resources!(delete_services, "service");
delete_resources!(delete_secrets, "secret");
delete_resources!(delete_configs, "config");
async fn delete(
    state: PlatformsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    body: Result<Json<DeleteSwarmResourcesInput>, JsonRejection>,
    kind: &str,
) -> IdentityHttpResult {
    let Path(pid) = identity_result(path.map_err(invalid_path), &headers)?;
    let platform = context(
        &state,
        principal,
        pid,
        if kind == "service" {
            PermissionLevel::Execute
        } else {
            PermissionLevel::Write
        },
        false,
        &headers,
    )
    .await?;
    let Json(mut input) = identity_result(body.map_err(invalid_json), &headers)?;
    identity_result(validation(input.validate()), &headers)?;
    for id in &input.ids {
        if kind == "service" {
            service_guard(&state, pid, id, &headers).await?;
        } else {
            material_guard(&state, pid, id, kind == "secret", true, None, &headers).await?;
        }
    }
    let runtime = identity_result(
        runtime_for(&state, pid)
            .await
            .map_err(|e| conflict(&e.to_string())),
        &headers,
    )?;
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let mut removed = Vec::new();
    let result = bounded(async {
        manager_identity(&runtime, &platform, &cancel).await?;
        for (index, id) in input.ids.iter().enumerate() {
            let result = if kind == "service" {
                client(&runtime).delete_service(id, &cancel).await
            } else {
                client(&runtime)
                    .delete_material(kind == "secret", id, &cancel)
                    .await
            };
            match result {
                Ok(()) => {}
                Err(error) if error.kind == RuntimeErrorKind::NotFound => {}
                Err(error) => return Err(partial(error, index, input.ids.len(), "Deleted")),
            }
            removed.push(id.clone());
        }
        Ok(())
    })
    .await;
    let response = finish(&state, &platform, result, &headers).await;
    // Confirmed removals must not remain as a stale, selectable row. Ambiguous
    // failures remain visible for reconciliation; successful siblings are removed.
    let sql = match kind {
        "service" => {
            "DELETE FROM swarmserviceprojections WHERE platformid=$1 AND dockerserviceid=ANY($2)"
        }
        "secret" => {
            "DELETE FROM swarmsecretprojections WHERE platformid=$1 AND dockersecretid=ANY($2)"
        }
        _ => "DELETE FROM swarmconfigprojections WHERE platformid=$1 AND dockerconfigid=ANY($2)",
    };
    identity_result(
        sqlx::query(sql)
            .bind(pid)
            .bind(&removed)
            .execute(&state.pool)
            .await
            .map_err(|e| IdentityError::Storage(e.to_string())),
        &headers,
    )?;
    for id in removed {
        publish_runtime_change(&state, pid, kind, "remove", &id);
    }
    response
}

pub(crate) fn inspect_service_view(value: SwarmServiceMessage) -> Value {
    json!({"id":value.id,"versionIndex":value.version_index,"name":value.name,"mode":value.mode,"image":value.image,"runningTaskCount":value.running_task_count,"desiredTaskCount":value.desired_task_count,"updateState":value.update_state,"updateMessage":(!value.update_message.is_empty()).then_some(value.update_message),"ports":value.ports,"networkIds":value.network_ids,"secretIds":value.secret_ids,"configIds":value.config_ids,"labels":value.labels,"createdAt":value.created_at.and_then(|t|chrono::DateTime::from_timestamp(t.seconds,t.nanos as u32)),"updatedAt":value.updated_at.and_then(|t|chrono::DateTime::from_timestamp(t.seconds,t.nanos as u32))})
}
macro_rules! reader {
    ($name:ident,$kind:expr) => {
        pub(super) async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<(Uuid, String)>, PathRejection>,
            headers: HeaderMap,
        ) -> IdentityHttpResult {
            read(state, principal, path, headers, $kind).await
        }
    };
}
reader!(inspect_node, "node");
reader!(inspect_service, "service");
reader!(config_data, "config");
async fn read(
    state: PlatformsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
    kind: &str,
) -> IdentityHttpResult {
    let Path((pid, id)) = identity_result(path.map_err(invalid_path), &headers)?;
    let platform = context(
        &state,
        principal,
        pid,
        PermissionLevel::Read,
        true,
        &headers,
    )
    .await?;
    identity_result(validation(resource_id(&id)), &headers)?;
    match kind {
        "node" => {
            required(
                state
                    .platforms
                    .get_swarm_node(pid, &id)
                    .await
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
                    .map_err(platform_error),
                &headers,
            )?;
        }
    }
    let runtime = identity_result(
        runtime_for(&state, pid)
            .await
            .map_err(|e| conflict(&e.to_string())),
        &headers,
    )?;
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let result=bounded(async {let client=client(&runtime);if kind == "config" { manager_identity(&runtime, &platform, &cancel).await?; } match kind {
        "service"=>{let service=client.inspect_service(&id,&cancel).await?;if service.id!=id {return Err(RuntimeCapabilityError::new(RuntimeErrorKind::Remote,"Docker returned a different Service.",false));}Ok(inspect_service_view(service))},
        "node"=>{let (node,running,desired)=client.inspect_node(&id,&cancel).await?;if node.id!=id {return Err(RuntimeCapabilityError::new(RuntimeErrorKind::Remote,"Docker returned a different Node.",false));}Ok(json!({"id":node.id,"versionIndex":node.version_index,"hostname":node.hostname,"role":node.role,"isLeader":node.is_leader,"reachability":node.reachability,"status":node.status,"statusMessage":node.status_message,"availability":node.availability,"engineVersion":node.engine_version,"operatingSystem":node.operating_system,"architecture":node.architecture,"address":node.address,"labels":node.labels,"runningTaskCount":running,"desiredTaskCount":desired,"createdAt":node.created_at,"updatedAt":node.updated_at}))},
        _=>Ok(json!({"content":client.config_data(&id,&cancel).await?}))
    }}).await;
    match result {
        Ok(value) => Ok(no_store(Json(value).into_response())),
        Err(error) => Ok(runtime_error_response(error, &headers)),
    }
}
