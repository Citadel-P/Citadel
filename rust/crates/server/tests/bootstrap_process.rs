//! Real Core process + PostgreSQL ports of FirstRunSetupAcceptanceTests.
//! No fake principal or in-process router bypasses startup/authentication here.
use std::{path::PathBuf, process::Stdio, time::Duration};

use reqwest::Client;
use serde_json::{Value, json};
use sqlx::{Connection, PgConnection, PgPool};
use tokio::{
    io::AsyncReadExt,
    process::{Child, Command},
    task::JoinHandle,
};
use uuid::Uuid;

const PASSWORD: &str = "citadel-bootstrap-parity-passphrase";

#[path = "bootstrap_process/setup.rs"]
mod setup;

#[cfg(unix)]
#[path = "bootstrap_process/lifecycle.rs"]
mod lifecycle;

struct Fixture {
    admin_url: String,
    database_name: String,
    database_url: String,
    directory: PathBuf,
    client: Client,
}

struct Server {
    child: Child,
    output: JoinHandle<String>,
    errors: JoinHandle<String>,
    url: String,
    #[cfg(unix)]
    edge_port: u16,
}

impl Fixture {
    async fn new() -> Self {
        let admin_url = std::env::var("CITADEL_TEST_DATABASE_URL")
            .expect("dedicated PostgreSQL test instance required");
        let database_name = format!("citadel_bootstrap_{}", Uuid::now_v7().simple());
        let mut admin = PgConnection::connect(&admin_url).await.unwrap();
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "CREATE DATABASE {database_name}"
        )))
        .execute(&mut admin)
        .await
        .unwrap();
        let mut url = url::Url::parse(&admin_url).unwrap();
        url.set_path(&database_name);
        let directory = std::env::temp_dir().join(&database_name);
        tokio::fs::create_dir(&directory).await.unwrap();
        Self {
            admin_url,
            database_name,
            database_url: url.into(),
            directory,
            client: Client::builder()
                .timeout(Duration::from_secs(3))
                .build()
                .unwrap(),
        }
    }

    fn start(&self, bootstrap: bool, partial: bool) -> Server {
        self.start_with_realtime(bootstrap, partial, true)
    }

    fn start_with_realtime(&self, bootstrap: bool, partial: bool, realtime: bool) -> Server {
        // Bind ephemeral test ports, never use the running development server.
        let http = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let edge = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let http_port = http.local_addr().unwrap().port();
        let edge_port = edge.local_addr().unwrap().port();
        let url = format!("http://127.0.0.1:{http_port}");
        let mut command = Command::new(env!("CARGO_BIN_EXE_citadel-server"));
        command
            .arg("serve")
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("DATABASE_URL", &self.database_url)
            .env("Transport__Mode", "Disabled")
            .env("EnableSwagger", "true")
            .env("CITADEL_RUST_REALTIME_ENABLED", realtime.to_string())
            .env("Transport__ApiPort", http_port.to_string())
            .env("Transport__EdgeGrpcPort", edge_port.to_string())
            .env("Transport__PublicUrl", &url)
            .env("Jwt__Key", "bootstrap-test-jwt-key-at-least-32-bytes")
            .env(
                "Secrets__EncryptionKey",
                "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=",
            )
            .env("CITADEL_DATA_ROOT", &self.directory)
            .env(
                "CITADEL_RUST_DOCKER_SOCKET",
                self.directory.join("absent-docker.sock"),
            )
            .current_dir(&self.directory)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if bootstrap {
            command.env("Bootstrap__AdminName", "bootstrap-admin");
            if !partial {
                command
                    .env("Bootstrap__AdminEmail", "bootstrap@example.test")
                    .env(
                        "Bootstrap__AdminPasswordFile",
                        self.directory.join("password"),
                    );
            }
        }
        drop((http, edge));
        let mut child = command.spawn().unwrap();
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        Server {
            child,
            output: tokio::spawn(capture(stdout)),
            errors: tokio::spawn(capture(stderr)),
            url,
            #[cfg(unix)]
            edge_port,
        }
    }

    async fn ready(&self, server: &mut Server) -> Value {
        tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                assert!(
                    server.child.try_wait().unwrap().is_none(),
                    "Core exited before serving setup status"
                );
                if let Ok(response) = self
                    .client
                    .get(format!("{}/api/v1/setup/status", server.url))
                    .send()
                    .await
                {
                    if response.status().is_success() {
                        return response.json().await.unwrap();
                    }
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("Core startup timeout")
    }

    async fn login(&self, server: &Server) {
        let response = self
            .client
            .post(format!("{}/api/v1/authentication/login", server.url))
            .json(&json!({"emailOrName":"bootstrap-admin", "password":PASSWORD}))
            .send()
            .await
            .unwrap();
        assert!(
            response.status().is_success(),
            "login status {}",
            response.status()
        );
        let result: Value = response.json().await.unwrap();
        assert_eq!(result["nextStep"], "Completed");
        let profile = self
            .client
            .get(format!("{}/api/v1/profile", server.url))
            .bearer_auth(result["accessToken"].as_str().unwrap())
            .send()
            .await
            .unwrap();
        assert!(profile.status().is_success());
    }

    async fn close(self) {
        let mut admin = PgConnection::connect(&self.admin_url).await.unwrap();
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP DATABASE {} WITH (FORCE)",
            self.database_name
        )))
        .execute(&mut admin)
        .await
        .unwrap();
        tokio::fs::remove_dir_all(&self.directory).await.unwrap();
    }
}

