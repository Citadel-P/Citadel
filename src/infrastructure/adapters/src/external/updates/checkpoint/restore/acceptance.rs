//! Opt-in daemon test. Both source and rehearsal databases are disposable tmpfs.
use super::super::super::{
    JournalDirectory, RegisteredComposeInstallation,
    docker::{LocalDocker, single},
};
use super::{
    runtime::*,
    sandbox::{DATABASE, Sandbox},
    tests::{fixture, probe, set_dump},
};
use citadel_execution::{ProcessError, ProcessOutput, ProcessRequest, ProcessRunner};
use citadel_updates::UpdateError;
use futures_util::FutureExt;
use futures_util::future::BoxFuture;
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

// Only this disposable fixture runner prints tool failures; production output
// remains redacted. Never use it with installation credentials or user data.
struct FixtureProcess;
impl ProcessRunner for FixtureProcess {
    fn run<'a>(
        &'a self,
        request: ProcessRequest,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ProcessOutput, ProcessError>> {
        Box::pin(async move {
            let action = request
                .arguments
                .iter()
                .skip(2)
                .take(3)
                .map(|v| v.to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            let result = citadel_processes::run(request, cancellation).await;
            if let Ok(output) = &result {
                if !output.succeeded() {
                    eprintln!(
                        "Disposable checkpoint fixture {action:?}: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
            }
            result
        })
    }
}

#[tokio::test]
#[ignore = "requires local Docker and cached postgres:18.6; creates only disposable tmpfs databases"]
async fn real_postgres_restore_verifies_identity_claim_catalog_and_removes_sandbox() {
    let f = fixture();
    let broken = fixture();
    let broken_stage = JournalDirectory::open(broken.stage()).unwrap();
    let process = FixtureProcess;
    let token = CancellationToken::new();
    let home = tempfile::tempdir().unwrap();
    let docker = LocalDocker {
        process: &process,
        cancellation: &token,
        home: home.path(),
        deadline: Instant::now() + Duration::from_secs(180),
    };
    let image = docker
        .json(&["image", "inspect", "postgres:18.6"])
        .await
        .unwrap();
    let image = single(&image).unwrap()["Id"].as_str().unwrap().to_string();
    let stage = JournalDirectory::open(f.stage()).unwrap();
    let checkpoint: super::super::PreparedUpdateCheckpoint = stage.read("checkpoint.json").unwrap();
    let source = Sandbox {
        nonce: Uuid::now_v7(),
        image,
        dump: checkpoint.bundle.join("database/citadel.dump"),
        maximum_bytes: 128 * 1024 * 1024,
    };
    let exercise = async {
        let id = source.create(&docker).await?;
        docker.run(&["container", "start", &id]).await?;
        loop {
            let comm = docker.run(&["exec", &id, "cat", "/proc/1/comm"]).await?;
            if comm == b"postgres\n"
                && docker
                    .run(&[
                        "exec",
                        &id,
                        "pg_isready",
                        "-h",
                        "/var/run/postgresql",
                        "-U",
                        "postgres",
                        "-d",
                        DATABASE,
                    ])
                    .await
                    .is_ok()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        let p = probe(&f);
        let mut sql = format!(
            "CREATE TABLE citadelinstanceidentity(id int PRIMARY KEY,instanceid uuid); INSERT INTO citadelinstanceidentity VALUES(1,'{}'); CREATE TABLE instancesetupstates(id int PRIMARY KEY,initializedat timestamptz); INSERT INTO instancesetupstates VALUES(1,now()); CREATE TABLE citadel_schema_migrations(id text PRIMARY KEY,checksum text,completed_at timestamptz,failure text); CREATE TABLE instanceupdates(id uuid,instanceid uuid,actorid uuid,idempotencykey uuid,revision bigint,ownsinstallation bool,operation jsonb); CREATE TABLE checkpoint_marker(value text); INSERT INTO checkpoint_marker VALUES('restored-data');",
            f.operation.instance_id
        );
        for m in p["migrations"].as_array().unwrap() {
            sql.push_str(&format!(
                "INSERT INTO citadel_schema_migrations VALUES('{}','{}',now(),NULL);",
                m["id"].as_str().unwrap(),
                m["checksum"].as_str().unwrap()
            ));
        }
        let owner = &p["owners"][0];
        sql.push_str(&format!(
            "INSERT INTO instanceupdates VALUES('{}','{}','{}','{}',0,true,'{}');",
            f.operation.id,
            f.operation.instance_id,
            f.operation.actor_id.value(),
            f.operation.idempotency_key,
            owner["operation"].to_string().replace('\'', "''")
        ));
        docker
            .run_with_input(
                &[
                    "exec",
                    "--interactive",
                    &id,
                    "psql",
                    "-h",
                    "/var/run/postgresql",
                    "-U",
                    "postgres",
                    "-d",
                    DATABASE,
                    "-v",
                    "ON_ERROR_STOP=1",
                ],
                sql.into_bytes(),
                Duration::from_secs(30),
            )
            .await?;
        let dump = docker
            .run(&[
                "exec",
                &id,
                "pg_dump",
                "-h",
                "/var/run/postgresql",
                "-U",
                "postgres",
                "-d",
                DATABASE,
                "--format=custom",
                "--no-owner",
                "--no-privileges",
            ])
            .await?;
        set_dump(&f, &dump);
        let mut reg: RegisteredComposeInstallation = f.state.read("installation.json")?;
        reg.database_container_id = id.clone();
        f.state.write("installation.json", &reg)?;
        stage.write("installation.json", &reg)?;
        let journal = f.state.journal(f.operation.id)?;
        let verified = verify_update_checkpoint(
            &f.state,
            &f.owner,
            &f.operation,
            &process,
            &CheckpointRestoreBudget {
                timeout: Duration::from_secs(90),
                maximum_bytes: 128 * 1024 * 1024,
            },
            &token,
        )
        .await?;
        assert_eq!(verified.instance_id, f.operation.instance_id);
        assert_eq!(f.state.journal(f.operation.id)?, journal);
        let sandbox: Sandbox = stage.read("restore-intent.json")?;
        assert!(
            docker
                .containers(&format!("name=^/{}$", sandbox.name()))
                .await?
                .is_empty()
        );
        // A checksummed custom-format header is insufficient: PostgreSQL must
        // actually restore the archive, including its data sections.
        set_dump(&broken, &dump[..64]);
        let mut broken_reg: RegisteredComposeInstallation =
            broken.state.read("installation.json")?;
        broken_reg.database_container_id = id.clone();
        broken.state.write("installation.json", &broken_reg)?;
        broken_stage.write("installation.json", &broken_reg)?;
        assert!(
            verify_update_checkpoint(
                &broken.state,
                &broken.owner,
                &broken.operation,
                &process,
                &CheckpointRestoreBudget {
                    timeout: Duration::from_secs(90),
                    maximum_bytes: 128 * 1024 * 1024
                },
                &token
            )
            .await
            .is_err()
        );
        assert!(
            broken_stage
                .read_optional::<RestoredUpdateCheckpoint>("restore-verified.json")?
                .is_none()
        );
        let rejected: Sandbox = broken_stage.read("restore-intent.json")?;
        assert!(
            docker
                .containers(&format!("name=^/{}$", rejected.name()))
                .await?
                .is_empty()
        );
        // The test's source was neither stopped nor modified by rehearsal.
        let state = docker.json(&["container", "inspect", &id]).await?;
        assert_eq!(single(&state)?["State"]["Running"], Value::Bool(true));
        let marker = docker
            .run(&[
                "exec",
                &id,
                "psql",
                "-h",
                "/var/run/postgresql",
                "-U",
                "postgres",
                "-d",
                DATABASE,
                "-Atc",
                "SELECT value FROM checkpoint_marker",
            ])
            .await?;
        assert_eq!(marker, b"restored-data\n");
        Ok::<_, UpdateError>(())
    };
    let result = std::panic::AssertUnwindSafe(exercise).catch_unwind().await;
    let cleanup_token = CancellationToken::new();
    let cleanup = LocalDocker {
        process: &process,
        cancellation: &cleanup_token,
        home: home.path(),
        deadline: Instant::now() + Duration::from_secs(60),
    };
    let cleaned = source.cleanup(&cleanup).await;
    // Also clean our own rehearsal on a test assertion/verification failure.
    for directory in [&stage, &broken_stage] {
        if let Ok(Some(attempt)) = directory.read_optional::<Sandbox>("restore-intent.json") {
            attempt.cleanup(&cleanup).await.unwrap();
        }
    }
    cleaned.unwrap();
    result.unwrap().unwrap();
}
