use super::{DockerReleaseInspection, probe::OWNER};
use citadel_execution::{ProcessError, ProcessOutput, ProcessRequest, ProcessRunner};
use citadel_updates::{
    ReleaseInspectionRuntime, UpdateError, UpgradeDescriptor, VerifiedReleaseArtifact,
};
use futures_util::future::BoxFuture;
use serde_json::{Value, json};
use std::{
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    Success,
    PullFailure,
    WrongArchitecture,
    WrongDigest,
    Volumes,
    CreateTimeout,
    CancelCreate,
    CancelStart,
    InvalidJson,
    Truncated,
    NonzeroExit,
    StillRunning,
    CleanupFailure,
    ForeignOwner,
}

struct State {
    calls: Vec<String>,
    name: String,
    nonce: String,
    home: Option<PathBuf>,
    started: bool,
    removed: bool,
}

struct Runner {
    state: Mutex<State>,
    case: Case,
    cancellation: CancellationToken,
    architecture: &'static str,
}

fn artifact() -> VerifiedReleaseArtifact {
    VerifiedReleaseArtifact {
        version: "1.2.3".into(),
        image: format!("ghcr.io/citadel-p/citadel@sha256:{}", "a".repeat(64)),
    }
}
fn image_id() -> String {
    format!("sha256:{}", "b".repeat(64))
}
fn container_id() -> String {
    "c".repeat(64)
}
fn output(value: impl Into<Vec<u8>>) -> ProcessOutput {
    ProcessOutput {
        exit_code: Some(0),
        stdout: value.into(),
        stderr: vec![],
        stdout_truncated: false,
        stderr_truncated: false,
    }
}
fn descriptor() -> UpgradeDescriptor {
    UpgradeDescriptor {
        schema_version: 1,
        product: "citadel".into(),
        target_version: "1.2.3".into(),
        target_catalog_sha256: "d".repeat(64),
        installer_format: citadel_updates::INSTALLER_FORMAT,
        update_protocol: citadel_updates::UPDATE_PROTOCOL,
        sources: vec![],
    }
}

impl Runner {
    fn new(case: Case) -> Self {
        Self {
            state: Mutex::new(State {
                calls: vec![],
                name: String::new(),
                nonce: String::new(),
                home: None,
                started: false,
                removed: false,
            }),
            case,
            cancellation: CancellationToken::new(),
            architecture: "amd64",
        }
    }
}

