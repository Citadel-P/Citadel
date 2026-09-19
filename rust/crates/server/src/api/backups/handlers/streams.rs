use super::*;
use axum::body::Body;
use citadel_backups::runs::progress::{BackupProgressItem, is_terminal};

#[utoipa::path(
    post,
    path = "/api/v1/backupPolicies/{id}/run",
    operation_id = "runBackupPolicy",
    tag = "BackupPolicies",
    summary = "Run a Backup Policy with progress",
    request_body = QueueInput,
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/BackupRunStream"), content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn run_backup(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, axum::extract::rejection::PathRejection>,
    h: HeaderMap,
    body: Result<Json<QueueInput>, axum::extract::rejection::JsonRejection>,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let Path(id) = identity_result(
        path.map_err(|_| IdentityError::Validation("Policy ID must be a UUID.".into())),
        &h,
    )?;
    let Json(input) = identity_result(body.map_err(crate::request_validation::invalid_json), &h)?;
    let progress = s.backups.subscribe_progress(); // Subscribe before enqueue: fast workers must not lose completion.
    let run = enqueue_backup(&s, &p, id, input, &h).await?;
    Ok(response(
        s,
        p,
        id,
        run.id,
        false,
        progress,
        format!("Backup run queued for \"{}\".", run.policy_name_snapshot),
    ))
}
#[utoipa::path(
    post,
    path = "/api/v1/backupRuns/{id}/restoreVolume/run",
    operation_id = "runBackupRestoreVolume",
    tag = "BackupRuns",
    summary = "Restore a Backup Volume with progress",
    request_body = RestoreInput,
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/BackupRestoreRunStream"), content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn run_restore(
    State(s): State<BackupsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, axum::extract::rejection::PathRejection>,
    h: HeaderMap,
    body: Result<Json<RestoreInput>, axum::extract::rejection::JsonRejection>,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let Path(id) = identity_result(
        path.map_err(|_| IdentityError::Validation("Backup Run ID must be a UUID.".into())),
        &h,
    )?;
    let Json(input) = identity_result(body.map_err(crate::request_validation::invalid_json), &h)?;
    let progress = s.backups.subscribe_progress();
    let run = enqueue_restore(&s, &p, id, input, &h).await?;
    let source = result(s.backups.store().get_run(id).await, &h)?;
    Ok(response(
        s,
        p,
        source.backup_policy_id,
        run.id,
        true,
        progress,
        "Volume restore queued.".into(),
    ))
}

fn response(
    state: BackupsHttpState,
    principal: ActorPrincipal,
    policy_id: Uuid,
    id: Uuid,
    restore: bool,
    mut progress: tokio::sync::broadcast::Receiver<Arc<BackupProgressItem>>,
    queued: String,
) -> axum::response::Response {
    // The durable worker owns execution. Dropping this HTTP body only releases this
    // bounded subscription; explicit Cancel remains the authority to cancel a run.
    let stream = async_stream::stream! {
        yield Ok::<_,std::convert::Infallible>(bytes::Bytes::from_static(b"["));
        yield Ok(encode(BackupProgressItem::message(id,"Queued",queued),restore,false));
        let mut refresh=tokio::time::interval(std::time::Duration::from_secs(15));
        let mut received_output=false;
        refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            let update=tokio::select! {
                _=state.cancellation.cancelled()=>break,
                _=refresh.tick()=>None,
                update=progress.recv()=>match update {
                    Ok(item) if item.run_id==id=>Some(item),
                    Ok(_)=>continue,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_))=>{received_output=false;None},
                    Err(tokio::sync::broadcast::error::RecvError::Closed)=>break,
                }
            };
            let authorized=state.identity.permission_for_resource(&principal,ResourceType::BackupPolicy,policy_id).await;
            if !authorized.ok().flatten().is_some_and(|p|p.level.grants(PermissionLevel::Execute) && (!restore || p.has_specific(citadel_domain::SpecificPermission::Restore))) {break;}
            if let Some(item)=update && !is_terminal(item.status.as_deref().unwrap_or("Running")) {
                received_output |= item.status.is_none();
                yield Ok(encode((*item).clone(),restore,true));
                continue;
            }
            // Recheck persisted state after lag/restart/completion. We never start a
            // second execution, and emit terminal success only after commit succeeds.
            let current=if restore {
                state.backups.store().get_restore(id).await.map(|r|(r.status,r.error_message,r.exit_code))
            } else {state.backups.store().get_run(id).await.map(|r|(r.status,r.error_message,r.exit_code))};
            let (status,error,exit)=match current {
                Ok(value)=>value,
                Err(_)=>{yield Ok(encode(BackupProgressItem::message(id,"Interrupted","Run status is unavailable. Reopen the run to inspect its persisted status."),restore,true));break;}
            };
            if is_terminal(&status) {
                let logs=if restore{state.backups.store().restore_logs(id).await}else{state.backups.store().backup_logs(id).await};
                if !received_output && let Ok(logs)=logs {for log in logs {yield Ok(encode(BackupProgressItem{run_id:id,status:None,message:Some(log.message),stream:Some(log.stream),exit_code:None},restore,true));}}
                yield Ok(encode(BackupProgressItem{run_id:id,status:Some(status),message:error,stream:None,exit_code:exit},restore,true));
                break;
            }
        }
        yield Ok(bytes::Bytes::from_static(b"]"));
    };
    let mut response = no_store(Body::from_stream(stream).into_response());
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json; charset=utf-8"),
    );
    response
}

fn encode(item: BackupProgressItem, restore: bool, comma: bool) -> bytes::Bytes {
    let mut value = serde_json::json!({"runId":item.run_id,"status":item.status,"message":item.message,"stream":item.stream,"exitCode":item.exit_code});
    if restore {
        let id = value
            .as_object_mut()
            .expect("progress object")
            .remove("runId");
        value["restoreRunId"] = id.unwrap_or_default();
    }
    let mut bytes = Vec::new();
    if comma {
        bytes.push(b',');
    }
    serde_json::to_writer(&mut bytes, &value).expect("progress contains JSON scalars");
    bytes.into()
}
