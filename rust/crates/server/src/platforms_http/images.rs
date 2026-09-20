use super::*;

use citadel_platforms::images::ImageInspectionPort;

static IMAGE_DELETE_SLOTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct DeleteImagesInput {
    platform_id: Uuid,
    ids: Vec<String>,
    #[serde(default)]
    force: bool,
    #[serde(default)]
    no_prune: bool,
}

#[utoipa::path(
    delete,
    path = "/api/v1/images",
    operation_id = "deleteImages",
    tag = "Images",
    summary = "Delete Images",
    request_body = DeleteImagesInput,
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/DeleteImageResult"), content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn delete(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<DeleteImagesInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Json(mut input) = identity_result(input.map_err(invalid_json), &headers)?;
    identity_result(
        validate_resource_ids(&mut input.ids, 100, "Image"),
        &headers,
    )?;
    if input.platform_id.is_nil()
        || input.ids.iter().any(|id| {
            let hash = id.strip_prefix("sha256:").unwrap_or(id);
            hash.len() != 64 || !hash.bytes().all(|c| c.is_ascii_hexdigit())
        })
    {
        return identity_result(
            Err(IdentityError::Validation(
                "A Platform and full Docker Image IDs are required.".into(),
            )),
            &headers,
        );
    }
    for id in &mut input.ids {
        *id = format!(
            "sha256:{}",
            id.strip_prefix("sha256:")
                .unwrap_or(id)
                .to_ascii_lowercase()
        );
    }
    input.ids.sort();
    input.ids.dedup();
    authorize_platform_level(
        &state,
        &principal,
        input.platform_id,
        PermissionLevel::Execute,
        &headers,
    )
    .await?;
    if identity_result(platform_is_swarm(&state, input.platform_id).await, &headers)? {
        return Ok(conflict_response(
            "Node-local Image deletion requires an explicit Node target and is not available."
                .into(),
            &headers,
        ));
    }
    // Keep bounded cleanup running if the browser disconnects after Docker accepts deletion.
    let Ok(permit) = IMAGE_DELETE_SLOTS.try_acquire() else {
        return Ok(runtime_error_response(
            RuntimeCapabilityError::new(
                RuntimeErrorKind::ResourceExhausted,
                "Image deletion capacity is busy. Try again later.",
                false,
            ),
            &headers,
        ));
    };
    let task = tokio::spawn(async move {
        let _permit = permit;
        let runtime = runtime_for(&state, input.platform_id).await?;
        let (mutation, inventory): (
            &dyn citadel_platforms::images::ImageDeletionPort,
            &dyn PlatformInventoryPort,
        ) = match &runtime {
            RuntimeRef::Local(runtime) => (*runtime, *runtime),
            RuntimeRef::Agent(runtime) => (runtime, runtime),
            RuntimeRef::Edge(runtime) => (runtime, runtime),
        };
        let result = citadel_adapters::image_deletion::delete(
            &state.pool,
            input.platform_id,
            &input.ids,
            input.force,
            input.no_prune,
            mutation,
            inventory,
        )
        .await;
        for id in &input.ids {
            publish_runtime_change(&state, input.platform_id, "image", "update", id);
        }
        result
    });
    let result = identity_result(
        task.await
            .map_err(|error| IdentityError::Storage(error.to_string())),
        &headers,
    )?;
    match result {
        Ok(items) => Ok(no_store(Json(serde_json::json!({"items":items.into_iter().map(|result| serde_json::json!({"result":result})).collect::<Vec<_>>()})).into_response())),
        Err(error) => Ok(runtime_error_response(error, &headers)),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/images/{platformId}/{imageId}/_ports",
    operation_id = "getExposedPorts",
    tag = "Images",
    summary = "Read exposed ports using the Citadel Image ID",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ExposedPortsResult"), content_type = "application/json"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("imageId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
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
        RuntimeRef::Agent(ref runtime) => {
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

#[utoipa::path(
    get,
    path = "/api/v1/images/{platformId}/{imageId}",
    operation_id = "inspectImage",
    tag = "Images",
    summary = "Inspect an Image on its owning Docker node",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/InspectImageView"), content_type = "application/json"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("imageId" = String, Path), ("dockerNodeId" = Option<String>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
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
                .map(|value| value.map(crate::platforms_http::views::PlatformView::from))
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
            ImageInspectionPort::inspect_image(runtime, &image_id, &cancellation).await
        }
        RuntimeRef::Edge(runtime) => {
            ImageInspectionPort::inspect_image(runtime, &image_id, &cancellation).await
        }
    };
    let mut image = match image {
        Ok(image) => image,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
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
                    .registries
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
    let image = super::views::ImageInspectionView {
        image,
        capabilities: Some(capabilities),
    };
    Ok(no_store(Json(image).into_response()))
}
