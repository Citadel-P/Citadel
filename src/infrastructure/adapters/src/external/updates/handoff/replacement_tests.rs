use super::super::{
    CheckpointInstallerInput, InstallerComposeInputs, JournalDirectory, PreparedUpdateCheckpoint,
    RestoredUpdateCheckpoint, bootstrap::fixtures::DATABASE,
};
use super::tests::{edit_execution, execution};
use super::{
    tests::{HELPER, Runner},
    *,
};
use citadel_execution::{ProcessError, ProcessOutput, ProcessRequest, ProcessRunner};
use citadel_updates::{UpdateError, UpdateHandoffRuntime, UpdatePhase};
use futures_util::future::BoxFuture;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

// Model a completed rehearsal with real on-disk bundle checksums. Actual
// PostgreSQL restoration/decryption is exercised by the restore acceptance suite.
fn checkpoint(helper: &OwnedUpdateHelper) -> JournalDirectory {
    let journal = helper.state.journal(helper.intent.operation.id).unwrap();
    let op = &journal.operation;
    let path = helper.state.path().join(format!("checkpoint-{}", op.id));
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    let stage = JournalDirectory::open(path).unwrap();
    stage.create("operation.json", op).unwrap();
    stage
        .create("installation.json", &helper.intent.registration)
        .unwrap();
    let reg = &helper.intent.registration;
    let mut inputs = vec![];
    for (index, path) in reg
        .compose_files
        .iter()
        .chain([&reg.environment_file])
        .enumerate()
    {
        let bytes = fs::read(path).unwrap();
        let saved_name = format!("input-{index}");
        write(&stage.path().join(&saved_name), &bytes);
        inputs.push(CheckpointInstallerInput {
            original_path: path.clone(),
            saved_name,
            sha256: hash(&bytes),
        });
    }
    let bundle = stage.path().join(op.id.simple().to_string());
    fs::create_dir(&bundle).unwrap();
    fs::set_permissions(&bundle, fs::Permissions::from_mode(0o700)).unwrap();
    let dump = b"PGDMP-fixture";
    write(&bundle.join("database/citadel.dump"), dump);
    let signing = [8_u8; 32];
    write(&bundle.join("recovery/agent/signing-key"), &signing);
    let manifest = serde_json::to_vec(&json!({"formatVersion":1,"product":"citadel","coreVersion":op.plan.source.version,"instanceId":op.instance_id,"createdAt":chrono::Utc::now(),"database":{"engine":"PostgreSQL","path":"database/citadel.dump","sha256":hash(dump)},"requiredExternalConfiguration":["CITADEL_JWT_KEY","CITADEL_SECRETS_ENCRYPTION_KEY"],"recoveryFiles":["recovery/agent/signing-key"]})).unwrap();
    write(&bundle.join("manifest.json"), &manifest);
    let checksums = serde_json::to_vec(
        &json!({"manifest.json":hash(&manifest),"database/citadel.dump":hash(dump),"recovery/agent/signing-key":hash(&signing)}),
    )
    .unwrap();
    write(&bundle.join("checksums.json"), &checksums);
    stage
        .create(
            "checkpoint.json",
            &PreparedUpdateCheckpoint {
                format: 1,
                operation_id: op.id,
                fence: op.fence,
                instance_id: op.instance_id,
                source: op.plan.source.clone(),
                bundle: bundle.clone(),
                bundle_checksums_sha256: hash(&checksums),
                configuration_sha256: reg.configuration_sha256.clone(),
                installer_inputs: inputs,
                required_external_configuration: vec![
                    "CITADEL_JWT_KEY".into(),
                    "CITADEL_SECRETS_ENCRYPTION_KEY".into(),
                ],
            },
        )
        .unwrap();
    let image = format!("sha256:{}", "d".repeat(64));
    let id = "f".repeat(64);
    stage.create("restore-intent.json", &json!({"nonce":Uuid::now_v7(),"image":image,"dump":bundle.join("database/citadel.dump"),"maximumBytes":268435456})).unwrap();
    stage.create("restore-container.json", &id).unwrap();
    stage
        .create(
            "restore-verified.json",
            &RestoredUpdateCheckpoint {
                format: 2,
                operation_id: op.id,
                fence: op.fence,
                instance_id: op.instance_id,
                bundle_checksums_sha256: hash(&checksums),
                catalog_sha256: op.plan.source.catalog_sha256.clone(),
                database_image: image,
                sandbox_id: id,
                decrypted_values: 0,
            },
        )
        .unwrap();
    stage
}

