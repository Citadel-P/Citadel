use super::*;
use citadel_builds::{
    BuildAdmission, BuildAdmissionStore, BuildCleanupAttempt, BuildCleanupStore,
    BuildExecutorCapacity, BuildRunStatus,
};

pub(super) async fn verify(pool: &sqlx::PgPool, actor: ActorId, input: &BuildProjectConfiguration) {
    let store = PostgresBuildRepository::new(pool.clone());
    let mut claims = Vec::new();
    for index in 0..7 {
        let mut input = input.clone();
        input.name = format!("host-admission-{index}-{}", Uuid::now_v7());
        input.retention_run_count = Some(1);
        let alias = Uuid::now_v7();
        sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'http://localhost/' || $1::text,'Local',0,0,0,$2,0,'{\"$type\":\"Docker\"}','Online',0)")
            .bind(alias).bind(format!("alias-{alias}")).execute(pool).await.unwrap();
        input.platform_id = Some(alias);
        let project = store.create(actor, &input).await.unwrap();
        let run = store.enqueue(actor, project.id, "Manual").await.unwrap();
        let claim = store
            .claim_next(Utc::now() - Duration::minutes(5))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(claim.run.id, run.id);
        claims.push(claim);
    }
    let host = BuildExecutorCapacity {
        daemon_id: format!("fixture-{}", Uuid::now_v7()),
        authority_id: Uuid::now_v7(),
        build_slots: 2,
        heavy_slots: 2,
        admission_generation: 1,
    };
    let commit = "a".repeat(40);
    let (a, b, c) = tokio::join!(
        store.reserve_executor(&claims[0], &host, &commit),
        store.reserve_executor(&claims[1], &host, &commit),
        store.reserve_executor(&claims[2], &host, &commit),
    );
    let outcomes = [a.unwrap(), b.unwrap(), c.unwrap()];
    let accepted: Vec<_> = outcomes
        .iter()
        .enumerate()
        .filter(|(_, r)| **r == BuildAdmission::Reserved)
        .map(|(i, _)| i)
        .collect();
    let waiting = outcomes
        .iter()
        .position(|r| *r == BuildAdmission::CapacityBusy)
        .unwrap();
    assert_eq!(
        accepted.len(),
        2,
        "aliases must not create extra executor slots"
    );
    let occupancy = store.executor_occupancy(&host.daemon_id).await.unwrap();
    assert_eq!((occupancy.reserved, occupancy.uncertain), (2, 0));
    assert_eq!(occupancy.authority_id, Some(host.authority_id));
    let first = &claims[accepted[0]];
    let second = &claims[accepted[1]];
    assert!(
        store.reserve_executor(first, &host, &commit).await.is_err(),
        "lost admission acknowledgments must not replay dispatch"
    );
    assert_eq!(
        store
            .get_run(first.run.id)
            .await
            .unwrap()
            .resolved_commit_sha
            .as_deref(),
        Some(commit.as_str())
    );
    assert!(
        store.pending_cleanup(None, 16).await.unwrap().is_empty(),
        "active work must never be reconciled by the terminal sweeper"
    );
    let active_cleanup = BuildCleanupAttempt {
        run_id: first.run.id,
        attempt_id: first.attempt_id,
        target: first.run.platform_snapshot.clone(),
        daemon_id: host.daemon_id.clone(),
        authority_id: host.authority_id,
        admission_generation: 1,
        receipt_recorded: false,
    };
    assert!(!store.finish_cleanup(&active_cleanup).await.unwrap());
    let mut stale = first.clone();
    stale.attempt_id = Uuid::now_v7();
    assert!(
        store
            .reserve_executor(&stale, &host, &commit)
            .await
            .is_err()
    );
    assert!(
        !store
            .finish(&stale, &success_result('a'), None)
            .await
            .unwrap()
    );

    // A new repository handle and a different Agent process still see the same holds.
    let restarted = PostgresBuildRepository::new(pool.clone());
    let new_authority = BuildExecutorCapacity {
        authority_id: Uuid::now_v7(),
        ..host.clone()
    };
    assert_eq!(
        restarted
            .reserve_executor(&claims[waiting], &new_authority, &commit)
            .await
            .unwrap(),
        BuildAdmission::CleanupPending
    );
    let reduced = BuildExecutorCapacity {
        build_slots: 1,
        heavy_slots: 1,
        ..host.clone()
    };
    assert_eq!(
        restarted
            .reserve_executor(&claims[waiting], &reduced, &commit)
            .await
            .unwrap(),
        BuildAdmission::CapacityBusy
    );

    // Evidence does not release active occupancy. Wrong authority/attempt and
    // runs without reservations cannot create receipts; duplicate acks are safe.
    assert!(!store.record_executor_cleanup(&stale, &host).await.unwrap());
    assert!(
        !store
            .record_executor_cleanup(second, &new_authority)
            .await
            .unwrap()
    );
    assert!(
        !store
            .record_executor_cleanup(&claims[waiting], &host)
            .await
            .unwrap()
    );
    assert!(store.record_executor_cleanup(second, &host).await.unwrap());
    assert!(store.record_executor_cleanup(second, &host).await.unwrap());
    assert_eq!(
        store
            .executor_occupancy(&host.daemon_id)
            .await
            .unwrap()
            .reserved,
        2
    );
    let logs: i64 = sqlx::query_scalar("SELECT count(*) FROM buildrunlogs WHERE buildrunid=$1 AND message LIKE 'Executor confirmed cleanup%'")
        .bind(second.run.id).fetch_one(pool).await.unwrap();
    assert_eq!(logs, 1);

    let wrong_generation = BuildExecutorCapacity {
        admission_generation: 2,
        ..host.clone()
    };
    assert!(
        !store
            .record_executor_cleanup(second, &wrong_generation)
            .await
            .unwrap()
    );
    // Completion and capacity release roll back together if persistence fails.
    let constraint = format!("reservation_finish_{}", second.project.id.simple());
    let ddl = format!(
        "ALTER TABLE activityevents ADD CONSTRAINT {constraint} CHECK(resourceid <> '{}' OR eventtype <> 'BuildRunSucceeded') NOT VALID",
        second.project.id
    );
    sqlx::raw_sql(sqlx::AssertSqlSafe(ddl.as_str()))
        .execute(pool)
        .await
        .unwrap();
    assert!(
        store
            .finish(second, &success_result('a'), None)
            .await
            .is_err()
    );
    assert_eq!(
        store
            .reserve_executor(&claims[waiting], &host, &commit)
            .await
            .unwrap(),
        BuildAdmission::CapacityBusy
    );
    let ddl = format!("ALTER TABLE activityevents DROP CONSTRAINT {constraint}");
    sqlx::raw_sql(sqlx::AssertSqlSafe(ddl.as_str()))
        .execute(pool)
        .await
        .unwrap();
    assert!(
        store
            .finish(second, &success_result('a'), None)
            .await
            .unwrap()
    );
    assert_eq!(
        store
            .reserve_executor(&claims[waiting], &host, &commit)
            .await
            .unwrap(),
        BuildAdmission::Reserved
    );
    let mut deferred = success_result('a');
    deferred.status = BuildRunStatus::Queued;
    deferred.error_code = Some("build.capacity_wait".into());
    deferred.error_message = Some("Waiting for executor build capacity.".into());
    assert!(
        store
            .finish(&claims[waiting], &deferred, None)
            .await
            .unwrap()
    );

    let mut failure = success_result('a');
    failure.status = BuildRunStatus::Failed;
    failure.resolved_commit_sha = None;
    assert!(store.finish(first, &failure, None).await.unwrap());
    let failed = store.get_run(first.run.id).await.unwrap();
    assert_eq!(failed.error_code.as_deref(), Some("build.cleanup_pending"));
    assert_eq!(failed.resolved_commit_sha.as_deref(), Some(commit.as_str()));
    assert_eq!(
        store
            .reserve_executor(&claims[4], &new_authority, &commit)
            .await
            .unwrap(),
        BuildAdmission::CleanupPending
    );
    assert_eq!(
        store
            .reserve_executor(&claims[4], &reduced, &commit)
            .await
            .unwrap(),
        BuildAdmission::CleanupPending
    );

    let other = BuildExecutorCapacity {
        daemon_id: format!("fixture-{}", Uuid::now_v7()),
        ..host.clone()
    };
    // Changing the project to another Local platform must not bypass remote ownership.
    let local_platform = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'http://localhost/' || $1::text,'Local',0,0,0,$2,0,'{\"$type\":\"Docker\"}','Online',0)")
        .bind(local_platform).bind(format!("admission-local-{local_platform}")).execute(pool).await.unwrap();
    let current = store.get(first.project.id).await.unwrap();
    let mut local = input.clone();
    local.name = current.name.clone();
    local.platform_id = Some(local_platform);
    local.builder_kind = "Platform".into();
    local.build_agent_pool_id = None;
    local.retention_run_count = Some(1);
    store.update(&current, &local, actor, false).await.unwrap();
    let replacement = store
        .enqueue(actor, first.project.id, "Manual")
        .await
        .unwrap();
    assert!(
        store
            .claim_next(Utc::now() - Duration::minutes(5))
            .await
            .unwrap()
            .is_none(),
        "unresolved Project ownership must block claims before selecting any executor"
    );
    assert!(store.cancel_queued(replacement.id).await.unwrap());
    assert_eq!(
        store
            .get_run(first.run.id)
            .await
            .unwrap()
            .error_code
            .as_deref(),
        Some("build.cleanup_pending"),
        "history retention must preserve unresolved ownership"
    );
    assert_eq!(
        store
            .reserve_executor(&claims[3], &other, &commit)
            .await
            .unwrap(),
        BuildAdmission::Reserved
    );
    let cleaned = &claims[5];
    let cleanup_host = BuildExecutorCapacity {
        daemon_id: format!("clean-{}", Uuid::now_v7()),
        ..host.clone()
    };
    assert_eq!(
        store
            .reserve_executor(cleaned, &cleanup_host, &commit)
            .await
            .unwrap(),
        BuildAdmission::Reserved
    );
    assert!(
        store
            .record_executor_cleanup(cleaned, &cleanup_host)
            .await
            .unwrap()
    );
    let mut known_failure = failure.clone();
    known_failure.error_code = Some("build.command".into());
    known_failure.error_message = Some("Push failed.".into());
    let constraint = format!("cleanup_finish_{}", cleaned.project.id.simple());
    let ddl = format!(
        "ALTER TABLE activityevents ADD CONSTRAINT {constraint} CHECK(resourceid <> '{}' OR eventtype <> 'BuildRunFailed') NOT VALID",
        cleaned.project.id
    );
    sqlx::raw_sql(sqlx::AssertSqlSafe(ddl.as_str()))
        .execute(pool)
        .await
        .unwrap();
    assert!(store.finish(cleaned, &known_failure, None).await.is_err());
    assert_eq!(
        store
            .executor_occupancy(&cleanup_host.daemon_id)
            .await
            .unwrap()
            .reserved,
        1
    );
    let ddl = format!("ALTER TABLE activityevents DROP CONSTRAINT {constraint}");
    sqlx::raw_sql(sqlx::AssertSqlSafe(ddl.as_str()))
        .execute(pool)
        .await
        .unwrap();
    assert!(store.finish(cleaned, &known_failure, None).await.unwrap());
    assert_eq!(
        store
            .executor_occupancy(&cleanup_host.daemon_id)
            .await
            .unwrap()
            .reserved,
        0
    );
    let cleaned_run = store.get_run(cleaned.run.id).await.unwrap();
    assert_eq!(cleaned_run.status, BuildRunStatus::Failed);
    assert_eq!(cleaned_run.error_code.as_deref(), Some("build.command"));
    assert_eq!(cleaned_run.error_message.as_deref(), Some("Push failed."));
    // Core crashes after receipt persistence, before terminal persistence.
    let interrupted = &claims[6];
    assert_eq!(
        store
            .reserve_executor(interrupted, &cleanup_host, &commit)
            .await
            .unwrap(),
        BuildAdmission::Reserved
    );
    assert!(
        store
            .record_executor_cleanup(interrupted, &cleanup_host)
            .await
            .unwrap()
    );
    citadel_adapters::persistence::postgres::maintenance::recover_on_startup(pool)
        .await
        .unwrap();
    assert_eq!(
        store
            .executor_occupancy(&cleanup_host.daemon_id)
            .await
            .unwrap()
            .reserved,
        0
    );
    let interrupted_run = store.get_run(interrupted.run.id).await.unwrap();
    assert_eq!(interrupted_run.status, BuildRunStatus::Interrupted);
    assert_eq!(
        interrupted_run.error_code.as_deref(),
        Some("build.interrupted")
    );
    let (count, uncertain): (i64,i64) = sqlx::query_as("SELECT count(*),count(*) FILTER(WHERE uncertain) FROM buildexecutorreservations WHERE daemonid IN ($1,$2)")
        .bind(&host.daemon_id).bind(&other.daemon_id).fetch_one(pool).await.unwrap();
    assert_eq!(
        (count, uncertain),
        (2, 2),
        "Core recovery cannot release remote occupancy"
    );
    assert_eq!(
        store
            .get_run(claims[3].run.id)
            .await
            .unwrap()
            .error_code
            .as_deref(),
        Some("build.cleanup_pending")
    );

    sqlx::query("UPDATE buildruns SET completedat=now()-interval '2 days' WHERE id IN ($1,$2)")
        .bind(first.run.id)
        .bind(replacement.id)
        .execute(pool)
        .await
        .unwrap();
    citadel_adapters::persistence::postgres::maintenance::cleanup(pool, Some(1))
        .await
        .unwrap();
    assert!(
        store.get_run(first.run.id).await.is_ok(),
        "global retention must keep unresolved ownership"
    );
    assert!(
        matches!(
            store.get_run(replacement.id).await,
            Err(citadel_builds::BuildError::NotFound)
        ),
        "unresolved runs must not block unrelated retention"
    );

    let candidates = store.pending_cleanup(None, 16).await.unwrap();
    let candidate = candidates
        .iter()
        .find(|candidate| candidate.run_id == first.run.id)
        .unwrap();
    assert_eq!(
        candidate.target.id, first.run.platform_snapshot.id,
        "cleanup must use the original run target after project edits"
    );
    assert_ne!(candidate.target.id, Some(local_platform));
    verify_cleanup_routing(pool, candidate).await;
    for wrong in [
        BuildCleanupAttempt {
            attempt_id: Uuid::now_v7(),
            ..candidate.clone()
        },
        BuildCleanupAttempt {
            authority_id: Uuid::now_v7(),
            ..candidate.clone()
        },
        BuildCleanupAttempt {
            daemon_id: "different-daemon".into(),
            ..candidate.clone()
        },
        BuildCleanupAttempt {
            admission_generation: 2,
            ..candidate.clone()
        },
    ] {
        assert!(!store.finish_cleanup(&wrong).await.unwrap());
    }
    let constraint = format!("cleanup_audit_{}", first.run.id.simple());
    let ddl = format!(
        "ALTER TABLE buildrunlogs ADD CONSTRAINT {constraint} CHECK(buildrunid <> '{}') NOT VALID",
        first.run.id
    );
    sqlx::raw_sql(sqlx::AssertSqlSafe(ddl.as_str()))
        .execute(pool)
        .await
        .unwrap();
    assert!(store.finish_cleanup(candidate).await.is_err());
    assert!(
        store
            .pending_cleanup(None, 16)
            .await
            .unwrap()
            .iter()
            .any(|a| a.attempt_id == candidate.attempt_id),
        "audit failure must roll back capacity release"
    );
    let ddl = format!("ALTER TABLE buildrunlogs DROP CONSTRAINT {constraint}");
    sqlx::raw_sql(sqlx::AssertSqlSafe(ddl.as_str()))
        .execute(pool)
        .await
        .unwrap();
    let before = store.get_run(first.run.id).await.unwrap();
    assert!(store.finish_cleanup(candidate).await.unwrap());
    assert!(
        !store.finish_cleanup(candidate).await.unwrap(),
        "duplicate confirmation is idempotent"
    );
    let after = store.get_run(first.run.id).await.unwrap();
    assert_eq!(after.status, before.status);
    assert_eq!(after.completed_at, before.completed_at);
    assert_eq!(after.error_code.as_deref(), Some("build.cleanup_confirmed"));
    let receipts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM buildexecutorcleanupreceipts WHERE attemptid=$1")
            .bind(candidate.attempt_id)
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(receipts, 1);
    let logs = store.logs(first.run.id).await.unwrap();
    assert_eq!(
        logs.iter()
            .filter(|log| log.message.starts_with("Executor cleanup verified"))
            .count(),
        1
    );
    assert!(
        logs.iter()
            .any(|log| log.message.contains("Original result:"))
    );

    super::operator_recovery::verify(pool, &store, claims[3].run.id, actor).await;

    // Explicit fixture cleanup, never a production reconciliation mechanism.
    for claim in claims {
        sqlx::query("DELETE FROM buildexecutorreservations WHERE runid=$1")
            .bind(claim.run.id)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM buildcompletionqueue WHERE buildrunid=$1")
            .bind(claim.run.id)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM activityevents WHERE resourceid=$1")
            .bind(claim.project.id)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM buildruns WHERE buildprojectid=$1")
            .bind(claim.project.id)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM resourcetags WHERE resourcetype='Build' AND resourceid=$1")
            .bind(claim.project.id)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM buildprojects WHERE id=$1")
            .bind(claim.project.id)
            .execute(pool)
            .await
            .unwrap();
    }
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(local_platform)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM buildexecutors WHERE daemonid IN ($1,$2,$3)")
        .bind(host.daemon_id)
        .bind(other.daemon_id)
        .bind(cleanup_host.daemon_id)
        .execute(pool)
        .await
        .unwrap();
}

