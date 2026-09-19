//! Maintenance runs independently of dispatch being enabled and of new statistics writes.
use sqlx::PgPool;

pub async fn reconcile(pool: &PgPool) -> Result<Vec<&'static str>, sqlx::Error> {
    let _iteration = citadel_application::runtime_metrics::RuntimeWork::Recovery.start();
    let mut changed = Vec::new();
    let mut tx = pool.begin().await?;
    // Existing recovery predicates protect live runs until their execution deadlines.
    let before = chrono::Utc::now();
    if crate::postgres::automation::recover_interrupted(&mut tx, before)
        .await
        .map_err(protocol)?
        > 0
    {
        changed.extend(["AutomationAction"]);
    }
    if crate::postgres::builds::recover(&mut tx, before)
        .await
        .map_err(protocol)?
        > 0
    {
        changed.extend(["BuildProject", "BuildRun"]);
    }
    if crate::postgres::backups::recover_stale(&mut tx, before)
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

pub async fn cleanup(pool: &PgPool, build_retention_days: Option<i32>) -> Result<u64, sqlx::Error> {
    let _iteration = citadel_application::runtime_metrics::RuntimeWork::Retention.start();
    let mut deleted = 0;
    // Bound each transaction. Repeated scheduled batches drain backlogs while keeping
    // locks short and retaining every active run and referenced build artifact.
    for sql in [
        "DELETE FROM containerstats WHERE id IN(SELECT id FROM containerstats WHERE created<EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-604800 ORDER BY created LIMIT 5000 FOR UPDATE SKIP LOCKED)",
        "DELETE FROM platformstats WHERE id IN(SELECT id FROM platformstats WHERE NOT alertpending AND created<EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-604800 ORDER BY created LIMIT 5000 FOR UPDATE SKIP LOCKED)",
        "DELETE FROM swarmservicestats WHERE id IN(SELECT id FROM swarmservicestats WHERE created<EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-604800 ORDER BY created LIMIT 5000 FOR UPDATE SKIP LOCKED)",
        "DELETE FROM activityevents WHERE id IN(SELECT id FROM activityevents WHERE createdat<CURRENT_TIMESTAMP-INTERVAL '90 days' LIMIT 5000)",
        "DELETE FROM actionruns WHERE id IN(SELECT r.id FROM actionruns r WHERE finishedat<CURRENT_TIMESTAMP-INTERVAL '90 days' AND status NOT IN('Queued','Running') AND NOT EXISTS(SELECT 1 FROM actions a WHERE a.currentrunid=r.id) LIMIT 5000)",
    ] {
        deleted += sqlx::query(sql).execute(pool).await?.rows_affected();
    }
    if let Some(days) = build_retention_days.filter(|days| *days > 0) {
        deleted += sqlx::query("DELETE FROM buildruns b WHERE b.id IN(SELECT r.id FROM buildruns r WHERE r.completedat<CURRENT_TIMESTAMP-make_interval(days=>$1) AND r.status IN('Succeeded','Failed','TimedOut','Cancelled','Interrupted') AND NOT EXISTS(SELECT 1 FROM buildcompletionqueue q WHERE q.buildrunid=r.id) AND NOT EXISTS(SELECT 1 FROM buildprojects p WHERE p.currentrunid=r.id) AND NOT EXISTS(SELECT 1 FROM deployments d WHERE jsonb_path_exists(d.spec::jsonb, '$.** ? (@ == $id)', jsonb_build_object('id',r.id::text))) AND NOT EXISTS(SELECT 1 FROM stackreleases s WHERE jsonb_path_exists(s.spec::jsonb, '$.** ? (@ == $id)', jsonb_build_object('id',r.id::text))) ORDER BY r.completedat LIMIT 1000)")
            .bind(days).execute(pool).await?.rows_affected();
    }
    citadel_application::runtime_metrics::RuntimeWork::Retention.units(deleted);
    Ok(deleted)
}
/// Short-lived leases and refresh tokens have a separate expiry cadence.
pub async fn expire_leases(pool: &PgPool) -> Result<(), sqlx::Error> {
    let _iteration = citadel_application::runtime_metrics::RuntimeWork::LeaseExpiry.start();
    for sql in [
        "DELETE FROM refreshtokens WHERE id IN(SELECT id FROM refreshtokens WHERE expiresat<CURRENT_TIMESTAMP ORDER BY expiresat LIMIT 5000)",
        "DELETE FROM backuprepositoryleases WHERE ctid IN(SELECT ctid FROM backuprepositoryleases WHERE expiresat<CURRENT_TIMESTAMP LIMIT 5000)",
        "DELETE FROM backupsourceleases WHERE ctid IN(SELECT ctid FROM backupsourceleases WHERE expiresat<CURRENT_TIMESTAMP LIMIT 5000)",
    ] {
        let count = sqlx::query(sql).execute(pool).await?.rows_affected();
        citadel_application::runtime_metrics::RuntimeWork::LeaseExpiry.units(count);
    }
    Ok(())
}

fn protocol(error: impl std::fmt::Display) -> sqlx::Error {
    sqlx::Error::Protocol(error.to_string())
}

/// Only call while holding the exclusive Core job lease, before dispatch starts.
pub async fn recover_on_startup(pool: &PgPool) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    let before = chrono::Utc::now();
    crate::postgres::automation::recover_interrupted_with_mode(&mut tx, before, true)
        .await
        .map_err(protocol)?;
    crate::postgres::backups::recover_stale_with_mode(&mut tx, before, true)
        .await
        .map_err(protocol)?;
    crate::postgres::builds::recover_with_mode(&mut tx, before, true)
        .await
        .map_err(protocol)?;
    tx.commit().await
}
