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
        atomic::{AtomicUsize, Ordering},
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
                return Ok(output(serde_json::to_vec(&receipt).unwrap()));
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
                    let ids = if filter.starts_with("label=com.citadel.update-operation=") {
                        if mode == "duplicate-helper" {
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
                        vec![TARGET, HELPER]
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
                ["container", "inspect", TARGET] => json!([self.core(mode)]),
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
