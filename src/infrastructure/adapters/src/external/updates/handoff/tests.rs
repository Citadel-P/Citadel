use super::super::{
    JournalDirectory,
    bootstrap::fixtures::{CORE, DATABASE, Fixture},
    docker::SOCKET,
};
use super::evidence::HelperAcknowledgement;
use super::*;
use citadel_execution::{ProcessError, ProcessOutput, ProcessRequest, ProcessRunner};
use citadel_updates::*;
use futures_util::future::BoxFuture;
use serde_json::{Value, json};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
const HELPER: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

struct Runner {
    fixture: Fixture,
    intent: HelperLaunchIntent,
    mode: Mutex<&'static str>,
    stops: AtomicUsize,
    creates: AtomicUsize,
    starts: AtomicUsize,
    launch_fixture: bool,
}
impl Runner {
    async fn new() -> Arc<Self> {
        Self::fixture(false).await
    }
    async fn fixture(launch_fixture: bool) -> Arc<Self> {
        let mut fixture = Fixture::new();
        fixture.runner.core["State"]["StartedAt"] = json!("2026-11-03T00:00:00Z");
        fixture.runner.core["RestartCount"] = json!(0);
        let registration = fixture.register().await.unwrap();
        let state = JournalDirectory::open(fixture.state.clone()).unwrap();
        let now = chrono::Utc::now();
        let source = CoreIdentity {
            version: "0.1.1".into(),
            image: format!("ghcr.io/citadel-p/citadel@sha256:{}", "a".repeat(64)),
            catalog_sha256: "b".repeat(64),
            architecture: "amd64".into(),
        };
        let operation = UpdateOperation::approve(
            UpdatePlan {
                id: Uuid::now_v7(),
                instance_id: registration.instance_id,
                source: source.clone(),
                target: CoreIdentity {
                    version: "0.1.2".into(),
                    ..source.clone()
                },
                helper_image: source.image,
                configuration_sha256: registration.configuration_sha256.clone(),
                registration_id: registration.id,
                reviewed_at: now,
                expires_at: now + chrono::Duration::minutes(10),
                checkpoint_directory: fixture.state.to_str().unwrap().into(),
                agent_requirements: vec![],
            },
            citadel_primitives::ActorId::new(Uuid::now_v7()),
            Uuid::now_v7(),
            now,
        )
        .unwrap();
        let intent = if launch_fixture {
            HelperLaunchIntent {
                protocol: UPDATE_PROTOCOL,
                nonce: Uuid::now_v7(),
                operation,
                registration,
            }
        } else {
            let intent = prepare_helper_launch(&state, &operation).unwrap();
            // Core's preparation lock must be released before launching the helper.
            drop(state.existing_lock().unwrap());
            bind_helper_launch(&state, &intent, HELPER).unwrap();
            state
                .create(
                    &intent.file("start-intent"),
                    &super::evidence::binding(&state, &intent).unwrap(),
                )
                .unwrap();
            intent
        };
        Arc::new(Self {
            fixture,
            intent,
            mode: Mutex::new("normal"),
            stops: AtomicUsize::new(0),
            creates: AtomicUsize::new(0),
            starts: AtomicUsize::new(0),
            launch_fixture,
        })
    }
    fn state(&self) -> JournalDirectory {
        JournalDirectory::open(self.fixture.state.clone()).unwrap()
    }
    async fn acknowledge(&self) -> Result<OwnedUpdateHelper, UpdateError> {
        OwnedUpdateHelper::acknowledge(
            self.state(),
            self.intent.clone(),
            HELPER,
            self,
            &CancellationToken::new(),
        )
        .await
    }
    fn helper(&self) -> Value {
        let intent: HelperLaunchIntent = self.state().read(&self.intent.file("launch")).unwrap();
        let reg = &intent.registration;
        json!({"Id": HELPER, "Name": format!("/{}", intent.helper_name()), "Image": format!("sha256:{}", "d".repeat(64)),
            "Config": {"Hostname": &HELPER[..12], "User": format!("{}:{}", reg.executor_uid, reg.executor_gid), "Healthcheck": {"Test": ["NONE"]}, "Image": intent.operation.plan.helper_image, "Labels": {verification::NONCE: intent.nonce.to_string(), verification::OPERATION: intent.operation.id.to_string()}},
            "State": {"Running": true, "Paused": false, "Restarting": false, "Status": "running", "StartedAt": "2026-11-03T00:00:00Z"}, "RestartCount": 0,
            "HostConfig": {"RestartPolicy": {"Name": "no"}, "NetworkMode": "none", "Privileged": false, "ReadonlyRootfs": true, "AutoRemove": false},
            "Path": "/app/citadel-server", "Args": ["run-update-helper", "--state-directory", reg.state_directory, "--operation-id", intent.operation.id.to_string(), "--fence", intent.operation.fence.to_string()],
            "Mounts": [{"Type":"bind", "Source":reg.directory, "Destination":reg.directory, "RW":true}, {"Type":"bind", "Source":reg.state_directory, "Destination":reg.state_directory, "RW":true}, {"Type":"bind", "Source":SOCKET, "Destination":SOCKET, "RW":true}]
        })
    }
}
impl ProcessRunner for Runner {
    fn run<'a>(
        &'a self,
        request: ProcessRequest,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ProcessOutput, ProcessError>> {
        Box::pin(async move {
            let mode = *self.mode.lock().unwrap();
            let args: Vec<_> = request.arguments[2..]
                .iter()
                .map(|s| s.to_str().unwrap())
                .collect();
            let value = match args.as_slice() {
                ["container", "create", rest @ ..] => {
                    assert!(self.state().existing_lock().is_err());
                    let saved: HelperLaunchIntent =
                        self.state().read(&self.intent.file("launch")).unwrap();
                    assert!(rest.contains(&saved.helper_name().as_str()));
                    for required in [
                        "--pull=never",
                        "--restart=no",
                        "--network=none",
                        "--read-only",
                        "run-update-helper",
                    ] {
                        assert!(rest.contains(&required));
                    }
                    assert_eq!(rest.iter().filter(|s| **s == "--mount").count(), 3);
                    assert!(!rest.contains(&"--rm"));
                    assert!(!rest.iter().any(|s| s.contains("private-test-password")));
                    assert_eq!(self.creates.fetch_add(1, Ordering::SeqCst), 0);
                    if mode == "create-timeout" {
                        return Err(ProcessError::Timeout(Duration::from_secs(15)));
                    }
                    if mode == "cancel-create" {
                        cancellation.cancel();
                        return Err(ProcessError::Cancelled);
                    }
                    return Ok(ProcessOutput {
                        exit_code: Some(0),
                        stdout: if mode == "bad-create-id" {
                            b"invalid".to_vec()
                        } else {
                            HELPER.as_bytes().to_vec()
                        },
                        stderr: vec![],
                        stdout_truncated: false,
                        stderr_truncated: false,
                    });
                }
                ["container", "start", HELPER] => {
                    // A newly running child can acquire ownership immediately.
                    drop(self.state().existing_lock().unwrap());
                    for kind in ["binding", "start-intent"] {
                        let binding: super::evidence::HelperBinding =
                            self.state().read(&self.intent.file(kind)).unwrap();
                        assert_eq!(binding.helper_container_id, HELPER);
                    }
                    assert_eq!(self.starts.fetch_add(1, Ordering::SeqCst), 0);
                    if mode == "start-timeout" {
                        return Err(ProcessError::Timeout(Duration::from_secs(15)));
                    }
                    if mode == "cancel-start" {
                        cancellation.cancel();
                        return Err(ProcessError::Cancelled);
                    }
                    json!(HELPER)
                }
                [
                    "exec",
                    "--interactive",
                    "--env",
                    _,
                    "--user",
                    _,
                    HELPER,
                    "/app/citadel-server",
                    "update-helper-handoff",
                ] => {
                    let output =
                        execute_helper_handoff(request.stdin.as_ref().unwrap(), self, cancellation)
                            .await;
                    if mode == "grant-timeout" && output.is_ok() {
                        return Err(ProcessError::Timeout(Duration::from_secs(45)));
                    }
                    return Ok(ProcessOutput {
                        exit_code: Some(if output.is_ok() { 0 } else { 1 }),
                        stdout: output.unwrap_or_default(),
                        stderr: vec![],
                        stdout_truncated: false,
                        stderr_truncated: false,
                    });
                }
                ["container", "inspect", HELPER] => {
                    let mut helper = self.helper();
                    if self.launch_fixture && self.starts.load(Ordering::SeqCst) == 0 {
                        helper["State"]["Running"] = json!(false);
                        helper["State"]["Status"] = json!("created");
                    }
                    match mode {
                        "wrong-user" => helper["Config"]["User"] = json!("unexpected"),
                        "auto-remove" => helper["HostConfig"]["AutoRemove"] = json!(true),
                        "wrong-mount" => helper["Mounts"][0]["RW"] = json!(false),
                        "wrong-helper" => {
                            helper["Config"]["Labels"][verification::NONCE] = json!("foreign")
                        }
                        "dead-helper" => helper["State"]["Running"] = json!(false),
                        "restarted-helper" => helper["RestartCount"] = json!(1),
                        "manual-restart" => {
                            helper["State"]["StartedAt"] = json!("2026-11-03T01:00:00Z")
                        }
                        _ => (),
                    }
                    json!([helper])
                }
                ["image", "inspect", rest @ ..] => {
                    let platform_specific = rest.contains(&"--platform");
                    let id = if mode == "platform-image-id" && platform_specific {
                        "f"
                    } else {
                        "d"
                    };
                    json!([{"Id":format!("sha256:{}", id.repeat(64)), "Os":"linux", "Architecture":"amd64", "RepoDigests":[self.intent.operation.plan.source.image], "Config": {"Volumes": if mode == "image-volume" { json!({"/unexpected": {}}) } else { Value::Null }}}])
                }
                [
                    "container",
                    "stop",
                    "--signal",
                    "SIGTERM",
                    "--timeout",
                    "-1",
                    CORE,
                ] => {
                    assert!(self.state().existing_lock().is_err());
                    assert!(
                        self.fixture
                            .state
                            .join(self.intent.file("shutdown-grant"))
                            .exists()
                    );
                    assert!(
                        self.fixture
                            .state
                            .join(self.intent.file("stop-intent"))
                            .exists()
                    );
                    assert_eq!(
                        self.stops.fetch_add(1, Ordering::SeqCst),
                        0,
                        "stop must not replay"
                    );
                    if mode == "stop-timeout" {
                        return Err(ProcessError::Timeout(Duration::from_secs(1)));
                    }
                    if mode == "cancel-stop" {
                        cancellation.cancel();
                        return Err(ProcessError::Cancelled);
                    }
                    json!(CORE)
                }
                ["container", "inspect", CORE] if self.stops.load(Ordering::SeqCst) > 0 => {
                    let mut core = self.fixture.runner.core.clone();
                    core["State"]["Running"] = json!(mode == "still-running");
                    core["State"]["Status"] = json!("exited");
                    core["State"]["ExitCode"] = json!(if mode == "bad-exit" { 1 } else { 0 });
                    core["State"]["OOMKilled"] = json!(false);
                    core["State"]["Error"] = json!("");
                    if mode == "restarted-core" {
                        core["RestartCount"] = json!(1);
                    }
                    json!([core])
                }
                ["container", "inspect", DATABASE]
                    if self.stops.load(Ordering::SeqCst) > 0 && mode == "lost-database" =>
                {
                    let mut database = self.fixture.runner.database.clone();
                    database["State"]["Running"] = json!(false);
                    json!([database])
                }
                _ => return self.fixture.runner.run(request, cancellation).await,
            };
            Ok(ProcessOutput {
                exit_code: Some(0),
                stdout: serde_json::to_vec(&value).unwrap(),
                stderr: vec![],
                stdout_truncated: false,
                stderr_truncated: false,
            })
        })
    }
}

