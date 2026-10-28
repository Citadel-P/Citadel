use super::*;

pub(super) async fn add_activity(
    tx: &mut Transaction<'_, Postgres>,
    script: &Script,
    actor: ActorId,
    info: ActivityEventInfo,
) -> Result<(), ScriptError> {
    let event =
        ActivityEvent::new_script_event(script.id, script.name.clone(), actor, info, Utc::now())
            .map_err(storage)?;
    crate::persistence::postgres::activities::store::insert_activity(tx, &event)
        .await
        .map_err(storage)
}

pub(super) async fn add_run_activity(
    tx: &mut Transaction<'_, Postgres>,
    run: &ScriptRun,
) -> Result<(), ScriptError> {
    let run_id = run.id;
    let trigger = run.trigger.clone();
    let duration_ms = run.duration_ms;
    let exit_code = run.exit_code;
    let info = match run.status {
        citadel_scripts::ScriptRunStatus::Queued => {
            ActivityEventInfo::ScriptRunQueued { run_id, trigger }
        }
        citadel_scripts::ScriptRunStatus::Running => {
            ActivityEventInfo::ScriptRunStarted { run_id, trigger }
        }
        citadel_scripts::ScriptRunStatus::Succeeded => ActivityEventInfo::ScriptRunSucceeded {
            run_id,
            trigger,
            exit_code,
            duration_ms,
        },
        citadel_scripts::ScriptRunStatus::Failed => ActivityEventInfo::ScriptRunFailed {
            run_id,
            trigger,
            exit_code,
            duration_ms,
            error_message: run.error_message.clone(),
        },
        citadel_scripts::ScriptRunStatus::TimedOut => ActivityEventInfo::ScriptRunTimedOut {
            run_id,
            trigger,
            duration_ms,
            error_message: run.error_message.clone(),
        },
        citadel_scripts::ScriptRunStatus::Cancelled => {
            ActivityEventInfo::ScriptRunCancelled { run_id, trigger }
        }
        citadel_scripts::ScriptRunStatus::Rejected => ActivityEventInfo::ScriptRunRejected {
            run_id,
            trigger,
            reason: run
                .error_message
                .clone()
                .unwrap_or_else(|| "Run rejected.".into()),
        },
    };
    let event = ActivityEvent::new_script_event(
        run.script_id,
        run.script_name.clone(),
        ActorId::new(run.triggered_by_actor_id.unwrap_or(run.run_as_actor_id)),
        info,
        Utc::now(),
    )
    .map_err(storage)?;
    crate::persistence::postgres::activities::store::insert_activity(tx, &event)
        .await
        .map_err(storage)
}
