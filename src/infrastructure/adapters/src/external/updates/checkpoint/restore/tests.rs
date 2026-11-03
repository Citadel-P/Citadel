use super::super::super::{JournalDirectory, RegisteredComposeInstallation};
use super::super::{CheckpointInstallerInput, PreparedUpdateCheckpoint, tests::Fixture};
use super::sandbox::Sandbox;
use super::*;
use crate::security::identity::crypto::AesGcmSecretProtector;
use base64::{Engine, engine::general_purpose::STANDARD};
use citadel_execution::{ProcessError, ProcessOutput, ProcessRequest, ProcessRunner};
use citadel_updates::UpdateError;
use futures_util::future::BoxFuture;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const ID: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const IMAGE: &str = "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
}

pub(super) fn fixture() -> Fixture {
    let mut f = Fixture::new(Uuid::now_v7());
    let catalog = citadel_database::MigrationRunner::catalog_sha256().unwrap();
    f.operation.plan.source.catalog_sha256 = catalog.clone();
    let mut journal = f.state.journal(f.operation.id).unwrap();
    journal.operation = f.operation.clone();
    for entry in &mut journal.history {
        entry.plan.source.catalog_sha256 = catalog.clone();
    }
    f.state.save(&journal).unwrap();
    fs::create_dir(f.stage()).unwrap();
    fs::set_permissions(f.stage(), fs::Permissions::from_mode(0o700)).unwrap();
    let stage = JournalDirectory::open(f.stage()).unwrap();
    let reg: RegisteredComposeInstallation = f.state.read("installation.json").unwrap();
    stage.create("operation.json", &f.operation).unwrap();
    stage.create("installation.json", &reg).unwrap();
    let mut inputs = vec![];
    for (index, path) in reg
        .compose_files
        .iter()
        .chain([&reg.environment_file])
        .enumerate()
    {
        let bytes = fs::read(path).unwrap();
        let name = format!("input-{index}");
        write(&f.stage().join(&name), &bytes);
        inputs.push(CheckpointInstallerInput {
            original_path: path.clone(),
            saved_name: name,
            sha256: hash(&bytes),
        });
    }
    let bundle = f.stage().join(f.operation.id.simple().to_string());
    fs::create_dir(&bundle).unwrap();
    fs::set_permissions(&bundle, fs::Permissions::from_mode(0o700)).unwrap();
    stage
        .create(
            "checkpoint.json",
            &PreparedUpdateCheckpoint {
                format: 1,
                operation_id: f.operation.id,
                fence: f.operation.fence,
                instance_id: f.operation.instance_id,
                source: f.operation.plan.source.clone(),
                bundle,
                bundle_checksums_sha256: String::new(),
                configuration_sha256: reg.configuration_sha256,
                installer_inputs: inputs,
                required_external_configuration: vec![],
            },
        )
        .unwrap();
    let receipt: PreparedUpdateCheckpoint = stage.read("checkpoint.json").unwrap();
    write(&receipt.bundle.join("recovery/jwtsecret"), &[b'j'; 32]);
    write(
        &receipt.bundle.join("recovery/secret-encryption-key"),
        STANDARD.encode([7; 32]).as_bytes(),
    );
    write(&receipt.bundle.join("recovery/agent/signing-key"), &[8; 32]);
    set_dump(&f, b"PGDMP-fixture");
    f
}

pub(super) fn set_dump(f: &Fixture, dump: &[u8]) {
    let stage = JournalDirectory::open(f.stage()).unwrap();
    let mut receipt: PreparedUpdateCheckpoint = stage.read("checkpoint.json").unwrap();
    write(&receipt.bundle.join("database/citadel.dump"), dump);
    let mut inventory = serde_json::Map::new();
    let mut recovery_files = vec![];
    for name in [
        "recovery/jwtsecret",
        "recovery/secret-encryption-key",
        "recovery/agent/signing-key",
    ] {
        if let Ok(bytes) = fs::read(receipt.bundle.join(name)) {
            inventory.insert(name.into(), json!(hash(&bytes)));
            recovery_files.push(name);
        }
    }
    let manifest = serde_json::to_vec(&json!({"formatVersion":1,"product":"citadel",
        "coreVersion":receipt.source.version,"instanceId":receipt.instance_id,"createdAt":chrono::Utc::now(),
        "database":{"engine":"PostgreSQL","path":"database/citadel.dump","sha256":hash(dump)},
        "requiredExternalConfiguration":receipt.required_external_configuration, "recoveryFiles":recovery_files})).unwrap();
    write(&receipt.bundle.join("manifest.json"), &manifest);
    inventory.insert("manifest.json".into(), json!(hash(&manifest)));
    inventory.insert("database/citadel.dump".into(), json!(hash(dump)));
    let checksums = serde_json::to_vec(&inventory).unwrap();
    write(&receipt.bundle.join("checksums.json"), &checksums);
    receipt.bundle_checksums_sha256 = hash(&checksums);
    stage.write("checkpoint.json", &receipt).unwrap();
}

