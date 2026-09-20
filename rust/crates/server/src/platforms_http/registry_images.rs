use super::*;

use citadel_adapters::registry_images::RegistryBrowser;

use citadel_registries::registry_images::{RegistryBrowseKind, validate_browse_name};

use sqlx::Row;

static BROWSER: std::sync::OnceLock<Result<RegistryBrowser, String>> = std::sync::OnceLock::new();

static BROWSE_SLOTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(8);

macro_rules! browse_one {
    ($(#[$handler_attr:meta])* $handler:ident,$kind:ident) => {
        $(#[$handler_attr])*
        pub(super) async fn $handler(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<String>, PathRejection>,
            browser: Option<Extension<RegistryBrowser>>,
            headers: HeaderMap,
        ) -> IdentityHttpResult {
            let principal = identity_result(require_actor(principal), &headers)?;
            let Path(registry) = identity_result(path.map_err(invalid_path), &headers)?;
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
            browser: Option<Extension<RegistryBrowser>>,
            headers: HeaderMap,
        ) -> IdentityHttpResult {
            let principal = identity_result(require_actor(principal), &headers)?;
            let Path((registry, name)) = identity_result(path.map_err(invalid_path), &headers)?;
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
        (status = 200, description = "Success", body = ref("#/components/schemas/getExternalRepositoriesResponse"), content_type = "application/json"),
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
        (status = 200, description = "Success", body = ref("#/components/schemas/getDockerHubRepositoriesResponse"), content_type = "application/json"),
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
        (status = 200, description = "Success", body = ref("#/components/schemas/getDockerHubRepositoryTagsResponse"), content_type = "application/json"),
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
        (status = 200, description = "Success", body = ref("#/components/schemas/getGhcrPackageVersionsResponse"), content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("registryName" = String, Path), ("packageName" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    github_versions,
    GithubVersions
);

async fn browse(
    state: &PlatformsHttpState,
    principal: &ActorPrincipal,
    registry: &str,
    name: Option<&str>,
    kind: RegistryBrowseKind,
    browser: Option<Extension<RegistryBrowser>>,
    headers: &HeaderMap,
) -> IdentityHttpResult {
    identity_result(
        validate_browse_name(registry, true)
            .and_then(|()| name.map_or(Ok(()), |n| validate_browse_name(n, false)))
            .map_err(|m| IdentityError::Validation(m.into())),
        headers,
    )?;
    let id = identity_result(
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM registries WHERE lower(name)=lower($1)")
            .bind(registry)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| IdentityError::Storage(e.to_string()))
            .and_then(|v| v.ok_or(IdentityError::NotFound)),
        headers,
    )?;
    identity_result(
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
    let row = identity_result(
        sqlx::query("SELECT configuration,status FROM registries WHERE id=$1")
            .bind(id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| IdentityError::Storage(e.to_string()))
            .and_then(|v| v.ok_or(IdentityError::NotFound)),
        headers,
    )?;
    if row.get::<String, _>("status") == "Disabled" {
        return identity_result(
            Err(IdentityError::Validation(
                "The Registry is disabled.".into(),
            )),
            headers,
        );
    }
    let configuration: serde_json::Value = row.get("configuration");
    let browser = match browser {
        Some(Extension(browser)) => browser,
        None => identity_result(
            BROWSER
                .get_or_init(|| RegistryBrowser::new().map_err(|e| e.to_string()))
                .as_ref()
                .cloned()
                .map_err(|m| IdentityError::Storage(m.clone())),
            headers,
        )?,
    };
    let _permit = identity_result(
        BROWSE_SLOTS
            .try_acquire()
            .map_err(|_| IdentityError::Conflict("Registry browsing capacity is busy.".into())),
        headers,
    )?;
    let result = identity_result(
        tokio::time::timeout(
            std::time::Duration::from_secs(35),
            browser.browse(&configuration, kind, name),
        )
        .await
        .map_err(|_| IdentityError::Validation("Registry request timed out.".into()))
        .and_then(|v| v.map_err(crate::api::registries::metadata_error)),
        headers,
    )?;
    Ok(no_store(Json(result).into_response()))
}
