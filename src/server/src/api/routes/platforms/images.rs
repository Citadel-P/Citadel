use super::*;

#[utoipa::path(
    get,
    path = "/api/v1/images/{platformId}",
    operation_id = "listImages",
    tag = "Images",
    summary = "List Platform Images",
    responses(
        (status = 200, description = "Success", body = ImagesResponse, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn list_images(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath(platform_id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    let platform_capabilities =
        authorize_platform(&state, &principal, platform_id, &headers).await?;
    let capabilities = image_capabilities(platform_capabilities);
    let inventory = api_result(
        state
            .platforms
            .list_images(platform_id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    let response = match crate::api::resources::platforms::image_views::image_list_view(
        &principal,
        platform_id,
        inventory,
        capabilities,
        state.image_store.as_ref(),
        state.image_updates.as_ref(),
        state.registries.as_ref(),
    )
    .await
    {
        Ok(response) => response,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    Ok(no_store(Json(response).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/images/pull",
    operation_id = "pullImage",
    tag = "Images",
    summary = "Pull a Docker image with progress",
    request_body = PullImageInput,
    responses(
        (status = 200, description = "Success", body = Vec<crate::api::resources::schema_models::platforms::PullImageStreamItemSchema>, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn pull(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    headers: HeaderMap,
    input: Result<Json<PullImageInput>, JsonRejection>,
) -> HttpResult {
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let mut input: citadel_platforms::image_pull::PullImageInput = input.into();
    if input.platform_id.is_nil()
        || input.registry_id.is_some_and(|id| id.is_nil())
        || input.image_tag.trim().is_empty()
        || input.image_tag.len() > 2048
        || input.image_tag.chars().any(char::is_control)
    {
        return api_result(
            Err(ApiError::Validation(
                "A Platform and valid image reference are required.".into(),
            )),
            &headers,
        );
    }
    let capabilities = authorize_platform_level(
        &state,
        &principal,
        input.platform_id,
        PermissionLevel::Read,
        &headers,
    )
    .await?;
    if !capabilities.can_pull {
        return api_result(Err(ApiError::Forbidden), &headers);
    }
    if let Some(registry_id) = input.registry_id {
        api_result(
            state
                .identity
                .authorize_resource(
                    &principal,
                    ResourceType::Registry,
                    registry_id,
                    PermissionLevel::Read,
                    None,
                )
                .await,
            &headers,
        )?;
    }
    if let Some(source) = &input.source_image_id {
        api_result(
            crate::api::routes::image_management::normalize_ids(vec![source.clone()]),
            &headers,
        )?;
        if api_result(platform_is_swarm(&state, input.platform_id).await, &headers)? {
            return api_result(
                Err(ApiError::Conflict(
                    "Contextual pulls are unavailable for node-local images.".into(),
                )),
                &headers,
            );
        }
    }
    let automatic_registry = input.registry_id.is_none();
    if automatic_registry {
        let registries = api_result(
            state
                .registries
                .list_registries(principal.actor_id, principal.is_administrator())
                .await
                .map_err(ApiError::internal),
            &headers,
        )?;
        let preferred = if let Some(source) = &input.source_image_id {
            api_result(
                state
                    .platforms
                    .image_registry_id(input.platform_id, source)
                    .await
                    .map_err(platform_error),
                &headers,
            )?
        } else {
            None
        };
        input.registry_id = match citadel_platforms::image_pull::select_pull_registry(
            &input.image_tag,
            preferred,
            registries
                .iter()
                .filter(|r| r.status != citadel_registries::RegistryStatus::Disabled)
                .map(|r| (r.id, r.registry_host.as_str())),
        ) {
            Ok(registry) => registry,
            Err(error) => return Ok(runtime_error_response(error, &headers)),
        };
        if let Some(registry_id) = input.registry_id {
            api_result(
                state
                    .identity
                    .authorize_resource(
                        &principal,
                        ResourceType::Registry,
                        registry_id,
                        PermissionLevel::Read,
                        None,
                    )
                    .await,
                &headers,
            )?;
        }
    }
    let permit = api_result(
        PULL_SLOTS
            .try_acquire()
            .map_err(|_| ApiError::Conflict("Image pull capacity is busy.".into())),
        &headers,
    )?;
    let requested_reference = if input.source_image_id.is_some() || automatic_registry {
        citadel_platforms::image_updates::canonical_reference(&input.image_tag)
    } else {
        input.image_tag.clone()
    };
    let (image, auth) = match state
        .image_store
        .prepare_pull(input.registry_id, &requested_reference)
        .await
    {
        Ok(value) => value,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    if automatic_registry
        && citadel_platforms::image_updates::canonical_reference(&image) != requested_reference
    {
        return api_result(
            Err(ApiError::Conflict(
                "Registry configuration changed the image reference. Refresh before pulling."
                    .into(),
            )),
            &headers,
        );
    }
    let Some(task) = state.tasks.reserve() else {
        return Ok(runtime_error_response(
            RuntimeCapabilityError::new(
                RuntimeErrorKind::ResourceExhausted,
                "Platform operations are shutting down.",
                false,
            ),
            &headers,
        ));
    };
    let cancel = state.tasks.cancellation();
    let guard = cancel.clone().drop_guard();
    let (sender, receiver) = tokio::sync::mpsc::channel(16);
    let (completed, completion) = tokio::sync::oneshot::channel();
    task.spawn(async move {
        let _permit = permit;
        let result=tokio::time::timeout(std::time::Duration::from_secs(600),async {
            let runtime = state.runtime.image_mutations(input.platform_id, &cancel).await?;
            citadel_platforms::image_mutations::pull_and_persist(
                state.image_store.as_ref(), runtime.as_ref(), &input, &image, auth.as_deref().map(String::as_str), &sender, &cancel,
                |kind, operation, id| publish_runtime_change(&state, input.platform_id, kind, operation, id),
            ).await
        }).await;
        let error = match result {
            Ok(Ok(observed)) => {
                // Inventory and invalidation are committed before refreshing metadata.
                // Give this best-effort check its own deadline: a completed pull must
                // not become a failure if metadata exhausts the pull's time budget.
                let refresh = state.image_updates.refresh_after_pull(
                    principal.actor_id, principal.is_administrator(), input.platform_id,
                    &observed, input.registry_id, &image, &cancel,
                );
                match tokio::time::timeout(std::time::Duration::from_secs(45), refresh).await {
                    Ok(Ok(())) => {},
                    Ok(Err(error)) => tracing::warn!(platform_id=%input.platform_id,kind=?error.kind,"Could not refresh image update status after pull"),
                    Err(_) => tracing::warn!(platform_id=%input.platform_id,"Image update status refresh after pull timed out"),
                }
                None
            },
            Ok(Err(e)) => Some(match e.kind {
                RuntimeErrorKind::Remote | RuntimeErrorKind::Authentication => "Image pull failed. Check the image reference and Registry credentials.".into(),
                RuntimeErrorKind::Unavailable => "Image pull could not complete or be persisted. Refresh inventory before retrying.".into(),
                _ => e.message,
            }),
            Err(_) => Some("Image pull timed out. Refresh inventory before retrying.".into()),
        };
        let terminal = error.map(|message| PullImageStreamItem {
            error_message: Some(message.clone()),
            error: Some(ImagePullError {
                code: Some(500),
                message: Some(message),
            }),
            ..Default::default()
        });
        // Completion must not wait for room in a stalled client's progress queue.
        // The body drains queued progress before delivering this final error.
        let _ = completed.send(terminal);
    });
    let mut response = no_store(progress_body(receiver, completion, guard).into_response());
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json; charset=utf-8"),
    );
    Ok(response)
}

pub(super) fn progress_body(
    mut receiver: tokio::sync::mpsc::Receiver<PullImageStreamItem>,
    completion: tokio::sync::oneshot::Receiver<Option<PullImageStreamItem>>,
    guard: tokio_util::sync::DropGuard,
) -> axum::body::Body {
    let stream = async_stream::stream! {
        let _guard=guard;
        yield Ok::<_,std::convert::Infallible>(bytes::Bytes::from_static(b"["));
        let mut first=true;
        while let Some(item)=receiver.recv().await {if !first{yield Ok(bytes::Bytes::from_static(b","));}first=false;yield Ok(bytes::Bytes::from(serde_json::to_vec(&item).expect("Image pull progress serializes")));}
        if let Ok(Some(item)) = completion.await {
            if !first { yield Ok(bytes::Bytes::from_static(b",")); }
            yield Ok(bytes::Bytes::from(serde_json::to_vec(&item).expect("Image pull error serializes")));
        }
        yield Ok(bytes::Bytes::from_static(b"]"));
    };
    axum::body::Body::from_stream(stream)
}

pub(super) static IMAGE_DELETE_SLOTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);

#[utoipa::path(
    delete,
    path = "/api/v1/images",
    operation_id = "deleteImages",
    tag = "Images",
    summary = "Delete Images",
    request_body = DeleteImagesInput,
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::operation_views::DeleteImagesView, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn delete_images(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    headers: HeaderMap,
    input: Result<Json<DeleteImagesInput>, JsonRejection>,
) -> HttpResult {
    let Json(mut input) = api_result(input.map_err(invalid_json), &headers)?;
    api_result(
        validate_resource_ids(&mut input.ids, 100, "Image"),
        &headers,
    )?;
    if input.platform_id.is_nil()
        || input.ids.iter().any(|id| {
            let hash = id.strip_prefix("sha256:").unwrap_or(id);
            hash.len() != 64 || !hash.bytes().all(|c| c.is_ascii_hexdigit())
        })
    {
        return api_result(
            Err(ApiError::Validation(
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
    if api_result(platform_is_swarm(&state, input.platform_id).await, &headers)? {
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
    let Some(task) = state.tasks.reserve() else {
        return Ok(runtime_error_response(
            RuntimeCapabilityError::new(
                RuntimeErrorKind::ResourceExhausted,
                "Platform operations are shutting down.",
                false,
            ),
            &headers,
        ));
    };
    let (completed, completion) = tokio::sync::oneshot::channel();
    task.spawn(async move {
        let result = async move {
            let _permit = permit;
            // This token belongs to the tracked operation, not the HTTP body:
            // accepted deletions still need reconciliation after disconnection.
            let cancel = state.tasks.cancellation();
            let _guard = cancel.clone().drop_guard();
            let runtime = tokio::select! {
                biased;
                () = cancel.cancelled() => return Err(RuntimeCapabilityError::new(
                    RuntimeErrorKind::Cancelled, "Image deletion interrupted by shutdown.", false)),
                runtime = state.runtime.image_mutations(input.platform_id, &cancel) => runtime?,
            };
            let result = citadel_platforms::image_mutations::delete_images(
                state.image_store.as_ref(),
                input.platform_id,
                &input.ids,
                input.force,
                input.no_prune,
                &input.reviews,
                runtime.as_ref(),
                &cancel,
                || async {
                    state
                        .identity
                        .authorize_resource(
                            &principal,
                            ResourceType::Platform,
                            input.platform_id,
                            PermissionLevel::Execute,
                            None,
                        )
                        .await
                        .map_err(|_| {
                            RuntimeCapabilityError::new(
                                RuntimeErrorKind::PermissionDenied,
                                "Image deletion is no longer authorized.",
                                false,
                            )
                        })
                },
            )
            .await;
            for id in &input.ids {
                publish_runtime_change(&state, input.platform_id, "image", "update", id);
            }
            result
        }
        .await;
        if let Err(Err(error)) = completed.send(result) {
            tracing::error!(%error, "Image deletion failed after the caller disconnected");
        }
    });
    let result = api_result(completion.await.map_err(ApiError::internal), &headers)?;
    match result {
        Ok(items) => Ok(no_store(
            Json(
                crate::api::resources::platforms::operation_views::DeleteImagesView {
                    items: items
                        .into_iter()
                        .map(|result| {
                            crate::api::resources::platforms::operation_views::DeleteImageView {
                                result,
                            }
                        })
                        .collect(),
                },
            )
            .into_response(),
        )),
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
        (status = 200, description = "Success", body = crate::api::resources::platforms::operation_views::ExposedPortsView, content_type = "application/json"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("imageId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn exposed_ports(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath((platform_id, image_id)): ApiPath<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> HttpResult {
    if platform_id.is_nil() || image_id.is_nil() {
        api_result::<()>(
            Err(ApiError::Validation(
                "Platform and Image IDs must not be empty.".into(),
            )),
            &headers,
        )?;
    }
    authorize_platform(&state, &principal, platform_id, &headers).await?;
    // The form passes a Citadel UUID, never a Docker content ID. Resolve it on
    // the requested Platform and retain node identity for projected Swarm images.
    let citadel_platforms::ImageIdentity {
        docker_image_id: docker_id,
        docker_node_id: node_id,
        is_stale: stale,
    } = api_result(
        state
            .platforms
            .image_identity(platform_id, image_id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
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
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let result = async {
        state
            .runtime
            .images(platform_id, node_id.as_deref(), &cancel)
            .await?
            .exposed_ports(&docker_id, &cancel)
            .await
    }
    .await;
    match result {
        Ok(ports) => Ok(no_store(
            Json(crate::api::resources::platforms::operation_views::ExposedPortsView { ports })
                .into_response(),
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
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::ImageInspectionView, content_type = "application/json"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("imageId" = String, Path), ("dockerNodeId" = Option<String>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn inspect_image(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath((platform_id, image_id)): ApiPath<(Uuid, String)>,
    selector: Result<Query<DockerNodeSelector>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let Query(mut selector) = api_result(selector.map_err(invalid_query), &headers)?;
    api_result(validate_docker_resource_id(&image_id), &headers)?;
    if let Some(node) = &selector.docker_node_id {
        api_result(validate_docker_resource_id(node), &headers)?;
    }
    let capabilities =
        image_capabilities(authorize_platform(&state, &principal, platform_id, &headers).await?);
    if selector.docker_node_id.is_none() {
        let platform = required(
            state
                .platforms
                .get_platform(platform_id)
                .await
                .map_err(platform_error)
                .and_then(|value| {
                    value
                        .map(PlatformView::try_from)
                        .transpose()
                        .map_err(ApiError::internal)
                }),
            &headers,
        )?;
        if platform.platform_type == "DockerSwarm" {
            selector.docker_node_id = platform
                .platform_descriptor
                .as_ref()
                .and_then(
                    crate::api::resources::platforms::descriptor_views::PlatformDescriptor::node_id,
                )
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
    let cancellation = CancellationToken::new();
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let image = async {
        state
            .runtime
            .images(
                platform_id,
                selector.docker_node_id.as_deref(),
                &cancellation,
            )
            .await?
            .inspect_image(&image_id, &cancellation)
            .await
    }
    .await;
    let mut image = match image {
        Ok(image) => image,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    if selector.docker_node_id.is_none() {
        let registry_id: Option<Uuid> = api_result(
            state
                .platforms
                .image_registry_id(platform_id, &image.id)
                .await
                .map_err(platform_error),
            &headers,
        )?;
        if let Some(id) = registry_id {
            let registries = api_result(
                state
                    .registries
                    .list_registries(principal.actor_id, principal.is_administrator())
                    .await
                    .map_err(ApiError::internal),
                &headers,
            )?;
            if let Some(registry) = registries.into_iter().find(|registry| registry.id == id) {
                image.registry = Some(api_result(
                    serde_json::to_value(registry).map_err(ApiError::internal),
                    &headers,
                )?);
            }
        }
    }
    image.docker_node_id = selector.docker_node_id;
    let inventory = api_result(
        state
            .platforms
            .list_images(platform_id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    let selected: Vec<_> = inventory
        .into_iter()
        .filter(|i| i.docker_image_id == image.id && i.docker_node_id == image.docker_node_id)
        .map(|mut i| {
            // Inspection is newer than the saved inventory during a tag move.
            i.tags.clone_from(&image.repo_tags);
            i
        })
        .collect();
    let updates = match state
        .image_updates
        .summaries(
            principal.actor_id,
            principal.is_administrator(),
            platform_id,
            &selected,
        )
        .await
    {
        Ok(mut results) => results.pop().unwrap_or_default(),
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let image = crate::api::resources::platforms::views::ImageInspectionView {
        updates,
        image,
        capabilities: Some(capabilities),
    };
    Ok(no_store(Json(image).into_response()))
}

pub(super) static BROWSE_SLOTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(8);

macro_rules! browse_one {
    ($(#[$handler_attr:meta])* $handler:ident,$kind:ident) => {
        $(#[$handler_attr])*
        pub(super) async fn $handler(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<String>, PathRejection>,
            browser: Option<Extension<Arc<dyn citadel_registries::registry_images::RegistryBrowsePort>>>,
            headers: HeaderMap,
        ) -> HttpResult {
            let principal = api_result(require_actor(principal), &headers)?;
            let Path(registry) = api_result(path.map_err(invalid_path), &headers)?;
            browse(
                &state,
                &principal,
                &registry,
                None,
                RegistryBrowseKind::$kind,
                browser,
                &headers,
            )
            .await
        }
    };
}

macro_rules! browse_two {
    ($(#[$handler_attr:meta])* $handler:ident,$kind:ident) => {
        $(#[$handler_attr])*
        pub(super) async fn $handler(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<(String, String)>, PathRejection>,
            browser: Option<Extension<Arc<dyn citadel_registries::registry_images::RegistryBrowsePort>>>,
            headers: HeaderMap,
        ) -> HttpResult {
            let principal = api_result(require_actor(principal), &headers)?;
            let Path((registry, name)) = api_result(path.map_err(invalid_path), &headers)?;
            browse(
                &state,
                &principal,
                &registry,
                Some(&name),
                RegistryBrowseKind::$kind,
                browser,
                &headers,
            )
            .await
        }
    };
}

browse_one!(
    #[utoipa::path(
    get,
    path = "/api/v1/images/{registryName}/repositories",
    operation_id = "getExternalRepositories",
    tag = "Images",
    summary = "List Registry repositories",
    responses(
        (status = 200, description = "Success", body = Vec<crate::api::resources::schema_models::registries::ExternalRepositorySchema>, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("registryName" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    repositories,
    Repositories
);

browse_one!(
    #[utoipa::path(
    get,
    path = "/api/v1/images/dockerhub/{registryName}/repositories",
    operation_id = "getDockerHubRepositories",
    tag = "Images",
    summary = "List Docker Hub repositories",
    responses(
        (status = 200, description = "Success", body = Vec<crate::api::resources::schema_models::registries::DockerHubRepositoryInfoSchema>, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("registryName" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    docker_repositories,
    DockerHubRepositories
);

browse_two!(
    #[utoipa::path(
    get,
    path = "/api/v1/images/dockerhub/{registryName}/{repositoryName}/tags",
    operation_id = "getDockerHubRepositoryTags",
    tag = "Images",
    summary = "List Docker Hub repository tags",
    responses(
        (status = 200, description = "Success", body = Vec<crate::api::resources::schema_models::registries::DockerHubTagSchema>, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("registryName" = String, Path), ("repositoryName" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    docker_tags,
    DockerHubTags
);

browse_two!(
    #[utoipa::path(
    get,
    path = "/api/v1/images/ghcr/{registryName}/{packageName}/versions",
    operation_id = "getGhcrPackageVersions",
    tag = "Images",
    summary = "List GitHub package versions",
    responses(
        (status = 200, description = "Success", body = Vec<crate::api::resources::schema_models::registries::GithubPackageVersionSchema>, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("registryName" = String, Path), ("packageName" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    github_versions,
    GithubVersions
);

pub(super) async fn browse(
    state: &PlatformsHttpState,
    principal: &ActorPrincipal,
    registry: &str,
    name: Option<&str>,
    kind: RegistryBrowseKind,
    browser: Option<Extension<Arc<dyn citadel_registries::registry_images::RegistryBrowsePort>>>,
    headers: &HeaderMap,
) -> HttpResult {
    api_result(
        validate_browse_name(registry, true)
            .and_then(|()| name.map_or(Ok(()), |n| validate_browse_name(n, false)))
            .map_err(|m| ApiError::Validation(m.into())),
        headers,
    )?;
    let id = api_result(
        state
            .registries
            .find_id_by_name(registry)
            .await
            .map_err(crate::api::routes::registries::metadata_error)
            .and_then(|id| id.ok_or(ApiError::NotFound)),
        headers,
    )?;
    api_result(
        state
            .identity
            .authorize_resource(
                principal,
                ResourceType::Registry,
                id,
                PermissionLevel::Read,
                None,
            )
            .await,
        headers,
    )?;
    // Resolve credentials only after resource-specific authorization succeeds.
    let registry = api_result(
        state
            .registries
            .get_registry(id)
            .await
            .map_err(crate::api::routes::registries::metadata_error),
        headers,
    )?;
    if registry.status == citadel_registries::RegistryStatus::Disabled {
        return api_result(
            Err(ApiError::Validation("The Registry is disabled.".into())),
            headers,
        );
    }
    let configuration = registry.configuration;
    let browser = browser
        .map(|Extension(browser)| browser)
        .unwrap_or_else(|| state.registry_browser.clone());
    let _permit = api_result(
        BROWSE_SLOTS
            .try_acquire()
            .map_err(|_| ApiError::Conflict("Registry browsing capacity is busy.".into())),
        headers,
    )?;
    let result = api_result(
        tokio::time::timeout(
            std::time::Duration::from_secs(35),
            browser.browse(&configuration, kind, name),
        )
        .await
        .map_err(|_| ApiError::Validation("Registry request timed out.".into()))
        .and_then(|v| v.map_err(crate::api::routes::registries::metadata_error)),
        headers,
    )?;
    Ok(no_store(Json(result).into_response()))
}

#[cfg(test)]
mod image_pull_tests {
    use crate::api::routes::platforms::*;

    #[tokio::test]
    async fn terminal_error_does_not_wait_for_space_in_a_stalled_progress_queue() {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        let (completed, completion) = tokio::sync::oneshot::channel();
        assert!(
            sender
                .try_send(PullImageStreamItem {
                    status: Some("Pulling".into()),
                    ..Default::default()
                })
                .is_ok()
        );
        assert_eq!(sender.capacity(), 0);
        assert!(
            completed
                .send(Some(PullImageStreamItem {
                    error_message: Some("Image pull timed out.".into()),
                    ..Default::default()
                }))
                .is_ok()
        );
        drop(sender);
        let cancel = CancellationToken::new();
        let bytes = axum::body::to_bytes(
            progress_body(receiver, completion, cancel.drop_guard()),
            4096,
        )
        .await
        .unwrap();
        let items: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(items.as_array().unwrap().len(), 2);
        assert_eq!(items[0]["status"], "Pulling");
        assert_eq!(items[1]["errorMessage"], "Image pull timed out.");
    }
}
