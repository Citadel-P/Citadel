//! Persisted placement and dispatch ownership. No transport calls in transactions.
use super::*;

pub(crate) struct Target {
    pub id: Uuid,
    pub kind: ResourceType,
    pub endpoint: Option<String>,
    pub available: bool,
}
pub(crate) struct Host {
    pub daemon: String,
    pub authority: Uuid,
    pub generation: u64,
    pub slots: u32,
    pub heavy: u32,
}
pub(crate) struct Reservation {
    pub run: Uuid,
    pub attempt: Uuid,
    pub agent: Uuid,
    pub endpoint: Option<String>,
    pub kind: ResourceType,
    pub daemon: String,
    pub authority: Uuid,
    pub generation: u64,
}
pub(crate) async fn target(pool: &PgPool, id: Uuid) -> Result<Option<Target>, AutomationError> {
    if let Some(platform) = super::super::platforms::connection::load(pool, id)
        .await
        .map_err(storage)?
    {
        let endpoint = match platform.connector {
            citadel_platforms::ConnectorKind::Agent => Some(platform.address),
            citadel_platforms::ConnectorKind::EdgeAgent => None,
            _ => {
                return Err(AutomationError::Validation(
                    "Select an Agent or Core as the executor.".into(),
                ));
            }
        };
        return Ok(Some(Target {
            id,
            kind: ResourceType::Platform,
            endpoint,
            available: platform.status == citadel_primitives::PlatformStatus::Online,
        }));
    }
    let row = sqlx::query("SELECT providerspec,enabled,archivedat IS NULL AS current FROM buildagentpools WHERE id=$1")
        .bind(id).fetch_optional(pool).await.map_err(storage)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let endpoint = match citadel_builds::pool_target(&row.try_get("providerspec").map_err(storage)?)
        .map_err(|_| AutomationError::Validation("Unsupported automation Agent pool.".into()))?
    {
        citadel_builds::BuildPoolTarget::Inbound(endpoint) => Some(endpoint),
        citadel_builds::BuildPoolTarget::Edge => None,
    };
    Ok(Some(Target {
        id,
        kind: ResourceType::BuildAgentPool,
        endpoint,
        available: row.try_get::<bool, _>("enabled").map_err(storage)?
            && row.try_get::<bool, _>("current").map_err(storage)?,
    }))
}
pub(crate) async fn reserve(
    pool: &PgPool,
    claim: &AutomationRunClaim,
    target: &Target,
    host: &Host,
) -> Result<bool, AutomationError> {
    if host.daemon.is_empty()
        || host.daemon.len() > 256
        || host.authority.is_nil()
        || host.generation == 0
        || host.generation > i64::MAX as u64
        || !(1..=64).contains(&host.slots)
        || !(1..=64).contains(&host.heavy)
    {
        return Err(AutomationError::External(
            "Agent reported invalid execution capacity.".into(),
        ));
    }
    let attempt = claim
        .run
        .attempt_id
        .ok_or_else(|| AutomationError::Conflict("Automation claim has no attempt.".into()))?;
    let mut tx = pool.begin().await.map_err(storage)?;
    let owned: Option<Uuid> = sqlx::query_scalar("SELECT id FROM actionruns WHERE id=$1 AND attemptid=$2 AND status='Running' AND NOT cancelrequested FOR UPDATE")
        .bind(claim.run.id).bind(attempt).fetch_optional(&mut *tx).await.map_err(storage)?;
    if owned.is_none() {
        return Err(AutomationError::Conflict(
            "Automation attempt no longer owns execution.".into(),
        ));
    }
    sqlx::query("INSERT INTO buildexecutors(daemonid,authorityid,buildcapacity) VALUES($1,$2,1) ON CONFLICT(daemonid) DO NOTHING")
        .bind(&host.daemon).bind(host.authority).execute(&mut *tx).await.map_err(storage)?;
    let authority: Uuid =
        sqlx::query_scalar("SELECT authorityid FROM buildexecutors WHERE daemonid=$1 FOR UPDATE")
            .bind(&host.daemon)
            .fetch_one(&mut *tx)
            .await
            .map_err(storage)?;
    let (builds,data,automation): (i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM buildexecutorreservations WHERE daemonid=$1),(SELECT count(*) FROM dataexecutorreservations WHERE daemonid=$1),(SELECT count(*) FROM automationexecutorreservations WHERE daemonid=$1)")
        .bind(&host.daemon).fetch_one(&mut *tx).await.map_err(storage)?;
    let total = builds + data + automation;
    if total > 0 && authority != host.authority
        || automation >= i64::from(host.slots)
        || total >= i64::from(host.heavy)
    {
        return Ok(false);
    }
    sqlx::query("UPDATE buildexecutors SET authorityid=$2,heavycapacity=$3 WHERE daemonid=$1")
        .bind(&host.daemon)
        .bind(host.authority)
        .bind(host.heavy as i32)
        .execute(&mut *tx)
        .await
        .map_err(storage)?;
    sqlx::query("INSERT INTO automationexecutorreservations(attemptid,runid,daemonid,authorityid,admissiongeneration,agentid,agenttype,endpoint) VALUES($1,$2,$3,$4,$5,$6,$7,$8)")
        .bind(attempt).bind(claim.run.id).bind(&host.daemon).bind(host.authority).bind(host.generation as i64).bind(target.id).bind(target.kind as i32).bind(&target.endpoint)
        .execute(&mut *tx).await.map_err(storage)?;
    sqlx::query("UPDATE actionruns SET executorid=$2 WHERE id=$1")
        .bind(claim.run.id)
        .bind(target.id)
        .execute(&mut *tx)
        .await
        .map_err(storage)?;
    add_run_activity(&mut tx, &claim.run).await?;
    tx.commit().await.map_err(storage)?;
    Ok(true)
}
pub(crate) async fn confirm(
    pool: &PgPool,
    reservation: &Reservation,
) -> Result<(), AutomationError> {
    let mut tx = pool.begin().await.map_err(storage)?;
    let row: Option<(Uuid, String)> = sqlx::query_as(
        "SELECT actionid,status FROM actionruns WHERE id=$1 AND attemptid=$2 FOR UPDATE",
    )
    .bind(reservation.run)
    .bind(reservation.attempt)
    .fetch_optional(&mut *tx)
    .await
    .map_err(storage)?;
    let Some((action, status)) = row else {
        return Ok(());
    };
    sqlx::query("UPDATE automationexecutorreservations SET cleanupconfirmed=true WHERE runid=$1 AND attemptid=$2 AND daemonid=$3 AND authorityid=$4 AND admissiongeneration=$5")
        .bind(reservation.run).bind(reservation.attempt).bind(&reservation.daemon).bind(reservation.authority).bind(reservation.generation as i64)
        .execute(&mut *tx).await.map_err(storage)?;
    if status != "Running" {
        sqlx::query("DELETE FROM automationexecutorreservations WHERE runid=$1 AND attemptid=$2 AND cleanupconfirmed")
            .bind(reservation.run).bind(reservation.attempt).execute(&mut *tx).await.map_err(storage)?;
        sqlx::query("UPDATE actions SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,rowversion=rowversion+1 WHERE id=$1 AND currentrunid=$2 AND NOT EXISTS(SELECT 1 FROM automationexecutorreservations WHERE runid=$2)")
            .bind(action).bind(reservation.run).execute(&mut *tx).await.map_err(storage)?;
    }
    tx.commit().await.map_err(storage)
}
pub(crate) async fn pending(pool: &PgPool) -> Result<Vec<Reservation>, AutomationError> {
    let rows = sqlx::query("UPDATE automationexecutorreservations SET lastcheckedat=now() WHERE attemptid IN (SELECT r.attemptid FROM automationexecutorreservations r JOIN actionruns a ON a.id=r.runid WHERE a.status NOT IN ('Running','Queued') ORDER BY r.lastcheckedat NULLS FIRST,r.attemptid LIMIT 16) RETURNING *")
        .fetch_all(pool).await.map_err(storage)?;
    rows.into_iter().map(reservation).collect()
}