async fn verify_cleanup_routing(pool: &sqlx::PgPool, candidate: &BuildCleanupAttempt) {
    use citadel_adapters::connectors::{
        agent::build_cleanup::AgentBuildCleanupChecker,
        edge::{EdgeRegistry, EdgeTarget},
    };
    use citadel_builds::BuildCleanupChecker;
    use citadel_contracts::citadel::{
        edge::v1::{EdgeCommandKind, core_envelope},
        images::v1::{BuildCleanupState, ReconcileBuildRequest, ReconcileBuildResponse},
    };
    use prost::Message;
    let platform = candidate.target.id.unwrap();
    sqlx::query("UPDATE platforms SET connectortype='EdgeAgent' WHERE id=$1")
        .bind(platform)
        .execute(pool)
        .await
        .unwrap();
    let edge = EdgeRegistry::default();
    let (session, mut receiver) = edge
        .register(EdgeTarget::platform(platform), Uuid::now_v7())
        .unwrap();
    let checker = AgentBuildCleanupChecker {
        local: None,
        pool: pool.clone(),
        agent: None,
        edge: edge.clone(),
    };
    let cancel = CancellationToken::new();
    for state in [
        BuildCleanupState::BuildCleanupActive,
        BuildCleanupState::BuildCleanupUncertain,
        BuildCleanupState::BuildCleanupConfirmed,
    ] {
        let response = async {
            let envelope = receiver.recv().await.unwrap();
            let id = Uuid::parse_str(&envelope.command_id).unwrap();
            let Some(core_envelope::Body::Command(command)) = envelope.body else {
                panic!("expected cleanup RPC")
            };
            assert_eq!(command.kind, EdgeCommandKind::ImageReconcileBuild as i32);
            let request = ReconcileBuildRequest::decode(command.payload.as_slice()).unwrap();
            assert_eq!(request.run_id, candidate.run_id.to_string());
            assert_eq!(request.attempt_id, candidate.attempt_id.to_string());
            assert_eq!(request.docker_daemon_id, candidate.daemon_id);
            assert_eq!(
                request.admission_authority_id,
                candidate.authority_id.to_string()
            );
            assert_eq!(request.admission_generation, candidate.admission_generation);
            session.output(
                id,
                ReconcileBuildResponse {
                    state: state as i32,
                }
                .encode_to_vec(),
            );
            session.complete(id, true);
        };
        let (result, ()) = tokio::time::timeout(std::time::Duration::from_secs(3), async {
            tokio::join!(checker.confirmed(candidate, &cancel), response)
        })
        .await
        .unwrap();
        assert_eq!(
            result.unwrap(),
            state == BuildCleanupState::BuildCleanupConfirmed
        );
    }
    for available in [true, false] {
        let authority = Uuid::now_v7();
        let response = async {
            let envelope = receiver.recv().await.unwrap();
            let id = Uuid::parse_str(&envelope.command_id).unwrap();
            let Some(core_envelope::Body::Command(command)) = envelope.body else {
                panic!("expected host check")
            };
            assert_eq!(command.kind, EdgeCommandKind::ImageCheckBuildHost as i32);
            session.output(
                id,
                citadel_contracts::citadel::images::v1::CheckBuildHostResponse {
                    available,
                    docker_daemon_id: candidate.daemon_id.clone(),
                    admission_authority_id: authority.to_string(),
                    build_capacity: 2,
                    admission_generation: 1,
                    ..Default::default()
                }
                .encode_to_vec(),
            );
            session.complete(id, true);
        };
        let (result, ()) = tokio::time::timeout(std::time::Duration::from_secs(3), async {
            tokio::join!(checker.recovery_host(&candidate.target, &cancel), response)
        })
        .await
        .unwrap();
        if available {
            let host = result.unwrap();
            assert_eq!(host.daemon_id, candidate.daemon_id);
            assert_eq!(host.authority_id, authority);
        } else {
            assert!(result.is_err());
        }
    }
    edge.disconnect(&EdgeTarget::platform(platform));
    assert!(
        checker
            .recovery_host(&candidate.target, &cancel)
            .await
            .is_err()
    );
    assert!(checker.confirmed(candidate, &cancel).await.is_err());
    sqlx::query("UPDATE platforms SET connectortype='Local' WHERE id=$1")
        .bind(platform)
        .execute(pool)
        .await
        .unwrap();
}

