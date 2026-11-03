use super::*;

async fn maintenance(fixture: &Fixture, server: &mut Server) {
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            assert!(
                server.child.try_wait().unwrap().is_none(),
                "maintenance Core exited"
            );
            if let Ok(response) = fixture
                .client
                .get(format!("{}/health", server.url))
                .send()
                .await
                && response.status() == 200
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("maintenance listener timeout");
    let ready = fixture
        .client
        .get(format!("{}/ready", server.url))
        .send()
        .await
        .unwrap();
    assert_eq!(ready.status(), 503);
    assert!(ready.headers().contains_key("x-request-id"));
    assert_eq!(ready.json::<Value>().await.unwrap()["status"], "not-ready");
    for (path, status) in [
        ("/api/v1/setup/status", 404),
        ("/api/v1/updates", 503),
        ("/metrics", 503),
        ("/", 503),
    ] {
        assert_eq!(
            fixture
                .client
                .get(format!("{}{path}", server.url))
                .send()
                .await
                .unwrap()
                .status(),
            status,
            "{path}"
        );
    }
    assert_eq!(
        fixture
            .client
            .post(format!("{}/api/v1/setup/initialize", server.url))
            .json(&json!({"name":"must-not-initialize"}))
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    assert_eq!(
        fixture
            .client
            .post(format!(
                "http://127.0.0.1:{}/citadel.edge.v1.EdgeAgentService/Connect",
                server.edge_port
            ))
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
}

async fn no_migrations(pool: &PgPool) {
    assert!(sqlx::query_scalar::<_, bool>("SELECT to_regclass('public.citadel_schema_migrations') IS NULL AND to_regclass('public.users') IS NULL").fetch_one(pool).await.unwrap(), "maintenance must precede migration and bootstrap");
}

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission; starts isolated Core processes"]
async fn lease_loss_stops_core_while_migrations_remain_blocked() {
    lease_loss_during_migrations(false).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission; starts isolated Core processes"]
async fn lease_loss_prevents_bootstrap_when_migrations_resume_immediately() {
    lease_loss_during_migrations(true).await;
}

async fn lease_loss_during_migrations(resume_migrations: bool) {
    use citadel_adapters::persistence::postgres::job_lease::CoreJobLease;

    let fixture = Fixture::new().await;
    tokio::fs::write(fixture.directory.join("password"), PASSWORD)
        .await
        .unwrap();
    let pool = PgPool::connect(&fixture.database_url).await.unwrap();
    let mut blocker = PgConnection::connect(&fixture.database_url).await.unwrap();
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(0x4369_7461_6465_6c31_i64)
        .execute(&mut blocker)
        .await
        .unwrap();
    let mut server = fixture.start(true, false);

    // Wait for the real migration runner, not merely the initial lease. This
    // makes both cases exercise the gap between admission and initialization.
    let lease_pid = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            assert!(server.child.try_wait().unwrap().is_none());
            let pid = sqlx::query_scalar::<_, i32>(
                "SELECT a.pid FROM pg_stat_activity a
                 JOIN pg_locks l ON l.pid=a.pid
                 WHERE a.datname=current_database() AND l.locktype='advisory'
                   AND l.classid::bigint=$1 AND l.objid::bigint=$2
                   AND l.objsubid=1 AND l.granted
                   AND EXISTS(SELECT 1 FROM pg_stat_activity m
                     WHERE m.datname=current_database() AND m.pid<>a.pid
                       AND m.query='SELECT pg_try_advisory_lock($1)')",
            )
            .bind(4848495441444547_i64 >> 32)
            .bind(4848495441444547_i64 & 0xffff_ffff)
            .fetch_optional(&pool)
            .await
            .unwrap();
            if let Some(pid) = pid {
                break pid;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("Core must reach the blocked migration");
    no_migrations(&pool).await;
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT pg_terminate_backend($1, 5000)")
            .bind(lease_pid)
            .fetch_one(&pool)
            .await
            .unwrap()
    );
    // A second owner now holds the same database's job lease. The original Core
    // must neither reacquire it nor continue setup/recovery/worker registration.
    let mut successor = CoreJobLease::acquire(&fixture.database_url).await.unwrap();
    if resume_migrations {
        sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(0x4369_7461_6465_6c31_i64)
            .execute(&mut blocker)
            .await
            .unwrap();
    }
    let status = tokio::time::timeout(Duration::from_secs(10), server.child.wait())
        .await
        .expect("Core must stop on lease loss without waiting for migrations")
        .unwrap();
    let logs = format!(
        "{}{}",
        server.output.await.unwrap(),
        server.errors.await.unwrap()
    );
    assert!(!status.success(), "{logs}");
    assert!(!logs.contains("initial administrator created"), "{logs}");
    assert!(!logs.contains("Citadel Core listening"), "{logs}");
    assert!(!logs.contains("Core is in update maintenance"), "{logs}");
    if !resume_migrations {
        no_migrations(&pool).await;
    } else if sqlx::query_scalar::<_, bool>("SELECT to_regclass('public.users') IS NOT NULL")
        .fetch_one(&pool)
        .await
        .unwrap()
    {
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM users")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0,
            "Core must not bootstrap after losing ownership"
        );
    }
    successor.verify().await.unwrap();
    drop(successor);
    blocker.close().await.unwrap();
    pool.close().await;
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission; starts isolated Core processes"]
async fn external_update_evidence_blocks_fresh_or_restored_database_before_migrations() {
    use std::os::unix::fs::PermissionsExt;
    let mut fixture = Fixture::new().await;
    let state = fixture.directory.join("update-state");
    fixture.update_state = Some(state.clone());
    let pool = PgPool::connect(&fixture.database_url).await.unwrap();
    // An absent bind must not be mistaken for an unregistered installation.
    let mut server = fixture.start(false, false);
    maintenance(&fixture, &mut server).await;
    no_migrations(&pool).await;
    server.stop().await;
    assert!(!state.exists());

    // Orphan evidence still blocks when the database has no update claim.
    tokio::fs::create_dir(&state).await.unwrap();
    tokio::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o700))
        .await
        .unwrap();
    let directory =
        citadel_adapters::external::updates::JournalDirectory::open(state.clone()).unwrap();
    drop(directory.lock().unwrap());
    directory
        .write(
            "maintenance.json",
            &json!({"phase":"Succeeded","maintenanceReleased":true}),
        )
        .unwrap();
    let before = tokio::fs::read(state.join("maintenance.json"))
        .await
        .unwrap();
    let mut restarted = fixture.start(false, false);
    maintenance(&fixture, &mut restarted).await;
    no_migrations(&pool).await;
    restarted.stop().await;
    assert_eq!(
        tokio::fs::read(state.join("maintenance.json"))
            .await
            .unwrap(),
        before
    );
    pool.close().await;
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission; starts isolated Core processes"]
async fn database_update_ownership_blocks_startup_and_lease_loss_stops_maintenance() {
    let fixture = Fixture::new().await;
    let pool = PgPool::connect(&fixture.database_url).await.unwrap();
    // Deliberately minimal/corrupt future operation format: admission must inspect
    // ownership without deserializing or trying to migrate the database first.
    sqlx::raw_sql("CREATE TABLE instanceupdates(ownsinstallation boolean NOT NULL, operation jsonb); INSERT INTO instanceupdates VALUES(true, '{}');").execute(&pool).await.unwrap();
    let mut server = fixture.start(false, false);
    maintenance(&fixture, &mut server).await;
    no_migrations(&pool).await;
    let mut contender = fixture.start(false, false);
    assert!(
        !tokio::time::timeout(Duration::from_secs(10), contender.child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
    assert!(
        format!(
            "{}{}",
            contender.output.await.unwrap(),
            contender.errors.await.unwrap()
        )
        .contains("Another Citadel Core owns")
    );
    server.stop().await;

    let mut restarted = fixture.start(false, false);
    maintenance(&fixture, &mut restarted).await;
    no_migrations(&pool).await;
    // Terminate only this fixture's advisory-lease connection.
    let terminated: Vec<bool> = sqlx::query_scalar("SELECT pg_terminate_backend(a.pid) FROM pg_stat_activity a WHERE a.datname=current_database() AND a.pid<>pg_backend_pid() AND EXISTS(SELECT 1 FROM pg_locks l WHERE l.pid=a.pid AND l.locktype='advisory' AND l.granted)").fetch_all(&pool).await.unwrap();
    assert_eq!(terminated, vec![true]);
    assert!(
        !tokio::time::timeout(Duration::from_secs(12), restarted.child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
    let _ = (
        restarted.output.await.unwrap(),
        restarted.errors.await.unwrap(),
    );
    assert!(
        fixture
            .client
            .get(format!("{}/health", restarted.url))
            .send()
            .await
            .is_err()
    );
    pool.close().await;
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission; starts isolated Core processes"]
async fn a_valid_foreign_target_claim_cannot_enter_startup_migrations() {
    use citadel_updates::{CoreIdentity, UpdateOperation, UpdatePlan};
    let fixture = Fixture::new().await;
    let pool = PgPool::connect(&fixture.database_url).await.unwrap();
    let now = chrono::Utc::now();
    let source = CoreIdentity {
        version: "0.0.1".into(),
        image: format!("ghcr.io/citadel-p/citadel@sha256:{}", "a".repeat(64)),
        catalog_sha256: citadel_database::MigrationRunner::catalog_sha256().unwrap(),
        architecture: "amd64".into(),
    };
    let operation = UpdateOperation::approve(
        UpdatePlan {
            id: Uuid::now_v7(),
            instance_id: Uuid::now_v7(),
            source: source.clone(),
            target: CoreIdentity {
                version: "0.0.2".into(),
                ..source.clone()
            },
            helper_image: source.image,
            configuration_sha256: "b".repeat(64),
            registration_id: Uuid::now_v7(),
            reviewed_at: now,
            expires_at: now + chrono::Duration::minutes(10),
            checkpoint_directory: "/private/test-checkpoint".into(),
            agent_requirements: vec![],
        },
        citadel_primitives::ActorId::new(Uuid::now_v7()),
        Uuid::now_v7(),
        now,
    )
    .unwrap();
    // Valid operation JSON must not become authorization for a different build
    // to initialize or migrate this database, even with a matching catalog.
    sqlx::raw_sql(
        "CREATE TABLE instanceupdates(ownsinstallation boolean NOT NULL, operation jsonb)",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO instanceupdates VALUES(true, $1)")
        .bind(sqlx::types::Json(&operation))
        .execute(&pool)
        .await
        .unwrap();
    let mut server = fixture.start(false, false);
    maintenance(&fixture, &mut server).await;
    no_migrations(&pool).await;
    let retained: sqlx::types::Json<UpdateOperation> =
        sqlx::query_scalar("SELECT operation FROM instanceupdates WHERE ownsinstallation")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(retained.0, operation);
    server.stop().await;
    pool.close().await;
    fixture.close().await;
}
