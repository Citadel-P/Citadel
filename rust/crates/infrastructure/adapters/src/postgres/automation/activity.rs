use super::*;

pub(super) async fn add_activity(
    tx: &mut Transaction<'_, Postgres>,
    action: &AutomationAction,
    actor: ActorId,
    info: ActivityEventInfo,
) -> Result<(), AutomationError> {
    let event = ActivityEvent::new_automation_event(
        action.id,
        action.name.clone(),
        actor,
        info,
        Utc::now(),
    )
    .map_err(storage)?;
    crate::activity_store::insert_activity(tx, &event)
        .await
        .map_err(storage)
}

pub(super) async fn add_run_activity(
    tx: &mut Transaction<'_, Postgres>,
    run: &AutomationRun,
) -> Result<(), AutomationError> {
    let run_id = run.id;
    let trigger = run.trigger.clone();
    let duration_ms = run.duration_ms;
    let exit_code = run.exit_code;
    let info = match run.status.as_str() {
        "Queued" => ActivityEventInfo::ActionRunQueued { run_id, trigger },
        "Running" => ActivityEventInfo::ActionRunStarted { run_id, trigger },
        "Succeeded" => ActivityEventInfo::ActionRunSucceeded {
            run_id,
            trigger,
            exit_code,
            duration_ms,
        },
        "Failed" => ActivityEventInfo::ActionRunFailed {
            run_id,
            trigger,
            exit_code,
            duration_ms,
            error_message: run.error_message.clone(),
        },
        "TimedOut" => ActivityEventInfo::ActionRunTimedOut {
            run_id,
            trigger,
            duration_ms,
            error_message: run.error_message.clone(),
        },
        "Cancelled" => ActivityEventInfo::ActionRunCancelled { run_id, trigger },
        "Rejected" => ActivityEventInfo::ActionRunRejected {
            run_id,
            trigger,
            reason: run
                .error_message
                .clone()
                .unwrap_or_else(|| "Run rejected.".into()),
        },
        _ => {
            return Err(AutomationError::Storage(
                "Unknown Automation run status.".into(),
            ));
        }
    };
    let event = ActivityEvent::new_automation_event(
        run.action_id,
        run.action_name.clone(),
        ActorId::new(run.triggered_by_actor_id.unwrap_or(run.run_as_actor_id)),
        info,
        Utc::now(),
    )
    .map_err(storage)?;
    crate::activity_store::insert_activity(tx, &event)
        .await
        .map_err(storage)
}
