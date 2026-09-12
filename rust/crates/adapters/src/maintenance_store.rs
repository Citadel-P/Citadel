//! Maintenance runs independently of dispatch being enabled and of new statistics writes.
use sqlx::PgPool;

pub async fn reconcile(pool: &PgPool) -> Result<Vec<&'static str>, sqlx::Error> {
    let mut changed = Vec::new();
    let mut tx = pool.begin().await?;
    // Existing recovery predicates protect live runs until their execution deadlines.
    let before = chrono::Utc::now();
    if crate::automation_store::recover_interrupted(&mut tx, before)
        .await
        .map_err(protocol)?
        > 0
    {
        changed.extend(["AutomationAction"]);
    }
    if crate::build_store::recover(&mut tx, before)
        .await
        .map_err(protocol)?
        > 0
    {
        changed.extend(["BuildProject", "BuildRun"]);
    }
    if crate::backup_store::recover_stale(&mut tx, before)
        .await
        .map_err(protocol)?
        > 0
    {
        changed.extend([
            "BackupRepository",
            "BackupPolicy",
            "BackupRun",
            "BackupRestoreRun",
        ]);
    }
    for (resource, sql) in [
        (
            "Deployment",
            "UPDATE deployments SET status='Unknown',controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1 WHERE controlstate='Processing' AND controlstartedat<EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-3600 AND containeroperationid IS NULL",
        ),
        (
            "Stack",
            "WITH stuck AS (UPDATE stacks SET controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1 WHERE controlstate='Processing' AND controlstartedat<EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-3600 AND containeroperationid IS NULL RETURNING currentstackreleaseid) UPDATE stackreleases SET status='Unknown' WHERE id IN(SELECT currentstackreleaseid FROM stuck)",
        ),
        (
            "Container",
            "UPDATE containers SET controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1 WHERE controlstate='Processing' AND containeroperationid IS NULL AND controlstartedat<EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-60",
        ),
        (
            "GitRepository",
            "UPDATE gitrepositories SET status='Degraded',controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1 WHERE controlstate='Processing' AND controlstartedat<EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-3600",
        ),
        (
            "Image",
            "UPDATE images SET controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1 WHERE controlstate='Processing' AND controlstartedat<EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-60",
        ),
        (
            "AutomationAction",
            "UPDATE actions a SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,rowversion=rowversion+1 WHERE controlstate='Processing' AND NOT EXISTS(SELECT 1 FROM actionruns r WHERE r.id=a.currentrunid AND r.status IN('Queued','Running'))",
        ),
        (
            "BuildProject",
            "UPDATE buildprojects p SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,rowversion=rowversion+1 WHERE controlstate='Processing' AND NOT EXISTS(SELECT 1 FROM buildruns r WHERE r.id=p.currentrunid AND r.status IN('Queued','Preparing','Running'))",
        ),
        (
            "BuildAgentPool",
            "UPDATE buildagentpools SET controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1 WHERE controlstate='Processing' AND controlstartedat<EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-300",
        ),
    ] {
        if sqlx::query(sql).execute(&mut *tx).await?.rows_affected() > 0 {
            changed.push(resource);
        }
    }
    tx.commit().await?;
    changed.sort_unstable();
    changed.dedup();
    Ok(changed)
}

pub async fn cleanup(pool: &PgPool, build_retention_days: Option<i32>) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    // Bound each transaction. Repeated scheduled batches drain backlogs while keeping
    // locks short and retaining every active run and referenced build artifact.
    for sql in [
        "DELETE FROM containerstats WHERE id IN(SELECT id FROM containerstats WHERE created<EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-604800 LIMIT 5000)",
        "DELETE FROM platformstats WHERE id IN(SELECT id FROM platformstats WHERE created<EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-604800 LIMIT 5000)",
        "DELETE FROM swarmservicestats WHERE id IN(SELECT id FROM swarmservicestats WHERE created<EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-604800 LIMIT 5000)",
        "DELETE FROM activityevents WHERE id IN(SELECT id FROM activityevents WHERE createdat<CURRENT_TIMESTAMP-INTERVAL '90 days' LIMIT 5000)",
        "DELETE FROM actionruns WHERE id IN(SELECT r.id FROM actionruns r WHERE finishedat<CURRENT_TIMESTAMP-INTERVAL '90 days' AND status NOT IN('Queued','Running') AND NOT EXISTS(SELECT 1 FROM actions a WHERE a.currentrunid=r.id) LIMIT 5000)",
        "DELETE FROM refreshtokens WHERE id IN(SELECT id FROM refreshtokens WHERE expiresat<CURRENT_TIMESTAMP LIMIT 5000)",
        "DELETE FROM backuprepositoryleases WHERE expiresat<CURRENT_TIMESTAMP",
        "DELETE FROM backupsourceleases WHERE expiresat<CURRENT_TIMESTAMP",
    ] {
        sqlx::query(sql).execute(&mut *tx).await?;
    }
    if let Some(days) = build_retention_days.filter(|days| *days > 0) {
        sqlx::query("DELETE FROM buildruns b WHERE b.id IN(SELECT r.id FROM buildruns r WHERE r.completedat<CURRENT_TIMESTAMP-make_interval(days=>$1) AND r.status IN('Succeeded','Failed','TimedOut','Cancelled','Interrupted') AND NOT EXISTS(SELECT 1 FROM buildcompletionqueue q WHERE q.buildrunid=r.id) AND NOT EXISTS(SELECT 1 FROM buildprojects p WHERE p.currentrunid=r.id) AND NOT EXISTS(SELECT 1 FROM deployments d WHERE jsonb_path_exists(d.spec::jsonb, '$.** ? (@ == $id)', jsonb_build_object('id',r.id::text))) AND NOT EXISTS(SELECT 1 FROM stackreleases s WHERE jsonb_path_exists(s.spec::jsonb, '$.** ? (@ == $id)', jsonb_build_object('id',r.id::text))) ORDER BY r.completedat LIMIT 1000)")
            .bind(days).execute(&mut *tx).await?;
    }
    tx.commit().await
}
fn protocol(error: impl std::fmt::Display) -> sqlx::Error {
    sqlx::Error::Protocol(error.to_string())
}

/// Only call while holding the exclusive Core job lease, before dispatch starts.
pub async fn recover_on_startup(pool: &PgPool) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    let before = chrono::Utc::now();
    crate::automation_store::recover_interrupted_with_mode(&mut tx, before, true)
        .await
        .map_err(protocol)?;
    crate::backup_store::recover_stale_with_mode(&mut tx, before, true)
        .await
        .map_err(protocol)?;
    crate::build_store::recover_with_mode(&mut tx, before, true)
        .await
        .map_err(protocol)?;
    tx.commit().await
}
