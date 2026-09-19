use super::*;

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct Projects {
    pub(super) projects: Vec<AuthorizedProject>,
    pub(super) capabilities: ResourceCapabilitiesView,
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct AuthorizedProject {
    #[serde(flatten)]
    pub(super) project: BuildProjectView,
    pub(super) capabilities: ResourceCapabilitiesView,
}

pub(crate) async fn authorized_projects(
    store: &dyn citadel_builds::BuildRepository,
    principal: &ActorPrincipal,
    projects: Vec<citadel_builds::BuildProject>,
) -> Result<Vec<AuthorizedProject>, BuildError> {
    let ids: Vec<_> = projects.iter().map(|project| project.id).collect();
    let permissions = if principal.is_administrator() {
        Default::default()
    } else {
        store.project_permissions(principal.actor_id, &ids).await?
    };
    Ok(projects
        .into_iter()
        .map(|project| {
            let level = if principal.is_administrator() {
                EffectivePermission::Administrator
            } else {
                granted(
                    permissions
                        .get(&project.id)
                        .copied()
                        .unwrap_or(PermissionLevel::None),
                )
            };
            AuthorizedProject {
                project: project.into(),
                capabilities: project_capabilities(level),
            }
        })
        .collect())
}

pub(super) async fn project_response(
    state: &BuildsHttpState,
    principal: &ActorPrincipal,
    project: citadel_builds::BuildProject,
    headers: &HeaderMap,
) -> IdentityHttpResult {
    let mut projects = identity_result(
        authorized_projects(state.builds.store().as_ref(), principal, vec![project])
            .await
            .map_err(map_error),
        headers,
    )?;
    Ok(no_store(Json(projects.remove(0)).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/buildProjects",
    operation_id = "listBuildProjects",
    tag = "BuildProjects",
    summary = "List Build Projects",
    responses(
        (status = 200, description = "Success", body = Projects, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("tags" = Option<Vec<String>>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn list_projects(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    let tags = identity_result(
        crate::resources_http::tags::parse_filters(query.as_deref()),
        &headers,
    )?;
    let mut projects = identity_result(
        state
            .builds
            .store()
            .list(principal.actor_id, principal.is_administrator())
            .await
            .map_err(map_error),
        &headers,
    )?;
    projects.retain(|project| crate::resources_http::tags::matches_filters(&project.tags, &tags));
    let projects = identity_result(
        authorized_projects(state.builds.store().as_ref(), &principal, projects)
            .await
            .map_err(map_error),
        &headers,
    )?;
    let permission = identity_result(
        state
            .identity
            .global_permission(&principal, ResourceType::Build)
            .await,
        &headers,
    )?;
    let capabilities = project_capabilities(if principal.is_administrator() {
        EffectivePermission::Administrator
    } else {
        granted(permission.map_or(PermissionLevel::None, |grant| grant.level))
    });
    Ok(no_store(
        Json(Projects {
            projects,
            capabilities,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/buildProjects",
    operation_id = "createBuildProject",
    tag = "BuildProjects",
    summary = "Create a Build Project",
    request_body = BuildProjectInput,
    responses(
        (status = 200, description = "Success", body = BuildProjectView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn create_project(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(input): ValidatedJson<BuildProjectInput>,
) -> IdentityHttpResult {
    let mut input: citadel_builds::BuildProjectConfiguration = input.into();
    let principal = actor(principal, &headers)?;
    authorize_global(&state, &principal, PermissionLevel::Write, &headers).await?;
    identity_result(input.validate().map_err(map_error), &headers)?;
    authorize_build_dependencies(&state, &principal, &input, &headers).await?;
    validate_configuration_entitlements(&state, None, &input, true, &headers).await?;
    let project = identity_result(
        state
            .builds
            .store()
            .create(principal.actor_id, &input)
            .await
            .map_err(map_error),
        &headers,
    )?;
    project_response(&state, &principal, project, &headers).await
}

pub(super) async fn validate_configuration_entitlements(
    state: &BuildsHttpState,
    current: Option<&citadel_builds::BuildProject>,
    input: &BuildProjectConfiguration,
    updates_webhook: bool,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    if input.builder_kind == "BuildAgentPool"
        && current.is_none_or(|value| {
            value.builder_kind != input.builder_kind
                || value.build_agent_pool_id != input.build_agent_pool_id
        })
    {
        identity_result(
            state
                .builds
                .ensure_entitled(citadel_domain::LicenseCapability::ElasticBuildExecution)
                .await
                .map_err(map_error),
            headers,
        )?;
    }
    if updates_webhook
        && input
            .webhook
            .as_ref()
            .and_then(|value| value.get("enabled"))
            .and_then(serde_json::Value::as_bool)
            == Some(true)
    {
        identity_result(
            state
                .builds
                .ensure_entitled(citadel_domain::LicenseCapability::AutomatedOperations)
                .await
                .map_err(map_error),
            headers,
        )?;
    }
    Ok(())
}

pub(super) async fn authorize_build_dependencies(
    state: &BuildsHttpState,
    principal: &ActorPrincipal,
    input: &BuildProjectConfiguration,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    authorize_for(
        state,
        principal,
        ResourceType::GitRepository,
        input.git_repository_id,
        PermissionLevel::Read,
        headers,
    )
    .await?;
    authorize_for(
        state,
        principal,
        ResourceType::Registry,
        input.registry_id,
        PermissionLevel::Read,
        headers,
    )
    .await?;
    if let Some(platform_id) = input.platform_id {
        authorize_for(
            state,
            principal,
            ResourceType::Platform,
            platform_id,
            PermissionLevel::Read,
            headers,
        )
        .await?;
    }
    if let Some(pool_id) = input.build_agent_pool_id {
        authorize_for(
            state,
            principal,
            ResourceType::BuildAgentPool,
            pool_id,
            PermissionLevel::Read,
            headers,
        )
        .await?;
    }
    if input
        .build_secrets
        .as_ref()
        .is_some_and(|secrets| !secrets.is_empty())
    {
        authorize_global_for(
            state,
            principal,
            ResourceType::Binding,
            PermissionLevel::Read,
            headers,
        )
        .await?;
    }
    Ok(())
}

#[utoipa::path(
    get,
    path = "/api/v1/buildProjects/{id}",
    operation_id = "getBuildProject",
    tag = "BuildProjects",
    summary = "Get a Build Project",
    responses(
        (status = 200, description = "Success", body = BuildProjectView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn get_project(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let project = identity_result(
        state.builds.store().get(id).await.map_err(map_error),
        &headers,
    )?;
    project_response(&state, &principal, project, &headers).await
}

#[utoipa::path(
    delete,
    path = "/api/v1/buildProjects/{id}",
    operation_id = "archiveBuildProject",
    tag = "BuildProjects",
    summary = "Archive a Build Project",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn archive_project(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Write, &headers).await?;
    identity_result(
        state
            .builds
            .store()
            .archive(id, principal.actor_id)
            .await
            .map_err(map_error),
        &headers,
    )?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(
    patch,
    path = "/api/v1/buildProjects/{id}",
    operation_id = "updateBuildProject",
    tag = "BuildProjects",
    summary = "Update Build Project",
    request_body(content(
        (ref("#/components/schemas/UpdateBuildProjectInput") = "application/merge-patch+json"),
        (ref("#/components/schemas/UpdateBuildProjectInput") = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = BuildProjectView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn update_project(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    ValidatedJson(patch): ValidatedJson<serde_json::Value>,
) -> IdentityHttpResult {
    save_project(&state, principal, id, patch, None, false, &headers).await
}

#[utoipa::path(
    patch,
    path = "/api/v1/buildProjects/{id}/_metadata",
    operation_id = "updateBuildMetadata",
    tag = "BuildProjects",
    summary = "Update Build Project",
    request_body(content(
        (ref("#/components/schemas/PatchResourceMetadata") = "application/merge-patch+json"),
        (ref("#/components/schemas/PatchResourceMetadata") = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = BuildProjectView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn update_project_metadata(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    ValidatedJson(patch): ValidatedJson<serde_json::Value>,
) -> IdentityHttpResult {
    save_project(&state, principal, id, patch, None, true, &headers).await
}

#[utoipa::path(
    post,
    path = "/api/v1/buildProjects/rename",
    operation_id = "renameBuild",
    tag = "BuildProjects",
    summary = "Update Build Project",
    request_body = RenamePool,
    responses(
        (status = 200, description = "Success", body = BuildProjectView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn rename_project(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(input): ValidatedJson<RenamePool>,
) -> IdentityHttpResult {
    save_project(
        &state,
        principal,
        input.id,
        serde_json::json!({}),
        Some(input.name),
        false,
        &headers,
    )
    .await
}

pub(super) async fn save_project(
    state: &BuildsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    id: Uuid,
    patch: serde_json::Value,
    name: Option<String>,
    metadata_only: bool,
    headers: &HeaderMap,
) -> IdentityHttpResult {
    let updates_webhook = patch.get("webhook").is_some();
    let principal = actor(principal, headers)?;
    authorize(state, &principal, id, PermissionLevel::Write, headers).await?;
    let current = identity_result(
        state.builds.store().get(id).await.map_err(map_error),
        headers,
    )?;
    let mut input = identity_result(
        current.apply_patch(patch, metadata_only).map_err(map_error),
        headers,
    )?;
    if let Some(name) = name {
        input.name = name;
    }
    identity_result(input.validate().map_err(map_error), headers)?;
    if !metadata_only {
        authorize_build_dependencies(state, &principal, &input, headers).await?;
        validate_configuration_entitlements(
            state,
            Some(&current),
            &input,
            updates_webhook,
            headers,
        )
        .await?;
    }
    let project = identity_result(
        state
            .builds
            .store()
            .update(&current, &input, principal.actor_id, metadata_only)
            .await
            .map_err(map_error),
        headers,
    )?;
    project_response(state, &principal, project, headers).await
}