impl ProcessRunner for Runner {
    fn run<'a>(
        &'a self,
        request: ProcessRequest,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ProcessOutput, ProcessError>> {
        Box::pin(async move {
            assert!(
                !cancellation.is_cancelled(),
                "cleanup/create must not inherit cancellation"
            );
            assert_eq!(request.program, "docker");
            assert!(request.stdin.is_none());
            assert!(request.output.is_none());
            assert_eq!(request.environment.len(), 2);
            assert_eq!(request.environment[0].0, "HOME");
            assert_eq!(request.environment[1].0, "DOCKER_CONFIG");
            assert_eq!(request.environment[0].1, request.environment[1].1);
            let home = PathBuf::from(&request.environment[0].1);
            assert_eq!(
                std::fs::metadata(&home).unwrap().permissions().mode() & 0o077,
                0
            );
            assert_eq!(request.limits.maximum_stdout_bytes, 1024 * 1024);
            assert_eq!(request.limits.maximum_stderr_bytes, 16 * 1024);
            let args: Vec<_> = request
                .arguments
                .iter()
                .map(|a| a.to_str().unwrap())
                .collect();
            assert_eq!(&args[..2], ["--host", "unix:///var/run/docker.sock"]);
            let args = &args[2..];
            let platform = format!("linux/{}", self.architecture);
            let mut state = self.state.lock().unwrap();
            state.home = Some(home);
            let command = match args[0] {
                "image" | "container" => format!("{} {}", args[0], args[1]),
                other => other.into(),
            };
            state.calls.push(command.clone());
            assert_eq!(
                request.limits.timeout.as_secs(),
                match command.as_str() {
                    "pull" => 300,
                    "create" | "start" | "rm" => 30,
                    _ => 15,
                }
            );
            match command.as_str() {
                "pull" => {
                    assert_eq!(args, ["pull", "--platform", &platform, &artifact().image]);
                    if self.case == Case::PullFailure {
                        return Err(ProcessError::Spawn(std::io::Error::other("private detail")));
                    }
                    Ok(output(vec![]))
                }
                "image inspect" => {
                    assert_eq!(args.last(), Some(&artifact().image.as_str()));
                    if args == ["image", "inspect", "--format", "{{.Id}}", &artifact().image] {
                        return Ok(output(image_id()));
                    }
                    // Model a containerd store with both architectures cached:
                    // inspection without a platform selects the amd64 host image.
                    let selected = if args.windows(2).any(|v| v == ["--platform", &platform]) {
                        self.architecture
                    } else {
                        "amd64"
                    };
                    Ok(output(serde_json::to_vec(&json!({
                        "Id": format!("sha256:{}", "e".repeat(64)), "Os": "linux",
                        "Architecture": if self.case == Case::WrongArchitecture { "other" } else { selected },
                        "RepoDigests": if self.case == Case::WrongDigest { vec!["other".to_string()] } else { vec![artifact().image] },
                        "Config": {"Volumes": if self.case == Case::Volumes { json!({"/data":{}}) } else { Value::Null }},
                    })).unwrap()))
                }
                "create" => {
                    state.name = args[2].into();
                    state.nonce = args[4].strip_prefix(&format!("{OWNER}=")).unwrap().into();
                    assert_eq!(
                        state.name,
                        format!("citadel-update-inspection-{}", state.nonce)
                    );
                    assert!(Uuid::parse_str(&state.nonce).is_ok());
                    assert_eq!(
                        &args[5..],
                        [
                            "--platform",
                            &platform,
                            "--pull=never",
                            "--network=none",
                            "--read-only",
                            "--cap-drop=ALL",
                            "--security-opt=no-new-privileges",
                            "--user=65532:65532",
                            "--env=TOKIO_WORKER_THREADS=2",
                            "--pids-limit=32",
                            "--memory=128m",
                            "--cpus=1",
                            "--restart=no",
                            "--log-driver=none",
                            "--no-healthcheck",
                            "--entrypoint=/app/citadel-server",
                            &image_id(),
                            "upgrade-descriptor-json"
                        ]
                    );
                    if self.case == Case::CreateTimeout {
                        return Err(ProcessError::Timeout(Duration::from_secs(30)));
                    }
                    if self.case == Case::CancelCreate {
                        self.cancellation.cancel();
                        assert!(!cancellation.is_cancelled());
                    }
                    Ok(output(container_id()))
                }
                "container inspect" => {
                    assert_eq!(args.last(), Some(&container_id().as_str()));
                    Ok(output(serde_json::to_vec(&json!({
                        "Id": container_id(), "Name": format!("/{}", state.name), "Image": image_id(),
                        "Config": {"Labels": {OWNER: if self.case == Case::ForeignOwner { "foreign" } else { &state.nonce }}},
                        "State": {"Running": self.case == Case::StillRunning, "ExitCode": if self.case == Case::NonzeroExit {1} else {0}, "OOMKilled": false, "Error": "", "Status": if state.started {"exited"} else {"created"}},
                    })).unwrap()))
                }
                "start" => {
                    assert_eq!(args, ["start", "--attach", &container_id()]);
                    state.started = true;
                    if self.case == Case::CancelStart {
                        self.cancellation.cancel();
                        return Err(ProcessError::Cancelled);
                    }
                    let mut result = output(if self.case == Case::InvalidJson {
                        b"not json: private detail".to_vec()
                    } else {
                        serde_json::to_vec(&descriptor()).unwrap()
                    });
                    result.stdout_truncated = self.case == Case::Truncated;
                    Ok(result)
                }
                "container ls" => {
                    assert_eq!(
                        args,
                        [
                            "container",
                            "ls",
                            "--all",
                            "--no-trunc",
                            "--filter",
                            &format!("label={OWNER}={}", state.nonce),
                            "--filter",
                            &format!("name=^/{}$", state.name),
                            "--format",
                            "{{json .}}"
                        ]
                    );
                    Ok(output(
                        serde_json::to_vec(&json!({"ID":container_id(), "Names":state.name}))
                            .unwrap(),
                    ))
                }
                "rm" => {
                    assert_eq!(args, ["rm", "--force", "--volumes", &container_id()]);
                    if self.case == Case::CleanupFailure {
                        return Err(ProcessError::Timeout(Duration::from_secs(30)));
                    }
                    state.removed = true;
                    Ok(output(container_id()))
                }
                other => panic!("unexpected command {other}"),
            }
        })
    }
}

