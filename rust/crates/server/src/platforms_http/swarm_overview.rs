use super::*;

pub(super) async fn get(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    if id.is_nil() {
        return identity_result(
            Err(IdentityError::Validation("Invalid Platform id.".into())),
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
    if platform.platform_type != "DockerSwarm" {
        return identity_result(
            Err(IdentityError::Validation(
                "Platform must be Docker Swarm.".into(),
            )),
            &headers,
        );
    }
    let mut summary = identity_result(
        state
            .platforms
            .swarm_summary(id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    if summary.node_count == 0 && platform.status == "Online" {
        match edge::initialize_swarm(&state, &platform).await {
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
        summary = identity_result(
            state
                .platforms
                .swarm_summary(id)
                .await
                .map_err(platform_error),
            &headers,
        )?;
    }
    let descriptor = &platform.platform_descriptor;
    let control = descriptor
        .get("controlAvailable")
        .or_else(|| descriptor.get("ControlAvailable"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let error = descriptor
        .get("error")
        .or_else(|| descriptor.get("Error"))
        .and_then(serde_json::Value::as_str);
    Ok(no_store(
        Json(summary.into_view(
            id,
            platform.status == "Online",
            control,
            error,
            capabilities,
        ))
        .into_response(),
    ))
}
