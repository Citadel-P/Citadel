use super::*;

#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = server::backups_http::Runs)]
#[serde(rename_all = "camelCase")]
pub(super) struct Runs {
    pub(super) runs: Vec<BackupRunView>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = server::backups_http::Logs)]
#[serde(rename_all = "camelCase")]
pub(super) struct Logs {
    pub(super) run_id: Uuid,
    pub(super) logs: String,
}

#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = server::backups_http::Events)]
#[serde(rename_all = "camelCase")]
pub(super) struct Events {
    pub(super) run_id: Uuid,
    pub(super) events: Vec<String>,
}

#[derive(Deserialize, Default, utoipa::ToSchema)]
#[schema(as = server::backups_http::QueueInput)]
#[serde(rename_all = "camelCase")]
pub(super) struct QueueInput {
    pub(super) trigger: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(super) struct RunFilter {
    pub(super) policy_id: Option<Uuid>,
    pub(super) backup_run_id: Option<Uuid>,
    pub(super) limit: Option<usize>,
}

#[utoipa::path(
    get,
    path = "/api/v1/backupPolicies/platform-summaries",
    operation_id = "getPlatformBackupSummaries",
    tag = "BackupPolicies",
    summary = "Get Platform Backup summaries",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/PlatformBackupSummariesView"), content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("platformIds" = Vec<uuid::Uuid>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn platform_summaries(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    axum::extract::RawQuery(query): axum::extract::RawQuery,
    h: HeaderMap,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Read,
        None,
        &h,
    )
    .await?;
    let mut ids = Vec::new();
    for (key, value) in url::form_urlencoded::parse(query.as_deref().unwrap_or_default().as_bytes())
    {
        if key == "platformIds" {
            let id = identity_result(
                Uuid::parse_str(&value)
                    .map_err(|_| IdentityError::Validation("Platform IDs must be UUIDs.".into())),
                &h,
            )?;
            if !id.is_nil() {
                ids.push(id);
            }
            if ids.len() > 256 {
                return identity_result(
                    Err(IdentityError::Validation(
                        "At most 256 Platform IDs can be requested.".into(),
                    )),
                    &h,
                );
            }
        }
    }
    ids.sort_unstable();
    ids.dedup();
    let platforms = result(
        s.backups
            .store()
            .platform_summaries(p.actor_id, p.is_administrator(), &ids)
            .await,
        &h,
    )?;
    Ok(no_store(
        Json(serde_json::json!({"platforms":platforms.into_iter().map(PlatformBackupSummary::from).collect::<Vec<_>>()})).into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/backupPolicies/{id}/runs",
    operation_id = "queueBackupRun",
    tag = "BackupPolicies",
    summary = "Queue a Backup Run",
    request_body = Option<QueueInput>,
    responses(
        (status = 200, description = "Success", body = BackupRunView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn queue_backup(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
    input: Option<ValidatedJson<QueueInput>>,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let input = input.map(|ValidatedJson(v)| v).unwrap_or_default();
    let run = enqueue_backup(&s, &p, id, input, &h).await?;
    Ok(no_store(Json(BackupRunView::from(run)).into_response()))
}

pub(super) async fn enqueue_backup(
    s: &BackupsHttpState,
    p: &ActorPrincipal,
    id: Uuid,
    input: QueueInput,
    h: &HeaderMap,
) -> IdentityHttpResult<citadel_backups::BackupRun> {
    auth(
        s,
        p,
        ResourceType::BackupPolicy,
        PermissionLevel::Execute,
        Some(id),
        h,
    )
    .await?;
    let trigger = input.trigger.unwrap_or_else(|| "Manual".into());
    if !matches!(trigger.as_str(), "Manual" | "Schedule" | "Webhook") {
        return identity_result(
            Err(IdentityError::Validation(
                "Backup trigger is invalid.".into(),
            )),
            h,
        );
    }
    if trigger != "Manual" {
        result(s.backups.ensure_automated_operations().await, h)?;
    }
    let run = result(
        s.backups
            .store()
            .enqueue_backup(p.actor_id, id, &trigger)
            .await,
        h,
    )?;
    Ok(run)
}

#[utoipa::path(
    get,
    path = "/api/v1/backupRuns",
    operation_id = "listBackupRuns",
    tag = "BackupRuns",
    summary = "List Backup Runs",
    responses(
        (status = 200, description = "Success", body = Runs, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("policyId" = Option<uuid::Uuid>, Query), ("limit" = Option<i32>, Query, minimum = 1, maximum = 100, extensions(("x-citadel-default" = json!(50))))),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn list_runs(
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
        Json(Runs {
            runs: result(
                s.backups
                    .store()
                    .list_runs(
                        p.actor_id,
                        p.is_administrator(),
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
    path = "/api/v1/backupRuns/{id}",
    operation_id = "getBackupRun",
    tag = "BackupRuns",
    summary = "Get a Backup Run",
    responses(
        (status = 200, description = "Success", body = BackupRunView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn get_run(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let run = result(s.backups.store().get_run(id).await, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Read,
        Some(run.backup_policy_id),
        &h,
    )
    .await?;
    Ok(no_store(Json(BackupRunView::from(run)).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/backupRuns/{id}/events",
    operation_id = "getBackupRunEvents",
    tag = "BackupRuns",
    summary = "Get Backup Run events",
    responses(
        (status = 200, description = "Success", body = Events, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn get_backup_events(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let run = result(s.backups.store().get_run(id).await, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Read,
        Some(run.backup_policy_id),
        &h,
    )
    .await?;
    // .NET exposes an empty events collection. Execution output remains in /logs.
    Ok(no_store(
        Json(Events {
            run_id: id,
            events: Vec::new(),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/backupRuns/{id}/logs",
    operation_id = "getBackupRunLogs",
    tag = "BackupRuns",
    summary = "Get Backup Run logs",
    responses(
        (status = 200, description = "Success", body = Logs, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn get_backup_logs(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let run = result(s.backups.store().get_run(id).await, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Read,
        Some(run.backup_policy_id),
        &h,
    )
    .await?;
    Ok(no_store(
        Json(Logs {
            run_id: id,
            logs: logs_text(result(s.backups.store().backup_logs(id).await, &h)?),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/backupRuns/{id}/cancel",
    operation_id = "cancelBackupRun",
    tag = "BackupRuns",
    summary = "Cancel a Backup Run",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn cancel_backup(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let run = result(s.backups.store().get_run(id).await, &h)?;
    auth(
        &s,
        &p,
        ResourceType::BackupPolicy,
        PermissionLevel::Execute,
        Some(run.backup_policy_id),
        &h,
    )
    .await?;
    result(s.backups.cancel_backup(id).await, &h)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
