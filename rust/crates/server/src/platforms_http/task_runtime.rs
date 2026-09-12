use super::*;
use citadel_platforms::{SwarmTaskRuntimePort, containers::ContainerInspectionPort};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskTerminalView {
    docker_container_id: String,
}

macro_rules! task_reader {
    ($(#[$name_attr:meta])* $name:ident, $permission:ident) => {
        $(#[$name_attr])*
        pub(super) async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<(Uuid, String)>, PathRejection>,
            headers: HeaderMap,
        ) -> IdentityHttpResult {
            read(
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
    summary = "Inspect the current Task container",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ContainerInspectView"), content_type = "application/json"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    inspect,
    Inspect
);
task_reader!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/tasks/{resourceId}/terminal",
    operation_id = "getSwarmTaskTerminalTarget",
    summary = "Resolve the current Task terminal target",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/SwarmTaskTerminalView"), content_type = "application/json"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    terminal,
    Terminal
);

async fn read(
    state: PlatformsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
    specific: SpecificPermission,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path((platform, id)) = identity_result(path.map_err(invalid_path), &headers)?;
    identity_result(validate_docker_resource_id(&id), &headers)?;
    if platform.is_nil() || id.len() > 255 {
        return identity_result(
            Err(IdentityError::Validation("Invalid Task identity.".into())),
            &headers,
        );
    }
    if !principal.is_administrator() {
        identity_result(
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
    logs::swarm_platform(&state, platform, &headers).await?;
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
        let live = match runtime_for(&state, platform).await? {
            RuntimeRef::Local(r) => r.inspect_task(&id, &cancel).await,
            RuntimeRef::Agent(r) => r.inspect_task(&id, &cancel).await,
            RuntimeRef::Edge(r) => r.inspect_task(&id, &cancel).await,
        }?;
        let docker_id = citadel_platforms::validate_running_task(&projection, &live)?;
        if specific == SpecificPermission::Terminal {
            return Ok(Json(TaskTerminalView {
                docker_container_id: docker_id.into(),
            })
            .into_response());
        }
        // Use the existing redacted Container inspection contract, not raw Task JSON.
        let inspection = match runtime_for_node(&state, platform, Some(&live.node_id)).await? {
            RuntimeRef::Local(r) => r.inspection(docker_id, &cancel).await,
            RuntimeRef::Agent(r) => r.inspection(docker_id, &cancel).await,
            RuntimeRef::Edge(r) => r.inspection(docker_id, &cancel).await,
        }?;
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
