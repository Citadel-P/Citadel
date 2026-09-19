use super::*;

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct Policies {
    pub(super) policies: Vec<BackupPolicyView>,
    pub(super) capabilities: ResourceCapabilities,
}

#[utoipa::path(
    get,
    path = "/api/v1/backupPolicies",
    operation_id = "listBackupPolicies",
    tag = "BackupPolicies",
    summary = "List Backup Policies",
    responses(
        (status = 200, description = "Success", body = Policies, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("tags" = Option<Vec<String>>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn list_policies(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let permission = identity_result(
        s.identity
            .global_permission(&p, ResourceType::BackupPolicy)
            .await,
        &h,
    )?;
    Ok(no_store(
        Json(Policies {
            policies: result(
                s.backups
                    .store()
                    .list_policies(p.actor_id, p.is_administrator())
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
    path = "/api/v1/backupPolicies",
    operation_id = "createBackupPolicy",
    tag = "BackupPolicies",
    summary = "Create a Backup Policy",
    request_body = BackupPolicyInput,
    responses(
        (status = 200, description = "Success", body = BackupPolicyView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn create_policy(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    ValidatedJson(i): ValidatedJson<BackupPolicyInput>,
) -> IdentityHttpResult {
    let mut i: citadel_backups::BackupPolicyConfiguration = i.into();
    let p = actor(p, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Write,
        None,
        &h,
    )
    .await?;
    result(i.validate(p.actor_id), &h)?;
    authorize_policy_dependencies(&s, &p, &i, &h).await?;
    identity_result(
        s.identity
            .ensure_run_as_allowed(
                &p,
                citadel_domain::ActorId::new(
                    i.run_as_actor_id.expect("validation sets the run-as Actor"),
                ),
            )
            .await,
        &h,
    )?;
    Ok(no_store(
        Json(BackupPolicyView::from(result(
            s.backups.store().create_policy(p.actor_id, &i).await,
            &h,
        )?))
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/backupPolicies/{id}",
    operation_id = "getBackupPolicy",
    tag = "BackupPolicies",
    summary = "Get a Backup Policy",
    responses(
        (status = 200, description = "Success", body = BackupPolicyView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn get_policy(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Read,
        Some(id),
        &h,
    )
    .await?;
    Ok(no_store(
        Json(BackupPolicyView::from(result(
            s.backups.store().get_policy(id).await,
            &h,
        )?))
        .into_response(),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/backupPolicies/{id}",
    operation_id = "updateBackupPolicy",
    tag = "BackupPolicies",
    summary = "Update a Backup Policy",
    request_body(content(
        (ref("#/components/schemas/UpdateBackupPolicyInput") = "application/merge-patch+json"),
        (ref("#/components/schemas/UpdateBackupPolicyInput") = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = BackupPolicyView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn update_policy(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, axum::extract::rejection::PathRejection>,
    h: HeaderMap,
    body: Result<Json<serde_json::Value>, axum::extract::rejection::JsonRejection>,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let Path(id) = identity_result(
        path.map_err(|_| IdentityError::Validation("Policy ID must be a UUID.".into())),
        &h,
    )?;
    if id.is_nil() {
        return identity_result(
            Err(IdentityError::Validation("Policy ID is required.".into())),
            &h,
        );
    }
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Write,
        Some(id),
        &h,
    )
    .await?;
    let Json(patch) = identity_result(body.map_err(crate::request_validation::invalid_json), &h)?;
    let current = result(s.backups.store().get_policy(id).await, &h)?;
    let input = result(
        citadel_backups::policies::patch::merge(&current, &patch),
        &h,
    )?;
    if citadel_backups::policies::patch::changes_paid_trigger(&current, &input) {
        result(s.backups.ensure_automated_operations().await, &h)?;
    }
    if citadel_backups::policies::patch::changes_execution(&patch) {
        identity_result(
            s.identity
                .ensure_run_as_allowed(
                    &p,
                    citadel_domain::ActorId::new(
                        input.run_as_actor_id.expect("validated run-as Actor"),
                    ),
                )
                .await,
            &h,
        )?;
    }
    if patch.get("source").is_some_and(|v| !v.is_null())
        || patch
            .get("backupRepositoryId")
            .is_some_and(|v| !v.is_null())
    {
        authorize_policy_dependencies(&s, &p, &input, &h).await?;
        let repository = result(
            s.backups
                .store()
                .get_repository(input.backup_repository_id)
                .await,
            &h,
        )?;
        let cancellation = s.cancellation.child_token();
        let _guard = cancellation.clone().drop_guard();
        result(
            tokio::time::timeout(
                std::time::Duration::from_secs(30),
                s.backups
                    .planner()
                    .validate_source(&input.source, &repository, &cancellation),
            )
            .await
            .unwrap_or_else(|_| {
                Err(BackupError::External(
                    "Backup source validation timed out.".into(),
                ))
            }),
            &h,
        )?;
    }
    Ok(no_store(
        Json(BackupPolicyView::from(result(
            s.backups
                .store()
                .update_policy(p.actor_id, id, current.row_version, &input)
                .await,
            &h,
        )?))
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/backupPolicies/rename",
    operation_id = "renameBackupPolicy",
    tag = "BackupPolicies",
    summary = "Rename a Backup Policy",
    request_body = RenameBackupPolicyInput,
    responses(
        (status = 200, description = "Success", body = BackupPolicyView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn rename_policy(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    body: Result<Json<RenameBackupPolicyInput>, axum::extract::rejection::JsonRejection>,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let Json(input) = identity_result(body.map_err(crate::request_validation::invalid_json), &h)?;
    let mut input: citadel_backups::policies::metadata::RenameBackupPolicyInput = input.into();
    result(input.validate(), &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Write,
        Some(input.id),
        &h,
    )
    .await?;
    Ok(no_store(
        Json(BackupPolicyView::from(result(
            s.backups.store().rename_policy(p.actor_id, &input).await,
            &h,
        )?))
        .into_response(),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/backupPolicies/{id}/_metadata",
    operation_id = "updateBackupPolicyMetadata",
    tag = "BackupPolicies",
    summary = "Update Backup Policy metadata",
    request_body(content(
        (ref("#/components/schemas/PatchResourceMetadata") = "application/merge-patch+json"),
        (ref("#/components/schemas/PatchResourceMetadata") = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = BackupPolicyView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn update_policy_metadata(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, axum::extract::rejection::PathRejection>,
    h: HeaderMap,
    body: Result<Json<serde_json::Value>, axum::extract::rejection::JsonRejection>,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let Path(id) = identity_result(
        path.map_err(|_| IdentityError::Validation("Policy ID must be a UUID.".into())),
        &h,
    )?;
    if id.is_nil() {
        return identity_result(
            Err(IdentityError::Validation("Policy ID is required.".into())),
            &h,
        );
    }
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Write,
        Some(id),
        &h,
    )
    .await?;
    let Json(patch) = identity_result(body.map_err(crate::request_validation::invalid_json), &h)?;
    let description = result(
        citadel_backups::policies::metadata::description_patch(&patch),
        &h,
    )?;
    Ok(no_store(
        Json(BackupPolicyView::from(result(
            s.backups
                .store()
                .update_policy_description(p.actor_id, id, description)
                .await,
            &h,
        )?))
        .into_response(),
    ))
}

pub(super) async fn authorize_policy_dependencies(
    state: &BackupsHttpState,
    principal: &ActorPrincipal,
    input: &BackupPolicyConfiguration,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    auth(
        state,
        principal,
        ResourceType::BackupRepository,
        PermissionLevel::Read,
        Some(input.backup_repository_id),
        headers,
    )
    .await?;
    let resource = match input
        .source
        .get("$type")
        .and_then(serde_json::Value::as_str)
    {
        Some("DockerVolume") => {
            json_uuid(&input.source, "platformId").map(|id| (ResourceType::Platform, id))
        }
        Some("Stack") => json_uuid(&input.source, "stackId").map(|id| (ResourceType::Stack, id)),
        Some("Deployment") => {
            json_uuid(&input.source, "deploymentId").map(|id| (ResourceType::Deployment, id))
        }
        Some("SwarmService") => {
            json_uuid(&input.source, "swarmServiceId").map(|id| (ResourceType::SwarmService, id))
        }
        _ => None,
    };
    if let Some((resource_type, resource_id)) = resource {
        auth(
            state,
            principal,
            resource_type,
            PermissionLevel::Read,
            Some(resource_id),
            headers,
        )
        .await?;
    }
    Ok(())
}

#[utoipa::path(
    delete,
    path = "/api/v1/backupPolicies/{id}",
    operation_id = "archiveBackupPolicy",
    tag = "BackupPolicies",
    summary = "Archive a Backup Policy",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn archive_policy(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Write,
        Some(id),
        &h,
    )
    .await?;
    result(s.backups.store().archive_policy(id).await, &h)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
