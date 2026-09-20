//! Operation outcomes are established by a complete, identity-validated manager
//! snapshot. A failed inspection is never evidence that a Service is absent.
use chrono::{DateTime, Utc};
use citadel_activities::{ActivityEventInfo, ActivityStatus};
use citadel_platforms::{RuntimeInventorySnapshot, RuntimeSwarmInventory, RuntimeSwarmService};
use citadel_primitives::ActorId;
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

struct Operation<'a> {
    id: Uuid,
    state: &'a str,
    kind: &'a str,
    attempted: Option<DateTime<Utc>>,
    completed: Option<DateTime<Utc>>,
    base_version: Option<i64>,
    force_update: Option<i64>,
    target_hash: Option<&'a str>,
}
#[derive(Debug, PartialEq)]
enum Outcome {
    Wait,
    Accepted,
    Complete,
    Failure(&'static str, &'static str, String),
}
fn evaluate(
    op: &Operation<'_>,
    live: Option<&RuntimeSwarmService>,
    tasks: &[citadel_platforms::RuntimeSwarmTask],
    now: DateTime<Utc>,
) -> Outcome {
    if op.state == "Prepared" && op.attempted.is_none() {
        return Outcome::Failure(
            "Canceled",
            "UndispatchedOperation",
            "The undispatched operation was canceled during reconciliation.".into(),
        );
    }
    if !matches!(
        op.state,
        "PendingAcceptance" | "Accepted" | "OutcomeUnknown"
    ) {
        return Outcome::Wait;
    }
    if op.kind == "Delete" && live.is_none() {
        return Outcome::Complete;
    }
    let identity = live.is_some_and(|service| {
        service
            .labels
            .get("com.citadel.operation-id")
            .and_then(|value| Uuid::parse_str(value).ok())
            == Some(op.id)
    });
    let accepted = identity
        && live.is_some_and(|service| {
            op.base_version
                .is_none_or(|base| service.version_index > base)
                && (op.kind != "ForceUpdate"
                    || op
                        .force_update
                        .is_none_or(|force| service.force_update >= force))
        });
    if accepted && op.kind != "Delete" {
        let live = live.unwrap();
        let task_error = tasks
            .iter()
            .find(|task| {
                task.service_id == live.id
                    && !task.desired_state.eq_ignore_ascii_case("shutdown")
                    && !task.desired_state.eq_ignore_ascii_case("remove")
                    && task
                        .error
                        .as_ref()
                        .is_some_and(|error| !error.trim().is_empty())
            })
            .and_then(|task| task.error.clone());
        let complete_state = citadel_platforms::jobs::rollout_complete_state(&live.update_state);
        if citadel_platforms::jobs::rollout_paused(&live.update_state) {
            return Outcome::Failure(
                "Rejected",
                "RolloutPaused",
                task_error
                    .or_else(|| live.update_message.clone())
                    .unwrap_or_else(|| "Docker paused the Service rollout.".into()),
            );
        }
        if complete_state
            && live.running_task_count < live.desired_task_count
            && let Some(error) = task_error
        {
            return Outcome::Failure("Rejected", "TaskFailed", error);
        }
        if op
            .target_hash
            .is_some_and(|hash| !hash.is_empty() && hash == live.runtime_hash)
        {
            return if complete_state && live.running_task_count >= live.desired_task_count {
                Outcome::Complete
            } else {
                Outcome::Accepted
            };
        }
    }
    let Some(attempted) = op.attempted else {
        return Outcome::Wait;
    };
    let not_accepted_after = match op.state {
        "PendingAcceptance" => Some(attempted + chrono::Duration::seconds(150)),
        "OutcomeUnknown" => op.completed.map(|at| at + chrono::Duration::seconds(30)),
        _ => None,
    };
    if not_accepted_after.is_some_and(|barrier| now >= barrier) {
        let unchanged = if op.kind == "Delete" {
            live.is_some_and(|s| Some(s.version_index) == op.base_version)
        } else if op.base_version.is_none() {
            live.is_none()
        } else {
            live.is_some_and(|s| Some(s.version_index) == op.base_version) && !identity
        };
        if unchanged {
            return Outcome::Failure(
                "NotAccepted",
                "NotAccepted",
                "A complete Swarm observation proved that Docker did not accept the operation."
                    .into(),
            );
        }
    }
    let grace_start = if op.state == "OutcomeUnknown" {
        op.completed.unwrap_or(attempted)
    } else {
        attempted
    };
    if now >= grace_start + chrono::Duration::seconds(150) {
        let (code, message) = if op.kind == "Delete" {
            (
                "DeleteStillPresentAfterAcceptance",
                "Docker accepted the delete, but the Service is still present after the observation grace period.",
            )
        } else if live.is_none() {
            (
                "RuntimeMissingAfterAcceptance",
                "Docker accepted the operation, but the Service was not present after the observation grace period.",
            )
        } else if !identity {
            (
                "AcceptanceCouldNotBeObserved",
                "Docker accepted the operation, but the observed Service does not contain its operation identity.",
            )
        } else {
            (
                "AcceptedRuntimeMismatch",
                "Docker accepted the operation, but the observed Service configuration does not match the requested configuration.",
            )
        };
        return Outcome::Failure("Rejected", code, message.into());
    }
    if accepted {
        Outcome::Accepted
    } else {
        Outcome::Wait
    }
}