async fn capture(mut source: impl tokio::io::AsyncRead + Unpin) -> String {
    let mut result = Vec::new();
    let mut buffer = [0_u8; 4096];
    while let Ok(count) = source.read(&mut buffer).await {
        if count == 0 {
            break;
        }
        let keep = count.min(64 * 1024 - result.len());
        result.extend_from_slice(&buffer[..keep]);
    }
    String::from_utf8_lossy(&result).into_owned()
}

impl Server {
    async fn stop(mut self) -> String {
        self.child.kill().await.unwrap();
        self.child.wait().await.unwrap();
        format!(
            "{}{}",
            self.output.await.unwrap(),
            self.errors.await.unwrap()
        )
    }
}

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission; starts isolated Core processes"]
async fn password_file_bootstrap_initializes_once_and_ignores_removed_file_after_restart() {
    // FirstRunSetupAcceptanceTests.PasswordFileBootstrap_ShouldInitializeOnceAndIgnoreOptionsAfterRestart.
    let fixture = Fixture::new().await;
    tokio::fs::write(fixture.directory.join("password"), format!("{PASSWORD}\n"))
        .await
        .unwrap();
    let mut first = fixture.start(true, false);
    assert_eq!(fixture.ready(&mut first).await["requiresSetup"], false);
    let pool = PgPool::connect(&fixture.database_url).await.unwrap();
    let sessions: i64 = sqlx::query_scalar("SELECT count(*) FROM refreshtokens")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        sessions, 0,
        "unattended setup must not issue a browser session"
    );
    fixture.login(&first).await;
    assert!(!first.stop().await.contains(PASSWORD));
    tokio::fs::remove_file(fixture.directory.join("password"))
        .await
        .unwrap();
    let mut restarted = fixture.start(true, false);
    assert_eq!(fixture.ready(&mut restarted).await["requiresSetup"], false);
    fixture.login(&restarted).await;
    assert!(!restarted.stop().await.contains(PASSWORD));
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM users WHERE name='bootstrap-admin' AND email='bootstrap@example.test'").fetch_one(&pool).await.unwrap(),1);
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM actorroles ar JOIN users u ON ar.actorid=u.actorid JOIN roles r ON r.id=ar.roleid WHERE u.name='bootstrap-admin' AND r.name='Admin'").fetch_one(&pool).await.unwrap(),1);
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM actions WHERE name IN ('Prune images','Restart unhealthy stacks') AND NOT enabled AND NOT scheduleenabled").fetch_one(&pool).await.unwrap(),2);
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM resourcetags rt JOIN actions a ON a.id=rt.resourceid WHERE rt.resourcetype='AutomationAction'").fetch_one(&pool).await.unwrap(),3);
    let activity: Value = sqlx::query_scalar(
        "SELECT info::jsonb FROM activityevents WHERE eventtype='InitialAdministratorCreated'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(activity["Mode"], "Unattended");
    assert!(!activity.to_string().contains(PASSWORD));
    pool.close().await;
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission; starts isolated Core processes"]
async fn partial_bootstrap_fails_before_serving_and_leaves_setup_pending_after_restart() {
    // FirstRunSetupAcceptanceTests.PartialBootstrapConfiguration_ShouldFailAndLeaveSetupPending.
    let fixture = Fixture::new().await;
    let mut failed = fixture.start(true, true);
    let status = tokio::time::timeout(Duration::from_secs(30), failed.child.wait())
        .await
        .expect("incomplete bootstrap must stop Core")
        .unwrap();
    assert!(!status.success());
    let output = format!(
        "{}{}",
        failed.output.await.unwrap(),
        failed.errors.await.unwrap()
    );
    assert!(
        output.contains("Incomplete bootstrap administrator configuration"),
        "{output}"
    );
    let pool = PgPool::connect(&fixture.database_url).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM users")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT initializedat IS NULL FROM instancesetupstates WHERE id=1"
        )
        .fetch_one(&pool)
        .await
        .unwrap()
    );
    pool.close().await;
    let mut restarted = fixture.start(false, false);
    assert_eq!(fixture.ready(&mut restarted).await["requiresSetup"], true);
    restarted.stop().await;
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission; starts an isolated Core process"]
async fn concurrent_interactive_setup_has_one_winner_and_one_set_of_disabled_defaults() {
    // SetupEndpointTests.Concurrent_Initialization_Should_Create_Exactly_One_Admin.
    let fixture = Fixture::new().await;
    let mut server = fixture.start(false, false);
    assert_eq!(fixture.ready(&mut server).await["requiresSetup"], true);
    let url = format!("{}/api/v1/setup/initialize", server.url);
    let create = |name| {
        fixture
            .client
            .post(&url)
            .json(&json!({"name":name,"email":format!("{name}@example.test"),"password":PASSWORD}))
            .send()
    };
    let (left, right) = tokio::join!(create("first-owner"), create("second-owner"));
    let mut statuses = [
        left.unwrap().status().as_u16(),
        right.unwrap().status().as_u16(),
    ];
    statuses.sort();
    assert_eq!(statuses, [200, 409]);
    server.stop().await;
    let pool = PgPool::connect(&fixture.database_url).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM users")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM actions WHERE NOT enabled AND NOT scheduleenabled"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        2
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM actions a JOIN instancesetupstates s ON s.initialadministratoractorid=a.runasactorid").fetch_one(&pool).await.unwrap(),2);
    let activity: Value = sqlx::query_scalar(
        "SELECT info::jsonb FROM activityevents WHERE eventtype='InitialAdministratorCreated'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(activity["Mode"], "Interactive");
    pool.close().await;
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission; starts an isolated Core process"]
async fn default_action_persistence_failure_rolls_back_entire_setup() {
    let fixture = Fixture::new().await;
    citadel_database::MigrationRunner::migrate(&fixture.database_url)
        .await
        .unwrap();
    let pool = PgPool::connect(&fixture.database_url).await.unwrap();
    let mut baseline = Vec::new();
    for table in ["users", "actors", "actions", "actorroles"] {
        let count: i64 =
            sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
                .fetch_one(&pool)
                .await
                .unwrap();
        baseline.push((table, count));
    }
    // Failure occurs after user/role creation and the first default action.
    sqlx::query("DELETE FROM tags WHERE id='40000000-0000-0000-0000-000000000002'")
        .execute(&pool)
        .await
        .unwrap();
    tokio::fs::write(fixture.directory.join("password"), PASSWORD)
        .await
        .unwrap();
    let mut failed = fixture.start(true, false);
    let status = tokio::time::timeout(Duration::from_secs(30), failed.child.wait())
        .await
        .unwrap()
        .unwrap();
    assert!(!status.success());
    assert!(!failed.output.await.unwrap().contains(PASSWORD));
    assert!(!failed.errors.await.unwrap().contains(PASSWORD));
    for (table, before) in baseline {
        let count: i64 =
            sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, before, "{table} must roll back");
    }
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT initializedat IS NULL FROM instancesetupstates WHERE id=1"
        )
        .fetch_one(&pool)
        .await
        .unwrap()
    );
    pool.close().await;
    fixture.close().await;
}

#[tokio::test]
async fn offline_restore_rejects_missing_or_malformed_encryption_key_before_accessing_target() {
    for key in [None, Some("not-a-base64-key")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_citadel-server"));
        command
            .env_clear()
            .args([
                "restore-system",
                "--bundle",
                "missing-test-bundle",
                "--confirm-instance-replacement",
            ])
            .env(
                "DATABASE_URL",
                "postgres://unused:unused@127.0.0.1:1/never-connect",
            )
            .env("Jwt__Key", "restore-test-jwt-key-at-least-32-bytes")
            .kill_on_drop(true);
        if let Some(key) = key {
            command.env("Secrets__EncryptionKey", key);
        }
        let output = tokio::time::timeout(Duration::from_secs(10), command.output())
            .await
            .unwrap()
            .unwrap();
        assert!(!output.status.success());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("Secrets__EncryptionKey"), "{error}");
        assert!(!error.contains("not-a-base64-key"));
        assert!(
            !error.contains("missing-test-bundle"),
            "key preflight must precede bundle/DB work"
        );
    }
}