#[tokio::test]
async fn helper_requires_a_durable_grant_and_retains_lock_through_clean_stop() {
    let runner = Runner::new().await;
    let runtime = DockerHelperHandoff::new(runner.intent.clone(), HELPER.into(), runner.clone());
    let token = CancellationToken::new();
    assert!(
        runtime
            .acknowledged(&runner.intent.operation, &token)
            .await
            .is_err()
    );
    let helper = runner.acknowledge().await.unwrap();
    assert!(
        helper
            .stop_source(runner.as_ref(), Duration::from_secs(2), &token)
            .await
            .is_err()
    );
    assert_eq!(runner.stops.load(Ordering::SeqCst), 0);
    runtime
        .acknowledged(&runner.intent.operation, &token)
        .await
        .unwrap();
    runtime
        .authorize_shutdown(&runner.intent.operation, &token)
        .await
        .unwrap();
    let journal = helper
        .stop_source(runner.as_ref(), Duration::from_secs(2), &token)
        .await
        .unwrap();
    assert_eq!(journal.operation.phase, UpdatePhase::BackingUp);
    assert_eq!(
        journal.history.iter().map(|o| o.phase).collect::<Vec<_>>(),
        [
            UpdatePhase::Preparing,
            UpdatePhase::Draining,
            UpdatePhase::BackingUp
        ]
    );
    assert!(runner.state().existing_lock().is_err());
    assert!(
        helper
            .stop_source(runner.as_ref(), Duration::from_secs(2), &token)
            .await
            .is_err()
    );
    assert_eq!(runner.stops.load(Ordering::SeqCst), 1);
    drop(helper);
    assert!(runner.state().existing_lock().is_ok());
    assert!(super::super::check_startup_registration(&runner.fixture.state).is_err());
}