pub(crate) async fn reconcile(
    tx: &mut Transaction<'_, Postgres>,
    snapshot: &RuntimeInventorySnapshot,
    swarm: &RuntimeSwarmInventory,
) -> Result<(), citadel_swarm_services::SwarmServiceError> {
    // Upgrade only when the live raw spec still exactly matches the previously
    // applied raw hash. Never adopt an externally changed configuration as applied.
    sqlx::query("UPDATE swarmservices s SET lastappliedruntimehash=live.runtime_hash FROM jsonb_to_recordset($2) AS live(id text,runtime_hash text,legacy_runtime_hash text) WHERE s.platformid=$1 AND s.dockerserviceid=live.id AND s.lastappliedruntimehash=live.legacy_runtime_hash AND s.controlstate='Idle'")
        .bind(snapshot.platform_id).bind(serde_json::to_value(&swarm.services).map_err(storage)?)
        .execute(&mut **tx).await.map_err(storage)?;
    let rows = sqlx::query("SELECT s.* FROM swarmservices s JOIN platforms p ON p.id=s.platformid WHERE s.platformid=$1 AND s.operationstate IN ('Prepared','PendingAcceptance','Accepted','OutcomeUnknown') AND s.operationclusterid=p.clusterid AND s.preparedat<$2 - interval '2 seconds' AND (s.operationstate<>'Prepared' OR s.preparedat<$2 - interval '150 seconds') ORDER BY s.id FOR UPDATE OF s")
        .bind(snapshot.platform_id).bind(snapshot.observed_at).fetch_all(&mut **tx).await.map_err(storage)?;
    for row in rows {
        let id: Uuid = row.try_get("id").map_err(storage)?;
        let operation_id: Uuid = row.try_get("operationid").map_err(storage)?;
        let state: String = row.try_get("operationstate").map_err(storage)?;
        let kind: String = row.try_get("operationkind").map_err(storage)?;
        let hash: Option<String> = row.try_get("targetruntimehash").map_err(storage)?;
        let docker_id: Option<String> = row.try_get("dockerserviceid").map_err(storage)?;
        let matching: Vec<_> = swarm
            .services
            .iter()
            .filter(|service| {
                service.swarm_service_id == Some(id)
                    || (service.stack_id.is_none()
                        && service.swarm_service_id.is_none()
                        && docker_id.as_deref() == Some(&service.id))
            })
            .collect();
        let live = matching.first().copied();
        let conflict = matching.len() > 1
            || live.is_some_and(|s| docker_id.as_deref().is_some_and(|id| id != s.id));
        let outcome = if conflict {
            Outcome::Failure(
                "OwnershipConflict",
                "OwnershipConflict",
                "Docker Service ownership is ambiguous.".into(),
            )
        } else {
            evaluate(
                &Operation {
                    id: operation_id,
                    state: &state,
                    kind: &kind,
                    attempted: row.try_get("attemptedat").map_err(storage)?,
                    completed: row.try_get("completedat").map_err(storage)?,
                    base_version: row.try_get("basedockerversion").map_err(storage)?,
                    force_update: row.try_get("expectedforceupdate").map_err(storage)?,
                    target_hash: hash.as_deref(),
                },
                live,
                &swarm.tasks,
                snapshot.observed_at,
            )
        };
        let actor = ActorId::new(row.try_get("operationactorid").map_err(storage)?);
        let mut activity = None;
        match outcome {
            Outcome::Wait => continue,
            Outcome::Accepted => {
                let live = live.unwrap();
                sqlx::query("UPDATE swarmservices SET operationstate='Accepted',dockerserviceid=$2,dockerversionindex=$3,observeddockerversion=$3,rowversion=rowversion+1 WHERE id=$1 AND (operationstate<>'Accepted' OR dockerserviceid IS DISTINCT FROM $2 OR dockerversionindex IS DISTINCT FROM $3)")
                    .bind(id).bind(&live.id).bind(live.version_index).execute(&mut **tx).await.map_err(storage)?;
            }
            Outcome::Complete if kind == "Delete" => {
                let service =
                    crate::persistence::postgres::swarm_services::activity_snapshot(&row)?;
                activity = Some((
                    ActivityEventInfo::swarm_service_deleted(service),
                    ActivityStatus::Success,
                ));
                sqlx::query("DELETE FROM swarmservices WHERE id=$1")
                    .bind(id)
                    .execute(&mut **tx)
                    .await
                    .map_err(storage)?;
            }
            Outcome::Complete => {
                let live = live.unwrap();
                let spec = citadel_swarm_services::SwarmServiceSpec::from_storage_value(
                    row.try_get("spec").map_err(storage)?,
                )?;
                sqlx::query("UPDATE swarmservices SET dockerserviceid=$2,dockerversionindex=$3,observeddockerversion=$3,operationstate='Completed',completedat=$4,lastapplieddesiredspechash=targetdesiredspechash,lastappliedruntimehash=$5,appliedimagedigest=COALESCE($6,appliedimagedigest),health=CASE WHEN $7=0 THEN 'Stopped' ELSE 'Healthy' END,synchronizationstate='InSync',controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1 WHERE id=$1")
                    .bind(id).bind(&live.id).bind(live.version_index).bind(snapshot.observed_at).bind(&live.runtime_hash).bind(live.image.split_once('@').map(|(_,digest)| digest)).bind(live.desired_task_count)
                    .execute(&mut **tx).await.map_err(storage)?;
                activity = Some((
                    ActivityEventInfo::swarm_service_completed(
                        &kind,
                        operation_id,
                        spec.replicas,
                        vec![],
                    ),
                    ActivityStatus::Success,
                ));
            }
            Outcome::Failure(state, code, message) => {
                sqlx::query("UPDATE swarmservices SET operationstate=$2,resultcode=$3,resultmessage=$4,completedat=$5,health='Failed',synchronizationstate=CASE WHEN $2='OwnershipConflict' THEN 'OwnershipConflict' ELSE 'DesiredChangesPending' END,controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1 WHERE id=$1")
                    .bind(id).bind(state).bind(code).bind(&message).bind(snapshot.observed_at).execute(&mut **tx).await.map_err(storage)?;
                activity = Some((
                    ActivityEventInfo::swarm_service_operation_failed(
                        operation_id,
                        kind.clone(),
                        message.clone(),
                    ),
                    ActivityStatus::Failure,
                ));
                let reason = message.chars().take(512).collect::<String>();
                let name: String = row.try_get("name").map_err(storage)?;
                let observation = citadel_alerts::AlertObservation {
                    alert_type: "SwarmServiceOperationFailed".into(),
                    info: serde_json::json!({"HumanMessage":format!("Swarm Service '{name}' {} failed: {reason}",kind.to_lowercase()),"ServiceName":name,"OperationKind":kind,"Reason":reason,"OperationId":operation_id}),
                    resource_id: id,
                    resource_name: row.try_get("name").map_err(storage)?,
                    resource_type: "SwarmService".into(),
                    deduplication_component: operation_id.simple().to_string(),
                    observed_at: snapshot.observed_at,
                    value: None,
                    matched: true,
                };
                sqlx::query("SELECT pg_notify('citadel_job_alerts',$1)")
                    .bind(serde_json::to_string(&observation).map_err(storage)?)
                    .execute(&mut **tx)
                    .await
                    .map_err(storage)?;
            }
        }
        if let Some((info, status)) = activity {
            crate::persistence::postgres::swarm_services::insert_swarm_activity(
                tx,
                id,
                &row.try_get::<String, _>("name").map_err(storage)?,
                snapshot.platform_id,
                actor,
                info,
                status,
            )
            .await?;
        }
    }
    Ok(())
}
fn storage(error: impl std::fmt::Display) -> citadel_swarm_services::SwarmServiceError {
    citadel_swarm_services::SwarmServiceError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn operation(now: DateTime<Utc>) -> Operation<'static> {
        Operation {
            id: Uuid::now_v7(),
            state: "PendingAcceptance",
            kind: "Apply",
            attempted: Some(now - chrono::Duration::seconds(151)),
            completed: None,
            base_version: Some(10),
            force_update: None,
            target_hash: Some("target"),
        }
    }
    fn live(op: &Operation<'_>) -> RuntimeSwarmService {
        RuntimeSwarmService {
            id: "service".into(),
            version_index: 11,
            runtime_hash: "target".into(),
            desired_task_count: 1,
            running_task_count: 1,
            update_state: "Completed".into(),
            labels: [("com.citadel.operation-id".into(), op.id.to_string())].into(),
            ..Default::default()
        }
    }
    #[test]
    fn complete_post_grace_snapshot_proves_only_unchanged_ambiguous_operations_not_accepted() {
        let now = Utc::now();
        let op = operation(now);
        let mut service = live(&op);
        service.version_index = 10;
        service.labels.clear();
        assert!(matches!(
            evaluate(&op, Some(&service), &[], now),
            Outcome::Failure("NotAccepted", "NotAccepted", _)
        ));
        service.version_index = 11;
        assert!(matches!(
            evaluate(&op, Some(&service), &[], now),
            Outcome::Failure("Rejected", "AcceptanceCouldNotBeObserved", _)
        ));
        assert_eq!(
            evaluate(&op, Some(&service), &[], now - chrono::Duration::seconds(2)),
            Outcome::Wait
        );
    }
    #[test]
    fn accepted_missing_and_mismatched_runtime_fail_after_grace() {
        let now = Utc::now();
        let mut op = operation(now);
        op.state = "Accepted";
        assert!(matches!(
            evaluate(&op, None, &[], now),
            Outcome::Failure("Rejected", "RuntimeMissingAfterAcceptance", _)
        ));
        let mut service = live(&op);
        service.runtime_hash = "other".into();
        assert!(matches!(
            evaluate(&op, Some(&service), &[], now),
            Outcome::Failure("Rejected", "AcceptedRuntimeMismatch", _)
        ));
        assert_eq!(
            evaluate(&op, Some(&service), &[], now - chrono::Duration::seconds(2)),
            Outcome::Accepted
        );
    }
    #[test]
    fn updating_rollout_remains_accepted_without_success_even_after_grace() {
        let now = Utc::now();
        let op = operation(now);
        let mut service = live(&op);
        service.update_state = "Updating".into();
        assert_eq!(evaluate(&op, Some(&service), &[], now), Outcome::Accepted);
        service.update_state = "Completed".into();
        assert_eq!(evaluate(&op, Some(&service), &[], now), Outcome::Complete);
    }
    #[test]
    fn retired_task_errors_do_not_reject_the_current_rollout() {
        let now = Utc::now();
        let op = operation(now);
        let mut service = live(&op);
        service.running_task_count = 0;
        for desired in ["shutdown", "Remove", "SHUTDOWN"] {
            let task = citadel_platforms::RuntimeSwarmTask {
                service_id: service.id.clone(),
                desired_state: desired.into(),
                error: Some("old image pull failure".into()),
                ..Default::default()
            };
            assert_eq!(
                evaluate(&op, Some(&service), &[task], now),
                Outcome::Accepted
            );
        }
    }
    #[test]
    fn paused_and_completed_failed_rollouts_use_task_error() {
        let now = Utc::now();
        let op = operation(now);
        let mut service = live(&op);
        let task = citadel_platforms::RuntimeSwarmTask {
            service_id: service.id.clone(),
            error: Some("image pull failed".into()),
            ..Default::default()
        };
        service.update_state = "Paused".into();
        assert_eq!(
            evaluate(&op, Some(&service), std::slice::from_ref(&task), now),
            Outcome::Failure("Rejected", "RolloutPaused", "image pull failed".into())
        );
        service.update_state = "Completed".into();
        service.running_task_count = 0;
        assert_eq!(
            evaluate(&op, Some(&service), &[task], now),
            Outcome::Failure("Rejected", "TaskFailed", "image pull failed".into())
        );
    }
    #[test]
    fn missing_runtime_satisfies_dispatched_delete_but_not_undispatched_operation() {
        let now = Utc::now();
        let mut op = operation(now);
        op.kind = "Delete";
        for state in ["PendingAcceptance", "Accepted", "OutcomeUnknown"] {
            op.state = state;
            assert_eq!(evaluate(&op, None, &[], now), Outcome::Complete);
        }
        op.state = "Prepared";
        op.attempted = None;
        assert!(matches!(
            evaluate(&op, None, &[], now),
            Outcome::Failure("Canceled", "UndispatchedOperation", _)
        ));
    }
    #[test]
    fn unknown_create_waits_thirty_seconds_and_force_update_requires_expected_counter() {
        let now = Utc::now();
        let mut op = operation(now);
        op.state = "OutcomeUnknown";
        op.base_version = None;
        op.completed = Some(now - chrono::Duration::seconds(29));
        assert_eq!(evaluate(&op, None, &[], now), Outcome::Wait);
        assert!(matches!(
            evaluate(&op, None, &[], now + chrono::Duration::seconds(1)),
            Outcome::Failure("NotAccepted", "NotAccepted", _)
        ));
        op.kind = "ForceUpdate";
        op.state = "PendingAcceptance";
        op.force_update = Some(2);
        let mut service = live(&op);
        service.force_update = 1;
        assert!(!matches!(
            evaluate(&op, Some(&service), &[], now),
            Outcome::Complete
        ));
        service.force_update = 2;
        assert_eq!(evaluate(&op, Some(&service), &[], now), Outcome::Complete);
    }
}
