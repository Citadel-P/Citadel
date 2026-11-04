use super::super::{files::validate_absolute, registration::fingerprint_inputs};
use super::resolution::validate_environment;
use super::*;
use citadel_execution::ProcessOutput;
use citadel_execution::{ProcessError, ProcessRequest, ProcessRunner};
use citadel_updates::UpdateError;
use futures_util::future::BoxFuture;
use serde_json::Value;
use serde_json::json;
use std::time::Duration;
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use std::{os::unix::fs::symlink, sync::Mutex};
use tokio_util::sync::CancellationToken;

fn template(name: &str) -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../deploy/install")
            .join(name),
    )
    .unwrap()
}

const ENV: &str = "PG_PASSWORD=private-fixture-password\nCITADEL_HTTP_PORT=19000\n";

pub(super) struct Fixture {
    _directory: tempfile::TempDir,
    pub(super) inputs: InstallerComposeInputs,
}

impl Fixture {
    pub(super) fn new(edge: bool) -> Self {
        let directory = tempfile::Builder::new()
            .prefix("citadel-compose-fixture-")
            .permissions(fs::Permissions::from_mode(0o700))
            .tempdir()
            .unwrap();
        let path = directory.path();
        fs::write(
            path.join("docker-compose.yml"),
            template("docker-compose.yml"),
        )
        .unwrap();
        fs::write(path.join("compose.edge.yml"), template("compose.edge.yml")).unwrap();
        fs::write(path.join(".env"), ENV).unwrap();
        fs::set_permissions(path.join(".env"), fs::Permissions::from_mode(0o600)).unwrap();
        let mut compose_files = vec![path.join("docker-compose.yml")];
        if edge {
            compose_files.push(path.join("compose.edge.yml"));
        }
        let inputs = InstallerComposeInputs {
            directory: path.into(),
            project: "citadel-fixture".into(),
            compose_files,
        };
        Self {
            _directory: directory,
            inputs,
        }
    }
}

pub(super) fn resolved() -> Value {
    let mut config = json!({
        "name": "citadel-fixture",
        "services": {
            "server": {"image": "ghcr.io/citadel-p/citadel:latest", "environment": {
                "PG_HOST": "pg_db", "PG_PORT": "5432", "PG_USER": "citadel",
                "PG_DATABASE": "citadel", "PG_PASSWORD": "private-fixture-password"
            }},
            "pg_db": {"environment": {"POSTGRES_USER": "citadel", "POSTGRES_DB": "citadel", "POSTGRES_PASSWORD": "private-fixture-password"}}
        },
        "volumes": {"citadel_data": {"name": "citadel-fixture_citadel_data"}, "postgres_data": {"name": "citadel-fixture_postgres_data"}},
        "networks": {"default": {"name": "citadel-fixture_default"}}
    });
    profile_fixture(&mut config);
    config
}

pub(in crate::external::updates) fn profile_fixture(config: &mut Value) {
    for role in ["server", "pg_db"] {
        config["services"][role]["restart"] = json!("unless-stopped");
        config["services"][role]["networks"] = json!({"default": null});
        config["services"][role]["healthcheck"] = json!({"test": ["CMD", "healthcheck"]});
    }
    config["services"]["server"]["depends_on"] = json!({"pg_db": {"condition":"service_healthy"}});
    config["services"]["pg_db"]["image"] = json!("postgres:18.6");
    config["services"]["server"]["volumes"] = json!([
        {"type":"volume", "source":"citadel_data", "target":"/app/data"},
        {"type":"bind", "source":"/", "target":"/host", "read_only":true},
        {"type":"bind", "source":"/var/run/docker.sock", "target":"/var/run/docker.sock"}
    ]);
    config["services"]["pg_db"]["volumes"] = json!([
        {"type":"volume", "source":"postgres_data", "target":"/var/lib/postgresql"}
    ]);
}

struct Runner {
    output: Mutex<Option<ProcessOutput>>,
    snapshot: Mutex<Option<PathBuf>>,
    change_input: Option<PathBuf>,
}