pub(super) fn probe(f: &Fixture) -> Value {
    let manifest: Value = serde_json::from_str(citadel_database::MIGRATION_MANIFEST_JSON).unwrap();
    let original = f.state.journal(f.operation.id).unwrap().history.remove(0);
    json!({"instance":f.operation.instance_id,"initialized":true,
        "migrations":manifest["migrations"].as_array().unwrap().iter().map(|m| json!({"id":m["id"],"checksum":m["sha256"],"completed":true,"failed":false})).collect::<Vec<_>>(),
        "owners":[{"id":original.id,"instance":original.instance_id,"actor":original.actor_id.value(),"key":original.idempotency_key,"revision":original.revision,"operation":original}]})
}

struct Runner {
    f: Fixture,
    mode: &'static str,
    sandbox: Mutex<Option<Sandbox>>,
    creates: AtomicUsize,
    starts: AtomicUsize,
    removals: AtomicUsize,
    secret_pages: AtomicUsize,
}
impl Runner {
    fn new(mode: &'static str) -> Self {
        let f = fixture();
        if mode.starts_with("external") {
            let stage = JournalDirectory::open(f.stage()).unwrap();
            let mut receipt: PreparedUpdateCheckpoint = stage.read("checkpoint.json").unwrap();
            receipt.required_external_configuration = vec![
                "CITADEL_JWT_KEY".into(),
                "CITADEL_SECRETS_ENCRYPTION_KEY".into(),
            ];
            for name in ["jwtsecret", "secret-encryption-key"] {
                fs::remove_file(receipt.bundle.join("recovery").join(name)).unwrap();
            }
            stage.write("checkpoint.json", &receipt).unwrap();
            set_dump(&f, b"PGDMP-fixture");
        }
        Self {
            f,
            mode,
            sandbox: Mutex::new(None),
            creates: AtomicUsize::new(0),
            starts: AtomicUsize::new(0),
            removals: AtomicUsize::new(0),
            secret_pages: AtomicUsize::new(0),
        }
    }
    async fn verify(
        &self,
        token: &CancellationToken,
    ) -> Result<RestoredUpdateCheckpoint, UpdateError> {
        verify_update_checkpoint(
            &self.f.state,
            &self.f.owner,
            &self.f.operation,
            self,
            &CheckpointRestoreBudget {
                timeout: Duration::from_secs(30),
                maximum_bytes: 128 * 1024 * 1024,
            },
            token,
        )
        .await
    }
    fn observed(&self) -> Value {
        let guard = self.sandbox.lock().unwrap();
        let s = guard.as_ref().unwrap();
        json!({"Id":ID,"Name":format!("/{}",s.name()),"Image":IMAGE,
            "Config":{"Image":IMAGE,"User":"0:0","Labels":{"com.citadel.checkpoint-restore":s.nonce.to_string()}},
            "State":{"Status":if self.starts.load(Ordering::SeqCst)>0 {"running"}else{"created"},"Running":self.starts.load(Ordering::SeqCst)>0,"Paused":false,"Restarting":false,"OOMKilled":false},
            "RestartCount":0,"Path":"/usr/local/bin/docker-entrypoint.sh","Args":["postgres","-c","listen_addresses=","-c","unix_socket_directories=/var/run/postgresql"],
            "Mounts":[{"Type":"bind","Source":s.dump,"Destination":"/checkpoint.dump","RW":false}],
            "NetworkSettings":{"Networks":{"none":{}}},
            "HostConfig":{"NetworkMode":"none","PublishAllPorts":false,"PortBindings":{},"Memory":s.maximum_bytes+256*1024*1024,"MemorySwap":s.maximum_bytes+256*1024*1024,"NanoCpus":1000000000,"PidsLimit":128,"Privileged":false,"ReadonlyRootfs":true,"AutoRemove":false,"RestartPolicy":{"Name":"no"},
                "Tmpfs":{"/var/lib/postgresql":format!("rw,size={}",s.maximum_bytes),"/var/run/postgresql":"rw,size=16777216"}}})
    }
}
fn output(bytes: Vec<u8>) -> ProcessOutput {
    ProcessOutput {
        exit_code: Some(0),
        stdout: bytes,
        stderr: vec![],
        stdout_truncated: false,
        stderr_truncated: false,
    }
}
impl ProcessRunner for Runner {
    fn run<'a>(
        &'a self,
        request: ProcessRequest,
        token: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ProcessOutput, ProcessError>> {
        Box::pin(async move {
            let args: Vec<_> = request.arguments[2..]
                .iter()
                .map(|v| v.to_str().unwrap())
                .collect();
            // No recovered key may be passed through process argv/environment/stdin.
            let key = STANDARD.encode([7; 32]);
            assert!(
                !request
                    .arguments
                    .iter()
                    .any(|s| s.to_string_lossy().contains(&key))
            );
            assert!(
                !request
                    .environment
                    .iter()
                    .any(|(_, s)| s.to_string_lossy().contains(&key))
            );
            assert!(
                request
                    .stdin
                    .as_ref()
                    .is_none_or(|s| !String::from_utf8_lossy(s).contains(&key))
            );
            let value = match args.as_slice() {
                ["compose", ..] => {
                    let fixture = super::super::super::bootstrap::fixtures::Fixture::new();
                    let mut config = fixture.runner.config;
                    config["name"] = json!("test-checkpoint");
                    config["services"]["server"]["environment"]["CITADEL_JWT_KEY"] =
                        json!("j".repeat(32));
                    if self.mode != "external-missing" {
                        config["services"]["server"]["environment"]["CITADEL_SECRETS_ENCRYPTION_KEY"] = json!(
                            STANDARD
                                .encode([if self.mode == "external-wrong" { 9 } else { 7 }; 32])
                        );
                    }
                    config
                }
                ["container", "inspect", ID] => {
                    let mut value = self.observed();
                    if self.mode == "foreign" {
                        value["Config"]["Labels"]["com.citadel.checkpoint-restore"] =
                            json!("foreign");
                    }
                    json!([value])
                }
                ["container", "inspect", id] => json!([{"Id":id,"Image":IMAGE}]),
                ["image", "inspect", IMAGE] => {
                    json!([{"Id":IMAGE,"Os":"linux","Architecture":"amd64","Config":{"Volumes":{"/var/lib/postgresql":{}}}}])
                }
                ["container", "create", rest @ ..] => {
                    let stage = JournalDirectory::open(self.f.stage()).unwrap();
                    *self.sandbox.lock().unwrap() =
                        Some(stage.read("restore-intent.json").unwrap());
                    assert_eq!(self.creates.fetch_add(1, Ordering::SeqCst), 0);
                    for arg in ["--network=none", "--read-only", "--pull=never"] {
                        assert!(rest.contains(&arg));
                    }
                    assert_eq!(rest.iter().filter(|a| **a == "--mount").count(), 1);
                    if self.mode == "create-timeout" {
                        return Err(ProcessError::Timeout(Duration::from_secs(15)));
                    }
                    return Ok(output(ID.as_bytes().to_vec()));
                }
                ["container", "start", ID] => {
                    self.starts.fetch_add(1, Ordering::SeqCst);
                    json!(ID)
                }
                ["exec", ID, "cat", _] => return Ok(output(b"postgres\n".to_vec())),
                ["exec", ID, "pg_isready", ..] => return Ok(output(vec![])),
                ["exec", ID, "pg_restore", ..] => {
                    if self.mode == "cancel" {
                        token.cancel();
                        return Err(ProcessError::Cancelled);
                    }
                    if self.mode == "restore-failure" {
                        let mut o = output(vec![]);
                        o.exit_code = Some(1);
                        o.stderr = b"private-checkpoint-fixture".to_vec();
                        return Ok(o);
                    }
                    return Ok(output(vec![]));
                }
                ["exec", ID, "psql", rest @ ..]
                    if rest.last().is_some_and(|s| s.contains("WITH envelopes")) =>
                {
                    let page = self.secret_pages.fetch_add(1, Ordering::SeqCst);
                    let protector = AesGcmSecretProtector::new(
                        &[if self.mode == "wrong-key" { 9 } else { 7 }; 32],
                    )
                    .unwrap();
                    let envelope = protector.protect(b"private-checkpoint-fixture").unwrap();
                    match self.mode {
                        "null-envelope" => {
                            json!([{"kind":1,"id":Uuid::from_u128(1),"envelope":null}])
                        }
                        "malformed-envelope" => {
                            json!([{"kind":1,"id":Uuid::from_u128(1),"envelope":"bad"}])
                        }
                        "empty-secrets" => json!([]),
                        "maximum-secrets" => {
                            if page == 0 {
                                let value = protector.protect(&vec![b's'; 256 * 1024]).unwrap();
                                let page = json!([{"kind":1,"id":Uuid::from_u128(1),"envelope":value},{"kind":1,"id":Uuid::from_u128(2),"envelope":value}]);
                                assert!(
                                    serde_json::to_vec(&page).unwrap().len()
                                        < request.limits.maximum_stdout_bytes
                                );
                                page
                            } else {
                                json!([])
                            }
                        }
                        "pagination" | "repeated-page" => {
                            let start = if self.mode == "repeated-page" {
                                0
                            } else {
                                page * 2
                            };
                            if start >= 32 {
                                json!([])
                            } else {
                                json!((start..start+2).map(|n| json!({"kind":1,"id":Uuid::from_u128(n as u128+1),"envelope":envelope})).collect::<Vec<_>>())
                            }
                        }
                        _ => json!([{"kind":1,"id":Uuid::from_u128(1),"envelope":envelope}]),
                    }
                }
                ["exec", ID, "psql", ..] => {
                    let mut p = probe(&self.f);
                    match self.mode {
                        "foreign-instance" => p["instance"] = json!(Uuid::now_v7()),
                        "migration" => p["migrations"][0]["checksum"] = json!("wrong"),
                        "owner" => p["owners"][0]["operation"]["fence"] = json!(Uuid::now_v7()),
                        "changed-input" => {
                            fs::write(self.f.stage().join("input-0"), b"changed").unwrap()
                        }
                        _ => (),
                    }
                    p
                }
                ["container", "ls", ..] => {
                    return Ok(output(
                        if self.creates.load(Ordering::SeqCst) > 0
                            && self.removals.load(Ordering::SeqCst) == 0
                        {
                            format!("\"{ID}\"\n").into_bytes()
                        } else {
                            vec![]
                        },
                    ));
                }
                ["container", "rm", "--force", ID] => {
                    assert!(!token.is_cancelled());
                    if self.mode == "cleanup-failure" {
                        return Err(ProcessError::Timeout(Duration::from_secs(30)));
                    }
                    self.removals.fetch_add(1, Ordering::SeqCst);
                    json!(ID)
                }
                _ => panic!("Unexpected Docker test command"),
            };
            Ok(output(serde_json::to_vec(&value).unwrap()))
        })
    }
}

