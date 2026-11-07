use super::super::{
    JournalDirectory,
    bootstrap::fixtures::{CORE, DATABASE, Fixture},
    docker::SOCKET,
};
use super::evidence::{HelperAcknowledgement, prepare_helper_launch};
use super::*;
use citadel_execution::{ProcessError, ProcessOutput, ProcessRequest, ProcessRunner};
use citadel_updates::*;
use futures_util::future::BoxFuture;
use serde_json::{Value, json};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
pub(super) const HELPER: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

pub(super) fn execution(state: &JournalDirectory, id: Uuid) -> Value {
    state.read(&format!("{id}.json")).unwrap()
}
pub(super) fn edit_execution(state: &JournalDirectory, id: Uuid, edit: impl FnOnce(&mut Value)) {
    let mut value = execution(state, id);
    edit(&mut value);
    state.write(&format!("{id}.json"), &value).unwrap();
}

pub(super) struct Runner {
    pub(super) fixture: Fixture,
    pub(super) intent: HelperLaunchIntent,
    mode: Mutex<&'static str>,
    stops: AtomicUsize,
    creates: AtomicUsize,
    starts: AtomicUsize,
    exchanges: AtomicUsize,
    launch_fixture: bool,
}
impl Runner {
    pub(super) async fn new() -> Arc<Self> {
        Self::fixture(false).await
    }
    async fn fixture(launch_fixture: bool) -> Arc<Self> {
        Self::with_fixture(launch_fixture, Fixture::new()).await
    }
    async fn with_fixture(launch_fixture: bool, mut fixture: Fixture) -> Arc<Self> {
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
                database: None,
                id: Uuid::now_v7(),
                instance_id: registration.instance_id,
                source: source.clone(),
                target: CoreIdentity {
                    version: "0.1.2".into(),
                    image: format!("ghcr.io/citadel-p/citadel@sha256:{}", "c".repeat(64)),
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
            state
                .create(
                    &intent.file("binding"),
                    &super::evidence::HelperBinding {
                        nonce: intent.nonce,
                        helper_container_id: HELPER.into(),
                    },
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
            exchanges: AtomicUsize::new(0),
            launch_fixture,
        })
    }
    pub(super) fn state(&self) -> JournalDirectory {
        JournalDirectory::open(self.fixture.state.clone()).unwrap()
    }
    pub(super) async fn acknowledge(&self) -> Result<OwnedUpdateHelper, UpdateError> {
        let helper = OwnedUpdateHelper::acknowledge(
            self.state(),
            self.intent.clone(),
            HELPER,
            self,
            &CancellationToken::new(),
        )
        .await?;
        // Model an independently completed trust/configuration/key preflight.
        // The dedicated gate test below deliberately omits this evidence.
        super::preparation::record_fixture(&self.state(), &self.intent);
        Ok(helper)
    }
    pub(super) fn helper(&self) -> Value {
        let intent: HelperLaunchIntent = self.state().read(&self.intent.file("launch")).unwrap();
        let reg = &intent.registration;
        json!({"Id": HELPER, "Name": format!("/{}", intent.helper_name()), "Image": format!("sha256:{}", "d".repeat(64)),
            "Config": {"Hostname": &DATABASE[..12], "Env": ["TMPDIR=/tmp"], "User": format!("{}:{}", reg.executor_uid, reg.executor_gid), "Healthcheck": {"Test": ["NONE"]}, "Image": intent.operation.plan.helper_image, "Labels": {verification::NONCE: intent.nonce.to_string(), verification::OPERATION: intent.operation.id.to_string()}},
            "State": {"Running": true, "Paused": false, "Restarting": false, "Status": "running", "StartedAt": "2026-11-03T00:00:00Z"}, "RestartCount": 0,
            "HostConfig": {"RestartPolicy": {"Name": "no"}, "NetworkMode": format!("container:{}", reg.database_container_id), "Privileged": false, "ReadonlyRootfs": true, "AutoRemove": false, "Tmpfs": {"/tmp":"rw,noexec,nosuid,size=64m"}},
            "Path": "/app/citadel-server", "Args": ["run-update-helper", "--state-directory", reg.state_directory, "--operation-id", intent.operation.id.to_string(), "--fence", intent.operation.fence.to_string()],
            "Mounts": [{"Type":"bind", "Source":reg.directory, "Destination":reg.directory, "RW":true}, {"Type":"bind", "Source":reg.state_directory, "Destination":reg.state_directory, "RW":true}, {"Type":"bind", "Source":reg.docker_socket, "Destination":SOCKET, "RW":true}, {"Type":"volume","Name":reg.core_volume,"Destination":"/app/data","RW":false}]
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
                ["exec", HELPER, "readlink", "/proc/self/ns/mnt"] => {
                    let namespace = if mode == "foreign-process" {
                        "mnt:[foreign]".into()
                    } else {
                        std::fs::read_link("/proc/self/ns/mnt")
                            .unwrap()
                            .to_string_lossy()
                            .into_owned()
                    };
                    return Ok(ProcessOutput {
                        exit_code: Some(0),
                        stdout: namespace.into_bytes(),
                        stderr: vec![],
                        stdout_truncated: false,
                        stderr_truncated: false,
                    });
                }

                ["container", "create", rest @ ..] => {
                    assert!(self.state().existing_lock().is_err());
                    let saved: HelperLaunchIntent =
                        self.state().read(&self.intent.file("launch")).unwrap();
                    assert!(rest.contains(&saved.helper_name().as_str()));
                    for required in [
                        "--pull=never",
                        "--restart=no",
                        "--read-only",
                        "run-update-helper",
                    ] {
                        assert!(rest.contains(&required));
                    }
                    assert!(rest.contains(&"TMPDIR=/tmp"));
                    assert!(rest.contains(&"--tmpfs=/tmp:rw,noexec,nosuid,size=64m"));
                    assert_eq!(rest.iter().filter(|s| **s == "--mount").count(), 4);
                    assert!(
                        rest.contains(
                            &format!(
                                "--network=container:{}",
                                saved.registration.database_container_id
                            )
                            .as_str()
                        )
                    );
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
                    let binding: super::evidence::HelperBinding =
                        self.state().read(&self.intent.file("binding")).unwrap();
                    assert_eq!(binding.helper_container_id, HELPER);
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
                    temporary,
                    "--user",
                    _,
                    HELPER,
                    "/app/citadel-server",
                    "update-helper-handoff",
                ] => {
                    assert_eq!(*temporary, "TMPDIR=/tmp");
                    self.exchanges.fetch_add(1, Ordering::SeqCst);
                    let output =
                        execute_helper_handoff(request.stdin.as_ref().unwrap(), self, cancellation)
                            .await;
                    if mode == "grant-timeout" && output.is_ok() {
                        return Err(ProcessError::Timeout(Duration::from_secs(45)));
                    }
                    if mode == "grant-pending" {
                        return Ok(ProcessOutput {
                            exit_code: Some(0),
                            stdout: serde_json::to_vec(&super::command::HelperResponse::Pending)
                                .unwrap(),
                            stderr: vec![],
                            stdout_truncated: false,
                            stderr_truncated: false,
                        });
                    }
                    return Ok(ProcessOutput {
                        exit_code: Some(if output.is_ok() { 0 } else { 1 }),
                        stdout: output.unwrap_or_default(),
                        stderr: vec![],
                        stdout_truncated: false,
                        stderr_truncated: false,
                    });
                }
                ["container", "ls", ..] if args[5].ends_with("citadel_data") => {
                    let mut ids = vec![CORE];
                    if !self.launch_fixture || self.creates.load(Ordering::SeqCst) > 0 {
                        ids.push(HELPER);
                    }
                    ids.sort();
                    return Ok(ProcessOutput {
                        exit_code: Some(0),
                        stdout: ids
                            .iter()
                            .map(|s| format!("\"{s}\"\n"))
                            .collect::<String>()
                            .into_bytes(),
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
                        helper["Config"]["Hostname"] = json!(&HELPER[..12]);
                    }
                    match mode {
                        "prepare-during-check" => {
                            super::preparation::record_fixture(&self.state(), &self.intent);
                        }
                        "temporary-state" => {
                            helper["Config"]["Env"] = json!([format!(
                                "TMPDIR={}",
                                self.intent.registration.state_directory.display()
                            )]);
                        }
                        "unbounded-temp" => helper["HostConfig"]["Tmpfs"] = json!({"/tmp":"rw"}),
                        "missing-tmpfs" => helper["HostConfig"]["Tmpfs"] = Value::Null,
                        "missing-temp" => helper["Config"]["Env"] = json!([]),
                        "wrong-user" => helper["Config"]["User"] = json!("unexpected"),
                        "auto-remove" => helper["HostConfig"]["AutoRemove"] = json!(true),
                        "wrong-mount" => helper["Mounts"][0]["RW"] = json!(false),
                        "writable-data" => helper["Mounts"][3]["RW"] = json!(true),
                        "wrong-network" => helper["HostConfig"]["NetworkMode"] = json!("host"),
                        "wrong-helper" => {
                            helper["Config"]["Labels"][verification::NONCE] = json!("foreign")
                        }
                        "exited-helper" => {
                            helper["State"]["Running"] = json!(false);
                            helper["State"]["Status"] = json!("exited");
                            helper["State"]["ExitCode"] = json!(1);
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
                    assert!(!execution(&self.state(), self.intent.operation.id)["stop"].is_null());
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
                ["network", "inspect", ..] if self.stops.load(Ordering::SeqCst) > 0 => {
                    let output = self.fixture.runner.run(request, cancellation).await?;
                    let mut value: Value = serde_json::from_slice(&output.stdout).unwrap();
                    value[0]["Containers"].as_object_mut().unwrap().remove(CORE);
                    value
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
    let waiting = CancellationToken::new();
    let (readiness, ()) = tokio::join!(
        runtime.acknowledged(&runner.intent.operation, &waiting),
        async {
            tokio::time::sleep(Duration::from_millis(25)).await;
            waiting.cancel();
        }
    );
    assert!(matches!(readiness, Err(UpdateError::Cancelled)));
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
    assert!(
        helper
            .prepare_checkpoint(runner.as_ref(), &token)
            .await
            .is_err()
    );
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    assert!(matches!(
        helper
            .check_checkpoint_access(runner.as_ref(), &cancelled)
            .await,
        Err(UpdateError::Cancelled)
    ));
    let journal = helper
        .stop_source(runner.as_ref(), Duration::from_secs(2), &token)
        .await
        .unwrap();
    assert_eq!(journal.operation.phase, UpdatePhase::BackingUp);
    helper
        .check_stopped_runtime(runner.as_ref(), &token)
        .await
        .unwrap();
    for mode in [
        "still-running",
        "restarted-core",
        "bad-exit",
        "lost-database",
        "writable-data",
        "wrong-network",
    ] {
        *runner.mode.lock().unwrap() = mode;
        assert!(
            helper
                .prepare_checkpoint(runner.as_ref(), &token)
                .await
                .is_err(),
            "{mode}"
        );
        assert!(
            !runner
                .fixture
                .state
                .join(format!("checkpoint-{}", runner.intent.operation.id))
                .exists()
        );
    }
    *runner.mode.lock().unwrap() = "normal";
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
            edit_execution(&runner.state(), runner.intent.operation.id, |v| {
                v["acknowledgement"]["binding"]["nonce"] = json!(Uuid::now_v7());
            });
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
        assert!(!execution(&runner.state(), runner.intent.operation.id)["stop"].is_null());
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
    assert!(
        launch_update_helper(
            &runner.state(),
            &runner.intent.operation,
            runner.as_ref(),
            &CancellationToken::new()
        )
        .await
        .is_err()
    );
    assert_eq!(runner.creates.load(Ordering::SeqCst), 0);
    assert_eq!(runner.starts.load(Ordering::SeqCst), 0);
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
                let mut payload: super::command::HelperRequest =
                    serde_json::from_slice(request.stdin.as_ref().unwrap()).unwrap();
                assert_eq!(payload.operation, self.intent.operation);
                assert!(request.arguments.iter().any(|a| a
                    == &std::ffi::OsString::from(format!(
                        "{}:{}",
                        self.intent.registration.executor_uid,
                        self.intent.registration.executor_gid
                    ))));
                payload.operation = self.helper.intent.operation.clone();
                let temporary = request
                    .arguments
                    .iter()
                    .find(|argument| argument.to_string_lossy().starts_with("TMPDIR="))
                    .unwrap();
                assert_eq!(*temporary, std::ffi::OsString::from("TMPDIR=/tmp"));
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
    use super::command::{HelperAction, HelperRequest};
    let runner = Runner::new().await;
    let _owner = runner.acknowledge().await.unwrap();
    let ack: HelperAcknowledgement = serde_json::from_value(
        execution(&runner.state(), runner.intent.operation.id)["acknowledgement"].clone(),
    )
    .unwrap();
    let mut request = HelperRequest {
        operation: runner.intent.operation.clone(),
        binding: ack.binding,
        started_at: ack.started_at,
        action: HelperAction::Grant,
    };
    request.operation.fence = Uuid::now_v7();
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
    for mode in ["grant-timeout", "grant-pending"] {
        let runner = Runner::new().await;
        let helper = runner.acknowledge().await.unwrap();
        let runtime =
            DockerHelperHandoff::new(runner.intent.clone(), HELPER.into(), runner.clone());
        *runner.mode.lock().unwrap() = mode;
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
        assert_eq!(runner.exchanges.load(Ordering::SeqCst), 1);
        assert!(runner.state().existing_lock().is_err());
        *runner.mode.lock().unwrap() = "normal";
        // A transport failure is not permission for Core to retract the grant.
        helper
            .stop_source(runner.as_ref(), Duration::from_secs(2), &token)
            .await
            .unwrap();
        assert_eq!(runner.stops.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn helper_cannot_acknowledge_without_matching_start_binding() {
    let runner = Runner::new().await;
    // Simulate a crash before the durable binding authorizes starting this helper.
    let path = runner.fixture.state.join(runner.intent.file("binding"));
    std::fs::remove_file(&path).unwrap();
    assert!(runner.acknowledge().await.is_err());
    assert!(
        !runner
            .fixture
            .state
            .join(format!("{}.json", runner.intent.operation.id))
            .exists()
    );
    runner
        .state()
        .create(
            &runner.intent.file("binding"),
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
            .join(format!("{}.json", launched.intent.operation.id))
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
    super::preparation::record_fixture(&runner.state(), &launched.intent);
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
        assert_eq!(
            runner
                .fixture
                .state
                .join(runner.intent.file("binding"))
                .exists(),
            started
        );
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
        "temporary-state",
        "unbounded-temp",
        "missing-tmpfs",
        "missing-temp",
        "writable-data",
        "wrong-network",
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

#[tokio::test]
async fn acknowledgement_without_target_preflight_never_authorizes_shutdown() {
    let runner = Runner::new().await;
    let token = CancellationToken::new();
    let _helper = OwnedUpdateHelper::acknowledge(
        runner.state(),
        runner.intent.clone(),
        HELPER,
        runner.as_ref(),
        &token,
    )
    .await
    .unwrap();
    let runtime = DockerHelperHandoff::new(runner.intent.clone(), HELPER.into(), runner.clone());
    assert!(
        runtime
            .authorize_shutdown(&runner.intent.operation, &token)
            .await
            .is_err()
    );
    assert!(
        !runner
            .state()
            .path()
            .join(runner.intent.file("shutdown-grant"))
            .exists()
    );
    super::preparation::record_fixture(&runner.state(), &runner.intent);
    let saved = execution(&runner.state(), runner.intent.operation.id)["prepared"].clone();
    for (key, value) in [
        ("format", json!(99)),
        (
            "checkedAt",
            json!(chrono::Utc::now() - chrono::Duration::minutes(16)),
        ),
        ("agentVersions", json!(["0.1.0"])),
        ("descriptor", json!({})),
    ] {
        let mut invalid = saved.clone();
        invalid[key] = value;
        edit_execution(&runner.state(), runner.intent.operation.id, |v| {
            v["prepared"] = invalid
        });
        assert!(
            runtime
                .authorize_shutdown(&runner.intent.operation, &token)
                .await
                .is_err(),
            "{key}"
        );
        assert!(
            !runner
                .state()
                .path()
                .join(runner.intent.file("shutdown-grant"))
                .exists()
        );
    }
    edit_execution(&runner.state(), runner.intent.operation.id, |v| {
        v["prepared"] = saved
    });
    runtime
        .authorize_shutdown(&runner.intent.operation, &token)
        .await
        .unwrap();
    assert_eq!(runner.stops.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn readiness_waits_for_preparation_without_granting_or_restarting() {
    use super::command::{HelperAction, HelperRequest, HelperResponse};
    let runner = Runner::new().await;
    let token = CancellationToken::new();
    let helper = OwnedUpdateHelper::acknowledge(
        runner.state(),
        runner.intent.clone(),
        HELPER,
        runner.as_ref(),
        &token,
    )
    .await
    .unwrap();
    let request = HelperRequest {
        operation: runner.intent.operation.clone(),
        binding: helper.acknowledgement.binding.clone(),
        started_at: helper.acknowledgement.started_at.clone(),
        action: HelperAction::Check,
    };
    let response = execute_helper_handoff(
        &serde_json::to_vec(&request).unwrap(),
        runner.as_ref(),
        &token,
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_slice::<HelperResponse>(&response).unwrap(),
        HelperResponse::Pending
    );
    let runtime = DockerHelperHandoff::new(runner.intent.clone(), HELPER.into(), runner.clone());
    let (ready, ()) = tokio::join!(
        runtime.acknowledged(&runner.intent.operation, &token),
        async {
            tokio::time::sleep(Duration::from_millis(25)).await;
            super::preparation::record_fixture(&runner.state(), &runner.intent);
        }
    );
    ready.unwrap();
    assert!(
        !runner
            .fixture
            .state
            .join(runner.intent.file("shutdown-grant"))
            .exists()
    );
    assert_eq!(runner.stops.load(Ordering::SeqCst), 0);
    assert_eq!(runner.starts.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn cancellation_ends_preparation_wait_without_a_shutdown_grant() {
    let runner = Runner::new().await;
    let token = CancellationToken::new();
    let _helper = OwnedUpdateHelper::acknowledge(
        runner.state(),
        runner.intent.clone(),
        HELPER,
        runner.as_ref(),
        &token,
    )
    .await
    .unwrap();
    let before = execution(&runner.state(), runner.intent.operation.id);
    let runtime = DockerHelperHandoff::new(runner.intent.clone(), HELPER.into(), runner.clone());
    let (result, ()) = tokio::join!(
        runtime.acknowledged(&runner.intent.operation, &token),
        async {
            tokio::time::sleep(Duration::from_millis(25)).await;
            token.cancel();
        }
    );
    assert!(matches!(result, Err(UpdateError::Cancelled)));
    assert_eq!(
        execution(&runner.state(), runner.intent.operation.id),
        before
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
async fn phase_projection_cannot_discard_execution_and_failure_retains_preparation() {
    let runner = Runner::new().await;
    let helper = runner.acknowledge().await.unwrap();
    let before = ExecutionJournal::read(&helper.state, helper.intent.operation.id).unwrap();
    assert!(helper.state.save(&before.journal).is_err());
    assert_eq!(
        ExecutionJournal::read(&helper.state, helper.intent.operation.id).unwrap(),
        before
    );
    // A failure belongs to the helper that accepted this execution, even when
    // the replacement acknowledgement is otherwise structurally valid.
    let mut changed = before.clone();
    changed.acknowledgement.started_at = "2026-11-03T00:03:00Z".into();
    changed.save(&helper.state, &helper.owner).unwrap();
    assert!(super::driver::record_failure(&helper).is_err());
    assert_eq!(
        ExecutionJournal::read(&helper.state, helper.intent.operation.id).unwrap(),
        changed
    );
    before.save(&helper.state, &helper.owner).unwrap();
    super::driver::record_failure(&helper).unwrap();
    let after = ExecutionJournal::read(&helper.state, helper.intent.operation.id).unwrap();
    assert_eq!(after.journal.operation.phase, UpdatePhase::Failed);
    assert_eq!(after.acknowledgement, before.acknowledgement);
    assert_eq!(after.prepared, before.prepared);
    assert!(after.stop.is_none());
    let mut entries: Vec<_> = std::fs::read_dir(helper.state.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    entries.sort();
    let mut expected = vec![
        "installation.json".into(),
        "installation.lock".into(),
        helper.intent.file("launch"),
        helper.intent.file("binding"),
        format!("{}.json", helper.intent.operation.id),
    ];
    expected.sort();
    assert_eq!(entries, expected);
}

#[tokio::test]
async fn readiness_accepts_preparation_finished_during_helper_verification() {
    use super::command::{HelperAction, HelperRequest, HelperResponse};
    let runner = Runner::new().await;
    let token = CancellationToken::new();
    let helper = OwnedUpdateHelper::acknowledge(
        runner.state(),
        runner.intent.clone(),
        HELPER,
        runner.as_ref(),
        &token,
    )
    .await
    .unwrap();
    *runner.mode.lock().unwrap() = "prepare-during-check";
    let request = HelperRequest {
        operation: runner.intent.operation.clone(),
        binding: helper.acknowledgement.binding.clone(),
        started_at: helper.acknowledgement.started_at.clone(),
        action: HelperAction::Check,
    };
    let response = execute_helper_handoff(
        &serde_json::to_vec(&request).unwrap(),
        runner.as_ref(),
        &token,
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_slice::<HelperResponse>(&response).unwrap(),
        HelperResponse::Acknowledged(helper.acknowledgement.clone())
    );
    assert!(
        runner
            .state()
            .read_optional::<Value>(&runner.intent.file("shutdown-grant"))
            .unwrap()
            .is_none()
    );
    assert_eq!(runner.stops.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn failed_journal_requires_terminal_owned_execution_and_exact_exited_helper() {
    let runner = Runner::new().await;
    let helper = runner.acknowledge().await.unwrap();
    let cancellation = CancellationToken::new();
    let read =
        || super::state::failed_journal(&runner.intent.operation, runner.as_ref(), &cancellation);
    assert!(read().await.is_err(), "live driver owns the lock");
    super::driver::record_failure(&helper).unwrap();
    drop(helper);
    for mode in [
        "normal",
        "wrong-helper",
        "manual-restart",
        "restarted-helper",
    ] {
        *runner.mode.lock().unwrap() = mode;
        assert!(read().await.is_err(), "{mode}");
    }
    *runner.mode.lock().unwrap() = "exited-helper";
    let journal = read().await.unwrap();
    assert_eq!(journal.operation.phase, UpdatePhase::Failed);
    assert!(!journal.maintenance_released);
    assert!(!journal.completion_acknowledged);
    assert_eq!(runner.stops.load(Ordering::SeqCst), 0);
    let mut foreign = runner.intent.operation.clone();
    foreign.fence = Uuid::now_v7();
    assert!(
        super::state::failed_journal(&foreign, runner.as_ref(), &cancellation)
            .await
            .is_err()
    );
    cancellation.cancel();
    assert!(read().await.is_err());
}

#[tokio::test]
async fn network_donor_hostname_is_not_helper_process_identity() {
    let runner = Runner::new().await;
    let token = CancellationToken::new();
    let home = runtime::private_home().unwrap();
    let docker = super::super::docker::LocalDocker {
        process: runner.as_ref(),
        cancellation: &token,
        home: home.path(),
        deadline: std::time::Instant::now() + Duration::from_secs(30),
    };
    let binding = evidence::binding(&runner.state(), &runner.intent).unwrap();
    assert_eq!(runner.helper()["Config"]["Hostname"], &DATABASE[..12]);
    verification::helper_process(&docker, &runner.intent, &binding)
        .await
        .unwrap();
    *runner.mode.lock().unwrap() = "foreign-process";
    assert!(
        verification::helper_process(&docker, &runner.intent, &binding)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn readiness_waits_for_initial_journal_but_never_grants_without_it() {
    use super::command::{HelperAction, HelperRequest, HelperResponse};
    let runner = Runner::new().await;
    let token = CancellationToken::new();
    let mut request = HelperRequest {
        operation: runner.intent.operation.clone(),
        binding: evidence::binding(&runner.state(), &runner.intent).unwrap(),
        started_at: "2026-11-03T00:00:00Z".into(),
        action: HelperAction::Check,
    };
    let response = execute_helper_handoff(
        &serde_json::to_vec(&request).unwrap(),
        runner.as_ref(),
        &token,
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_slice::<HelperResponse>(&response).unwrap(),
        HelperResponse::Pending
    );
    assert!(
        runner
            .state()
            .read_optional::<Value>(&format!("{}.json", request.operation.id))
            .unwrap()
            .is_none()
    );
    request.action = HelperAction::Grant;
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
        !runner
            .fixture
            .state
            .join(runner.intent.file("shutdown-grant"))
            .exists()
    );
    request.action = HelperAction::Check;
    for mode in [
        "wrong-helper",
        "dead-helper",
        "restarted-helper",
        "manual-restart",
    ] {
        *runner.mode.lock().unwrap() = mode;
        assert!(
            execute_helper_handoff(
                &serde_json::to_vec(&request).unwrap(),
                runner.as_ref(),
                &token
            )
            .await
            .is_err()
        );
    }
    *runner.mode.lock().unwrap() = "normal";
    let runtime = DockerHelperHandoff::new(runner.intent.clone(), HELPER.into(), runner.clone());
    let (ready, helper) = tokio::join!(
        runtime.acknowledged(&runner.intent.operation, &token),
        async {
            tokio::time::sleep(Duration::from_millis(25)).await;
            runner.acknowledge().await.unwrap()
        }
    );
    ready.unwrap();
    assert!(
        !helper
            .state
            .path()
            .join(runner.intent.file("shutdown-grant"))
            .exists()
    );
}

struct SourceRecoveryRunner {
    base: Arc<Runner>,
    removed: AtomicBool,
    fail_after_removal: bool,
    present: bool,
}
impl ProcessRunner for SourceRecoveryRunner {
    fn run<'a>(
        &'a self,
        request: ProcessRequest,
        token: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ProcessOutput, ProcessError>> {
        Box::pin(async move {
            let args: Vec<_> = request.arguments[2..]
                .iter()
                .map(|s| s.to_str().unwrap())
                .collect();
            let alive = self.present && !self.removed.load(Ordering::SeqCst);
            let bytes = if args == ["container", "rm", HELPER] {
                assert!(alive);
                self.removed.store(true, Ordering::SeqCst);
                if self.fail_after_removal {
                    return Err(ProcessError::Timeout(Duration::from_secs(1)));
                }
                Some(vec![])
            } else if args.first() == Some(&"container")
                && args.get(1) == Some(&"ls")
                && args.iter().any(|arg| {
                    arg.contains("com.citadel.update-operation=") || *arg == format!("id={HELPER}")
                })
            {
                Some(if alive {
                    format!("\"{HELPER}\"\n").into_bytes()
                } else {
                    vec![]
                })
            } else if args.first() == Some(&"container")
                && args.get(1) == Some(&"ls")
                && args[5].ends_with("citadel_data")
                && !alive
            {
                Some(format!("\"{CORE}\"\n").into_bytes())
            } else {
                None
            };
            if let Some(stdout) = bytes {
                return Ok(ProcessOutput {
                    exit_code: Some(0),
                    stdout,
                    stderr: vec![],
                    stdout_truncated: false,
                    stderr_truncated: false,
                });
            }
            self.base.run(request, token).await
        })
    }
}

#[tokio::test]
async fn source_recovery_fences_missing_and_exited_helpers_and_retains_evidence() {
    for before_launch in [true, false] {
        let base = if before_launch {
            Runner::fixture(true).await
        } else {
            Runner::new().await
        };
        let operation = base.intent.operation.clone();
        if !before_launch {
            let helper = base.acknowledge().await.unwrap();
            // A delayed grant is safe only once its exact executor is fenced and
            // the journal proves no stop intent was ever published.
            helper
                .state
                .create(
                    &helper.intent.file("shutdown-grant"),
                    &helper.acknowledgement,
                )
                .unwrap();
            super::driver::record_failure(&helper).unwrap();
            drop(helper);
        }
        *base.mode.lock().unwrap() = "exited-helper";
        let runner = SourceRecoveryRunner {
            base: base.clone(),
            removed: AtomicBool::new(false),
            fail_after_removal: false,
            present: !before_launch,
        };
        super::recover_source(&operation, &runner, &CancellationToken::new())
            .await
            .unwrap();
        super::recover_source(&operation, &runner, &CancellationToken::new())
            .await
            .unwrap();
        let state = base.state();
        state.require_registration_only().unwrap();
        assert!(prepare_helper_launch(&state, &operation).is_err());
        assert!(base.intent.verify_saved_identity(&state).is_err());
        assert_eq!(base.stops.load(Ordering::SeqCst), 0);
        let archive =
            JournalDirectory::open(state.path().join(format!("checkpoint-{}", operation.id)))
                .unwrap();
        assert_eq!(
            archive
                .read::<UpdateOperation>("source-recovered.json")
                .unwrap(),
            operation
        );
        if !before_launch {
            assert_eq!(
                archive.journal(operation.id).unwrap().operation.phase,
                UpdatePhase::Failed
            );
        }
    }
}

#[tokio::test]
async fn ambiguous_creation_and_start_are_reconciled_without_a_fabricated_journal() {
    for mode in ["create-timeout", "start-timeout"] {
        let base = Runner::fixture(true).await;
        let operation = base.intent.operation.clone();
        *base.mode.lock().unwrap() = mode;
        assert!(
            launch_update_helper(
                &base.state(),
                &operation,
                base.as_ref(),
                &CancellationToken::new()
            )
            .await
            .is_err()
        );
        *base.mode.lock().unwrap() = if mode == "create-timeout" {
            "normal"
        } else {
            "exited-helper"
        };
        let runner = SourceRecoveryRunner {
            base: base.clone(),
            removed: AtomicBool::new(false),
            fail_after_removal: false,
            present: true,
        };
        super::recover_source(&operation, &runner, &CancellationToken::new())
            .await
            .unwrap_or_else(|error| panic!("{mode}: {error}"));
        assert!(
            base.state()
                .read_optional::<Value>(&format!("{}.json", operation.id))
                .unwrap()
                .is_none()
        );
        assert!(prepare_helper_launch(&base.state(), &operation).is_err());
    }
}

#[tokio::test]
async fn source_recovery_refuses_an_ambiguous_stop_even_without_replacement() {
    let base = Runner::new().await;
    let helper = base.acknowledge().await.unwrap();
    helper
        .state
        .create(
            &helper.intent.file("shutdown-grant"),
            &helper.acknowledgement,
        )
        .unwrap();
    *base.mode.lock().unwrap() = "stop-timeout";
    assert!(
        helper
            .stop_source(
                base.as_ref(),
                Duration::from_secs(1),
                &CancellationToken::new()
            )
            .await
            .is_err()
    );
    drop(helper);
    *base.mode.lock().unwrap() = "exited-helper";
    let runner = SourceRecoveryRunner {
        base: base.clone(),
        removed: AtomicBool::new(false),
        fail_after_removal: false,
        present: true,
    };
    assert!(
        super::recover_source(&base.intent.operation, &runner, &CancellationToken::new())
            .await
            .is_err()
    );
    assert!(!runner.removed.load(Ordering::SeqCst));
    let archive = JournalDirectory::open(
        base.fixture
            .state
            .join(format!("checkpoint-{}", base.intent.operation.id)),
    )
    .unwrap();
    assert!(
        archive
            .read_optional::<Value>("source-recovery.json")
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn source_recovery_resumes_after_helper_removal_and_partial_archival() {
    for archived_files in 0..=3 {
        let base = Runner::new().await;
        let operation = base.intent.operation.clone();
        let helper = base.acknowledge().await.unwrap();
        super::driver::record_failure(&helper).unwrap();
        drop(helper);
        *base.mode.lock().unwrap() = "exited-helper";
        let runner = SourceRecoveryRunner {
            base: base.clone(),
            removed: AtomicBool::new(false),
            fail_after_removal: true,
            present: true,
        };
        let token = CancellationToken::new();
        assert!(
            super::recover_source(&operation, &runner, &token)
                .await
                .is_err()
        );
        assert!(runner.removed.load(Ordering::SeqCst));
        let state = base.state();
        let archive =
            JournalDirectory::open(state.path().join(format!("checkpoint-{}", operation.id)))
                .unwrap();
        assert!(
            archive
                .read_optional::<Value>("source-recovered.json")
                .unwrap()
                .is_none()
        );
        assert!(base.intent.verify_saved_identity(&state).is_err());
        // Reproduce the durable prefixes left by a crash during file archival.
        let owner = state.existing_lock().unwrap();
        for name in [
            base.intent.file("launch"),
            base.intent.file("binding"),
            format!("{}.json", operation.id),
        ]
        .into_iter()
        .take(archived_files)
        {
            state.archive(&owner, &archive, &name).unwrap();
        }
        drop(owner);
        super::recover_source(&operation, &runner, &token)
            .await
            .unwrap();
        super::recover_source(&operation, &runner, &token)
            .await
            .unwrap();
        state.require_registration_only().unwrap();
        assert_eq!(
            archive.journal(operation.id).unwrap().operation.phase,
            UpdatePhase::Failed
        );
        assert_eq!(base.stops.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn rootless_helper_launch_retains_socket_and_fences_shutdown() {
    let runner = Runner::with_fixture(true, Fixture::rootless()).await;
    let launched = launch_update_helper(
        &runner.state(),
        &runner.intent.operation,
        runner.as_ref(),
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(
        launched.intent.registration.docker_socket,
        std::path::Path::new("/run/user/1004/docker.sock")
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
    let args = super::launcher::create_arguments(&launched.intent).unwrap();
    assert!(args.iter().any(|arg| arg
        == "type=bind,\"source=/run/user/1004/docker.sock\",\"destination=/var/run/docker.sock\""));
}
