use super::*;
pub(super) async fn insert_deployment_activity(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    name: &str,
    platform_id: Uuid,
    actor_id: ActorId,
    info: ActivityEventInfo,
    now: DateTime<Utc>,
) -> Result<(), DeploymentError> {
    let activity =
        ActivityEvent::new_deployment_event(id, name.to_owned(), platform_id, actor_id, info, now)
            .map_err(|error| {
                DeploymentError::Storage(format!("invalid Deployment activity: {error}"))
            })?;
    insert_activity(tx, &activity)
        .await
        .map_err(|error| DeploymentError::Storage(error.to_string()))
}

pub(super) async fn insert_apply_activity(
    tx: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
    claim: &ApplyClaim,
    status: ActivityStatus,
    result: DeploymentResultActivitySnapshot,
    applied_spec: Option<&DeploymentSpec>,
) -> Result<(), DeploymentError> {
    let info = ActivityEventInfo::deployment_applied(
        Some(snapshot(
            claim.id,
            &claim.name,
            claim.platform_id,
            claim.description.clone(),
            applied_spec.unwrap_or(&claim.spec).to_storage_value()?,
        )),
        result,
    );
    let activity = ActivityEvent::new_deployment_result_event(
        claim.id,
        claim.name.clone(),
        claim.platform_id,
        actor_id,
        info,
        status,
        Utc::now(),
    )
    .map_err(|error| DeploymentError::Storage(format!("invalid Deployment activity: {error}")))?;
    insert_activity(tx, &activity)
        .await
        .map_err(|error| DeploymentError::Storage(error.to_string()))
}

pub(super) fn apply_result(
    container_ids: Option<Vec<String>>,
    message: Option<String>,
    bindings: &[DeploymentBindingSnapshot],
) -> Result<DeploymentResultActivitySnapshot, DeploymentError> {
    Ok(DeploymentResultActivitySnapshot {
        container_ids,
        message,
        resource_bindings: if bindings.is_empty() {
            None
        } else {
            Some(serde_json::to_value(bindings).map_err(storage)?)
        },
    })
}

pub(super) fn snapshot(
    id: Uuid,
    name: &str,
    platform_id: Uuid,
    description: Option<String>,
    spec: Value,
) -> DeploymentActivitySnapshot {
    DeploymentActivitySnapshot {
        id,
        name: name.to_owned(),
        platform_id,
        description,
        spec,
    }
}
