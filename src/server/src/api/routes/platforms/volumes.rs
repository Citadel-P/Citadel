use super::*;

#[utoipa::path(
    get,
    path = "/api/v1/volumes/{platformId}",
    operation_id = "listVolumes",
    tag = "Volumes",
    summary = "List Platform Volumes",
    responses(
        (status = 200, description = "Success", body = VolumesResponse, content_type = "application/json"),
        crate::openapi::errors::CreateErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("Dangling" = Option<bool>, Query), ("Driver" = Option<String>, Query), ("Name" = Option<String>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn list_volumes(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath(platform_id): ApiPath<Uuid>,
    filters: Result<Query<VolumeFilters>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let Query(filters) = api_result(filters.map_err(invalid_query), &headers)?;
    validate_filter_lengths(
        [filters.driver.as_deref(), filters.name.as_deref()],
        &headers,
    )?;
    let platform_capabilities =
        authorize_platform(&state, &principal, platform_id, &headers).await?;
    let capabilities = volume_capabilities(
        &state,
        &principal,
        platform_id,
        platform_capabilities,
        &headers,
    )
    .await?;
    let cancellation = CancellationToken::new();
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let values = async {
        state
            .runtime
            .inventory(platform_id, &cancellation)
            .await?
            .list_volumes(&cancellation)
            .await
    }
    .await;
    let values = match values {
        Ok(values) => values,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let mut volumes: Vec<_> = api_result(
        values
            .into_iter()
            .filter(|volume| volume_matches(volume, &filters))
            .map(|volume| map_volume(volume, capabilities))
            .collect::<Result<_, _>>()
            .map_err(ApiError::internal),
        &headers,
    )?;
    let node_volumes = api_result(
        state
            .platforms
            .list_node_volumes(platform_id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    volumes.extend(api_result(
        node_volumes
            .into_iter()
            .filter(|volume| volume_matches(&volume.resource, &filters))
            .map(|volume| map_node_volume(volume, capabilities))
            .collect::<Result<Vec<_>, _>>()
            .map_err(ApiError::internal),
        &headers,
    )?);
    attach_volume_coverage(&state, &principal, platform_id, &mut volumes, &headers).await?;
    Ok(no_store(
        Json(VolumesResponse {
            volumes,
            capabilities: ResourceCapabilitiesView {
                can_read: platform_capabilities.can_read,
                can_write: platform_capabilities.can_write,
                can_execute: platform_capabilities.can_execute,
            },
        })
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/volumes/{platformId}/{name}",
    operation_id = "inspectVolume",
    tag = "Volumes",
    summary = "Inspect a Platform Volume",
    responses(
        (status = 200, description = "Success", body = VolumeView, content_type = "application/json"),
        crate::openapi::errors::CreateErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("name" = String, Path), ("dockerNodeId" = Option<String>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn get_volume(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath((platform_id, name)): ApiPath<(Uuid, String)>,
    selector: Result<Query<DockerNodeSelector>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let Query(selector) = api_result(selector.map_err(invalid_query), &headers)?;
    api_result(validate_docker_resource_id(&name), &headers)?;
    if let Some(node_id) = selector.docker_node_id.as_deref() {
        api_result(validate_docker_resource_id(node_id), &headers)?;
    }
    let platform_capabilities =
        authorize_platform(&state, &principal, platform_id, &headers).await?;
    let capabilities = volume_capabilities(
        &state,
        &principal,
        platform_id,
        platform_capabilities,
        &headers,
    )
    .await?;
    let cancellation = CancellationToken::new();
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let value = async {
        state
            .runtime
            .volumes(
                platform_id,
                selector.docker_node_id.as_deref(),
                &cancellation,
            )
            .await?
            .inspect_volume(&name, &cancellation)
            .await
    }
    .await;
    let mut volume = match value {
        Ok(volume) => api_result(
            map_volume(volume, capabilities).map_err(ApiError::internal),
            &headers,
        )?,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    volume.docker_node_id = selector.docker_node_id;
    attach_volume_coverage(
        &state,
        &principal,
        platform_id,
        std::slice::from_mut(&mut volume),
        &headers,
    )
    .await?;
    Ok(no_store(Json(volume).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/volumes",
    operation_id = "createVolume",
    tag = "Volumes",
    summary = "Create a Volume",
    request_body = CreateVolumeInput,
    responses(
        (status = 200, description = "Success", body = VolumeView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn create_volume(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    headers: HeaderMap,
    input: Result<Json<CreateVolumeInput>, JsonRejection>,
) -> HttpResult {
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    api_result(validate_volume_input(&input.volume), &headers)?;
    let platform_capabilities = authorize_platform_level(
        &state,
        &principal,
        input.platform_id,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    let capabilities = volume_capabilities(
        &state,
        &principal,
        input.platform_id,
        platform_capabilities,
        &headers,
    )
    .await?;
    let cancellation = CancellationToken::new();
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let result = async {
        state
            .runtime
            .volumes(input.platform_id, None, &cancellation)
            .await?
            .create_volume(&input.volume.into(), &cancellation)
            .await
    }
    .await;
    let volume = match result {
        Ok(volume) => volume,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    publish_runtime_change(&state, input.platform_id, "volume", "create", &volume.name);
    Ok(no_store(
        Json(api_result(
            map_volume(volume, capabilities).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/volumes",
    operation_id = "deleteVolumes",
    tag = "Volumes",
    summary = "Delete Volumes",
    request_body = DeleteVolumesInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn delete_volumes(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    headers: HeaderMap,
    input: Result<Json<DeleteVolumesInput>, JsonRejection>,
) -> HttpResult {
    let Json(mut input) = api_result(input.map_err(invalid_json), &headers)?;
    api_result(
        validate_resource_ids(&mut input.names, 100, "Volume"),
        &headers,
    )?;
    authorize_platform_level(
        &state,
        &principal,
        input.platform_id,
        PermissionLevel::Execute,
        &headers,
    )
    .await?;
    if api_result(platform_is_swarm(&state, input.platform_id).await, &headers)? {
        return Ok(conflict_response(
            "Node-local Volume deletion requires an explicit Node target and is not available."
                .to_owned(),
            &headers,
        ));
    }
    let cancellation = CancellationToken::new();
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let result = async {
        let runtime = state
            .runtime
            .volumes(input.platform_id, None, &cancellation)
            .await?;
        citadel_platforms::resource_mutations::delete_volumes(
            runtime.as_ref(),
            &input.names,
            input.force.unwrap_or(false),
            &cancellation,
            |name| publish_runtime_change(&state, input.platform_id, "volume", "remove", name),
        )
        .await
    }
    .await;
    match result {
        Ok(()) => Ok(no_store(StatusCode::NO_CONTENT.into_response())),
        Err(error) => Ok(runtime_error_response(error, &headers)),
    }
}

pub(super) async fn authorize(
    state: &PlatformsHttpState,
    principal: &ActorPrincipal,
    platform: Uuid,
    specific: SpecificPermission,
    headers: &HeaderMap,
) -> HttpResult<()> {
    authorize_platform_level(state, principal, platform, PermissionLevel::Read, headers).await?;
    if !principal.is_administrator() {
        api_result(
            state
                .identity
                .authorize_resource(
                    principal,
                    ResourceType::Volume,
                    platform,
                    PermissionLevel::Read,
                    Some(specific),
                )
                .await,
            headers,
        )?;
    }
    Ok(())
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/volumes/{name}/files",
    operation_id = "listVolumeDirectory",
    tag = "Platforms",
    summary = "Browse a Volume directory",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::schema_models::platforms::VolumeDirectorySchema, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("name" = String, Path), ("path" = Option<String>, Query), ("dockerNodeId" = Option<String>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn list(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath((platform, name)): ApiPath<(Uuid, String)>,
    query: Result<Query<ContentQuery>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
    authorize(
        &state,
        &principal,
        platform,
        SpecificPermission::Browse,
        &headers,
    )
    .await?;
    let Query(query) = api_result(query.map_err(invalid_query), &headers)?;
    let path = match normalize_path(query.path.as_deref()) {
        Ok(path) => path,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let cancellation = CancellationToken::new();
    let _guard = cancellation.clone().drop_guard();
    match state
        .volume_content
        .list(
            platform,
            &name,
            path,
            query.docker_node_id.as_deref(),
            &cancellation,
        )
        .await
    {
        Ok(listing) => Ok(no_store(Json(listing).into_response())),
        Err(error) => Ok(runtime_error_response(error, &headers)),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/volumes/{name}/files/download",
    operation_id = "downloadVolumePath",
    tag = "Platforms",
    summary = "Download a Volume file or directory",
    responses(
        (status = 200, description = "Success"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("name" = String, Path), ("path" = Option<String>, Query), ("dockerNodeId" = Option<String>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn download(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath((platform, name)): ApiPath<(Uuid, String)>,
    query: Result<Query<ContentQuery>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
    authorize(
        &state,
        &principal,
        platform,
        SpecificPermission::Download,
        &headers,
    )
    .await?;
    let Query(query) = api_result(query.map_err(invalid_query), &headers)?;
    let path = match normalize_path(query.path.as_deref()) {
        Ok(path) => path,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let cancellation = CancellationToken::new();
    let guard = cancellation.clone().drop_guard();
    let download = match state
        .volume_content
        .download(
            platform,
            &name,
            path,
            query.docker_node_id.as_deref(),
            &cancellation,
        )
        .await
    {
        Ok(download) => download,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let disposition = format!(
        "attachment; filename=\"download{}\"; filename*=UTF-8''{}",
        if download.directory { ".tar" } else { "" },
        urlencoding::encode(&download.filename)
    );
    let directory = download.directory;
    let filename = download.filename;
    let path = path.to_owned();
    let mut input = download.stream;
    let stream: futures_util::stream::BoxStream<'static, Result<Vec<u8>, std::io::Error>> =
        Box::pin(async_stream::try_stream! {
            let _guard = guard;
            while let Some(chunk) = input.next().await { yield chunk?; }
            let activity = state.volume_activity.clone();
            let details = citadel_activities::VolumeContentDownloaded { volume_name: name, path, is_directory: directory, file_name: filename };
            if !matches!(tokio::time::timeout(std::time::Duration::from_secs(5), activity.record_volume_download(principal.actor_id, platform, details)).await, Ok(Ok(()))) {
                tracing::warn!("Could not record completed Volume download activity.");
            }
        });
    let mut response = axum::response::Response::new(axum::body::Body::from_stream(stream));
    response.headers_mut().insert(
        "Content-Type",
        HeaderValue::from_static(if directory {
            "application/x-tar"
        } else {
            "application/octet-stream"
        }),
    );
    response.headers_mut().insert(
        "Content-Disposition",
        HeaderValue::from_str(&disposition).expect("percent-encoded download filename"),
    );
    response.headers_mut().insert(
        "X-Content-Type-Options",
        HeaderValue::from_static("nosniff"),
    );
    // No guessed Content-Length: the underlying Volume is live and may change.
    Ok(no_store(response))
}

pub(super) async fn attach_volume_coverage(
    state: &PlatformsHttpState,
    principal: &ActorPrincipal,
    platform_id: Uuid,
    volumes: &mut [VolumeView],
    headers: &HeaderMap,
) -> HttpResult<()> {
    api_result(
        crate::api::resources::backups::coverage::attach_volume_coverage(
            state.volume_coverage.as_ref(),
            state.identity.as_ref(),
            principal,
            platform_id,
            volumes,
            match &state.backup_coverage {
                Some(service) => service
                    .backups
                    .scheduling_available()
                    .await
                    .unwrap_or(false),
                None => false,
            },
        )
        .await
        .map_err(ApiError::internal),
        headers,
    )
}
