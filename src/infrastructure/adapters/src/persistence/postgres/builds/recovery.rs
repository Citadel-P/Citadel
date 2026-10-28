use super::*;

pub(crate) async fn recover(
    tx: &mut Transaction<'_, Postgres>,
    stale_before: DateTime<Utc>,
) -> Result<u64, BuildError> {
    recover_with_mode(tx, stale_before, false).await
}

pub(crate) async fn recover_with_mode(
    tx: &mut Transaction<'_, Postgres>,
    stale_before: DateTime<Utc>,
    startup: bool,
) -> Result<u64, BuildError> {
    let rows=sqlx::query("UPDATE buildruns SET status='Interrupted',completedat=CURRENT_TIMESTAMP,errorcode='build.interrupted',errormessage='Build interrupted by Core restart.' WHERE status IN ('Preparing','Running') AND ($2 OR startedat<$1) AND ($2 OR startedat+make_interval(secs=>timeoutseconds+300)<CURRENT_TIMESTAMP) RETURNING id,buildprojectid").bind(stale_before).bind(startup).fetch_all(&mut **tx).await.map_err(storage)?;
    let changed = rows.len() as u64;
    for row in rows {
        let id: Uuid = row.try_get("id").map_err(storage)?;
        let project: Uuid = row.try_get("buildprojectid").map_err(storage)?;
        sqlx::query("DELETE FROM buildexecutorreservations reservation USING buildexecutorcleanupreceipts receipt WHERE reservation.runid=$1 AND receipt.runid=reservation.runid AND receipt.attemptid=reservation.attemptid AND receipt.daemonid=reservation.daemonid AND receipt.authorityid=reservation.authorityid AND receipt.admissiongeneration=reservation.admissiongeneration")
            .bind(id).execute(&mut **tx).await.map_err(storage)?;
        sqlx::query("DELETE FROM buildexecutorreservations WHERE runid=$1 AND NOT dispatchstarted")
            .bind(id)
            .execute(&mut **tx)
            .await
            .map_err(storage)?;
        sqlx::query("UPDATE buildexecutorreservations SET uncertain=true WHERE runid=$1")
            .bind(id)
            .execute(&mut **tx)
            .await
            .map_err(storage)?;
        sqlx::query("UPDATE buildruns SET errorcode='build.cleanup_pending',errormessage='Build interrupted. Executor capacity remains reserved until cleanup is verified.' WHERE id=$1 AND EXISTS(SELECT 1 FROM buildexecutorreservations WHERE runid=$1)")
            .bind(id).execute(&mut **tx).await.map_err(storage)?;
        record_run_activity(tx, id).await?;
        sqlx::query("UPDATE buildprojects SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,rowversion=rowversion+1 WHERE id=$1 AND currentrunid=$2").bind(project).bind(id).execute(&mut **tx).await.map_err(storage)?;
    }
    Ok(changed)
}
