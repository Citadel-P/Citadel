use super::*;
pub(crate) async fn recover_interrupted(
    tx: &mut Transaction<'_, Postgres>,
    stale_before: DateTime<Utc>,
) -> Result<Vec<Uuid>, AutomationError> {
    recover_actions(tx, stale_before, false).await
}

pub(crate) async fn recover_interrupted_with_mode(
    tx: &mut Transaction<'_, Postgres>,
    stale_before: DateTime<Utc>,
    startup: bool,
) -> Result<u64, AutomationError> {
    Ok(recover_actions(tx, stale_before, startup).await?.len() as u64)
}

async fn recover_actions(
    tx: &mut Transaction<'_, Postgres>,
    stale_before: DateTime<Utc>,
    startup: bool,
) -> Result<Vec<Uuid>, AutomationError> {
    let rows = sqlx::query("UPDATE actionruns SET status='Failed',finishedat=CURRENT_TIMESTAMP,errormessage='Automation run was interrupted by a Core restart.' WHERE status='Running' AND ($2 OR startedat<$1) AND ($2 OR startedat+make_interval(secs=>timeoutseconds+300)<CURRENT_TIMESTAMP) RETURNING actionid,id").bind(stale_before).bind(startup).fetch_all(&mut **tx).await.map_err(storage)?;
    let mut changed = Vec::with_capacity(rows.len());
    for row in rows {
        let action: Uuid = row.try_get("actionid").map_err(storage)?;
        changed.push(action);
        let run: Uuid = row.try_get("id").map_err(storage)?;
        sqlx::query("UPDATE actions SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,rowversion=rowversion+1 WHERE id=$1 AND currentrunid=$2 AND NOT EXISTS(SELECT 1 FROM automationexecutorreservations WHERE runid=$2)").bind(action).bind(run).execute(&mut **tx).await.map_err(storage)?;
        let run = get_run_tx(tx, run).await?;
        add_run_activity(tx, &run).await?;
    }
    Ok(changed)
}
