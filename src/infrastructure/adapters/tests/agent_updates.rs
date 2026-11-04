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
    let schedule = AgentUpdateSchedule {
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
    let (a, b) = tokio::join!(
        repo.scheduled(&schedule, &review, minute),
        repo.scheduled(&schedule, &review, minute)
    );
    assert_eq!(
        [a.unwrap(), b.unwrap()].into_iter().filter(|v| *v).count(),
        1
    );
    assert!(repo.state().await.unwrap().operation.unwrap().scheduled);
    let disabled = AgentUpdateSchedule {
        enabled: false,
        ..schedule
    };
    repo.save_schedule(actor, &disabled, false).await.unwrap();
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
