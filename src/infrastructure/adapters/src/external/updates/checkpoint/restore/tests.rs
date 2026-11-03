use super::super::super::{JournalDirectory, RegisteredComposeInstallation};
use super::super::{CheckpointInstallerInput, PreparedUpdateCheckpoint, tests::Fixture};
use super::sandbox::Sandbox;
use super::*;
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
                required_external_configuration: vec![
                    "CITADEL_JWT_KEY".into(),
                    "CITADEL_SECRETS_ENCRYPTION_KEY".into(),
                ],
            },
        )
        .unwrap();
    set_dump(&f, b"PGDMP-fixture");
    f
}

pub(super) fn set_dump(f: &Fixture, dump: &[u8]) {
    let stage = JournalDirectory::open(f.stage()).unwrap();
    let mut receipt: PreparedUpdateCheckpoint = stage.read("checkpoint.json").unwrap();
    write(&receipt.bundle.join("database/citadel.dump"), dump);
    let manifest = serde_json::to_vec(&json!({"formatVersion":1,"product":"citadel",
        "coreVersion":receipt.source.version,"instanceId":receipt.instance_id,"createdAt":chrono::Utc::now(),
        "database":{"engine":"PostgreSQL","path":"database/citadel.dump","sha256":hash(dump)},
        "requiredExternalConfiguration":receipt.required_external_configuration})).unwrap();
    write(&receipt.bundle.join("manifest.json"), &manifest);
    let checksums = serde_json::to_vec(
        &json!({"manifest.json":hash(&manifest),"database/citadel.dump":hash(dump)}),
    )
    .unwrap();
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
}
impl Runner {
    fn new(mode: &'static str) -> Self {
        Self {
            f: fixture(),
            mode,
            sandbox: Mutex::new(None),
            creates: AtomicUsize::new(0),
            starts: AtomicUsize::new(0),
            removals: AtomicUsize::new(0),
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
            let value = match args.as_slice() {
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
