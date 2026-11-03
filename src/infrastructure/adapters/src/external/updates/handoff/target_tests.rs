use super::super::bootstrap::fixtures::{CORE, DATABASE};
use super::tests::{edit_execution, execution};
use super::{
    OwnedUpdateHelper,
    replacement_tests::{ReplacementRunner, ready},
    tests::HELPER,
};
use citadel_execution::{ProcessError, ProcessOutput, ProcessRequest, ProcessRunner};
use citadel_updates::{UpdateJournal, UpdatePhase};
use futures_util::future::BoxFuture;
use serde_json::{Value, json};
use std::{
    fs,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;

const TARGET: &str = "9999999999999999999999999999999999999999999999999999999999999999";
struct TargetRunner {
    previous: ReplacementRunner,
    mode: Mutex<&'static str>,
    scans: AtomicUsize,
    calls: AtomicUsize,
    removed: AtomicBool,
}
impl TargetRunner {
    async fn new() -> (Self, OwnedUpdateHelper) {
        let (runner, helper, _) = ready("normal").await;
        helper
            .replace_core(&runner, &CancellationToken::new())
            .await
            .unwrap();
        (
            Self {
                previous: runner,
                mode: Mutex::new("normal"),
                scans: AtomicUsize::new(0),
                calls: AtomicUsize::new(0),
                removed: AtomicBool::new(false),
            },
            helper,
        )
    }
    fn core(&self, mode: &str) -> Value {
        let mut core = self.previous.base.fixture.runner.core.clone();
        core["Id"] = json!(TARGET);
        core["Config"]["Hostname"] = json!(&TARGET[..12]);
        core["Image"] = json!(format!("sha256:{}", "c".repeat(64)));
        core["Config"]["Image"] = json!(self.previous.base.intent.operation.plan.target.image);
        core["State"]["Running"] = json!(true);
        core["State"]["Status"] = json!("running");
        core["State"]["StartedAt"] = json!("2026-11-03T00:02:00Z");
        core["State"]["OOMKilled"] = json!(false);
        core["State"]["Error"] = json!("");
        core["State"]["Health"]["Status"] = json!("unhealthy");
        let network = &self.previous.base.intent.registration.network;
        match mode {
            "healthy" => core["State"]["Health"]["Status"] = json!("healthy"),
            "starting" => core["State"]["Health"]["Status"] = json!("starting"),
            "stopped" => core["State"]["Running"] = json!(false),
            "wrong-image" => core["Image"] = json!(format!("sha256:{}", "f".repeat(64))),
            "wrong-env" => core["Config"]["Env"] = json!(["PG_USER=wrong"]),
            "bad-mount" => core["Mounts"][0]["Name"] = json!("other-data"),
            "restarted" => core["RestartCount"] = json!(1),
            "manual-restart" => core["State"]["StartedAt"] = json!("2026-11-03T00:03:00Z"),
            "restart-during-read" if self.scans.load(Ordering::SeqCst) >= 2 => {
                core["State"]["StartedAt"] = json!("2026-11-03T00:03:00Z")
            }
            "missing-start" => core["State"]["StartedAt"] = Value::Null,
            "network-recreated" => {
                core["NetworkSettings"]["Networks"][network]["NetworkID"] = json!("8".repeat(64))
            }
            _ => (),
        }
        core
    }
}
impl ProcessRunner for TargetRunner {
    fn run<'a>(
        &'a self,
        request: ProcessRequest,
        token: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ProcessOutput, ProcessError>> {
        Box::pin(async move {
            let mode = *self.mode.lock().unwrap();
            self.calls.fetch_add(1, Ordering::SeqCst);
            let args: Vec<_> = request.arguments[2..]
                .iter()
                .map(|s| s.to_str().unwrap())
                .collect();
            if args.last() == Some(&"update-helper-handoff") {
                assert_eq!(args[0], "exec");
                assert_eq!(args[6], HELPER);
                let result =
                    super::execute_helper_handoff(request.stdin.as_ref().unwrap(), self, token)
                        .await;
                return Ok(match result {
                    Ok(value) => output(value),
                    Err(_) => ProcessOutput {
                        exit_code: Some(1),
                        ..output(vec![])
                    },
                });
            }
            if args
                == [
                    "exec",
                    "--interactive",
                    "--user",
                    "65532:0",
                    TARGET,
                    "/app/citadel-server",
                    "update-target-receipt",
                ]
            {
                let completion: super::completion::TargetUpdateCompletion =
                    serde_json::from_slice(request.stdin.as_ref().unwrap()).unwrap();
                if mode == "uncommitted" {
                    return Ok(ProcessOutput {
                        exit_code: Some(1),
                        ..output(vec![])
                    });
                }
                let mut receipt = completion.receipt;
                if mode == "foreign-completion" {
                    receipt.fence = uuid::Uuid::now_v7();
                }
                if mode == "preparation-changed-during-receipt" {
                    let state = self.previous.base.state();
                    edit_execution(&state, self.previous.base.intent.operation.id, |saved| {
                        let checked: chrono::DateTime<chrono::Utc> =
                            serde_json::from_value(saved["prepared"]["checkedAt"].clone()).unwrap();
                        saved["prepared"]["checkedAt"] =
                            json!(checked - chrono::Duration::milliseconds(1));
                    });
                }
                return Ok(output(serde_json::to_vec(&receipt).unwrap()));
            }
            if args == ["container", "rm", HELPER] {
                assert_eq!(mode, "exit-clean");
                assert!(!self.removed.swap(true, Ordering::SeqCst));
                return Ok(output(HELPER.as_bytes().to_vec()));
            }
            assert!(matches!(
                args.as_slice(),
                ["compose", ..]
                    | ["image", "inspect", ..]
                    | ["container", "inspect", ..]
                    | ["container", "ls", ..]
                    | ["volume", "inspect", ..]
                    | ["network", "inspect", ..]
                    | ["info", ..]
            ));
            if args[0] == "compose" {
                assert!(args.ends_with(&["config", "--format", "json"]));
            }
            let reg = &self.previous.base.intent.registration;
            let value = match args.as_slice() {
                ["container", "ls", ..] => {
                    let filter = args[5];
                    let ids = if filter.starts_with("label=com.citadel.update-operation=")
                        || filter == format!("id={HELPER}")
                    {
                        if self.removed.load(Ordering::SeqCst) {
                            vec![]
                        } else if mode == "duplicate-helper" {
                            vec![HELPER, CORE]
                        } else {
                            vec![HELPER]
                        }
                    } else if filter.starts_with("label=") {
                        let mut ids = vec![TARGET, DATABASE];
                        if mode == "duplicate-core" {
                            ids.push(CORE);
                        }
                        ids
                    } else if filter.ends_with("citadel_data") {
                        if self.removed.load(Ordering::SeqCst) {
                            vec![TARGET]
                        } else {
                            vec![TARGET, HELPER]
                        }
                    } else if filter == format!("id={CORE}") {
                        if mode == "old-source" {
                            vec![CORE]
                        } else {
                            vec![]
                        }
                    } else if filter.ends_with("postgres_data") {
                        vec![DATABASE]
                    } else {
                        panic!("unexpected lookup {filter}");
                    };
                    return Ok(output(
                        ids.iter()
                            .map(|id| format!("\"{id}\"\n"))
                            .collect::<String>()
                            .into_bytes(),
                    ));
                }
                ["container", "inspect", TARGET] => {
                    if matches!(mode, "saved-target-race" | "saved-preparation-race") {
                        let state = self.previous.base.state();
                        edit_execution(&state, self.previous.base.intent.operation.id, |saved| {
                            if mode == "saved-target-race" {
                                saved["target"]["startedAt"] = json!("2026-11-03T00:03:00Z");
                            } else {
                                let checked: chrono::DateTime<chrono::Utc> =
                                    serde_json::from_value(saved["prepared"]["checkedAt"].clone())
                                        .unwrap();
                                saved["prepared"]["checkedAt"] =
                                    json!(checked - chrono::Duration::milliseconds(1));
                            }
                        });
                    }
                    json!([self.core(mode)])
                }
                ["container", "inspect", DATABASE] => {
                    let mut database = self.previous.base.fixture.runner.database.clone();
                    match mode {
                        "database-restarted" => {
                            database["State"]["StartedAt"] = json!("2026-11-03T00:04:00Z")
                        }
                        "database-env" => database["Config"]["Env"]
                            .as_array_mut()
                            .unwrap()
                            .push(json!("EXTRA=unexpected")),
                        "database-unhealthy" => {
                            database["State"]["Health"]["Status"] = json!("unhealthy")
                        }
                        "network-recreated" => {
                            database["NetworkSettings"]["Networks"][&reg.network]["NetworkID"] =
                                json!("8".repeat(64))
                        }
                        _ => (),
                    }
                    json!([database])
                }
                ["info", ..] => {
                    self.scans.fetch_add(1, Ordering::SeqCst);
                    if mode == "timeout" {
                        return Err(ProcessError::Timeout(Duration::from_secs(15)));
                    }
                    if mode == "cancel" {
                        token.cancel();
                        return Err(ProcessError::Cancelled);
                    }
                    if mode == "malformed" {
                        return Ok(output(b"not-json".to_vec()));
                    }
                    return self.previous.run(request, token).await;
                }
                ["network", "inspect", ..] => {
                    let out = self
                        .previous
                        .base
                        .fixture
                        .runner
                        .run(request, token)
                        .await?;
                    let mut value: Value = serde_json::from_slice(&out.stdout).unwrap();
                    value[0]["Containers"] = json!({TARGET:{},DATABASE:{}});
                    if mode == "network-recreated" {
                        value[0]["Id"] = json!("8".repeat(64));
                    }
                    if mode == "input-race" {
                        use std::io::Write;
                        fs::OpenOptions::new()
                            .append(true)
                            .open(&reg.environment_file)
                            .unwrap()
                            .write_all(b"CITADEL_HTTP_PORT=19999\n")
                            .unwrap();
                    }
                    value
                }
                ["volume", "inspect", ..] if mode == "volume-recreated" => {
                    let out = self.previous.run(request, token).await?;
                    let mut value: Value = serde_json::from_slice(&out.stdout).unwrap();
                    value[0]["CreatedAt"] = json!("2026-11-03T00:04:00Z");
                    value
                }
                ["container", "inspect", HELPER] if mode.starts_with("exit-") => {
                    let out = self.previous.run(request, token).await?;
                    let mut value: Value = serde_json::from_slice(&out.stdout).unwrap();
                    let helper = &mut value[0];
                    helper["State"]["Running"] = json!(false);
                    helper["State"]["Status"] = json!("exited");
                    helper["State"]["ExitCode"] = json!(0);
                    helper["State"]["OOMKilled"] = json!(false);
                    helper["State"]["Error"] = json!("");
                    match mode {
                        "exit-clean" => (),
                        "exit-running" => {
                            helper["State"]["Running"] = json!(true);
                            helper["State"]["Status"] = json!("running");
                        }
                        "exit-image" => {
                            helper["Image"] = json!(format!("sha256:{}", "f".repeat(64)))
                        }
                        "exit-nonce" => {
                            helper["Config"]["Labels"][super::verification::NONCE] =
                                json!(uuid::Uuid::now_v7())
                        }
                        "exit-operation" => {
                            helper["Config"]["Labels"][super::verification::OPERATION] =
                                json!(uuid::Uuid::now_v7())
                        }
                        "exit-mount" => helper["Mounts"][0]["RW"] = json!(false),
                        "exit-user" => helper["Config"]["User"] = json!("wrong"),
                        "exit-command" => helper["Args"] = json!(["serve"]),
                        "exit-network" => helper["HostConfig"]["NetworkMode"] = json!("host"),
                        "exit-privileged" => helper["HostConfig"]["Privileged"] = json!(true),
                        "exit-restarted" => {
                            helper["State"]["StartedAt"] = json!("2026-11-03T00:04:00Z")
                        }
                        "exit-nonzero" => helper["State"]["ExitCode"] = json!(1),
                        "exit-oom" => helper["State"]["OOMKilled"] = json!(true),
                        "exit-error" => helper["State"]["Error"] = json!("failed"),
                        "exit-created" => helper["State"]["Status"] = json!("created"),
                        "exit-removing" => helper["State"]["Status"] = json!("removing"),
                        _ => panic!("unexpected exit mode {mode}"),
                    }
                    value
                }
                ["container", "inspect", HELPER] if mode == "helper-restarted" => {
                    let out = self.previous.run(request, token).await?;
                    let mut value: Value = serde_json::from_slice(&out.stdout).unwrap();
                    value[0]["State"]["StartedAt"] = json!("2026-11-03T00:04:00Z");
                    value
                }
                _ => return self.previous.run(request, token).await,
            };
            Ok(output(serde_json::to_vec(&value).unwrap()))
        })
    }
}
fn output(stdout: Vec<u8>) -> ProcessOutput {
    ProcessOutput {
        exit_code: Some(0),
        stdout,
        stderr: vec![],
        stdout_truncated: false,
        stderr_truncated: false,
    }
}

#[tokio::test]
async fn binds_maintenance_target_without_completing_and_pins_its_incarnation() {
    let (runner, helper) = TargetRunner::new().await;
    let token = CancellationToken::new();
    let before = helper.state.journal(helper.intent.operation.id).unwrap();
    let journal = helper.observe_replacement(&runner, &token).await.unwrap();
    assert_eq!(journal.operation.phase, UpdatePhase::Verifying);
    assert_eq!(journal.operation.revision, 4);
    assert!(journal.verified_target.is_none());
    assert!(!journal.completion_acknowledged && !journal.maintenance_released);
    assert_eq!(journal.operation.checkpoint, before.operation.checkpoint);
    let receipt = execution(&helper.state, helper.intent.operation.id)["target"].clone();
    assert_eq!(receipt["containerId"], TARGET);
    assert!(
        !serde_json::to_string(&receipt)
            .unwrap()
            .contains("private-test-password")
    );
    // Observation can be repeated and health can change without completing the update.
    for mode in ["healthy", "starting", "normal"] {
        *runner.mode.lock().unwrap() = mode;
        assert_eq!(
            helper.observe_replacement(&runner, &token).await.unwrap(),
            journal
        );
        assert_eq!(
            execution(&helper.state, helper.intent.operation.id)["target"],
            receipt
        );
    }
    // Phase and target identity are published in the same atomic record.
    let saved = execution(&helper.state, helper.intent.operation.id);
    assert_eq!(saved["journal"], serde_json::to_value(&journal).unwrap());
    assert_eq!(saved["target"], receipt);
    *runner.mode.lock().unwrap() = "manual-restart";
    assert!(helper.observe_replacement(&runner, &token).await.is_err());
    assert_eq!(
        execution(&helper.state, helper.intent.operation.id)["target"],
        receipt
    );
    assert_eq!(
        helper
            .state
            .journal(helper.intent.operation.id)
            .unwrap()
            .operation
            .phase,
        UpdatePhase::Verifying
    );
}

#[tokio::test]
async fn changed_or_ambiguous_runtime_never_binds_target_or_releases_maintenance() {
    for mode in [
        "stopped",
        "wrong-image",
        "wrong-env",
        "bad-mount",
        "restarted",
        "missing-start",
        "duplicate-core",
        "old-source",
        "database-restarted",
        "database-env",
        "database-unhealthy",
        "network-recreated",
        "volume-recreated",
        "helper-restarted",
        "restart-during-read",
        "input-race",
        "timeout",
        "cancel",
        "malformed",
    ] {
        let (runner, helper) = TargetRunner::new().await;
        *runner.mode.lock().unwrap() = mode;
        let before = helper.state.journal(helper.intent.operation.id).unwrap();
        let error = helper
            .observe_replacement(&runner, &CancellationToken::new())
            .await
            .unwrap_err();
        assert!(!error.to_string().contains("private-test-password"));
        assert_eq!(
            helper.state.journal(helper.intent.operation.id).unwrap(),
            before,
            "{mode}"
        );
        assert!(
            execution(&helper.state, helper.intent.operation.id)["target"].is_null(),
            "{mode}"
        );
    }
}

#[tokio::test]
async fn unconfirmed_or_foreign_replacement_evidence_never_reaches_docker() {
    for mode in [
        "missing-submission",
        "foreign-replacement",
        "invalid-format",
        "missing-baseline",
        "changed-launch",
        "foreign-checkpoint",
        "premature-verification",
        "success-flag",
    ] {
        let (runner, helper) = TargetRunner::new().await;
        let mut journal = helper.state.journal(helper.intent.operation.id).unwrap();
        match mode {
            "missing-submission" => {
                edit_execution(&helper.state, helper.intent.operation.id, |v| {
                    v["submitted"] = json!(false)
                })
            }
            "foreign-replacement" => edit(&helper, "replacement-intent", |v| {
                v["targetImageId"] = json!("sha256:wrong")
            }),
            "invalid-format" => edit(&helper, "replacement-intent", |v| v["format"] = json!(99)),
            "missing-baseline" => edit(&helper, "replacement-intent", |v| {
                v.as_object_mut().unwrap().remove("retainedRuntimeSha256");
            }),
            "changed-launch" => edit(&helper, "launch", |v| {
                v["nonce"] = json!(uuid::Uuid::now_v7())
            }),
            "foreign-checkpoint" => edit(&helper, "replacement-intent", |v| {
                v["checkpoint"]["fence"] = json!(uuid::Uuid::now_v7())
            }),
            "premature-verification" => {
                journal
                    .operation
                    .advance(3, UpdatePhase::Verifying, chrono::Utc::now())
                    .unwrap();
                journal.history.push(journal.operation.clone());
                edit_execution(&helper.state, helper.intent.operation.id, |v| {
                    v["journal"] = serde_json::to_value(&journal).unwrap()
                });
            }
            "success-flag" => {
                journal.verified_target = Some(journal.operation.plan.target.clone());
                edit_execution(&helper.state, helper.intent.operation.id, |v| {
                    v["journal"] = serde_json::to_value(&journal).unwrap()
                });
            }
            _ => unreachable!(),
        }
        let before = execution(&helper.state, helper.intent.operation.id);
        assert!(
            helper
                .observe_replacement(&runner, &CancellationToken::new())
                .await
                .is_err(),
            "{mode}"
        );
        assert_eq!(runner.calls.load(Ordering::SeqCst), 0, "{mode}");
        assert_eq!(execution(&helper.state, helper.intent.operation.id), before);
    }
}
fn edit(helper: &OwnedUpdateHelper, kind: &str, mutate: impl FnOnce(&mut Value)) {
    if kind == "replacement-intent" {
        edit_execution(&helper.state, helper.intent.operation.id, |v| {
            mutate(&mut v["replacement"])
        });
    } else {
        let name = helper.intent.file(kind);
        let mut value: Value = helper.state.read(&name).unwrap();
        mutate(&mut value);
        helper.state.write(&name, &value).unwrap();
    }
}

fn completion(journal: &UpdateJournal) -> super::completion::TargetUpdateCompletion {
    let journal = journal
        .completed(&journal.operation.plan.target, chrono::Utc::now())
        .unwrap();
    let op = &journal.operation;
    super::completion::TargetUpdateCompletion {
        receipt: citadel_updates::CompletionReceipt {
            instance_id: op.instance_id,
            operation_id: op.id,
            fence: op.fence,
            revision: op.revision,
            target: op.plan.target.clone(),
        },
        journal,
    }
}

#[tokio::test]
async fn completion_mailbox_requires_exact_target_receipt_and_does_not_release_maintenance() {
    use super::command::{HelperAction, HelperRequest};
    let (runner, helper) = TargetRunner::new().await;
    let token = CancellationToken::new();
    let journal = helper.observe_replacement(&runner, &token).await.unwrap();
    let done = completion(&journal);
    let mut request = HelperRequest {
        operation: helper.intent.operation.clone(),
        binding: helper.acknowledgement.binding.clone(),
        started_at: helper.acknowledgement.started_at.clone(),
        action: HelperAction::InspectTarget,
    };
    let snapshot = exchange(&serde_json::to_vec(&request).unwrap(), &runner, &token)
        .await
        .unwrap();
    let super::command::HelperResponse::Ready(ref evidence) = snapshot else {
        panic!("target binding must be ready");
    };
    assert_eq!(evidence.journal, journal);
    assert_eq!(evidence.container_id, TARGET);
    assert!(
        !helper
            .accept_target_completion(&runner, &token)
            .await
            .unwrap()
    );
    for kind in ["fence", "revision", "target", "history", "released"] {
        let mut invalid = done.clone();
        match kind {
            "fence" => invalid.receipt.fence = uuid::Uuid::now_v7(),
            "revision" => invalid.receipt.revision += 1,
            "target" => invalid.receipt.target = journal.operation.plan.source.clone(),
            "history" => {
                invalid.journal.history.remove(1);
            }
            "released" => invalid.journal.maintenance_released = true,
            _ => unreachable!(),
        }
        request.action = HelperAction::CompleteTarget(Box::new(invalid));
        assert!(
            exchange(&serde_json::to_vec(&request).unwrap(), &runner, &token)
                .await
                .is_err(),
            "{kind}"
        );
        assert!(
            helper
                .state
                .read_optional::<Value>(&helper.intent.file("target-completion"))
                .unwrap()
                .is_none()
        );
        assert_eq!(helper.state.journal(journal.operation.id).unwrap(), journal);
    }
    request.action = HelperAction::CompleteTarget(Box::new(done.clone()));
    for _ in 0..2 {
        assert_eq!(
            exchange(&serde_json::to_vec(&request).unwrap(), &runner, &token)
                .await
                .unwrap(),
            snapshot
        );
    }
    // Publishing a committed receipt never races the driver by writing the journal.
    assert_eq!(helper.state.journal(journal.operation.id).unwrap(), journal);
    assert!(
        helper
            .accept_target_completion(&runner, &token)
            .await
            .unwrap()
    );
    let acknowledged = helper.state.journal(journal.operation.id).unwrap();
    assert_eq!(acknowledged.operation, done.journal.operation);
    assert!(acknowledged.completion_acknowledged);
    assert!(!acknowledged.maintenance_released);
    assert!(
        done.receipt
            .permits_resume(&acknowledged, &journal.operation.plan.target)
            .is_err()
    );
}

#[tokio::test]
async fn completion_rechecks_target_and_preserves_receipt_on_incarnation_change() {
    let (runner, helper) = TargetRunner::new().await;
    let token = CancellationToken::new();
    let journal = helper.observe_replacement(&runner, &token).await.unwrap();
    helper
        .state
        .create(
            &helper.intent.file("target-completion"),
            &completion(&journal),
        )
        .unwrap();
    *runner.mode.lock().unwrap() = "manual-restart";
    assert!(
        helper
            .accept_target_completion(&runner, &token)
            .await
            .is_err()
    );
    assert_eq!(helper.state.journal(journal.operation.id).unwrap(), journal);
    assert!(
        helper
            .state
            .path()
            .join(helper.intent.file("target-completion"))
            .exists()
    );
    assert!(
        !helper
            .state
            .journal(journal.operation.id)
            .unwrap()
            .completion_acknowledged
    );
}

#[tokio::test]
async fn helper_cannot_acknowledge_a_receipt_that_target_persistence_does_not_confirm() {
    for mode in ["uncommitted", "foreign-completion"] {
        let (runner, helper) = TargetRunner::new().await;
        let token = CancellationToken::new();
        let journal = helper.observe_replacement(&runner, &token).await.unwrap();
        helper
            .state
            .create(
                &helper.intent.file("target-completion"),
                &completion(&journal),
            )
            .unwrap();
        *runner.mode.lock().unwrap() = mode;
        assert!(
            helper
                .accept_target_completion(&runner, &token)
                .await
                .is_err()
        );
        assert_eq!(helper.state.journal(journal.operation.id).unwrap(), journal);
        assert!(
            !helper
                .state
                .journal(journal.operation.id)
                .unwrap()
                .completion_acknowledged
        );
    }
}

#[tokio::test]
async fn completion_rejects_execution_changes_during_receipt_verification() {
    let (runner, helper) = TargetRunner::new().await;
    let token = CancellationToken::new();
    let journal = helper.observe_replacement(&runner, &token).await.unwrap();
    let done = completion(&journal);
    helper
        .state
        .create(&helper.intent.file("target-completion"), &done)
        .unwrap();
    let mut expected = execution(&helper.state, helper.intent.operation.id);
    let checked: chrono::DateTime<chrono::Utc> =
        serde_json::from_value(expected["prepared"]["checkedAt"].clone()).unwrap();
    expected["prepared"]["checkedAt"] = json!(checked - chrono::Duration::milliseconds(1));
    *runner.mode.lock().unwrap() = "preparation-changed-during-receipt";

    assert!(
        helper
            .accept_target_completion(&runner, &token)
            .await
            .is_err()
    );
    assert_eq!(
        execution(&helper.state, helper.intent.operation.id),
        expected
    );
    assert_eq!(
        helper
            .state
            .read::<super::TargetUpdateCompletion>(&helper.intent.file("target-completion"))
            .unwrap(),
        done,
    );
}

#[tokio::test]
async fn interrupted_replacement_records_recovery_without_replaying_daemon_mutations() {
    let (runner, helper) = TargetRunner::new().await;
    let before = runner.calls.load(Ordering::SeqCst);
    super::driver::record_failure(&helper).unwrap();
    let failed = helper.state.journal(helper.intent.operation.id).unwrap();
    assert_eq!(failed.operation.phase, UpdatePhase::RecoveryRequired);
    assert!(failed.operation.checkpoint.is_some());
    assert!(!failed.maintenance_released);
    super::driver::record_failure(&helper).unwrap();
    assert_eq!(
        helper.state.journal(helper.intent.operation.id).unwrap(),
        failed
    );
    assert_eq!(runner.calls.load(Ordering::SeqCst), before);
}

#[tokio::test]
async fn target_exchange_waits_only_for_the_owned_in_flight_replacement() {
    use super::command::{HelperAction, HelperRequest, HelperResponse};
    let (runner, helper) = TargetRunner::new().await;
    let token = CancellationToken::new();
    let request = HelperRequest {
        operation: helper.intent.operation.clone(),
        binding: helper.acknowledgement.binding.clone(),
        started_at: helper.acknowledgement.started_at.clone(),
        action: HelperAction::InspectTarget,
    };
    let input = serde_json::to_vec(&request).unwrap();
    let before = helper.state.journal(helper.intent.operation.id).unwrap();
    for _ in 0..2 {
        assert_eq!(
            exchange(&input, &runner, &token).await.unwrap(),
            HelperResponse::Pending
        );
        assert_eq!(
            helper.state.journal(helper.intent.operation.id).unwrap(),
            before
        );
    }
    helper.observe_replacement(&runner, &token).await.unwrap();
    assert!(matches!(
        exchange(&input, &runner, &token).await.unwrap(),
        HelperResponse::Ready(_)
    ));
    super::driver::record_failure(&helper).unwrap();
    assert!(exchange(&input, &runner, &token).await.is_err());
}

#[tokio::test]
async fn pending_target_exchange_rejects_conflicting_grants_and_fences() {
    use super::command::{HelperAction, HelperRequest};
    for mode in ["fence", "grant", "journal", "unlocked"] {
        let (runner, helper) = TargetRunner::new().await;
        let mut request = HelperRequest {
            operation: helper.intent.operation.clone(),
            binding: helper.acknowledgement.binding.clone(),
            started_at: helper.acknowledgement.started_at.clone(),
            action: HelperAction::InspectTarget,
        };
        match mode {
            "fence" => request.operation.fence = uuid::Uuid::now_v7(),
            "grant" => edit(&helper, "shutdown-grant", |value| {
                value["incarnation"] = json!(uuid::Uuid::now_v7());
            }),
            "journal" => {
                let mut journal = helper.state.journal(helper.intent.operation.id).unwrap();
                journal.helper_container_id = TARGET.into();
                edit_execution(&helper.state, helper.intent.operation.id, |v| {
                    v["journal"] = serde_json::to_value(&journal).unwrap()
                });
            }
            "unlocked" => drop(helper),
            _ => unreachable!(),
        }
        assert!(
            exchange(
                &serde_json::to_vec(&request).unwrap(),
                &runner,
                &CancellationToken::new(),
            )
            .await
            .is_err(),
            "{mode}"
        );
    }
}

#[tokio::test]
async fn target_connection_waits_for_binding_then_checks_its_own_container() {
    use super::TargetUpdateSession;
    let (runner, helper) = TargetRunner::new().await;
    let token = CancellationToken::new();
    let connect =
        TargetUpdateSession::connect_from(&helper.intent.operation, &runner, &token, &TARGET[..12]);
    let publish = async {
        tokio::time::sleep(Duration::from_millis(30)).await;
        helper.observe_replacement(&runner, &token).await.unwrap();
    };
    let (result, ()) = tokio::join!(connect, publish);
    let session = result.unwrap();
    assert_eq!(session.evidence.container_id, TARGET);
    assert_eq!(
        session.evidence.journal.operation.phase,
        UpdatePhase::Verifying
    );
    for (hostname, mode) in [
        (&CORE[..12], "normal"),
        (&TARGET[..12], "duplicate-helper"),
        (&TARGET[..12], "manual-restart"),
    ] {
        *runner.mode.lock().unwrap() = mode;
        assert!(
            TargetUpdateSession::connect_from(&helper.intent.operation, &runner, &token, hostname,)
                .await
                .is_err(),
            "{mode}"
        );
    }
}

#[tokio::test]
async fn cancellation_ends_the_pending_target_wait_without_changing_the_journal() {
    let (runner, helper) = TargetRunner::new().await;
    let before = helper.state.journal(helper.intent.operation.id).unwrap();
    let token = CancellationToken::new();
    let connect = super::TargetUpdateSession::connect_from(
        &helper.intent.operation,
        &runner,
        &token,
        &TARGET[..12],
    );
    let cancel = async {
        tokio::time::sleep(Duration::from_millis(30)).await;
        token.cancel();
    };
    let (result, ()) = tokio::join!(connect, cancel);
    assert!(matches!(
        result,
        Err(citadel_updates::UpdateError::Cancelled)
    ));
    assert_eq!(
        helper.state.journal(helper.intent.operation.id).unwrap(),
        before
    );
}

#[tokio::test]
async fn target_cannot_verify_keys_against_a_changed_checkpoint() {
    use super::command::{HelperAction, HelperRequest};
    let (runner, helper) = TargetRunner::new().await;
    let token = CancellationToken::new();
    let journal = helper.observe_replacement(&runner, &token).await.unwrap();
    let request = HelperRequest {
        operation: helper.intent.operation.clone(),
        binding: helper.acknowledgement.binding.clone(),
        started_at: helper.acknowledgement.started_at.clone(),
        action: HelperAction::InspectTarget,
    };
    let input = serde_json::to_vec(&request).unwrap();
    exchange(&input, &runner, &token).await.unwrap();
    let bundle = std::path::Path::new(journal.operation.checkpoint.as_ref().unwrap());
    for file in ["recovery/agent/signing-key", "checksums.json"] {
        let path = bundle.join(file);
        let original = fs::read(&path).unwrap();
        fs::write(&path, b"changed-checkpoint-evidence").unwrap();
        assert!(exchange(&input, &runner, &token).await.is_err(), "{file}");
        assert_eq!(helper.state.journal(journal.operation.id).unwrap(), journal);
        fs::write(path, original).unwrap();
    }
}

async fn exchange(
    input: &[u8],
    process: &dyn ProcessRunner,
    cancellation: &CancellationToken,
) -> Result<super::command::HelperResponse, citadel_updates::UpdateError> {
    let output = super::execute_helper_handoff(input, process, cancellation).await?;
    serde_json::from_slice(&output).map_err(|_| super::evidence::conflict())
}

#[tokio::test]
async fn shared_helper_protocol_rejects_wrong_identity_and_source_actions_after_replacement() {
    use super::command::{HelperAction, HelperRequest};
    let (runner, helper) = TargetRunner::new().await;
    let token = CancellationToken::new();
    let journal = helper.observe_replacement(&runner, &token).await.unwrap();
    for mode in [
        "operation",
        "plan",
        "nonce",
        "helper",
        "started",
        "check",
        "grant",
    ] {
        let mut request = HelperRequest {
            operation: helper.intent.operation.clone(),
            binding: helper.acknowledgement.binding.clone(),
            started_at: helper.acknowledgement.started_at.clone(),
            action: HelperAction::InspectTarget,
        };
        match mode {
            "operation" => request.operation.id = uuid::Uuid::now_v7(),
            "plan" => request.operation.plan.target.version = "9.9.9".into(),
            "nonce" => request.binding.nonce = uuid::Uuid::now_v7(),
            "helper" => request.binding.helper_container_id = TARGET.into(),
            "started" => request.started_at = "2026-11-03T00:03:00Z".into(),
            "check" => request.action = HelperAction::Check,
            "grant" => request.action = HelperAction::Grant,
            _ => unreachable!(),
        }
        assert!(
            exchange(&serde_json::to_vec(&request).unwrap(), &runner, &token)
                .await
                .is_err(),
            "{mode}"
        );
        assert_eq!(
            helper.state.journal(journal.operation.id).unwrap(),
            journal,
            "{mode}"
        );
        assert!(
            helper
                .state
                .read_optional::<Value>(&helper.intent.file("target-completion"))
                .unwrap()
                .is_none()
        );
    }
}

#[tokio::test]
async fn observation_and_target_exchange_share_checkpoint_and_submission_checks() {
    use super::command::{HelperAction, HelperRequest};
    let (runner, helper) = TargetRunner::new().await;
    let token = CancellationToken::new();
    let journal = helper.observe_replacement(&runner, &token).await.unwrap();
    let request = HelperRequest {
        operation: helper.intent.operation.clone(),
        binding: helper.acknowledgement.binding.clone(),
        started_at: helper.acknowledgement.started_at.clone(),
        action: HelperAction::InspectTarget,
    };
    let input = serde_json::to_vec(&request).unwrap();
    let checkpoint = helper
        .state
        .path()
        .join(format!("checkpoint-{}", journal.operation.id))
        .join("checkpoint.json");
    let original = fs::read(&checkpoint).unwrap();
    let mut changed: Value = serde_json::from_slice(&original).unwrap();
    changed["format"] = json!(999);
    fs::write(&checkpoint, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert!(helper.observe_replacement(&runner, &token).await.is_err());
    assert!(exchange(&input, &runner, &token).await.is_err());
    assert_eq!(helper.state.journal(journal.operation.id).unwrap(), journal);
    fs::write(&checkpoint, original).unwrap();
    edit_execution(&helper.state, journal.operation.id, |v| {
        v["submitted"] = json!(false)
    });
    assert!(helper.observe_replacement(&runner, &token).await.is_err());
    assert!(exchange(&input, &runner, &token).await.is_err());
    edit_execution(&helper.state, journal.operation.id, |v| {
        v["submitted"] = json!(true)
    });
    // Restoring fixture evidence makes reads possible again without replaying replacement.
    exchange(&input, &runner, &token).await.unwrap();
    assert_eq!(helper.state.journal(journal.operation.id).unwrap(), journal);
}

#[tokio::test]
async fn target_identity_cannot_be_published_separately_from_verifying() {
    let (runner, helper) = TargetRunner::new().await;
    let before = helper.state.journal(helper.intent.operation.id).unwrap();
    helper
        .observe_replacement(&runner, &CancellationToken::new())
        .await
        .unwrap();
    // A half-published phase/binding is corrupt, not a resumable transition.
    edit_execution(&helper.state, helper.intent.operation.id, |v| {
        v["journal"] = serde_json::to_value(&before).unwrap();
    });
    let calls = runner.calls.load(Ordering::SeqCst);
    assert!(
        helper
            .observe_replacement(&runner, &CancellationToken::new())
            .await
            .is_err()
    );
    assert_eq!(runner.calls.load(Ordering::SeqCst), calls);
}

#[tokio::test]
async fn changed_saved_target_during_inspection_is_rejected() {
    let (runner, helper) = TargetRunner::new().await;
    let token = CancellationToken::new();
    let journal = helper.observe_replacement(&runner, &token).await.unwrap();
    *runner.mode.lock().unwrap() = "saved-target-race";
    assert!(
        super::ExecutionJournal::read_replacement(&helper.state, &helper.intent)
            .unwrap()
            .target_evidence(&helper.state, &helper.intent, &runner, &token)
            .await
            .is_err()
    );
    assert_eq!(helper.state.journal(journal.operation.id).unwrap(), journal);
    assert_eq!(
        execution(&helper.state, journal.operation.id)["target"]["startedAt"],
        "2026-11-03T00:03:00Z"
    );
    assert!(
        helper
            .state
            .read_optional::<Value>(&helper.intent.file("target-completion"))
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn changed_preparation_during_observation_is_not_overwritten() {
    let (runner, helper) = TargetRunner::new().await;
    let token = CancellationToken::new();
    let before = execution(&helper.state, helper.intent.operation.id);
    *runner.mode.lock().unwrap() = "saved-preparation-race";
    assert!(helper.observe_replacement(&runner, &token).await.is_err());
    let saved = execution(&helper.state, helper.intent.operation.id);
    assert_eq!(saved["journal"], before["journal"]);
    assert_eq!(saved["target"], Value::Null);
    assert_ne!(saved["prepared"], before["prepared"]);
}

#[tokio::test]
async fn helper_exit_requires_the_same_verified_container_profile() {
    let (runner, helper) = TargetRunner::new().await;
    let token = CancellationToken::new();
    let journal = helper.observe_replacement(&runner, &token).await.unwrap();
    let session = super::TargetUpdateSession::connect_from(
        &helper.intent.operation,
        &runner,
        &token,
        &TARGET[..12],
    )
    .await
    .unwrap();
    for mode in [
        "exit-image",
        "exit-nonce",
        "exit-operation",
        "exit-mount",
        "exit-user",
        "exit-command",
        "exit-network",
        "exit-privileged",
        "exit-restarted",
        "exit-nonzero",
        "exit-oom",
        "exit-error",
        "exit-created",
        "exit-removing",
    ] {
        *runner.mode.lock().unwrap() = mode;
        assert!(
            session.wait_for_exit(&runner, &token).await.is_err(),
            "{mode}"
        );
        assert_eq!(helper.state.journal(journal.operation.id).unwrap(), journal);
    }
    *runner.mode.lock().unwrap() = "exit-clean";
    session.wait_for_exit(&runner, &token).await.unwrap();
    // An exit observation cannot acknowledge completion or release maintenance.
    assert_eq!(helper.state.journal(journal.operation.id).unwrap(), journal);
}

#[tokio::test]
async fn helper_exit_wait_is_cancellable_and_preserves_maintenance() {
    let (runner, helper) = TargetRunner::new().await;
    let token = CancellationToken::new();
    let journal = helper.observe_replacement(&runner, &token).await.unwrap();
    let session = super::TargetUpdateSession::connect_from(
        &helper.intent.operation,
        &runner,
        &token,
        &TARGET[..12],
    )
    .await
    .unwrap();
    *runner.mode.lock().unwrap() = "exit-running";
    let cancel = async {
        tokio::time::sleep(Duration::from_millis(30)).await;
        token.cancel();
    };
    let (result, ()) = tokio::join!(session.wait_for_exit(&runner, &token), cancel);
    assert!(matches!(
        result,
        Err(citadel_updates::UpdateError::Cancelled)
    ));
    assert_eq!(helper.state.journal(journal.operation.id).unwrap(), journal);
    let fresh = CancellationToken::new();
    let finish = async {
        tokio::time::sleep(Duration::from_millis(30)).await;
        *runner.mode.lock().unwrap() = "exit-clean";
    };
    let (result, ()) = tokio::join!(session.wait_for_exit(&runner, &fresh), finish);
    result.unwrap();
    assert_eq!(helper.state.journal(journal.operation.id).unwrap(), journal);
}

#[tokio::test]
async fn finalization_reconciles_registration_and_archives_without_moving_checkpoint() {
    let (runner, helper) = TargetRunner::new().await;
    let token = CancellationToken::new();
    let journal = helper.observe_replacement(&runner, &token).await.unwrap();
    let done = completion(&journal);
    helper
        .state
        .create(&helper.intent.file("target-completion"), &done)
        .unwrap();
    helper
        .accept_target_completion(&runner, &token)
        .await
        .unwrap();
    let intent = helper.intent.clone();
    let checkpoint = done.journal.operation.checkpoint.clone().unwrap();
    drop(helper);
    *runner.mode.lock().unwrap() = "exit-clean";
    let released = super::finalize_update(
        &super::FinalizeUpdateRequest {
            intent: intent.clone(),
            completion: done.clone(),
        },
        &runner,
        &token,
    )
    .await
    .unwrap();
    assert!(released.maintenance_released && released.completion_acknowledged);
    done.receipt
        .permits_resume(&released, &intent.operation.plan.target)
        .unwrap();
    let state = runner.previous.base.state();
    state.require_registration_only().unwrap();
    let registration: super::super::RegisteredComposeInstallation =
        state.read("installation.json").unwrap();
    assert_eq!(registration.core_container_id, TARGET);
    assert_eq!(registration.id, intent.registration.id);
    assert_eq!(
        registration.configuration_sha256,
        registration.fingerprint(std::path::Path::new("/")).unwrap()
    );
    assert!(std::path::Path::new(&checkpoint).exists());
    let archive = super::super::JournalDirectory::open(
        state
            .path()
            .join(format!("checkpoint-{}", intent.operation.id)),
    )
    .unwrap();
    let saved: super::finalization::CompletedUpdate = archive.read("completed.json").unwrap();
    saved.validate(&archive).unwrap();
    assert!(runner.removed.load(Ordering::SeqCst));
    // Reconstruct each interrupted archive boundary. The clean-exit receipt is
    // retained, the helper is absent, and container replacement must not repeat.
    let names = [
        intent.file("launch"),
        intent.file("binding"),
        intent.file("shutdown-grant"),
        intent.file("target-completion"),
        format!("{}.json", intent.operation.id),
    ];
    for completed_moves in 0..=names.len() {
        for name in names.iter().skip(completed_moves) {
            fs::rename(archive.path().join(name), state.path().join(name)).unwrap();
        }
        fs::remove_file(archive.path().join("completed.json")).unwrap();
        state
            .write("installation.json", &intent.registration)
            .unwrap();
        if completed_moves < names.len() {
            edit_execution(&state, intent.operation.id, |value| {
                value["journal"]["maintenanceReleased"] = json!(false)
            });
        }
        let recovered = super::completed_update_target(&released.operation, &runner, &token)
            .await
            .unwrap();
        assert_eq!(
            recovered.journal, released,
            "archive boundary {completed_moves}"
        );
        state.require_registration_only().unwrap();
    }
    for mode in ["normal", "manual-restart"] {
        *runner.mode.lock().unwrap() = mode;
        let recovered = super::completed_update_target(&released.operation, &runner, &token)
            .await
            .unwrap();
        assert_eq!(recovered.journal, released);
        assert_eq!(recovered.container_id, TARGET);
    }
    *runner.mode.lock().unwrap() = "wrong-image";
    assert!(
        super::completed_update_target(&released.operation, &runner, &token)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn finalization_rejects_changed_helper_before_cleanup() {
    for mode in ["exit-nonce", "exit-nonzero", "exit-image", "exit-restarted"] {
        let (runner, helper) = TargetRunner::new().await;
        let token = CancellationToken::new();
        let journal = helper.observe_replacement(&runner, &token).await.unwrap();
        let done = completion(&journal);
        helper
            .state
            .create(&helper.intent.file("target-completion"), &done)
            .unwrap();
        helper
            .accept_target_completion(&runner, &token)
            .await
            .unwrap();
        let intent = helper.intent.clone();
        drop(helper);
        *runner.mode.lock().unwrap() = mode;
        assert!(
            super::finalize_update(
                &super::FinalizeUpdateRequest {
                    intent: intent.clone(),
                    completion: done
                },
                &runner,
                &token
            )
            .await
            .is_err(),
            "{mode}"
        );
        assert!(!runner.removed.load(Ordering::SeqCst));
        let state = runner.previous.base.state();
        assert!(
            !state
                .journal(intent.operation.id)
                .unwrap()
                .maintenance_released
        );
        let registration: super::super::RegisteredComposeInstallation =
            state.read("installation.json").unwrap();
        assert_eq!(registration, intent.registration);
    }
}