impl Runner {
    fn new(value: Value) -> Self {
        Self {
            output: Mutex::new(Some(ProcessOutput {
                exit_code: Some(0),
                stdout: serde_json::to_vec(&value).unwrap(),
                stderr: vec![],
                stdout_truncated: false,
                stderr_truncated: false,
            })),
            snapshot: Mutex::new(None),
            change_input: None,
        }
    }
}

impl ProcessRunner for Runner {
    fn run<'a>(
        &'a self,
        request: ProcessRequest,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ProcessOutput, ProcessError>> {
        Box::pin(async move {
            let snapshot = request.current_directory.as_ref().unwrap();
            assert_eq!(request.program, "docker");
            assert!(request.output.is_none());
            assert!(request.stdin.is_none());
            assert_eq!(request.limits.timeout, Duration::from_secs(30));
            assert_eq!(request.limits.maximum_stdout_bytes, 1024 * 1024);
            assert_eq!(request.limits.maximum_stderr_bytes, 16 * 1024);
            assert_eq!(
                &request.arguments[..5],
                [
                    "--host",
                    "unix:///var/run/docker.sock",
                    "compose",
                    "--project-name",
                    "citadel-fixture"
                ]
            );
            assert_eq!(
                &request.arguments[request.arguments.len() - 3..],
                ["config", "--format", "json"]
            );
            assert_eq!(
                request.environment,
                [
                    ("HOME".into(), snapshot.join("home").into()),
                    ("DOCKER_CONFIG".into(), snapshot.join("home").into())
                ]
            );
            for path in [snapshot.clone(), snapshot.join(".env")] {
                assert_eq!(fs::metadata(path).unwrap().permissions().mode() & 0o077, 0);
            }
            assert_eq!(
                fs::read(snapshot.join("docker-compose.yml")).unwrap(),
                template("docker-compose.yml")
            );
            assert_eq!(fs::read_to_string(snapshot.join(".env")).unwrap(), ENV);
            *self.snapshot.lock().unwrap() = Some(snapshot.clone());
            if let Some(path) = &self.change_input {
                fs::write(path, "PG_PASSWORD=changed\n").unwrap();
            }
            Ok(self.output.lock().unwrap().take().unwrap())
        })
    }
}