async fn run(case: Case) -> (Arc<Runner>, Result<UpgradeDescriptor, UpdateError>) {
    let runner = Arc::new(Runner::new(case));
    let result = DockerReleaseInspection::new(runner.clone())
        .inspect(&artifact(), "amd64", &runner.cancellation)
        .await;
    assert!(!runner.state.lock().unwrap().home.as_ref().unwrap().exists());
    (runner, result)
}

#[tokio::test]
async fn runs_only_the_digest_pinned_descriptor_with_isolation_and_awaits_cleanup() {
    let (runner, result) = run(Case::Success).await;
    assert_eq!(result.unwrap(), descriptor());
    assert_eq!(
        runner.state.lock().unwrap().calls,
        [
            "pull",
            "image inspect",
            "image inspect",
            "create",
            "container inspect",
            "start",
            "container inspect",
            "container ls",
            "container inspect",
            "rm"
        ]
    );
    assert!(runner.state.lock().unwrap().removed);
}

#[tokio::test]
async fn selects_requested_architecture_when_both_platforms_are_cached() {
    for architecture in ["amd64", "arm64"] {
        let mut runner = Runner::new(Case::Success);
        runner.architecture = architecture;
        let runner = Arc::new(runner);
        let result = DockerReleaseInspection::new(runner.clone())
            .inspect(&artifact(), architecture, &runner.cancellation)
            .await
            .unwrap();
        assert_eq!(result, descriptor());
        assert!(runner.state.lock().unwrap().removed);
    }
}

#[tokio::test]
async fn refuses_mismatched_images_and_implicit_storage_before_create() {
    for case in [
        Case::PullFailure,
        Case::WrongArchitecture,
        Case::WrongDigest,
        Case::Volumes,
    ] {
        let (runner, result) = run(case).await;
        assert!(result.is_err());
        assert!(!result.unwrap_err().to_string().contains("private detail"));
        assert!(runner.state.lock().unwrap().name.is_empty());
    }
}

#[tokio::test]
async fn cleans_up_cancelled_ambiguous_and_failed_execution_without_replaying_it() {
    for case in [
        Case::CreateTimeout,
        Case::CancelCreate,
        Case::CancelStart,
        Case::InvalidJson,
        Case::Truncated,
        Case::NonzeroExit,
        Case::StillRunning,
    ] {
        let (runner, result) = run(case).await;
        let error = result.unwrap_err();
        assert!(!error.to_string().contains("private detail"));
        if matches!(case, Case::CancelCreate | Case::CancelStart) {
            assert!(matches!(error, UpdateError::Cancelled));
        }
        let state = runner.state.lock().unwrap();
        assert!(state.removed);
        assert_eq!(
            state
                .calls
                .iter()
                .filter(|c| c.as_str() == "create")
                .count(),
            1
        );
        if case == Case::CancelCreate {
            assert!(!state.started);
        }
    }
}

#[tokio::test]
async fn cleanup_failure_never_returns_a_successful_inspection_or_deletes_foreign_resources() {
    for case in [Case::CleanupFailure, Case::ForeignOwner] {
        let (runner, result) = run(case).await;
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Could not verify cleanup")
        );
        let state = runner.state.lock().unwrap();
        assert!(!state.removed);
        if case == Case::ForeignOwner {
            assert!(!state.calls.iter().any(|c| c == "rm"));
            assert!(!state.started);
        }
    }
}

#[tokio::test]
async fn invalid_inputs_and_initial_cancellation_never_invoke_docker() {
    let runner = Arc::new(Runner::new(Case::Success));
    let runtime = DockerReleaseInspection::new(runner.clone());
    let mut invalid = artifact();
    invalid.image = "ghcr.io/citadel-p/citadel:latest".into();
    assert!(
        runtime
            .inspect(&invalid, "amd64", &runner.cancellation)
            .await
            .is_err()
    );
    assert!(
        runtime
            .inspect(&artifact(), "other", &runner.cancellation)
            .await
            .is_err()
    );
    runner.cancellation.cancel();
    assert!(matches!(
        runtime
            .inspect(&artifact(), "amd64", &runner.cancellation)
            .await,
        Err(UpdateError::Cancelled)
    ));
    assert!(runner.state.lock().unwrap().calls.is_empty());
}