#[tokio::test]
async fn stale_or_dead_helpers_and_changed_inputs_never_receive_shutdown_grants() {
    for mode in [
        "wrong-helper",
        "dead-helper",
        "restarted-helper",
        "manual-restart",
        "lost-lock",
        "changed-inputs",
        "changed-ack",
    ] {
        let runner = Runner::new().await;
        let helper = runner.acknowledge().await.unwrap();
        *runner.mode.lock().unwrap() = mode;
        if mode == "lost-lock" {
            drop(helper);
        }
        if mode == "changed-inputs" {
            std::fs::write(
                runner.fixture.inputs.directory.join(".env"),
                "PG_PASSWORD=changed\n",
            )
            .unwrap();
        }
        if mode == "changed-ack" {
            let mut ack: HelperAcknowledgement =
                runner.state().read(&runner.intent.file("ack")).unwrap();
            ack.binding.nonce = Uuid::now_v7();
            runner
                .state()
                .write(&runner.intent.file("ack"), &ack)
                .unwrap();
        }
        let runtime =
            DockerHelperHandoff::new(runner.intent.clone(), HELPER.into(), runner.clone());
        assert!(
            runtime
                .authorize_shutdown(&runner.intent.operation, &CancellationToken::new())
                .await
                .is_err(),
            "{mode}"
        );
        assert!(
            !runner
                .fixture
                .state
                .join(runner.intent.file("shutdown-grant"))
                .exists()
        );
        assert_eq!(runner.stops.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn ambiguous_or_failed_stops_never_advance_to_checkpoint_or_replay() {
    for mode in [
        "stop-timeout",
        "cancel-stop",
        "bad-exit",
        "restarted-core",
        "lost-database",
        "still-running",
    ] {
        let runner = Runner::new().await;
        let helper = runner.acknowledge().await.unwrap();
        let token = CancellationToken::new();
        DockerHelperHandoff::new(runner.intent.clone(), HELPER.into(), runner.clone())
            .authorize_shutdown(&runner.intent.operation, &token)
            .await
            .unwrap();
        *runner.mode.lock().unwrap() = mode;
        let error = helper
            .stop_source(runner.as_ref(), Duration::from_millis(300), &token)
            .await
            .unwrap_err();
        assert!(!error.to_string().contains("private-test-password"));
        assert_eq!(
            runner
                .state()
                .journal(runner.intent.operation.id)
                .unwrap()
                .operation
                .phase,
            UpdatePhase::Draining
        );
        assert!(
            runner
                .fixture
                .state
                .join(runner.intent.file("stop-intent"))
                .exists()
        );
        assert!(
            helper
                .stop_source(
                    runner.as_ref(),
                    Duration::from_secs(1),
                    &CancellationToken::new()
                )
                .await
                .is_err()
        );
        assert_eq!(runner.stops.load(Ordering::SeqCst), 1);
        assert!(runner.state().existing_lock().is_err());
    }
}

#[tokio::test]
async fn conflicting_launches_and_helpers_cannot_overwrite_durable_evidence() {
    let runner = Runner::new().await;
    assert!(prepare_helper_launch(&runner.state(), &runner.intent.operation).is_err());
    assert!(bind_helper_launch(&runner.state(), &runner.intent, &"f".repeat(64)).is_err());
    let helper = runner.acknowledge().await.unwrap();
    assert!(runner.acknowledge().await.is_err());
    drop(helper);
    // A dead helper's journal/ack cannot be replaced by a new incarnation.
    assert!(runner.acknowledge().await.is_err());
}

/// Simulate separate mount/user namespaces: Core receives host paths that do not
/// exist locally. Only the exec endpoint maps them to the helper's private files.
struct CoreNamespace {
    helper: Arc<Runner>,
    intent: HelperLaunchIntent,
}
impl ProcessRunner for CoreNamespace {
    fn run<'a>(
        &'a self,
        mut request: ProcessRequest,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ProcessOutput, ProcessError>> {
        Box::pin(async move {
            if request
                .arguments
                .iter()
                .any(|a| a == "update-helper-handoff")
            {
                let mut payload: super::command::HandoffRequest =
                    serde_json::from_slice(request.stdin.as_ref().unwrap()).unwrap();
                assert_eq!(payload.intent, self.intent);
                assert!(request.arguments.iter().any(|a| a
                    == &std::ffi::OsString::from(format!(
                        "{}:{}",
                        self.intent.registration.executor_uid,
                        self.intent.registration.executor_gid
                    ))));
                payload.intent = self.helper.intent.clone();
                request.stdin = Some(serde_json::to_vec(&payload).unwrap());
                return self.helper.run(request, cancellation).await;
            }
            let is_helper = request.arguments.iter().any(|a| a == HELPER);
            let mut output = self.helper.run(request, cancellation).await?;
            if is_helper {
                let mut value: Value = serde_json::from_slice(&output.stdout).unwrap();
                let reg = &self.intent.registration;
                value[0]["Args"][2] = json!(reg.state_directory);
                value[0]["Config"]["User"] =
                    json!(format!("{}:{}", reg.executor_uid, reg.executor_gid));
                for (index, path) in [(0, &reg.directory), (1, &reg.state_directory)] {
                    value[0]["Mounts"][index]["Source"] = json!(path);
                    value[0]["Mounts"][index]["Destination"] = json!(path);
                }
                output.stdout = serde_json::to_vec(&value).unwrap();
            }
            Ok(output)
        })
    }
}

#[tokio::test]
async fn core_uses_helper_exec_without_host_paths_or_executor_uid() {
    let helper = Runner::new().await;
    let _owner = helper.acknowledge().await.unwrap();
    let mut intent = helper.intent.clone();
    let root = std::path::PathBuf::from(format!("/citadel-unmounted-{}", Uuid::now_v7()));
    assert!(!root.exists());
    intent.registration.directory = root.join("installer");
    intent.registration.environment_file = intent.registration.directory.join(".env");
    intent.registration.compose_files =
        vec![intent.registration.directory.join("docker-compose.yml")];
    intent.registration.state_directory = root.join("state");
    intent.operation.plan.checkpoint_directory =
        intent.registration.state_directory.to_str().unwrap().into();
    intent.registration.executor_uid = rustix::process::geteuid().as_raw().wrapping_add(1);
    intent.registration.executor_gid = rustix::process::getegid().as_raw().wrapping_add(1);
    let transport = Arc::new(CoreNamespace {
        helper: helper.clone(),
        intent: intent.clone(),
    });
    let runtime = DockerHelperHandoff::new(intent.clone(), HELPER.into(), transport);
    let token = CancellationToken::new();
    runtime
        .acknowledged(&intent.operation, &token)
        .await
        .unwrap();
    assert!(
        !helper
            .fixture
            .state
            .join(helper.intent.file("shutdown-grant"))
            .exists()
    );
    runtime
        .authorize_shutdown(&intent.operation, &token)
        .await
        .unwrap();
    assert!(
        helper
            .fixture
            .state
            .join(helper.intent.file("shutdown-grant"))
            .exists()
    );
    assert!(!root.exists());
    assert_eq!(helper.stops.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn helper_endpoint_rejects_unbound_requests_before_grant() {
    use super::command::{HandoffAction, HandoffRequest};
    let runner = Runner::new().await;
    let _owner = runner.acknowledge().await.unwrap();
    let ack: HelperAcknowledgement = runner.state().read(&runner.intent.file("ack")).unwrap();
    let mut request = HandoffRequest {
        intent: runner.intent.clone(),
        binding: ack.binding,
        started_at: ack.started_at,
        action: HandoffAction::Grant,
    };
    request.intent.operation.fence = Uuid::now_v7();
    let token = CancellationToken::new();
    assert!(
        execute_helper_handoff(
            &serde_json::to_vec(&request).unwrap(),
            runner.as_ref(),
            &token
        )
        .await
        .is_err()
    );
    assert!(
        execute_helper_handoff(
            &vec![b' '; MAX_HANDOFF_REQUEST + 1],
            runner.as_ref(),
            &token
        )
        .await
        .is_err()
    );
    assert!(
        !runner
            .fixture
            .state
            .join(runner.intent.file("shutdown-grant"))
            .exists()
    );
}

#[tokio::test]
async fn interrupted_exec_retains_a_published_shutdown_grant() {
    let runner = Runner::new().await;
    let helper = runner.acknowledge().await.unwrap();
    let runtime = DockerHelperHandoff::new(runner.intent.clone(), HELPER.into(), runner.clone());
    *runner.mode.lock().unwrap() = "grant-timeout";
    let token = CancellationToken::new();
    assert!(
        runtime
            .authorize_shutdown(&runner.intent.operation, &token)
            .await
            .is_err()
    );
    assert!(
        runner
            .fixture
            .state
            .join(runner.intent.file("shutdown-grant"))
            .exists()
    );
    assert!(runner.state().existing_lock().is_err());
    *runner.mode.lock().unwrap() = "normal";
    // A transport failure is not permission for Core to retract the grant.
    helper
        .stop_source(runner.as_ref(), Duration::from_secs(2), &token)
        .await
        .unwrap();
    assert_eq!(runner.stops.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn helper_cannot_acknowledge_without_matching_start_intent() {
    let runner = Runner::new().await;
    // Simulate a crash before durable start intent; binding alone is insufficient.
    let path = runner
        .fixture
        .state
        .join(runner.intent.file("start-intent"));
    std::fs::remove_file(&path).unwrap();
    assert!(runner.acknowledge().await.is_err());
    assert!(
        !runner
            .fixture
            .state
            .join(runner.intent.file("ack"))
            .exists()
    );
    runner
        .state()
        .create(
            &runner.intent.file("start-intent"),
            &super::evidence::HelperBinding {
                nonce: runner.intent.nonce,
                helper_container_id: "f".repeat(64),
            },
        )
        .unwrap();
    assert!(runner.acknowledge().await.is_err());
    assert!(
        !runner
            .fixture
            .state
            .join(format!("{}.json", runner.intent.operation.id))
            .exists()
    );
}

#[tokio::test]
async fn launch_binds_before_start_and_leaves_shutdown_to_handoff() {
    let runner = Runner::fixture(true).await;
    *runner.mode.lock().unwrap() = "platform-image-id";
    let launched = launch_update_helper(
        &runner.state(),
        &runner.intent.operation,
        runner.as_ref(),
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(launched.container_id, HELPER);
    assert_eq!(runner.creates.load(Ordering::SeqCst), 1);
    assert_eq!(runner.starts.load(Ordering::SeqCst), 1);
    assert_eq!(runner.stops.load(Ordering::SeqCst), 0);
    assert!(
        !runner
            .fixture
            .state
            .join(launched.intent.file("ack"))
            .exists()
    );
    assert!(
        !runner
            .fixture
            .state
            .join(launched.intent.file("shutdown-grant"))
            .exists()
    );
    let helper = OwnedUpdateHelper::acknowledge(
        runner.state(),
        launched.intent.clone(),
        &launched.container_id,
        runner.as_ref(),
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    let runtime = DockerHelperHandoff::new(
        launched.intent.clone(),
        launched.container_id,
        runner.clone(),
    );
    runtime
        .acknowledged(&launched.intent.operation, &CancellationToken::new())
        .await
        .unwrap();
    assert!(
        helper
            .stop_source(
                runner.as_ref(),
                Duration::from_secs(1),
                &CancellationToken::new()
            )
            .await
            .is_err()
    );
    assert_eq!(runner.stops.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn interrupted_launches_retain_evidence_and_never_replay() {
    for mode in [
        "create-timeout",
        "cancel-create",
        "bad-create-id",
        "start-timeout",
        "cancel-start",
    ] {
        let runner = Runner::fixture(true).await;
        *runner.mode.lock().unwrap() = mode;
        assert!(
            launch_update_helper(
                &runner.state(),
                &runner.intent.operation,
                runner.as_ref(),
                &CancellationToken::new(),
            )
            .await
            .is_err(),
            "{mode}"
        );
        assert_eq!(runner.creates.load(Ordering::SeqCst), 1);
        assert_eq!(runner.stops.load(Ordering::SeqCst), 0);
        assert!(
            runner
                .fixture
                .state
                .join(runner.intent.file("launch"))
                .exists()
        );
        let started = matches!(mode, "start-timeout" | "cancel-start");
        assert_eq!(runner.starts.load(Ordering::SeqCst), usize::from(started));
        for kind in ["binding", "start-intent"] {
            assert_eq!(
                runner.fixture.state.join(runner.intent.file(kind)).exists(),
                started
            );
        }
        drop(runner.state().existing_lock().unwrap());
        *runner.mode.lock().unwrap() = "normal";
        assert!(
            launch_update_helper(
                &runner.state(),
                &runner.intent.operation,
                runner.as_ref(),
                &CancellationToken::new(),
            )
            .await
            .is_err()
        );
        assert_eq!(runner.creates.load(Ordering::SeqCst), 1);
        assert_eq!(runner.starts.load(Ordering::SeqCst), usize::from(started));
    }
}

#[tokio::test]
async fn invalid_created_helpers_never_start() {
    for mode in [
        "wrong-helper",
        "wrong-user",
        "auto-remove",
        "wrong-mount",
        "restarted-helper",
    ] {
        let runner = Runner::fixture(true).await;
        *runner.mode.lock().unwrap() = mode;
        assert!(
            launch_update_helper(
                &runner.state(),
                &runner.intent.operation,
                runner.as_ref(),
                &CancellationToken::new(),
            )
            .await
            .is_err(),
            "{mode}"
        );
        assert_eq!(runner.creates.load(Ordering::SeqCst), 1);
        assert_eq!(runner.starts.load(Ordering::SeqCst), 0);
        assert_eq!(runner.stops.load(Ordering::SeqCst), 0);
        assert!(
            !runner
                .fixture
                .state
                .join(runner.intent.file("binding"))
                .exists()
        );
    }
}

#[tokio::test]
async fn cancellation_or_changed_inputs_prevents_create() {
    let runner = Runner::fixture(true).await;
    let token = CancellationToken::new();
    token.cancel();
    assert!(matches!(
        launch_update_helper(
            &runner.state(),
            &runner.intent.operation,
            runner.as_ref(),
            &token,
        )
        .await,
        Err(UpdateError::Cancelled)
    ));
    assert!(
        !runner
            .fixture
            .state
            .join(runner.intent.file("launch"))
            .exists()
    );
    std::fs::write(
        runner.intent.registration.directory.join(".env"),
        "changed=true",
    )
    .unwrap();
    assert!(
        launch_update_helper(
            &runner.state(),
            &runner.intent.operation,
            runner.as_ref(),
            &CancellationToken::new(),
        )
        .await
        .is_err()
    );
    assert_eq!(runner.creates.load(Ordering::SeqCst), 0);
    assert_eq!(runner.starts.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn image_declared_volumes_are_rejected_before_create() {
    let runner = Runner::fixture(true).await;
    *runner.mode.lock().unwrap() = "image-volume";
    assert!(
        launch_update_helper(
            &runner.state(),
            &runner.intent.operation,
            runner.as_ref(),
            &CancellationToken::new(),
        )
        .await
        .is_err()
    );
    assert_eq!(runner.creates.load(Ordering::SeqCst), 0);
    assert_eq!(runner.starts.load(Ordering::SeqCst), 0);
}