#[tokio::test]
async fn checks_explicit_inputs_in_private_snapshot_and_returns_no_secrets() {
    for edge in [false, true] {
        let fixture = Fixture::new(edge);
        let runner = Runner::new(resolved());
        let checked = fixture
            .inputs
            .check(&runner, &CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(checked.core_volume, "citadel-fixture_citadel_data");
        assert_eq!(checked.database_volume, "citadel-fixture_postgres_data");
        assert_eq!(checked.network, "citadel-fixture_default");
        assert_eq!(
            checked.configuration_sha256,
            fingerprint_inputs(&fixture.inputs.project, &fixture.inputs.capture().unwrap())
        );
        let json = serde_json::to_string(&checked).unwrap();
        assert!(!json.contains("private-fixture-password"));
        assert!(!json.contains("PG_PASSWORD"));
        assert!(!runner.snapshot.lock().unwrap().as_ref().unwrap().exists());
    }
}

#[tokio::test]
async fn changed_inputs_cannot_produce_review_evidence() {
    let fixture = Fixture::new(false);
    let mut runner = Runner::new(resolved());
    runner.change_input = Some(fixture.inputs.directory.join(".env"));
    assert!(matches!(
        fixture
            .inputs
            .check(&runner, &CancellationToken::new())
            .await,
        Err(UpdateError::Conflict(_))
    ));
    assert!(!runner.snapshot.lock().unwrap().as_ref().unwrap().exists());
}

#[test]
fn rejects_invalid_projects_paths_and_external_dependencies_before_running_compose() {
    let mut fixture = Fixture::new(true);
    for project in ["", "-option", "UPPERCASE", "../other", "project;touch-file"] {
        fixture.inputs.project = project.into();
        assert!(fixture.inputs.capture().is_err());
    }
    fixture.inputs.project = "citadel-fixture".into();
    fs::write(&fixture.inputs.compose_files[1], b"include: /etc/secret\n").unwrap();
    assert!(fixture.inputs.capture().is_err());
    for path in [
        "relative",
        "/tmp/../etc",
        "/tmp/./installer",
        "/tmp//installer",
        "/tmp/installer/",
    ] {
        assert!(
            validate_absolute(std::path::Path::new(path)).is_err(),
            "{path}"
        );
    }
}

#[test]
fn rejects_nonprivate_symlinked_and_oversized_inputs() {
    let fixture = Fixture::new(false);
    let env = fixture.inputs.directory.join(".env");
    fs::set_permissions(&env, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(fixture.inputs.capture().is_err());
    fs::set_permissions(&env, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(&env, vec![b'x'; 1024 * 1024 + 1]).unwrap();
    assert!(fixture.inputs.capture().is_err());
    fs::remove_file(&env).unwrap();
    symlink("compose.edge.yml", &env).unwrap();
    assert!(fixture.inputs.capture().is_err());
}

#[test]
fn ambiguous_environment_and_compose_overrides_are_rejected() {
    for env in [
        "PG_PASSWORD=a\nPG_PASSWORD=b",
        "PG_PASSWORD=$OTHER",
        "export PG_PASSWORD=a",
        "COMPOSE_FILE=other.yml",
        "COMPOSE_ENV_FILES=other.env",
        "COMPOSE_PROFILES=other",
        "COMPOSE_PROJECT_NAME=other",
        "PG_PASSWORD=one\ntwo",
        "1BAD=value",
    ] {
        assert!(
            validate_environment(env.as_bytes(), "citadel-fixture").is_err(),
            "{env}"
        );
    }
    validate_environment(
        b"# comment\nCOMPOSE_PROJECT_NAME=citadel-fixture\nPG_PASSWORD=abc123\n",
        "citadel-fixture",
    )
    .unwrap();
}

#[test]
fn rejects_external_database_remote_agent_and_changed_persistence_roots() {
    let fixture = Fixture::new(false);
    for (key, value) in [
        ("PG_HOST", "external"),
        ("PG_PORT", "5433"),
        ("PG_PASSWORD", "wrong"),
        ("CITADEL_DATABASE_URL", "postgres://secret@external/db"),
        ("CITADEL_CORE_AGENT_ADDRESS", "https://agent"),
        ("CITADEL_DOCKER_SOCKET", "/other.sock"),
        ("CITADEL_DATA_ROOT", "/tmp/data"),
    ] {
        let mut config = resolved();
        config["services"]["server"]["environment"][key] = json!(value);
        let error = fixture
            .inputs
            .validate_resolved(&config, "a".repeat(64))
            .unwrap_err();
        assert!(
            !error.to_string().contains(value),
            "secret or raw value leaked: {error}"
        );
    }
}

#[test]
fn rejects_missing_resources_and_unofficial_images() {
    let fixture = Fixture::new(false);
    for image in [
        "ghcr.io/citadel-p/citadel:v0.1.1",
        "docker.io/citadelplane/citadel:v0.1.1",
        "citadelplane/citadel:v0.1.1",
    ] {
        let mut config = resolved();
        config["services"]["server"]["image"] = json!(image);
        fixture
            .inputs
            .validate_resolved(&config, "a".repeat(64))
            .unwrap();
    }
    for image in [
        "attacker/citadel:latest",
        "ghcr.io/citadel-p/citadel:",
        "ghcr.io/citadel-p/citadel:tag\nsecret",
        "ghcr.io/citadel-p/citadel@sha256:bad",
    ] {
        let mut config = resolved();
        config["services"]["server"]["image"] = json!(image);
        assert!(
            fixture
                .inputs
                .validate_resolved(&config, "a".repeat(64))
                .is_err()
        );
    }
    for key in ["volumes", "networks", "services", "name"] {
        let mut config = resolved();
        config.as_object_mut().unwrap().remove(key);
        assert!(
            fixture
                .inputs
                .validate_resolved(&config, "a".repeat(64))
                .is_err()
        );
    }
}

#[tokio::test]
async fn tool_failures_and_truncated_or_invalid_output_do_not_leak_secrets() {
    let fixture = Fixture::new(false);
    for mode in 0..4 {
        let runner = Runner::new(resolved());
        {
            let mut guard = runner.output.lock().unwrap();
            let output = guard.as_mut().unwrap();
            output.stderr = b"private-fixture-password".to_vec();
            match mode {
                0 => output.exit_code = Some(1),
                1 => output.stdout_truncated = true,
                2 => output.stderr_truncated = true,
                _ => output.stdout = b"private-fixture-password".to_vec(),
            }
        }
        let error = fixture
            .inputs
            .check(&runner, &CancellationToken::new())
            .await
            .unwrap_err();
        assert!(!error.to_string().contains("private-fixture-password"));
        assert!(!runner.snapshot.lock().unwrap().as_ref().unwrap().exists());
    }
}

#[tokio::test]
async fn pre_cancelled_check_does_not_run_compose() {
    let fixture = Fixture::new(false);
    let runner = Runner::new(resolved());
    let token = CancellationToken::new();
    token.cancel();
    assert!(matches!(
        fixture.inputs.check(&runner, &token).await,
        Err(UpdateError::Cancelled)
    ));
    assert!(runner.snapshot.lock().unwrap().is_none());
}

struct CancellingRunner;
impl ProcessRunner for CancellingRunner {
    fn run<'a>(
        &'a self,
        request: ProcessRequest,
        token: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ProcessOutput, ProcessError>> {
        Box::pin(async move {
            token.cancel();
            tokio::task::yield_now().await;
            // The owner must await runner cleanup, retaining secret inputs until
            // the child has actually exited, even after cancellation is signalled.
            assert!(request.current_directory.unwrap().join(".env").exists());
            Err(ProcessError::Cancelled)
        })
    }
}

#[tokio::test]
async fn cancellation_awaits_tool_cleanup() {
    let fixture = Fixture::new(false);
    assert!(matches!(
        fixture
            .inputs
            .check(&CancellingRunner, &CancellationToken::new())
            .await,
        Err(UpdateError::Cancelled)
    ));
}

#[tokio::test]
async fn external_dependencies_are_rejected_before_compose_can_read_them() {
    for input in [
        "include: /private/secret.yml\nservices: {}",
        "include: https://example.test/compose.yml\nservices: {}",
        "services: {server: {extends: {file: /private/secret.yml, service: core}}}",
        "services: {server: {build: /private/context}}",
        "services: {server: {label_file: /private/labels}}",
        "services: {server: {env_file: [{path: /private/env, format: raw}]}}",
        "services: {server: {env_file: [{path: '${ENV_FILE}', format: raw}]}}",
        "services: {server: {env_file: [{path: ../secret, format: raw}]}}",
        "services: {}\nsecrets: {password: {file: /private/password}}",
        "services: {}\nconfigs: {config: {file: /private/config}}",
        "services: {server: {env_file: [{path: .env, format: raw}, {path: /private/env}]}}",
        "services: {server: {image: good, image: bad}}",
        "services: {server: {ports: !override []}}",
        "services: {server: {<<: {env_file: /private/env}}}",
    ] {
        let fixture = Fixture::new(false);
        fs::write(&fixture.inputs.compose_files[0], input).unwrap();
        let runner = Runner::new(resolved());
        assert!(
            fixture
                .inputs
                .check(&runner, &CancellationToken::new())
                .await
                .is_err(),
            "{input}"
        );
        assert!(runner.snapshot.lock().unwrap().is_none());
    }
}

#[test]
fn registered_file_names_and_order_are_not_bound_to_templates() {
    let mut fixture = Fixture::new(true);
    let base = fixture.inputs.directory.join("installation.yaml");
    let overlay = fixture.inputs.directory.join("ports.yaml");
    fs::rename(&fixture.inputs.compose_files[0], &base).unwrap();
    fs::rename(&fixture.inputs.compose_files[1], &overlay).unwrap();
    fixture.inputs.compose_files = vec![base.clone(), overlay.clone()];
    let first = fixture.inputs.capture().unwrap();
    let original = fingerprint_inputs(&fixture.inputs.project, &first);
    fs::write(
        &base,
        [
            b"# Local installation notes\n".as_slice(),
            &fs::read(&base).unwrap(),
        ]
        .concat(),
    )
    .unwrap();
    let changed = fixture.inputs.capture().unwrap();
    assert_ne!(
        fingerprint_inputs(&fixture.inputs.project, &changed),
        original
    );
    fixture.inputs.compose_files.reverse();
    let reversed = fixture.inputs.capture().unwrap();
    assert_eq!(reversed[0].0, overlay);
    assert_eq!(reversed[1].0, base.clone());
    assert_ne!(
        fingerprint_inputs(&fixture.inputs.project, &reversed),
        fingerprint_inputs(&fixture.inputs.project, &changed)
    );
    fixture.inputs.compose_files.push(base);
    assert!(fixture.inputs.capture().is_err());
}

#[test]
fn explicit_installer_database_path_preserves_the_supported_volume_layout() {
    let fixture = Fixture::new(false);
    let mut config = resolved();
    config["services"]["pg_db"]["environment"]["PGDATA"] = json!("/var/lib/postgresql/18/docker");
    assert!(
        fixture
            .inputs
            .validate_resolved(&config, "a".repeat(64))
            .is_ok()
    );
    config["services"]["pg_db"]["environment"]["PGDATA"] = json!("/var/lib/postgresql/16/docker");
    assert!(
        fixture
            .inputs
            .validate_resolved(&config, "a".repeat(64))
            .is_err()
    );
}

#[test]
fn resolved_configuration_must_keep_the_supported_storage_and_service_layout() {
    let fixture = Fixture::new(false);
    for mutation in 0..9 {
        let mut config = resolved();
        match mutation {
            0 => config["services"]["server"]["volumes"][0]["source"] = json!("other_data"),
            1 => config["services"]["server"]["volumes"][1]["read_only"] = json!(false),
            2 => config["services"]["pg_db"]["volumes"][0]["type"] = json!("bind"),
            3 => config["services"]["pg_db"]["environment"]["PGDATA"] = json!("/tmp/data"),
            4 => config["volumes"]["postgres_data"]["driver_opts"] = json!({"device":"/host/data"}),
            5 => config["services"]["pg_db"]["ports"] = json!([{"target":5432,"published":"5432"}]),
            6 => config["services"]["server"]["networks"] = json!({"unregistered": null}),
            7 => config["services"]["server"]["healthcheck"]["disable"] = json!(true),
            _ => config["services"]["server"]["volumes"][0]["volume"] = json!({"subpath":"other"}),
        }
        assert!(
            fixture
                .inputs
                .validate_resolved(&config, "a".repeat(64))
                .is_err(),
            "mutation {mutation}"
        );
    }
}

#[tokio::test]
#[ignore = "requires Docker Compose CLI; resolves disposable customized inputs without a daemon"]
async fn actual_compose_accepts_compatible_customization_without_embedded_templates() {
    let mut fixture = Fixture::new(true);
    let base = fixture.inputs.directory.join("installation.yaml");
    let overlay = fixture.inputs.directory.join("ports.yaml");
    let mut config: Value = serde_yaml_ng::from_slice(&template("docker-compose.yml")).unwrap();
    config["volumes"]["citadel_data"]["name"] = json!("custom_core");
    config["volumes"]["postgres_data"]["name"] = json!("custom_database");
    config["networks"] = json!({"default": {"name":"custom_network"}});
    fs::write(
        &base,
        format!(
            "# Installation-specific comments and resource names\n{}",
            serde_yaml_ng::to_string(&config).unwrap()
        ),
    )
    .unwrap();
    fs::rename(&fixture.inputs.compose_files[1], &overlay).unwrap();
    fixture.inputs.compose_files = vec![base, overlay];
    let checked = fixture
        .inputs
        .check(&citadel_processes::SystemProcess, &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(checked.core_volume, "custom_core");
    assert_eq!(checked.database_volume, "custom_database");
    assert_eq!(checked.network, "custom_network");
}

#[tokio::test]
#[ignore = "requires Docker Compose CLI; resolves disposable files only, without contacting a daemon"]
async fn actual_compose_resolves_official_base_and_edge_without_a_daemon() {
    for edge in [false, true] {
        let fixture = Fixture::new(edge);
        let checked = fixture
            .inputs
            .check(&citadel_processes::SystemProcess, &CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(checked.core_volume, "citadel-fixture_citadel_data");
        assert_eq!(checked.database_volume, "citadel-fixture_postgres_data");
        assert_eq!(checked.network, "citadel-fixture_default");
        assert_eq!(checked.core_image, "ghcr.io/citadel-p/citadel:latest");
    }
}
