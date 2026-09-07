use super::*;
use citadel_platforms::images::ImageInspectionPort;

pub(super) async fn exposed_ports(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, Uuid)>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path((platform_id, image_id)) = identity_result(path.map_err(invalid_path), &headers)?;
    if platform_id.is_nil() || image_id.is_nil() {
        identity_result::<()>(
            Err(IdentityError::Validation(
                "Platform and Image IDs must not be empty.".into(),
            )),
            &headers,
        )?;
    }
    authorize_platform(&state, &principal, platform_id, &headers).await?;
    // The form passes a Citadel UUID, never a Docker content ID. Resolve it on
    // the requested Platform and retain node identity for projected Swarm images.
    let image: Option<(String, Option<String>, bool)> = identity_result(
        sqlx::query_as("SELECT dockerimageid,NULL::text,false FROM images WHERE platformid=$1 AND id=$2 UNION ALL SELECT dockerimageid,dockernodeid,isstale FROM swarmnodeimageprojections WHERE platformid=$1 AND id=$2")
            .bind(platform_id).bind(image_id).fetch_optional(&state.pool).await
            .map_err(|error| IdentityError::Storage(error.to_string())),
        &headers,
    )?;
    let (docker_id, node_id, stale) =
        identity_result(image.ok_or(IdentityError::NotFound), &headers)?;
    if stale {
        return Ok(runtime_error_response(
            RuntimeCapabilityError::new(
                RuntimeErrorKind::Conflict,
                "The Image projection is stale.",
                false,
            ),
            &headers,
        ));
    }
    let runtime = match runtime_for_node(&state, platform_id, node_id.as_deref()).await {
        Ok(runtime) => runtime,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let result = match runtime {
        RuntimeRef::Local(runtime) => {
            ImageInspectionPort::exposed_ports(runtime, &docker_id, &cancel).await
        }
        RuntimeRef::Agent(runtime) => {
            ImageInspectionPort::exposed_ports(runtime, &docker_id, &cancel).await
        }
        RuntimeRef::Edge(runtime) => runtime.exposed_ports(&docker_id, &cancel).await,
    };
    match result {
        Ok(ports) => Ok(no_store(
            Json(serde_json::json!({"ports":ports})).into_response(),
        )),
        Err(error) => Ok(runtime_error_response(error, &headers)),
    }
}

pub(super) async fn inspect(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    selector: Result<Query<DockerNodeSelector>, QueryRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path((platform_id, image_id)) = identity_result(path.map_err(invalid_path), &headers)?;
    let Query(mut selector) = identity_result(selector.map_err(invalid_query), &headers)?;
    identity_result(validate_docker_resource_id(&image_id), &headers)?;
    if let Some(node) = &selector.docker_node_id {
        identity_result(validate_docker_resource_id(node), &headers)?;
    }
    let capabilities =
        image_capabilities(authorize_platform(&state, &principal, platform_id, &headers).await?);
    if selector.docker_node_id.is_none() {
        let platform = required(
            state
                .platforms
                .get_platform(platform_id)
                .await
                .map_err(platform_error),
            &headers,
        )?;
        if platform.platform_type == "DockerSwarm" {
            selector.docker_node_id = platform
                .platform_descriptor
                .get("nodeID")
                .and_then(serde_json::Value::as_str)
                .filter(|id| !id.is_empty())
                .map(str::to_owned);
            if selector.docker_node_id.is_none() {
                return Ok(runtime_error_response(
                    RuntimeCapabilityError::new(
                        RuntimeErrorKind::Unavailable,
                        "The connected Swarm Node identity is unavailable.",
                        false,
                    ),
                    &headers,
                ));
            }
        }
    }
    let runtime =
        match runtime_for_node(&state, platform_id, selector.docker_node_id.as_deref()).await {
            Ok(runtime) => runtime,
            Err(error) => return Ok(runtime_error_response(error, &headers)),
        };
    let cancellation = CancellationToken::new();
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let image = match &runtime {
        RuntimeRef::Local(runtime) => {
            ImageInspectionPort::inspect_image(*runtime, &image_id, &cancellation).await
        }
        RuntimeRef::Agent(runtime) => {
            ImageInspectionPort::inspect_image(*runtime, &image_id, &cancellation).await
        }
        RuntimeRef::Edge(runtime) => {
            ImageInspectionPort::inspect_image(runtime, &image_id, &cancellation).await
        }
    };
    let mut image = match image {
        Ok(image) => image,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    image.capabilities = Some(capabilities);
    if selector.docker_node_id.is_none() {
        let registry_id: Option<Uuid> = identity_result(
            sqlx::query_scalar::<_, Option<Uuid>>(
                "SELECT registryid FROM images WHERE platformid=$1 AND dockerimageid=$2",
            )
            .bind(platform_id)
            .bind(&image.id)
            .fetch_optional(&state.pool)
            .await
            .map(Option::flatten)
            .map_err(|error| IdentityError::Storage(error.to_string())),
            &headers,
        )?;
        if let Some(id) = registry_id {
            let registry = identity_result(
                state
                    .resource_metadata
                    .get_registry(id)
                    .await
                    .map_err(|error| IdentityError::Storage(error.to_string())),
                &headers,
            )?;
            image.registry = Some(identity_result(
                serde_json::to_value(registry)
                    .map_err(|error| IdentityError::Storage(error.to_string())),
                &headers,
            )?);
        }
    }
    image.docker_node_id = selector.docker_node_id;
    Ok(no_store(Json(image).into_response()))
}
