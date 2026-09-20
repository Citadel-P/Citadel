use super::*;

use citadel_platforms::containers::ContainerAction;

use crate::platforms_http::dto::DeleteContainerOptions;

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct DeleteInput {
    container_ids: Vec<String>,
    #[serde(flatten)]
    options: DeleteContainerOptions,
}

macro_rules! action_handler {
    ($(#[$name_attr:meta])* $name:ident, $action:ident) => {
        $(#[$name_attr])*
        pub(super) async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            headers: HeaderMap,
            body: Result<Json<Vec<String>>, JsonRejection>,
        ) -> IdentityHttpResult {
            let principal = identity_result(require_actor(principal), &headers)?;
            let Json(ids) = identity_result(body.map_err(invalid_json), &headers)?;
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
    request_body = ref("#/components/schemas/ContainerIdsInput"),
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
    request_body = ref("#/components/schemas/ContainerIdsInput"),
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
    request_body = ref("#/components/schemas/ContainerIdsInput"),
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
    request_body = ref("#/components/schemas/ContainerIdsInput"),
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
    request_body = ref("#/components/schemas/ContainerIdsInput"),
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
        ) -> IdentityHttpResult {
            let principal = identity_result(require_actor(principal), &headers)?;
            let Json(ids) = identity_result(body.map_err(invalid_json), &headers)?;
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
    request_body = ref("#/components/schemas/DeploymentIds"),
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
    request_body = ref("#/components/schemas/DeploymentIds"),
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
    request_body = ref("#/components/schemas/DeploymentIds"),
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
    request_body = ref("#/components/schemas/DeploymentIds"),
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
    request_body = ref("#/components/schemas/DeploymentIds"),
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
pub(super) async fn delete(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    body: Result<Json<DeleteInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Json(input) = identity_result(body.map_err(invalid_json), &headers)?;
    execute(
        state,
        principal,
        input.container_ids,
        ContainerAction::Delete(input.options.into()),
        &headers,
    )
    .await
}

async fn execute(
    state: PlatformsHttpState,
    principal: ActorPrincipal,
    ids: Vec<String>,
    action: ContainerAction,
    headers: &HeaderMap,
) -> IdentityHttpResult {
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
