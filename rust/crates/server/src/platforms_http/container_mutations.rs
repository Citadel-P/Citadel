use super::*;
use citadel_platforms::containers::{ContainerAction, DeleteContainerOptions};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DeleteInput {
    container_ids: Vec<String>,
    #[serde(flatten)]
    options: DeleteContainerOptions,
}

macro_rules! action_handler {
    ($name:ident, $action:ident) => {
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
action_handler!(start, Start);
action_handler!(stop, Stop);
action_handler!(restart, Restart);
action_handler!(pause, Pause);
action_handler!(unpause, Unpause);

macro_rules! deployment_action_handler {
    ($name:ident, $action:ident) => {
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
deployment_action_handler!(start_deployments, Start);
deployment_action_handler!(stop_deployments, Stop);
deployment_action_handler!(restart_deployments, Restart);
deployment_action_handler!(pause_deployments, Pause);
deployment_action_handler!(resume_deployments, Unpause);

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
        ContainerAction::Delete(input.options),
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
