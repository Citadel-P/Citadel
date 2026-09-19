use super::*;
use citadel_platforms::{
    SwarmTaskRuntimePort,
    logs::{LogReadPort, LogResource},
};
use citadel_swarm_services::SwarmServiceRepository;

#[derive(Deserialize)]
pub(super) struct Tail {
    #[serde(default = "default_tail", alias = "Tail")]
    tail: u16,
}
const fn default_tail() -> u16 {
    100
}

fn tail(
    query: Result<Query<Tail>, QueryRejection>,
    headers: &HeaderMap,
) -> IdentityHttpResult<u16> {
    let Query(query) = identity_result(query.map_err(invalid_query), headers)?;
    identity_result(
        if (1..=200).contains(&query.tail) {
            Ok(query.tail)
        } else {
            Err(IdentityError::Validation(
                "Tail must be between 1 and 200.".into(),
            ))
        },
        headers,
    )
}

async fn authorize_logs(
    state: &PlatformsHttpState,
    principal: &ActorPrincipal,
    kind: ResourceType,
    id: Uuid,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    if principal.is_administrator() {
        return Ok(());
    }
    match kind {
        ResourceType::Stack => {
            return identity_result(
                state
                    .identity
                    .require_resource::<citadel_stacks::permissions::ViewStackLogs>(principal, id)
                    .await,
                headers,
            );
        }
        ResourceType::SwarmService => {
            return identity_result(
                state
                    .identity
                    .require_resource::<citadel_swarm_services::permissions::ViewSwarmServiceLogs>(
                        principal, id,
                    )
                    .await,
                headers,
            );
        }
        _ => {}
    }
    identity_result(
        state
            .identity
            .authorize_resource(
                principal,
                kind,
                id,
                PermissionLevel::Read,
                Some(SpecificPermission::Logs),
            )
            .await,
        headers,
    )
}

pub(super) async fn swarm_platform(
    state: &PlatformsHttpState,
    platform_id: Uuid,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    let platform = required(
        state
            .platforms
            .get_platform(platform_id)
            .await
            .map_err(platform_error),
        headers,
    )?;
    if platform.platform_type != "DockerSwarm" {
        return identity_result(
            Err(IdentityError::Validation(
                "Platform must be Docker Swarm.".into(),
            )),
            headers,
        );
    }
    if platform.status != "Online" {
        return identity_result(
            Err(IdentityError::Conflict(
                "Platform is disconnected or unavailable.".into(),
            )),
            headers,
        );
    }
    Ok(())
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/services/{resourceId}/logs",
    operation_id = "getSwarmServiceLogs",
    tag = "Platforms",
    summary = "Read bounded Service logs",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/SwarmLogsView"), content_type = "application/json"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path), ("tail" = Option<i32>, Query, minimum = 1, maximum = 200, extensions(("x-citadel-default" = json!(100))))),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn service(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    query: Result<Query<Tail>, QueryRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path((platform, id)) = identity_result(path.map_err(invalid_path), &headers)?;
    let tail = tail(query, &headers)?;
    identity_result(validate_docker_resource_id(&id), &headers)?;
    authorize_logs(
        &state,
        &principal,
        ResourceType::Platform,
        platform,
        &headers,
    )
    .await?;
    service_logs(&state, platform, &id, tail, &headers).await
}

async fn service_logs(
    state: &PlatformsHttpState,
    platform: Uuid,
    id: &str,
    tail: u16,
    headers: &HeaderMap,
) -> IdentityHttpResult {
    swarm_platform(state, platform, headers).await?;
    required(
        state
            .platforms
            .get_swarm_service(platform, id)
            .await
            .map_err(platform_error),
        headers,
    )?;
    read(
        state,
        platform,
        None,
        LogResource::Service(id),
        tail,
        headers,
    )
    .await
}

#[utoipa::path(
    get,
    path = "/api/v1/swarmServices/{id}/logs",
    operation_id = "getManagedSwarmServiceLogs",
    tag = "SwarmServices",
    summary = "Read managed Service logs",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/SwarmLogsView"), content_type = "application/json"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("id" = uuid::Uuid, Path), ("tail" = Option<i32>, Query, minimum = 1, maximum = 200, extensions(("x-citadel-default" = json!(100))))),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn managed_service(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    query: Result<Query<Tail>, QueryRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    let tail = tail(query, &headers)?;
    authorize_logs(&state, &principal, ResourceType::SwarmService, id, &headers).await?;
    let store = citadel_adapters::postgres::swarm_services::PostgresSwarmServiceRepository::new(
        state.pool.clone(),
    );
    let service = identity_result(
        store
            .get_authorized(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(crate::api::swarm_services::service_error),
        &headers,
    )?;
    let Some(docker_id) = service.docker_service_id.as_ref() else {
        return identity_result(
            Err(IdentityError::Conflict(
                "The Service has not been applied.".into(),
            )),
            &headers,
        );
    };
    let permissions =
        effective_permissions(&state, &principal, &[service.platform_id], &headers).await?;
    if permission_for(
        &permissions,
        service.platform_id,
        principal.is_administrator(),
    )
    .level_mask
        & ALL_LEVELS
        == 0
    {
        return identity_result(Err(IdentityError::NotFound), &headers);
    }
    service_logs(&state, service.platform_id, docker_id, tail, &headers).await
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/tasks/{resourceId}/logs",
    operation_id = "getSwarmTaskLogs",
    tag = "Platforms",
    summary = "Read current Task logs on its owning node",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/SwarmLogsView"), content_type = "application/json"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path), ("tail" = Option<i32>, Query, minimum = 1, maximum = 200, extensions(("x-citadel-default" = json!(100))))),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn task(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    query: Result<Query<Tail>, QueryRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path((platform, id)) = identity_result(path.map_err(invalid_path), &headers)?;
    let tail = tail(query, &headers)?;
    identity_result(validate_docker_resource_id(&id), &headers)?;
    authorize_logs(
        &state,
        &principal,
        ResourceType::Platform,
        platform,
        &headers,
    )
    .await?;
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
    let live = match runtime_for(&state, platform).await {
        Ok(RuntimeRef::Local(r)) => r.inspect_task(&id, &cancel).await,
        Ok(RuntimeRef::Agent(r)) => r.inspect_task(&id, &cancel).await,
        Ok(RuntimeRef::Edge(r)) => r.inspect_task(&id, &cancel).await,
        Err(error) => Err(error),
    };
    let live = match live {
        Ok(live) => live,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let docker_id = match citadel_platforms::validate_running_task(&projection, &live) {
        Ok(id) => id,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    read(
        &state,
        platform,
        Some(&live.node_id),
        LogResource::Container(docker_id),
        tail,
        &headers,
    )
    .await
}

async fn read(
    state: &PlatformsHttpState,
    platform: Uuid,
    node: Option<&str>,
    resource: LogResource<'_>,
    tail: u16,
    headers: &HeaderMap,
) -> IdentityHttpResult {
    let cancel = CancellationToken::new();
    let result = match runtime_for_node(state, platform, node).await {
        Ok(RuntimeRef::Local(r)) => r.read_logs(resource, tail, &cancel).await,
        Ok(RuntimeRef::Agent(r)) => r.read_logs(resource, tail, &cancel).await,
        Ok(RuntimeRef::Edge(r)) => r.read_logs(resource, tail, &cancel).await,
        Err(error) => Err(error),
    };
    match result {
        Ok(value) => Ok(no_store(Json(value).into_response())),
        Err(error) => Ok(runtime_error_response(error, headers)),
    }
}