pub(crate) async fn retained(
    pool: &PgPool,
    run: Uuid,
) -> Result<Option<Reservation>, AutomationError> {
    let row = sqlx::query("SELECT * FROM automationexecutorreservations WHERE runid=$1")
        .bind(run)
        .fetch_optional(pool)
        .await
        .map_err(storage)?;
    row.map(reservation).transpose()
}
fn reservation(row: sqlx::postgres::PgRow) -> Result<Reservation, AutomationError> {
    Ok(Reservation {
        run: row.try_get("runid").map_err(storage)?,
        attempt: row.try_get("attemptid").map_err(storage)?,
        agent: row.try_get("agentid").map_err(storage)?,
        endpoint: row.try_get("endpoint").map_err(storage)?,
        daemon: row.try_get("daemonid").map_err(storage)?,
        kind: ResourceType::from_i32(row.try_get("agenttype").map_err(storage)?)
            .ok_or_else(|| AutomationError::Storage("Invalid executor resource type".into()))?,
        authority: row.try_get("authorityid").map_err(storage)?,
        generation: row
            .try_get::<i64, _>("admissiongeneration")
            .map_err(storage)? as u64,
    })
}
pub(crate) async fn resolve(
    pool: &PgPool,
    r: &Reservation,
    actor: ActorId,
    reason: &str,
) -> Result<(), AutomationError> {
    let mut tx = pool.begin().await.map_err(storage)?;
    let action: Option<Uuid> = sqlx::query_scalar("SELECT actionid FROM actionruns WHERE id=$1 AND attemptid=$2 AND status NOT IN ('Queued','Running') FOR UPDATE")
        .bind(r.run).bind(r.attempt).fetch_optional(&mut *tx).await.map_err(storage)?;
    let action =
        action.ok_or_else(|| AutomationError::Conflict("Run ownership changed.".into()))?;
    let removed = sqlx::query("DELETE FROM automationexecutorreservations WHERE runid=$1 AND attemptid=$2 AND daemonid=$3 AND authorityid=$4")
        .bind(r.run).bind(r.attempt).bind(&r.daemon).bind(r.authority).execute(&mut *tx).await.map_err(storage)?.rows_affected();
    if removed != 1 {
        return Err(AutomationError::Conflict(
            "Cleanup was already resolved. Refresh the run.".into(),
        ));
    }
    sqlx::query("UPDATE actions SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,rowversion=rowversion+1 WHERE id=$1 AND currentrunid=$2")
        .bind(action).bind(r.run).execute(&mut *tx).await.map_err(storage)?;
    let action = get_action_tx(&mut tx, action).await?;
    add_activity(
        &mut tx,
        &action,
        actor,
        ActivityEventInfo::ActionRunCleanupResolved {
            run_id: r.run,
            attempt_id: r.attempt,
            reason: reason.into(),
        },
    )
    .await?;
    tx.commit().await.map_err(storage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use citadel_identity::IdentityStore;
    #[tokio::test]
    #[ignore = "requires disposable CITADEL_EXECUTION_DATABASE_URL"]
    async fn remote_attempts_fence_tokens_deferral_shared_capacity_and_recovery() {
        let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
        citadel_database::MigrationRunner::migrate(&url)
            .await
            .unwrap();
        let pool = PgPool::connect(&url).await.unwrap();
        let store = PostgresAutomationRepository::new(pool.clone());
        let identity = crate::persistence::postgres::identity::authentication::store::PostgresIdentityStore::new(pool.clone());
        let actor = ActorId::new(Uuid::now_v7());
        sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'System')")
            .bind(actor.value())
            .execute(&pool)
            .await
            .unwrap();
        let agent = Uuid::now_v7();
        let mut input = citadel_automation::AutomationActionConfiguration {
            name: Uuid::now_v7().to_string(),
            description: None,
            code: "console.log('fixture')".into(),
            duplicate_source_id: None,
            default_args_json: None,
            enabled: true,
            schedule_enabled: false,
            schedule_cron: None,
            schedule_time_zone: None,
            webhook: None,
            timeout_seconds: Some(30),
            alert_on_failure: false,
            run_as_actor_id: None,
            tag_ids: vec![],
            executor: citadel_automation::AutomationExecutor::Agent { agent_id: agent },
        };
        input.validate(actor).unwrap();
        let action = store.create(actor, &input).await.unwrap();
        let run = store
            .enqueue(actor, action.id, "Manual", &serde_json::json!({}), None)
            .await
            .unwrap();
        assert!(
            store
                .claim_next(Utc::now() - chrono::Duration::minutes(10), false)
                .await
                .unwrap()
                .claim
                .is_none()
        );
        let first = store
            .claim_next(Utc::now() - chrono::Duration::minutes(10), true)
            .await
            .unwrap()
            .claim
            .unwrap();
        assert_eq!(first.run.id, run.id);
        let old = first.run.attempt_id.unwrap();
        assert!(
            identity
                .automation_attempt_active(actor, run.id, old)
                .await
                .unwrap()
        );
        assert!(
            !identity
                .automation_attempt_active(ActorId::new(Uuid::now_v7()), run.id, old)
                .await
                .unwrap()
        );
        store.defer(&first).await.unwrap();
        assert!(
            !identity
                .automation_attempt_active(actor, run.id, old)
                .await
                .unwrap()
        );
        sqlx::query("UPDATE actionruns SET nexteligibleat=now() WHERE id=$1")
            .bind(run.id)
            .execute(&pool)
            .await
            .unwrap();
        let claim = store
            .claim_next(Utc::now() - chrono::Duration::minutes(10), true)
            .await
            .unwrap()
            .claim
            .unwrap();
        assert_ne!(claim.run.attempt_id, Some(old));
        assert!(
            !identity
                .automation_attempt_active(actor, run.id, old)
                .await
                .unwrap()
        );
        let target = Target {
            id: agent,
            kind: ResourceType::Platform,
            endpoint: None,
            available: true,
        };
        let host = Host {
            daemon: Uuid::now_v7().to_string(),
            authority: Uuid::now_v7(),
            generation: 1,
            slots: 4,
            heavy: 1,
        };
        assert!(reserve(&pool, &claim, &target, &host).await.unwrap());
        assert!(
            store.defer(&claim).await.is_err(),
            "uncertain dispatch must never requeue"
        );
        input.name = Uuid::now_v7().to_string();
        let second_action = store.create(actor, &input).await.unwrap();
        store
            .enqueue(
                actor,
                second_action.id,
                "Manual",
                &serde_json::json!({}),
                None,
            )
            .await
            .unwrap();
        let second = store
            .claim_next(Utc::now() - chrono::Duration::minutes(10), true)
            .await
            .unwrap()
            .claim
            .unwrap();
        assert!(
            !reserve(&pool, &second, &target, &host).await.unwrap(),
            "host heavy budget includes automation"
        );
        let changed_host = Host {
            authority: Uuid::now_v7(),
            heavy: 8,
            ..host
        };
        assert!(
            !reserve(&pool, &second, &target, &changed_host)
                .await
                .unwrap(),
            "new authority cannot erase old occupancy"
        );
        store.cancel(action.id, run.id).await.unwrap();
        assert!(
            !identity
                .automation_attempt_active(actor, run.id, claim.run.attempt_id.unwrap())
                .await
                .unwrap()
        );
        store
            .finish(
                &claim,
                &citadel_automation::AutomationRunResult {
                    status: citadel_automation::AutomationRunStatus::Cancelled,
                    exit_code: None,
                    logs: String::new(),
                    error: None,
                },
            )
            .await
            .unwrap();
        assert!(
            store
                .get_run(action.id, run.id)
                .await
                .unwrap()
                .cleanup_pending
        );
        assert_eq!(
            store.get(action.id).await.unwrap().current_run_id,
            Some(run.id)
        );
        let r = retained(&pool, run.id).await.unwrap().unwrap();
        confirm(
            &pool,
            &Reservation {
                authority: Uuid::now_v7(),
                ..Reservation {
                    run: r.run,
                    attempt: r.attempt,
                    agent: r.agent,
                    endpoint: r.endpoint.clone(),
                    kind: r.kind,
                    daemon: r.daemon.clone(),
                    authority: r.authority,
                    generation: r.generation,
                }
            },
        )
        .await
        .unwrap();
        assert!(
            store
                .get_run(action.id, run.id)
                .await
                .unwrap()
                .cleanup_pending,
            "wrong authority cannot release"
        );
        resolve(
            &pool,
            &r,
            actor,
            "Verified process stopped and host restarted",
        )
        .await
        .unwrap();
        assert!(resolve(&pool, &r, actor, "Duplicate").await.is_err());
        assert!(
            !store
                .get_run(action.id, run.id)
                .await
                .unwrap()
                .cleanup_pending
        );
        assert!(store.get(action.id).await.unwrap().current_run_id.is_none());
        assert!(
            reserve(&pool, &second, &target, &changed_host)
                .await
                .unwrap()
        );
        let r = retained(&pool, second.run.id).await.unwrap().unwrap();
        confirm(&pool, &r).await.unwrap();
        store
            .finish(
                &second,
                &citadel_automation::AutomationRunResult {
                    status: citadel_automation::AutomationRunStatus::Succeeded,
                    exit_code: Some(0),
                    logs: String::new(),
                    error: None,
                },
            )
            .await
            .unwrap();
        assert!(
            !store
                .get_run(second_action.id, second.run.id)
                .await
                .unwrap()
                .cleanup_pending
        );
        let audited: i64 = sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE resourceid=$1 AND eventtype='ActionRunCleanupResolved'").bind(action.id).fetch_one(&pool).await.unwrap();
        assert_eq!(audited, 1);
        input.name = Uuid::now_v7().to_string();
        let cancelled_action = store.create(actor, &input).await.unwrap();
        let run = store
            .enqueue(
                actor,
                cancelled_action.id,
                "Manual",
                &serde_json::json!({}),
                None,
            )
            .await
            .unwrap();
        let claim = store
            .claim_next(Utc::now() - chrono::Duration::minutes(10), true)
            .await
            .unwrap()
            .claim
            .unwrap();
        assert_eq!(claim.run.id, run.id);
        store.cancel(cancelled_action.id, run.id).await.unwrap();
        assert_eq!(
            store.defer(&claim).await.unwrap(),
            citadel_automation::AutomationRunStatus::Cancelled
        );
        assert!(
            store
                .get(cancelled_action.id)
                .await
                .unwrap()
                .current_run_id
                .is_none(),
            "cancellation winning deferral must not leave queued work"
        );
    }
}