pub(super) async fn verify_target_fairness(
    pool: &sqlx::PgPool,
    actor: ActorId,
    input: &BuildProjectConfiguration,
) {
    let store = PostgresBuildRepository::new(pool.clone());
    let mut runs = Vec::new();
    for i in 0..5 {
        let mut input = input.clone();
        input.name = format!("fair-target-{i}-{}", Uuid::now_v7());
        if i == 4 {
            let alias = Uuid::now_v7();
            sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'http://localhost/' || $1::text,'Local',0,0,0,$2,0,'{\"$type\":\"Docker\"}','Online',0)")
                .bind(alias).bind(format!("healthy-{alias}")).execute(pool).await.unwrap();
            input.platform_id = Some(alias);
        }
        let project = store.create(actor, &input).await.unwrap();
        runs.push(store.enqueue(actor, project.id, "Manual").await.unwrap());
    }
    let old = store
        .claim_next(Utc::now() - Duration::minutes(5))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(old.run.id, runs[0].id);
    // Keep the slow connection in Preparing. The second preparation slot must
    // go to the independent target, without waiting for its 30-second timeout.
    let healthy = store
        .claim_next(Utc::now() - Duration::minutes(5))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(healthy.run.id, runs[4].id);
    assert!(
        store
            .claim_next(Utc::now() - Duration::minutes(5))
            .await
            .unwrap()
            .is_none()
    );
    let mut deferred = success_result('a');
    deferred.status = BuildRunStatus::Queued;
    deferred.error_code = Some("build.capacity_wait".into());
    assert!(store.finish(&old, &deferred, None).await.unwrap());
    assert!(
        store
            .claim_next(Utc::now() - Duration::minutes(5))
            .await
            .unwrap()
            .is_none(),
        "target cooldown covers sibling projects, not just the failed run"
    );
    sqlx::query("UPDATE buildruns SET nextattemptat=clock_timestamp() WHERE id=$1")
        .bind(old.run.id)
        .execute(pool)
        .await
        .unwrap();
    let resumed = store
        .claim_next(Utc::now() - Duration::minutes(5))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        resumed.run.id, runs[1].id,
        "untouched work precedes deferred retries"
    );
    let host = BuildExecutorCapacity {
        daemon_id: format!("fair-{}", Uuid::now_v7()),
        authority_id: Uuid::now_v7(),
        build_slots: 2,
        heavy_slots: 2,
        admission_generation: 1,
    };
    assert_eq!(
        store.reserve_preparation(&resumed, &host).await.unwrap(),
        BuildAdmission::Reserved
    );
    assert!(
        store
            .claim_next(Utc::now() - Duration::minutes(5))
            .await
            .unwrap()
            .is_none()
    );
    store
        .dispatch_executor(&resumed, &"a".repeat(40))
        .await
        .unwrap();
    // Dispatch ends the probe guard even while the run status is Preparing.
    // The daemon governor, not the target guard, limits executing builds.
    let sibling = store
        .claim_next(Utc::now() - Duration::minutes(5))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(sibling.run.id, runs[2].id);
    assert_eq!(
        store
            .reserve_executor(&sibling, &host, &"a".repeat(40))
            .await
            .unwrap(),
        BuildAdmission::Reserved
    );
    assert_eq!(
        store
            .executor_occupancy(&host.daemon_id)
            .await
            .unwrap()
            .reserved,
        2
    );
    assert!(
        store
            .record_executor_cleanup(&resumed, &host)
            .await
            .unwrap()
    );
    assert!(
        store
            .record_executor_cleanup(&sibling, &host)
            .await
            .unwrap()
    );
    for claim in [healthy, resumed, sibling] {
        assert!(
            store
                .finish(&claim, &success_result('a'), None)
                .await
                .unwrap()
        );
    }
    for run in runs {
        let _ = store.cancel_queued(run.id).await.unwrap();
        for query in [
            "DELETE FROM activityevents WHERE resourceid=$1",
            "DELETE FROM buildruns WHERE buildprojectid=$1",
            "DELETE FROM resourcetags WHERE resourcetype='Build' AND resourceid=$1",
            "DELETE FROM buildprojects WHERE id=$1",
        ] {
            sqlx::query(query)
                .bind(run.build_project_id)
                .execute(pool)
                .await
                .unwrap();
        }
    }
}