pub(super) struct ReplacementRunner {
    pub(super) base: Arc<Runner>,
    mode: &'static str,
    submissions: AtomicUsize,
}
impl ProcessRunner for ReplacementRunner {
    fn run<'a>(
        &'a self,
        request: ProcessRequest,
        token: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ProcessOutput, ProcessError>> {
        Box::pin(async move {
            let args: Vec<_> = request
                .arguments
                .iter()
                .map(|s| s.to_str().unwrap())
                .collect();
            let reg = &self.base.intent.registration;
            let target = &self.base.intent.operation.plan.target.image;
            let value = if args.contains(&"compose") && args.contains(&"config") {
                let mut config = self.base.fixture.runner.config.clone();
                let env =
                    fs::read_to_string(request.current_directory.as_ref().unwrap().join(".env"))
                        .unwrap();
                let image = env
                    .lines()
                    .find_map(|line| line.strip_prefix("CITADEL_IMAGE="))
                    .unwrap();
                config["services"]["server"]["image"] = json!(image);
                config
            } else if args.contains(&"compose") && args.contains(&"up") {
                assert!(self.base.state().existing_lock().is_err());
                assert_eq!(
                    self.base
                        .state()
                        .journal(self.base.intent.operation.id)
                        .unwrap()
                        .operation
                        .phase,
                    UpdatePhase::ReplacingCore
                );
                assert!(
                    !execution(&self.base.state(), self.base.intent.operation.id)["replacement"]
                        .is_null()
                );
                assert!(
                    !execution(&self.base.state(), self.base.intent.operation.id)["submitted"]
                        .as_bool()
                        .unwrap()
                );
                assert!(
                    fs::read_to_string(&reg.environment_file)
                        .unwrap()
                        .contains(target)
                );
                let mut expected = vec![
                    "--host",
                    "unix:///var/run/docker.sock",
                    "compose",
                    "--project-name",
                    &reg.project,
                    "--project-directory",
                    reg.directory.to_str().unwrap(),
                    "--env-file",
                    reg.environment_file.to_str().unwrap(),
                ];
                for path in &reg.compose_files {
                    expected.extend(["--file", path.to_str().unwrap()]);
                }
                expected.extend([
                    "up",
                    "--detach",
                    "--no-deps",
                    "--no-build",
                    "--pull",
                    "never",
                    "server",
                ]);
                assert_eq!(args, expected);
                assert_eq!(
                    self.submissions.fetch_add(1, Ordering::SeqCst),
                    0,
                    "no replay"
                );
                match self.mode {
                    "timeout" => return Err(ProcessError::Timeout(Duration::from_secs(300))),
                    "cancel" => {
                        token.cancel();
                        return Err(ProcessError::Cancelled);
                    }
                    "failure" => return Ok(output(b"", false)),
                    "truncated" => {
                        let mut out = output(b"", true);
                        out.stderr_truncated = true;
                        return Ok(out);
                    }
                    _ => (),
                }
                return Ok(output(b"", true));
            } else if args.get(2) == Some(&"image") && args.last() == Some(&target.as_str()) {
                json!([{"Id":format!("sha256:{}", "c".repeat(64)),"Os":"linux","Architecture":if self.mode == "wrong-image" {"arm64"} else {"amd64"},"RepoDigests":[target],"Config":{"Volumes":null}}])
            } else if args.get(2) == Some(&"container")
                && args.get(3) == Some(&"ls")
                && args
                    .iter()
                    .any(|s| s.starts_with("id=") || s.starts_with("name=^/citadel-checkpoint-"))
            {
                return Ok(output(
                    if self.mode == "sandbox-present" {
                        b"\"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff\"\n"
                    } else {
                        b""
                    },
                    true,
                ));
            } else if matches!(self.mode, "database-restarted" | "input-race")
                && args.ends_with(&["container", "inspect", DATABASE])
                && fs::read_to_string(&reg.environment_file)
                    .unwrap()
                    .contains(target)
            {
                let mut db = self.base.fixture.runner.database.clone();
                if self.mode == "input-race" {
                    use std::io::Write;
                    fs::OpenOptions::new()
                        .append(true)
                        .open(&reg.environment_file)
                        .unwrap()
                        .write_all(b"CITADEL_HTTP_PORT=19999\n")
                        .unwrap();
                } else {
                    db["State"]["StartedAt"] = json!("2026-11-03T00:01:00Z");
                }
                json!([db])
            } else {
                return self.base.run(request, token).await;
            };
            Ok(output(&serde_json::to_vec(&value).unwrap(), true))
        })
    }
}
fn output(bytes: &[u8], success: bool) -> ProcessOutput {
    ProcessOutput {
        exit_code: Some(if success { 0 } else { 1 }),
        stdout: bytes.to_vec(),
        stderr: vec![],
        stdout_truncated: false,
        stderr_truncated: false,
    }
}

pub(super) async fn ready(
    mode: &'static str,
) -> (ReplacementRunner, OwnedUpdateHelper, JournalDirectory) {
    let base = Runner::new().await;
    let helper = base.acknowledge().await.unwrap();
    let runner = ReplacementRunner {
        base: base.clone(),
        mode,
        submissions: AtomicUsize::new(0),
    };
    let token = CancellationToken::new();
    let reg = &base.intent.registration;
    let inputs = InstallerComposeInputs {
        directory: reg.directory.clone(),
        project: reg.project.clone(),
        compose_files: reg.compose_files.clone(),
    };
    let checked = inputs
        .check_replacement(&base.intent.operation.plan.target.image, &runner, &token)
        .await
        .unwrap();
    edit_execution(&helper.state, base.intent.operation.id, |v| {
        v["prepared"]["configuration"] = serde_json::to_value(checked).unwrap();
    });
    let runtime = DockerHelperHandoff::new(base.intent.clone(), HELPER.into(), base.clone());
    runtime
        .authorize_shutdown(&base.intent.operation, &token)
        .await
        .unwrap();
    helper
        .stop_source(base.as_ref(), Duration::from_secs(5), &token)
        .await
        .unwrap();
    let stage = checkpoint(&helper);
    (runner, helper, stage)
}

