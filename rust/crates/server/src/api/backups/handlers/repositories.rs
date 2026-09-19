use super::*;

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct Repositories {
    pub(super) repositories: Vec<BackupRepositoryView>,
    pub(super) capabilities: ResourceCapabilities,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct RepositoryLocationInput {
    pub(super) location: String,
    pub(super) platform_id: Option<Uuid>,
}

#[utoipa::path(
    get,
    path = "/api/v1/backupRepositories",
    operation_id = "listBackupRepositories",
    tag = "BackupRepositories",
    summary = "List Backup Repositories",
    responses(
        (status = 200, description = "Success", body = Repositories, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn list_repositories(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let permission = identity_result(
        s.identity
            .global_permission(&p, ResourceType::BackupRepository)
            .await,
        &h,
    )?;
    Ok(no_store(
        Json(Repositories {
            repositories: result(
                s.backups
                    .store()
                    .list_repositories(p.actor_id, p.is_administrator())
                    .await,
                &h,
            )?
            .into_iter()
            .map(Into::into)
            .collect(),
            capabilities: permission.into(),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/backupRepositories",
    operation_id = "createBackupRepository",
    tag = "BackupRepositories",
    summary = "Create a Backup Repository",
    request_body = BackupRepositoryInput,
    responses(
        (status = 200, description = "Success", body = BackupRepositoryView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn create_repository(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    ValidatedJson(i): ValidatedJson<BackupRepositoryInput>,
) -> IdentityHttpResult {
    let mut i: citadel_backups::BackupRepositoryConfiguration = i.into();
    let p = actor(p, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupRepository,
        PermissionLevel::Write,
        None,
        &h,
    )
    .await?;
    result(i.validate(), &h)?;
    authorize_repository_dependencies(&s, &p, &i, &h).await?;
    Ok(no_store(
        Json(BackupRepositoryView::from(result(
            s.backups.store().create_repository(p.actor_id, &i).await,
            &h,
        )?))
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/backupRepositories/{id}",
    operation_id = "getBackupRepository",
    tag = "BackupRepositories",
    summary = "Get a Backup Repository",
    responses(
        (status = 200, description = "Success", body = BackupRepositoryView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn get_repository(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupRepository,
        PermissionLevel::Read,
        Some(id),
        &h,
    )
    .await?;
    Ok(no_store(
        Json(BackupRepositoryView::from(result(
            s.backups.store().get_repository(id).await,
            &h,
        )?))
        .into_response(),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/backupRepositories/{id}",
    operation_id = "updateBackupRepository",
    tag = "BackupRepositories",
    summary = "Update a Backup Repository",
    request_body(content(
        (ref("#/components/schemas/UpdateBackupRepositoryInput") = "application/merge-patch+json"),
        (ref("#/components/schemas/UpdateBackupRepositoryInput") = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = BackupRepositoryView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn update_repository(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, axum::extract::rejection::PathRejection>,
    h: HeaderMap,
    body: Result<Json<serde_json::Value>, axum::extract::rejection::JsonRejection>,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let Path(id) = identity_result(
        path.map_err(|_| IdentityError::Validation("Repository ID must be a UUID.".into())),
        &h,
    )?;
    if id.is_nil() {
        return identity_result(
            Err(IdentityError::Validation(
                "Repository ID is required.".into(),
            )),
            &h,
        );
    }
    auth(
        &s,
        &p,
        ResourceType::BackupRepository,
        PermissionLevel::Write,
        Some(id),
        &h,
    )
    .await?;
    let Json(patch) = identity_result(body.map_err(crate::request_validation::invalid_json), &h)?;
    if patch.get("spec").is_some_and(|v| !v.is_null()) {
        let current = result(s.backups.store().get_repository(id).await, &h)?;
        let input = result(
            citadel_backups::repositories::patch::apply(&current, &patch),
            &h,
        )?;
        authorize_repository_dependencies(&s, &p, &input, &h).await?;
    }
    Ok(no_store(
        Json(BackupRepositoryView::from(result(
            s.backups.store().update_repository(id, &patch).await,
            &h,
        )?))
        .into_response(),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/backupRepositories/{id}",
    operation_id = "archiveBackupRepository",
    tag = "BackupRepositories",
    summary = "Archive a Backup Repository",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn archive_repository(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupRepository,
        PermissionLevel::Write,
        Some(id),
        &h,
    )
    .await?;
    result(s.backups.store().archive_repository(id).await, &h)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(
    post,
    path = "/api/v1/backupRepositories/{id}/validate",
    operation_id = "validateBackupRepository",
    tag = "BackupRepositories",
    summary = "Validate a Backup Repository",
    request_body = RepositoryLocationInput,
    responses(
        (status = 200, description = "Success", body = BackupRepositoryValidationView, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn validate_repository(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
    ValidatedJson(input): ValidatedJson<RepositoryLocationInput>,
) -> IdentityHttpResult {
    repository_operation(s, p, id, h, input, "Validate").await
}

#[utoipa::path(
    post,
    path = "/api/v1/backupRepositories/{id}/initialize",
    operation_id = "initializeBackupRepository",
    tag = "BackupRepositories",
    summary = "Initialize a Backup Repository",
    request_body = RepositoryLocationInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn initialize_repository(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
    ValidatedJson(input): ValidatedJson<RepositoryLocationInput>,
) -> IdentityHttpResult {
    repository_operation(s, p, id, h, input, "Initialize").await
}

#[utoipa::path(
    post,
    path = "/api/v1/backupRepositories/{id}/check",
    operation_id = "checkBackupRepository",
    tag = "BackupRepositories",
    summary = "Check a Backup Repository",
    request_body = RepositoryLocationInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn check_repository(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
    ValidatedJson(input): ValidatedJson<RepositoryLocationInput>,
) -> IdentityHttpResult {
    repository_operation(s, p, id, h, input, "Check").await
}

#[utoipa::path(
    post,
    path = "/api/v1/backupRepositories/{id}/prune",
    operation_id = "pruneBackupRepository",
    tag = "BackupRepositories",
    summary = "Prune a Backup Repository",
    request_body = RepositoryLocationInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn prune_repository(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
    ValidatedJson(input): ValidatedJson<RepositoryLocationInput>,
) -> IdentityHttpResult {
    repository_operation(s, p, id, h, input, "Prune").await
}

pub(super) async fn repository_operation(
    s: BackupsHttpState,
    p: Option<Extension<ActorPrincipal>>,
    id: Uuid,
    h: HeaderMap,
    input: RepositoryLocationInput,
    op: &str,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupRepository,
        PermissionLevel::Execute,
        Some(id),
        &h,
    )
    .await?;
    validate_repository_location(&input, &h)?;
    if let Some(platform_id) = input.platform_id {
        auth(
            &s,
            &p,
            ResourceType::Platform,
            PermissionLevel::Execute,
            Some(platform_id),
            &h,
        )
        .await?;
    }
    let operation_cancellation = s.cancellation.child_token();
    let operation = result(
        s.backups
            .repository_operation(
                id,
                op,
                &input.location,
                input.platform_id,
                &operation_cancellation,
            )
            .await,
        &h,
    )?;
    if op == "Validate" {
        return Ok(no_store(
            Json(BackupRepositoryValidationView::from(operation.validation)).into_response(),
        ));
    }
    if let Some(message) = operation.error_message {
        return result(Err(BackupError::External(message)), &h);
    }
    Ok(StatusCode::NO_CONTENT.into_response())
}

pub(super) async fn authorize_repository_dependencies(
    state: &BackupsHttpState,
    principal: &ActorPrincipal,
    input: &BackupRepositoryConfiguration,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    auth(
        state,
        principal,
        ResourceType::Binding,
        PermissionLevel::Read,
        None,
        headers,
    )
    .await?;
    if let Some(platform_id) = json_uuid(&input.spec, "platformId") {
        auth(
            state,
            principal,
            ResourceType::Platform,
            PermissionLevel::Read,
            Some(platform_id),
            headers,
        )
        .await?;
    }
    Ok(())
}

pub(super) fn validate_repository_location(
    input: &RepositoryLocationInput,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    let valid = match input.location.as_str() {
        "Core" => input.platform_id.is_none(),
        "Platform" => input.platform_id.is_some_and(|id| !id.is_nil()),
        _ => false,
    };
    identity_result(
        if valid {
            Ok(())
        } else {
            Err(IdentityError::Validation(
                "Backup execution location is invalid.".to_owned(),
            ))
        },
        headers,
    )
}