#[tokio::test]
async fn publishes_restore_proof_only_after_cleanup_and_preserves_journal() {
    let runner = Runner::new("normal");
    let token = CancellationToken::new();
    let before = runner.f.state.journal(runner.f.operation.id).unwrap();
    let receipt = runner.verify(&token).await.unwrap();
    assert_eq!(runner.removals.load(Ordering::SeqCst), 1);
    assert_eq!(receipt.instance_id, runner.f.operation.instance_id);
    assert_eq!(receipt.decrypted_values, 1);
    assert_eq!(
        runner.f.state.journal(runner.f.operation.id).unwrap(),
        before
    );
    assert!(runner.f.state.existing_lock().is_err());
    assert!(runner.verify(&token).await.is_err());
    assert_eq!(runner.creates.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn failed_interrupted_or_untrusted_restore_never_publishes_success() {
    for mode in [
        "create-timeout",
        "cancel",
        "restore-failure",
        "foreign-instance",
        "migration",
        "owner",
        "changed-input",
        "foreign",
        "cleanup-failure",
        "wrong-key",
        "null-envelope",
        "malformed-envelope",
        "repeated-page",
    ] {
        let runner = Runner::new(mode);
        let error = runner
            .verify(&CancellationToken::new())
            .await
            .err()
            .unwrap();
        assert!(!error.to_string().contains("private-checkpoint-fixture"));
        assert!(
            !runner.f.stage().join("restore-verified.json").exists(),
            "{mode}"
        );
        assert!(runner.f.stage().join("restore-intent.json").exists());
        assert_eq!(
            runner.removals.load(Ordering::SeqCst),
            if ["foreign", "cleanup-failure"].contains(&mode) {
                0
            } else {
                1
            },
            "{mode}"
        );
    }
}

#[tokio::test]
async fn rejects_changed_checkpoint_and_cancellation_before_docker() {
    for mode in ["dump", "input", "receipt", "cancel"] {
        let runner = Runner::new("normal");
        let token = CancellationToken::new();
        match mode {
            "dump" => set_dump_corrupt(&runner.f),
            "input" => fs::write(runner.f.stage().join("input-1"), b"changed").unwrap(),
            "receipt" => {
                let stage = JournalDirectory::open(runner.f.stage()).unwrap();
                let mut r: PreparedUpdateCheckpoint = stage.read("checkpoint.json").unwrap();
                r.fence = Uuid::now_v7();
                stage.write("checkpoint.json", &r).unwrap();
            }
            _ => token.cancel(),
        }
        assert!(runner.verify(&token).await.is_err());
        assert_eq!(runner.creates.load(Ordering::SeqCst), 0);
    }
}
fn set_dump_corrupt(f: &Fixture) {
    fs::write(
        f.stage()
            .join(f.operation.id.simple().to_string())
            .join("database/citadel.dump"),
        b"PGDMP-corrupt",
    )
    .unwrap();
}

#[tokio::test]
async fn verifies_every_page_and_records_empty_encrypted_state_explicitly() {
    for (mode, expected, pages) in [
        ("pagination", 32, 17),
        ("empty-secrets", 0, 1),
        ("maximum-secrets", 2, 2),
    ] {
        let runner = Runner::new(mode);
        let receipt = runner.verify(&CancellationToken::new()).await.unwrap();
        assert_eq!(receipt.decrypted_values, expected);
        assert_eq!(runner.secret_pages.load(Ordering::SeqCst), pages);
        let serialized = serde_json::to_string(&receipt).unwrap();
        assert!(!serialized.contains("private-checkpoint-fixture"));
    }
}

#[tokio::test]
async fn malformed_or_unavailable_recovery_keys_fail_before_creating_a_container() {
    for mode in ["jwt", "signing", "encryption", "missing", "unsafe"] {
        let runner = Runner::new("normal");
        let stage = JournalDirectory::open(runner.f.stage()).unwrap();
        let receipt: PreparedUpdateCheckpoint = stage.read("checkpoint.json").unwrap();
        let path = receipt.bundle.join(match mode {
            "jwt" => "recovery/jwtsecret",
            "signing" => "recovery/agent/signing-key",
            _ => "recovery/secret-encryption-key",
        });
        if mode == "missing" {
            fs::remove_file(&path).unwrap();
        } else {
            write(&path, b"invalid");
        }
        set_dump(&runner.f, b"PGDMP-fixture");
        if mode == "unsafe" {
            fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        }
        assert!(runner.verify(&CancellationToken::new()).await.is_err());
        assert_eq!(runner.creates.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn external_keys_are_resolved_from_approved_compose_inputs_and_not_sent_to_postgres() {
    let normal = Runner::new("external");
    let receipt = normal.verify(&CancellationToken::new()).await.unwrap();
    assert_eq!(receipt.decrypted_values, 1);
    let wrong = Runner::new("external-wrong");
    assert!(
        wrong
            .verify(&CancellationToken::new())
            .await
            .unwrap_err()
            .to_string()
            .contains("cannot decrypt")
    );
    assert_eq!(wrong.removals.load(Ordering::SeqCst), 1);
    let missing = Runner::new("external-missing");
    assert!(missing.verify(&CancellationToken::new()).await.is_err());
    assert_eq!(missing.creates.load(Ordering::SeqCst), 0);
}
