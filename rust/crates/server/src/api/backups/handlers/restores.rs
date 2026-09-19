use super::*;

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct Restores {
    pub(super) runs: Vec<BackupRestoreRunView>,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct RestoreInput {
    pub(super) target_platform_id: Uuid,
    pub(super) target_volume_name: String,
    pub(super) overwrite_existing: bool,
    pub(super) target_docker_node_id: Option<String>,
    pub(super) source_backup_run_item_id: Option<Uuid>,
}

#[utoipa::path(
    get,
    path = "/api/v1/backupRestoreRuns/{id}/events",
    operation_id = "getBackupRestoreRunEvents",
    tag = "BackupRestoreRuns",
    summary = "Get Backup Restore Run events",
    responses(
        (status = 200, description = "Success", body = Events, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn get_restore_events(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let restore = result(s.backups.store().get_restore(id).await, &h)?;
    let source = result(s.backups.store().get_run(restore.backup_run_id).await, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Read,
        Some(source.backup_policy_id),
        &h,
    )
    .await?;
    Ok(no_store(
        Json(Events {
            run_id: id,
            events: Vec::new(),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/backupRuns/{id}/restoreVolume",
    operation_id = "restoreBackupVolume",
    tag = "BackupRuns",
    summary = "Queue a Volume restore",
    request_body = RestoreInput,
    responses(
        (status = 200, description = "Success", body = BackupRestoreRunView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn queue_restore(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
    ValidatedJson(i): ValidatedJson<RestoreInput>,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    Ok(no_store(
        Json(BackupRestoreRunView::from(
            enqueue_restore(&s, &p, id, i, &h).await?,
        ))
        .into_response(),
    ))
}

pub(super) async fn enqueue_restore(
    s: &BackupsHttpState,
    p: &ActorPrincipal,
    id: Uuid,
    i: RestoreInput,
    h: &HeaderMap,
) -> IdentityHttpResult<citadel_backups::BackupRestoreRun> {
    let source = result(s.backups.store().get_run(id).await, h)?;
    let permission = auth(
        s,
        p,
        ResourceType::BackupPolicy,
        PermissionLevel::Execute,
        Some(source.backup_policy_id),
        h,
    )
    .await?;
    identity_result(
        permission
            .has_specific(citadel_domain::SpecificPermission::Restore)
            .then_some(())
            .ok_or(IdentityError::Forbidden),
        h,
    )?;
    auth(
        s,
        p,
        ResourceType::Platform,
        PermissionLevel::Write,
        Some(i.target_platform_id),
        h,
    )
    .await?;
    let run = result(
        s.backups
            .store()
            .enqueue_restore(BackupRestoreRequest {
                actor: p.actor_id,
                backup_run_id: id,
                target_platform_id: i.target_platform_id,
                target_volume_name: i.target_volume_name,
                overwrite_existing: i.overwrite_existing,
                target_docker_node_id: i.target_docker_node_id,
                source_backup_run_item_id: i.source_backup_run_item_id,
            })
            .await,
        h,
    )?;
    Ok(run)
}

#[utoipa::path(
    get,
    path = "/api/v1/backupRestoreRuns",
    operation_id = "listBackupRestoreRuns",
    tag = "BackupRestoreRuns",
    summary = "List Backup Restore Runs",
    responses(
        (status = 200, description = "Success", body = Restores, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("backupRunId" = Option<uuid::Uuid>, Query), ("policyId" = Option<uuid::Uuid>, Query), ("limit" = Option<i32>, Query, minimum = 1, maximum = 100, extensions(("x-citadel-default" = json!(50))))),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn list_restores(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiQuery(f): ApiQuery<RunFilter>,
    h: HeaderMap,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    if let Some(id) = f.policy_id {
        auth(
            &s,
            &p,
            ResourceType::BackupPolicy,
            PermissionLevel::Read,
            Some(id),
            &h,
        )
        .await?;
    }
    Ok(no_store(
        Json(Restores {
            runs: result(
                s.backups
                    .store()
                    .list_restores(
                        p.actor_id,
                        p.is_administrator(),
                        f.backup_run_id,
                        f.policy_id,
                        f.limit.unwrap_or(50),
                    )
                    .await,
                &h,
            )?
            .into_iter()
            .map(Into::into)
            .collect(),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/backupRestoreRuns/{id}",
    operation_id = "getBackupRestoreRun",
    tag = "BackupRestoreRuns",
    summary = "Get a Backup Restore Run",
    responses(
        (status = 200, description = "Success", body = BackupRestoreRunView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn get_restore(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let restore = result(s.backups.store().get_restore(id).await, &h)?;
    let source = result(s.backups.store().get_run(restore.backup_run_id).await, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Read,
        Some(source.backup_policy_id),
        &h,
    )
    .await?;
    Ok(no_store(
        Json(BackupRestoreRunView::from(restore)).into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/backupRestoreRuns/{id}/logs",
    operation_id = "getBackupRestoreRunLogs",
    tag = "BackupRestoreRuns",
    summary = "Get Backup Restore Run logs",
    responses(
        (status = 200, description = "Success", body = Logs, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn get_restore_logs(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let restore = result(s.backups.store().get_restore(id).await, &h)?;
    let source = result(s.backups.store().get_run(restore.backup_run_id).await, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Read,
        Some(source.backup_policy_id),
        &h,
    )
    .await?;
    Ok(no_store(
        Json(Logs {
            run_id: id,
            logs: logs_text(result(s.backups.store().restore_logs(id).await, &h)?),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/backupRestoreRuns/{id}/cancel",
    operation_id = "cancelBackupRestoreRun",
    tag = "BackupRestoreRuns",
    summary = "Cancel a Backup Restore Run",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn cancel_restore(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let restore = result(s.backups.store().get_restore(id).await, &h)?;
    let source = result(s.backups.store().get_run(restore.backup_run_id).await, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Execute,
        Some(source.backup_policy_id),
        &h,
    )
    .await?;
    result(s.backups.cancel_restore(id).await, &h)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
