//! Real PostgreSQL claims and Activities; deliberately separate from runtime mocks.
use chrono::{Duration, Timelike, Utc};
use citadel_adapters::persistence::postgres::updates::PostgresUpdateRepository;
use citadel_database::MigrationRunner;
use citadel_identity::{ADMIN_ROLE_ID, SYSTEM_ACTOR_ID};
use citadel_primitives::ActorId;
use citadel_updates::*;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires a fresh disposable CITADEL_UPDATE_TEST_DATABASE_URL named citadel_update_test_*"]
async fn claims_replay_progress_and_schedule_are_transactional() {
    let url = std::env::var("CITADEL_UPDATE_TEST_DATABASE_URL").unwrap();
    assert!(
        url::Url::parse(&url)
            .unwrap()
            .path()
            .starts_with("/citadel_update_test_")
    );
    let pool = PgPoolOptions::new()
        .max_connections(6)
        .connect(&url)
        .await
        .unwrap();
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT to_regclass('public.users') IS NULL")
            .fetch_one(&pool)
            .await
            .unwrap()
    );
    MigrationRunner::migrate(&url).await.unwrap();
    let actor = UpdateActor {
        subject_id: Uuid::now_v7(),
        actor_id: ActorId::new(Uuid::now_v7()),
    };
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,TRUE,'User')")
        .bind(actor.actor_id.value())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users(id,actorid,createdat,createdbyactorid,email,name,password) VALUES($1,$2,now(),$3,'agent-update@example.test','agent-update',NULL)").bind(actor.subject_id).bind(actor.actor_id.value()).bind(SYSTEM_ACTOR_ID).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO actorroles(actorid,roleid) VALUES($1,$2)")
        .bind(actor.actor_id.value())
        .bind(ADMIN_ROLE_ID)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO citadelinstanceidentity(id,instanceid,createdat) VALUES(1,$1,now())")
        .bind(Uuid::now_v7())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE instancesetupstates SET initializedat=now(),initialadministratoractorid=$1 WHERE id=1").bind(actor.actor_id.value()).execute(&pool).await.unwrap();
    let repo = PostgresUpdateRepository::new(pool.clone());
    let review = AgentUpdateReview {
        id: Uuid::now_v7(),
        actor_id: actor.actor_id,
        release: AgentUpdateRelease {
            version: "0.1.2".into(),
            image: format!("ghcr.io/citadel-p/citadel.agent@sha256:{}", "a".repeat(64)),
        },
        targets: vec![AgentUpdateTarget {
            id: "daemon/agent".into(),
            resource_id: Uuid::now_v7(),
            kind: AgentUpdateKind::Platform,
            name: "Agent C".into(),
            installed_version: Some("0.1.1".into()),
            connected: true,
            supported: true,
            reason: None,
            nodes: 1,
            drain_nodes: vec![],
            revision: "container:image".into(),
        }],
        expires_at: Utc::now() + Duration::minutes(5),
    };
    AgentUpdateRepository::save_review(&repo, actor, &review)
        .await
        .unwrap();
    let request = Uuid::now_v7();
    let (a, b) = tokio::join!(
        repo.approve(actor, review.id, request),
        repo.approve(actor, review.id, request)
    );
    let mut operation = a.unwrap();
    assert_eq!(operation, b.unwrap());
    assert!(
        repo.approve(actor, review.id, Uuid::now_v7())
            .await
            .is_err()
    );
    assert_eq!(
        repo.state().await.unwrap().operation,
        Some(operation.clone())
    );
    let stale = operation.clone();
    operation.targets[0].phase = AgentUpdatePhase::Updating;
    operation = repo.save_operation(&operation).await.unwrap();
    assert!(repo.save_operation(&stale).await.is_err());
    // A worker restart sees the same unresolved attempt, and cannot approve another.
    let restarted = PostgresUpdateRepository::new(pool.clone());
    assert!(!restarted.state().await.unwrap().operation.unwrap().finished);
    assert!(
        restarted
            .approve(actor, review.id, Uuid::now_v7())
            .await
            .is_err()
    );
    operation.targets[0].phase = AgentUpdatePhase::Succeeded;
    operation.finished = true;
    operation = repo.save_operation(&operation).await.unwrap();
    assert_eq!(
        AgentUpdateRepository::operation(&repo, operation.id)
            .await
            .unwrap(),
        operation
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM activityevents WHERE eventtype='AgentUpdate'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        2
    );
    let schedule = UpdateSchedule {
        include_core: false,
        enabled: true,
        targets: vec![review.targets[0].id.clone()],
        cron: "* * * * *".into(),
        time_zone: "UTC".into(),
    };
    assert!(matches!(
        repo.save_schedule(actor, &schedule, false).await,
        Err(UpdateError::Forbidden)
    ));
    repo.save_schedule(actor, &schedule, true).await.unwrap();
    // Simulate a saved schedule from before the current occurrence.
    sqlx::query("UPDATE agentupdatesettings SET lastscheduledminute=NULL")
        .execute(&pool)
        .await
        .unwrap();
    let minute = Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    assert!(
        repo.reserve_schedule(&schedule, minute - Duration::minutes(1))
            .await
            .unwrap()
            .is_none()
    );
    let (a, b) = tokio::join!(
        repo.reserve_schedule(&schedule, minute),
        repo.reserve_schedule(&schedule, minute)
    );
    let reservations: Vec<_> = [a.unwrap(), b.unwrap()].into_iter().flatten().collect();
    assert_eq!(reservations.len(), 1);
    let mut run = reservations[0].clone();
    // Persist a frozen plan before crossing the minute boundary or restarting the worker.
    run.release = Some(review.release.clone());
    run.targets = review.targets.clone();
    run.phase = UpdateSchedulePhase::Agents;
    run = repo.save_run(&run).await.unwrap();
    let previous = minute - Duration::minutes(1);
    run.minute = previous;
    sqlx::query("UPDATE updatescheduleruns SET minute=$2,run=$3 WHERE id=$1")
        .bind(run.id)
        .bind(previous)
        .bind(sqlx::types::Json(&run))
        .execute(&pool)
        .await
        .unwrap();
    let restarted = PostgresUpdateRepository::new(pool.clone());
    assert_eq!(restarted.state().await.unwrap().run, Some(run.clone()));
    // Editing future occurrences cannot overwrite this claimed selection.
    let disabled = UpdateSchedule {
        enabled: false,
        ..schedule.clone()
    };
    repo.save_schedule(actor, &disabled, false).await.unwrap();
    assert!(
        repo.approve(actor, review.id, Uuid::now_v7())
            .await
            .is_err()
    );
    restarted.claim_agents(&run).await.unwrap();
    assert!(repo.claim_agents(&run).await.is_err());
    let owned = repo.state().await.unwrap();
    assert!(owned.operation.as_ref().unwrap().scheduled);
    assert_eq!(
        owned.operation.as_ref().unwrap().targets[0].target,
        review.targets[0]
    );
    let mut finished = owned.run.unwrap();
    finished.phase = UpdateSchedulePhase::Finished;
    assert!(
        repo.save_run(&finished).await.is_err(),
        "active Agent must retain shared ownership"
    );
    let mut completed = owned.operation.unwrap();
    completed.finished = true;
    repo.save_operation(&completed).await.unwrap();
    repo.save_run(&finished).await.unwrap();
    // A stale pre-claim configuration is refused after an administrator edits the scope.
    sqlx::query("UPDATE agentupdatesettings SET lastscheduledminute=NULL")
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        repo.reserve_schedule(&schedule, minute)
            .await
            .unwrap()
            .is_none()
    );

    // A combined occurrence keeps the exclusive owner between verified Core cleanup
    // and the Agent claim, including after a new repository/worker is constructed.
    let combined = UpdateSchedule {
        include_core: true,
        ..schedule.clone()
    };
    repo.save_schedule(actor, &combined, true).await.unwrap();
    sqlx::query("UPDATE agentupdatesettings SET lastscheduledminute=NULL")
        .execute(&pool)
        .await
        .unwrap();
    let minute = Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    let mut run = repo
        .reserve_schedule(&combined, minute)
        .await
        .unwrap()
        .unwrap();
    let source = CoreIdentity {
        version: "0.1.1".into(),
        image: format!("{CORE_IMAGE}@sha256:{}", "a".repeat(64)),
        architecture: "amd64".into(),
        catalog_sha256: "b".repeat(64),
    };
    let plan = UpdatePlan {
        database: None,
        id: Uuid::now_v7(),
        instance_id: sqlx::query_scalar(
            "SELECT instanceid FROM citadelinstanceidentity WHERE id=1",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        source: source.clone(),
        target: CoreIdentity {
            version: "0.1.2".into(),
            image: format!("{CORE_IMAGE}@sha256:{}", "c".repeat(64)),
            ..source.clone()
        },
        helper_image: source.image,
        configuration_sha256: "d".repeat(64),
        registration_id: Uuid::now_v7(),
        reviewed_at: Utc::now(),
        expires_at: Utc::now() + Duration::minutes(10),
        checkpoint_directory: "/opt/update-state".into(),
        agent_requirements: vec![],
    };
    run.core_plan = Some(plan.clone());
    run.release = Some(review.release.clone());
    run.targets = review.targets.clone();
    run.phase = UpdateSchedulePhase::Core;
    run = repo.save_run(&run).await.unwrap();
    let core = repo.claim_core(&run).await.unwrap();
    assert_eq!(core.actor_id.value(), SYSTEM_ACTOR_ID);
    assert!(
        repo.claim_core(&run).await.is_err(),
        "lost responses must not redispatch Core"
    );
    UpdateRepository::save_review(&repo, actor, &plan)
        .await
        .unwrap();
    assert!(
        UpdateRepository::claim(&repo, actor, &plan, Uuid::now_v7(), Utc::now())
            .await
            .is_err()
    );
    let restarted = PostgresUpdateRepository::new(pool.clone());
    let mut run = restarted.state().await.unwrap().run.unwrap();
    run.phase = UpdateSchedulePhase::Agents;
    assert!(
        restarted.save_run(&run).await.is_err(),
        "Core cleanup is not verified"
    );
    // Simulate the existing verified completion transaction, not an observation by the scheduler.
    sqlx::query("UPDATE instanceupdates SET ownsinstallation=false,operation=jsonb_set(operation,'{phase}','\"Succeeded\"') WHERE id=$1")
        .bind(core.id).execute(&pool).await.unwrap();
    run = restarted.save_run(&run).await.unwrap();
    assert!(
        repo.approve(actor, review.id, Uuid::now_v7())
            .await
            .is_err(),
        "no gap between component phases"
    );
    restarted.claim_agents(&run).await.unwrap();
    let state = restarted.state().await.unwrap();
    let mut agents = state.operation.unwrap();
    agents.finished = true;
    restarted.save_operation(&agents).await.unwrap();
    let mut run = state.run.unwrap();
    run.phase = UpdateSchedulePhase::Finished;
    restarted.save_run(&run).await.unwrap();
    sqlx::query("DELETE FROM actorroles WHERE actorid=$1")
        .bind(actor.actor_id.value())
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        repo.approve(actor, review.id, request).await,
        Err(UpdateError::Forbidden)
    ));
}
