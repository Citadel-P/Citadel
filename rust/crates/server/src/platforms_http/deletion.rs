use super::*;
use citadel_adapters::platform_deletion::PostgresPlatformDeletionStore;
use citadel_platforms::deletion::{
    DeletePlatformsInput, PlatformDeletionError, PlatformDeletionStore,
};

pub(super) async fn delete(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<DeletePlatformsInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Json(mut input) = identity_result(input.map_err(invalid_json), &headers)?;
    identity_result(input.validate().map_err(deletion_error), &headers)?;
    let permissions = effective_permissions(&state, &principal, &input.ids, &headers).await?;
    if input.ids.iter().any(|id| {
        !resource_capabilities(permission_for(
            &permissions,
            *id,
            principal.is_administrator(),
        ))
        .can_execute
    }) {
        return identity_result(Err(IdentityError::Forbidden), &headers);
    }
    identity_result(
        PostgresPlatformDeletionStore::new(state.pool.clone())
            .delete(principal.actor_id, &input.ids)
            .await
            .map_err(deletion_error),
        &headers,
    )?;
    // No fallible I/O after commit, and no request cancellation interrupting cleanup.
    for id in input.ids {
        state.edge.disconnect_platform(id);
        if let Some(realtime) = &state.realtime {
            realtime.publish_resource_change("Platform", id, "deleted");
        }
    }
    Ok(no_store(StatusCode::OK.into_response()))
}

fn deletion_error(error: PlatformDeletionError) -> IdentityError {
    match error {
        PlatformDeletionError::Validation => IdentityError::Validation(error.to_string()),
        PlatformDeletionError::NotFound => IdentityError::NotFound,
        PlatformDeletionError::InUse => IdentityError::Conflict(error.to_string()),
        PlatformDeletionError::Storage(message) => IdentityError::Storage(message),
    }
}