#[tokio::test]
async fn core_only_submission_is_durable_and_never_reports_update_success_or_replays() {
    for mode in [
        "normal",
        "timeout",
        "cancel",
        "failure",
        "truncated",
        "database-restarted",
        "input-race",
    ] {
        let (runner, helper, stage) = ready(mode).await;
        let reg = &helper.intent.registration;
        let original = fs::read(&reg.environment_file).unwrap();
        let compose = fs::read(&reg.compose_files[0]).unwrap();
        let result = helper
            .replace_core(&runner, &CancellationToken::new())
            .await;
        assert_eq!(result.is_ok(), mode == "normal", "{mode}: {result:?}");
        assert_eq!(
            runner.submissions.load(Ordering::SeqCst),
            usize::from(!matches!(mode, "database-restarted" | "input-race")),
            "{mode}"
        );
        let journal = helper.state.journal(helper.intent.operation.id).unwrap();
        assert_eq!(journal.operation.phase, UpdatePhase::ReplacingCore);
        assert_eq!(journal.operation.revision, 3);
        assert!(journal.verified_target.is_none());
        assert!(!journal.maintenance_released && !journal.completion_acknowledged);
        assert!(!execution(&helper.state, helper.intent.operation.id)["replacement"].is_null());
        assert_eq!(
            execution(&helper.state, helper.intent.operation.id)["submitted"]
                .as_bool()
                .unwrap(),
            mode == "normal"
        );
        assert_ne!(fs::read(&reg.environment_file).unwrap(), original);
        assert_eq!(fs::read(stage.path().join("input-1")).unwrap(), original);
        assert_eq!(fs::read(&reg.compose_files[0]).unwrap(), compose);
        assert!(stage.path().join("restore-verified.json").exists());
        let evidence = fs::read(
            helper
                .state
                .path()
                .join(format!("{}.json", helper.intent.operation.id)),
        )
        .unwrap();
        assert!(
            helper
                .replace_core(&runner, &CancellationToken::new())
                .await
                .is_err()
        );
        assert_eq!(
            fs::read(
                helper
                    .state
                    .path()
                    .join(format!("{}.json", helper.intent.operation.id))
            )
            .unwrap(),
            evidence
        );
        assert_eq!(
            runner.submissions.load(Ordering::SeqCst),
            usize::from(!matches!(mode, "database-restarted" | "input-race"))
        );
    }
}

#[tokio::test]
async fn missing_or_changed_recovery_evidence_and_wrong_target_cannot_mutate_installer() {
    for mode in [
        "missing-receipt",
        "foreign-receipt",
        "changed-dump",
        "sandbox-present",
        "wrong-image",
        "cancel-before",
        "changed-inputs",
        "existing-intent",
    ] {
        let (runner, helper, stage) = ready(mode).await;
        let token = CancellationToken::new();
        match mode {
            "missing-receipt" => {
                fs::remove_file(stage.path().join("restore-verified.json")).unwrap()
            }
            "foreign-receipt" => {
                let mut receipt: RestoredUpdateCheckpoint =
                    stage.read("restore-verified.json").unwrap();
                receipt.fence = Uuid::now_v7();
                stage.write("restore-verified.json", &receipt).unwrap();
            }
            "changed-dump" => {
                let checkpoint: PreparedUpdateCheckpoint = stage.read("checkpoint.json").unwrap();
                write(&checkpoint.bundle.join("database/citadel.dump"), b"changed");
            }
            "cancel-before" => token.cancel(),
            "changed-inputs" => fs::write(
                &helper.intent.registration.environment_file,
                "PG_PASSWORD=changed\n",
            )
            .unwrap(),
            "existing-intent" => edit_execution(&helper.state, helper.intent.operation.id, |v| {
                v["replacement"] = json!({"retained":"ambiguous"});
            }),
            _ => (),
        }
        let before = fs::read(&helper.intent.registration.environment_file).unwrap();
        let error: UpdateError = helper.replace_core(&runner, &token).await.unwrap_err();
        assert!(!error.to_string().contains("private-test-password"));
        assert_eq!(
            fs::read(&helper.intent.registration.environment_file).unwrap(),
            before,
            "{mode}"
        );
        assert_eq!(runner.submissions.load(Ordering::SeqCst), 0);
        assert_eq!(
            execution(&helper.state, helper.intent.operation.id)["journal"]["operation"]["phase"],
            json!(UpdatePhase::BackingUp)
        );
        assert_eq!(
            !execution(&helper.state, helper.intent.operation.id)["replacement"].is_null(),
            mode == "existing-intent"
        );
    }
}
