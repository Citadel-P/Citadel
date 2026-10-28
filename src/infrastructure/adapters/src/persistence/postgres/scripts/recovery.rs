use super::*;
pub(crate) async fn recover_interrupted(
    tx: &mut Transaction<'_, Postgres>,
    stale_before: DateTime<Utc>,
) -> Result<Vec<Uuid>, ScriptError> {
    recover_actions(tx, stale_before, false).await
}

pub(crate) async fn recover_interrupted_with_mode(
    tx: &mut Transaction<'_, Postgres>,
    stale_before: DateTime<Utc>,
    startup: bool,
) -> Result<u64, ScriptError> {
    Ok(recover_actions(tx, stale_before, startup).await?.len() as u64)
}

async fn recover_actions(
    tx: &mut Transaction<'_, Postgres>,
    stale_before: DateTime<Utc>,
    startup: bool,
) -> Result<Vec<Uuid>, ScriptError> {
    let rows = sqlx::query("UPDATE scriptruns SET status='Failed',finishedat=CURRENT_TIMESTAMP,errormessage='Script run was interrupted by a Core restart.' WHERE status='Running' AND ($2 OR startedat<$1) AND ($2 OR startedat+make_interval(secs=>timeoutseconds+300)<CURRENT_TIMESTAMP) RETURNING scriptid,id").bind(stale_before).bind(startup).fetch_all(&mut **tx).await.map_err(storage)?;
    let mut changed = Vec::with_capacity(rows.len());
    for row in rows {
        let script: Uuid = row.try_get("scriptid").map_err(storage)?;
        changed.push(script);
        let run: Uuid = row.try_get("id").map_err(storage)?;
        sqlx::query("UPDATE scripts SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,rowversion=rowversion+1 WHERE id=$1 AND currentrunid=$2 AND NOT EXISTS(SELECT 1 FROM scriptexecutorreservations WHERE runid=$2)").bind(script).bind(run).execute(&mut **tx).await.map_err(storage)?;
        let run = get_run_tx(tx, run).await?;
        add_run_activity(tx, &run).await?;
    }
    Ok(changed)
}
