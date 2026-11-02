use super::*;
use citadel_execution::ProcessOutput;
use futures_util::future::BoxFuture;
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

const CORE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const DATABASE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const NETWORK: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const IMAGE: &str = "ghcr.io/citadel-p/citadel:v0.1.1";
const INSTANCE: &str = "01900000-0000-7000-8000-000000000001";

#[tokio::test]
async fn startup_observes_registration_without_repairing_or_holding_the_helper_lock() {
    use crate::external::updates::check_startup_registration;
    let fixture = Fixture::new();
    // Explicitly configured empty/missing state must not become a new registration.
    assert!(check_startup_registration(&fixture.state).is_err());
    assert_eq!(fs::read_dir(&fixture.state).unwrap().count(), 0);
    assert!(check_startup_registration(&fixture.state.join("missing")).is_err());
    let registration = fixture.register().await.unwrap();
    assert_eq!(
        check_startup_registration(&fixture.state).unwrap(),
        registration
    );
    let state = JournalDirectory::open(fixture.state.clone()).unwrap();
    let lock = state.lock().unwrap();
    assert!(check_startup_registration(&fixture.state).is_err());
    drop(lock);
    assert_eq!(
        check_startup_registration(&fixture.state).unwrap(),
        registration
    );
    // Startup does not retain ownership while waiting for any future helper.
    drop(state.lock().unwrap());
    fs::remove_file(fixture.state.join("installation.lock")).unwrap();
    assert!(check_startup_registration(&fixture.state).is_err());
    assert!(!fixture.state.join("installation.lock").exists());
}

#[tokio::test]
async fn startup_never_trusts_completed_or_orphaned_execution_evidence() {
    use crate::external::updates::check_startup_registration;
    for name in [
        "maintenance.json",
        "01900000-0000-7000-8000-000000000002.json",
        ".interrupted.tmp",
    ] {
        let fixture = Fixture::new();
        fixture.register().await.unwrap();
        let evidence =
            br#"{"phase":"Succeeded","maintenanceReleased":true,"completionAcknowledged":true}"#;
        fs::write(fixture.state.join(name), evidence).unwrap();
        assert!(
            check_startup_registration(&fixture.state).is_err(),
            "{name}"
        );
        assert_eq!(fs::read(fixture.state.join(name)).unwrap(), evidence);
    }
}

#[tokio::test]
async fn startup_rejects_corrupt_foreign_or_unprotected_registration() {
    use crate::external::updates::check_startup_registration;
    for mutation in 0..6 {
        let fixture = Fixture::new();
        let mut registration = fixture.register().await.unwrap();
        let state = JournalDirectory::open(fixture.state.clone()).unwrap();
        match mutation {
            0 => fs::write(fixture.state.join(REGISTRATION), b"corrupt").unwrap(),
            1 => {
                registration.state_directory = PathBuf::from("/another-state-directory");
                state.write(REGISTRATION, &registration).unwrap();
            }
            2 => {
                registration.executor_uid = registration.executor_uid.wrapping_add(1);
                state.write(REGISTRATION, &registration).unwrap();
            }
            3 => fs::set_permissions(
                fixture.state.join(REGISTRATION),
                fs::Permissions::from_mode(0o644),
            )
            .unwrap(),
            4 => {
                fs::rename(
                    fixture.state.join(REGISTRATION),
                    fixture.state.join("original.json"),
                )
                .unwrap();
                std::os::unix::fs::symlink("original.json", fixture.state.join(REGISTRATION))
                    .unwrap();
            }
            _ => fs::set_permissions(&fixture.state, fs::Permissions::from_mode(0o755)).unwrap(),
        }
        let before = fs::read(fixture.state.join(REGISTRATION)).unwrap();
        assert!(
            check_startup_registration(&fixture.state).is_err(),
            "{mutation}"
        );
        assert_eq!(fs::read(fixture.state.join(REGISTRATION)).unwrap(), before);
    }
}

#[test]
fn containerized_registration_requires_its_own_identity_and_exact_persistent_mounts() {
    let fixture = Fixture::new();
    let executor = json!({"Id": CORE, "Config":{"Hostname": &CORE[..12]}, "Mounts":[
        {"Type":"bind","Source":fixture.inputs.directory,"Destination":fixture.inputs.directory,"RW":false},
        {"Type":"bind","Source":fixture.state,"Destination":fixture.state,"RW":true},
        {"Type":"bind","Source":SOCKET,"Destination":SOCKET,"RW":true}
    ]});
    validate_executor(&executor, &CORE[..12], &fixture.inputs, &fixture.state).unwrap();
    for mutation in 0..6 {
        let mut changed = executor.clone();
        match mutation {
            0 => changed["Id"] = json!(DATABASE),
            1 => changed["Mounts"][0]["RW"] = json!(true),
            2 => {
                changed["Mounts"].as_array_mut().unwrap().remove(1);
            }
            3 => changed["Mounts"][1]["Source"] = json!("/different-host-directory"),
            4 => changed["Mounts"][1]["Type"] = json!("volume"),
            _ => changed["Mounts"]
                .as_array_mut()
                .unwrap()
                .push(json!({"Type":"bind","Source":"/","Destination":"/host","RW":true})),
        }
        assert!(validate_executor(&changed, &CORE[..12], &fixture.inputs, &fixture.state).is_err());
    }
}

struct Fixture {
    _root: tempfile::TempDir,
    inputs: InstallerComposeInputs,
    state: PathBuf,
    runner: Runner,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::Builder::new()
            .prefix("citadel-register-test-")
            .permissions(fs::Permissions::from_mode(0o700))
            .tempdir()
            .unwrap();
        let directory = root.path().join("installer");
        let state = root.path().join("state");
        for path in [&directory, &state] {
            fs::create_dir(path).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let base = directory.join("docker-compose.yml");
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../deploy/install/docker-compose.yml"),
            &base,
        )
        .unwrap();
        fs::write(
            directory.join(".env"),
            "PG_PASSWORD=private-test-password\nCITADEL_IMAGE=ghcr.io/citadel-p/citadel:v0.1.1\n",
        )
        .unwrap();
        fs::set_permissions(directory.join(".env"), fs::Permissions::from_mode(0o600)).unwrap();
        let inputs = InstallerComposeInputs {
            directory,
            project: "citadel-test".into(),
            compose_files: vec![base],
        };
        let mut config = json!({
            "name": "citadel-test", "services": {
                "server": {"image": IMAGE, "environment": {"PG_HOST":"pg_db", "PG_PORT":"5432", "PG_USER":"citadel", "PG_PASSWORD":"private-test-password", "PG_DATABASE":"citadel"}, "ports":[{"target":8000,"published":"18000","host_ip":"127.0.0.1","protocol":"tcp"}]},
                "pg_db": {"image":"postgres@sha256:fixture", "environment":{"POSTGRES_USER":"citadel","POSTGRES_PASSWORD":"private-test-password","POSTGRES_DB":"citadel"}}
            },
            "volumes":{"citadel_data":{"name":"citadel-test_citadel_data"},"postgres_data":{"name":"citadel-test_postgres_data"}},
            "networks":{"default":{"name":"citadel-test_default"}}
        });
        super::super::compose::tests::profile_fixture(&mut config);
        let container = |role: &str, id: &str| {
            let service = &config["services"][role];
            let mounts = if role == "server" {
                json!([
                    {"Type":"volume","Name":"citadel-test_citadel_data","Destination":"/app/data","RW":true},
                    {"Type":"bind","Source":SOCKET,"Destination":SOCKET,"RW":true},
                    {"Type":"bind","Source":"/","Destination":"/host","RW":false}
                ])
            } else {
                json!([{"Type":"volume","Name":"citadel-test_postgres_data","Destination":"/var/lib/postgresql","RW":true}])
            };
            json!({"Id":id,"Image":format!("sha256:{}", "d".repeat(64)),
                "State":{"Running":true,"Paused":false,"Restarting":false,"Health":{"Status":"healthy"}},
                "Config":{"Image":service["image"],"Env": service["environment"].as_object().unwrap().iter().map(|(k,v)|format!("{k}={}",v.as_str().unwrap())).collect::<Vec<_>>(),
                    "Labels":{"com.docker.compose.project":"citadel-test","com.docker.compose.service":role,"com.docker.compose.oneoff":"False","com.docker.compose.container-number":"1", "com.docker.compose.project.working_dir":inputs.directory,"com.docker.compose.project.config_files":inputs.compose_files[0]}},
                "HostConfig":{"Privileged":false,"NetworkMode":"citadel-test_default","RestartPolicy":{"Name":"unless-stopped"},"PortBindings":if role=="server" {json!({"8000/tcp":[{"HostIp":"127.0.0.1","HostPort":"18000"}]})} else {json!({})}},
                "Mounts":mounts,"NetworkSettings":{"Networks":{"citadel-test_default":{"NetworkID":NETWORK,"Aliases":[role]}}}
            })
        };
        let core = container("server", CORE);
        let database = container("pg_db", DATABASE);
        let runner = Runner {
            config,
            core,
            database,
            inputs_directory: inputs.directory.clone(),
            state: state.clone(),
            mode: "normal",
            calls: AtomicUsize::new(0),
            observations: AtomicUsize::new(0),
            homes: Mutex::new(vec![]),
        };
        Self {
            _root: root,
            inputs,
            state,
            runner,
        }
    }
    async fn register(&self) -> Result<RegisteredComposeInstallation, UpdateError> {
        register_compose_installation(
            &self.inputs,
            self.state.clone(),
            &self.runner,
            &CancellationToken::new(),
        )
        .await
    }
}

struct Runner {
    config: Value,
    core: Value,
    database: Value,
    inputs_directory: PathBuf,
    state: PathBuf,
    mode: &'static str,
    calls: AtomicUsize,
    observations: AtomicUsize,
    homes: Mutex<Vec<PathBuf>>,
}
impl ProcessRunner for Runner {
    fn run<'a>(
        &'a self,
        request: ProcessRequest,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ProcessOutput, ProcessError>> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            assert_eq!(request.program, "docker");
            assert_eq!(
                &request.arguments[..2],
                ["--host", "unix:///var/run/docker.sock"]
            );
            assert!(request.output.is_none());
            assert!(request.stdin.is_none());
            assert!(request.limits.timeout <= Duration::from_secs(30));
            assert_eq!(request.limits.maximum_stdout_bytes, 1024 * 1024);
            let home = PathBuf::from(&request.environment[0].1);
            assert!(home.exists());
            self.homes.lock().unwrap().push(home);
            let args = request.arguments[2..]
                .iter()
                .map(|v| v.to_str().unwrap())
                .collect::<Vec<_>>();
            let mut output = ProcessOutput {
                exit_code: Some(0),
                stdout: vec![],
                stderr: vec![],
                stdout_truncated: false,
                stderr_truncated: false,
            };
            if self.mode == "tool-failure" {
                output.exit_code = Some(1);
                output.stderr = b"private-test-password".to_vec();
                return Ok(output);
            }
            let value = match args.as_slice() {
                ["compose", ..] => self.config.clone(),
                ["info", ..] => {
                    let n = self.observations.fetch_add(1, Ordering::SeqCst);
                    json!({"ID":if self.mode=="changed-daemon" && n%2==1 {"other"} else {"daemon"}, "OSType":"linux","DockerRootDir":"/var/lib/docker", "SecurityOptions": if self.mode=="rootless" {json!(["name=rootless"])} else {json!(["name=seccomp"])}})
                }
                ["container", "ls", ..] => {
                    let filter = args[5];
                    let ids = if filter.starts_with("label=") {
                        if self.mode == "replicas" {
                            vec![CORE, DATABASE, NETWORK]
                        } else {
                            vec![CORE, DATABASE]
                        }
                    } else if filter.ends_with("citadel_data") {
                        if self.mode == "shared-volume" {
                            vec![CORE, NETWORK]
                        } else {
                            vec![CORE]
                        }
                    } else {
                        vec![DATABASE]
                    };
                    output.stdout = ids
                        .iter()
                        .map(|id| format!("{}\n", serde_json::to_string(id).unwrap()))
                        .collect::<String>()
                        .into_bytes();
                    return Ok(output);
                }
                ["container", "inspect", id] if *id == CORE => json!([self.core]),
                ["container", "inspect", id] if *id == DATABASE => json!([self.database]),
                ["container", "inspect", hostname] => {
                    json!([{"Id":format!("{hostname}{}","0".repeat(64-hostname.len())),"Config":{"Hostname":hostname},"Mounts":[
                    {"Type":"bind","Source":self.inputs_directory,"Destination":self.inputs_directory,"RW":false},
                    {"Type":"bind","Source":self.state,"Destination":self.state,"RW":true},
                    {"Type":"bind","Source":SOCKET,"Destination":SOCKET,"RW":true}]}])
                }
                ["volume", "inspect", name] => {
                    json!([{"Name":name,"Driver":"local","Scope":"local","Options":null,"Mountpoint":format!("/var/lib/docker/volumes/{name}/_data"), "CreatedAt":"2026-11-01T00:00:00Z", "Labels":{"com.citadel.system":"true","com.citadel.system-role":if name.ends_with("citadel_data") {"core"} else {"database"}}}])
                }
                ["network", "inspect", name] => {
                    let mut members = json!({CORE: {}, DATABASE: {}});
                    if self.mode == "shared-network" {
                        members[NETWORK] = json!({});
                    }
                    json!([{"Id":NETWORK,"Name":name,"Driver":"bridge","Scope":"local","Containers":members,"Labels":{"com.docker.compose.project":"citadel-test"}}])
                }
                ["exec", id, "/app/citadel-server", "instance-identity-json"] => {
                    assert_eq!(*id, CORE);
                    if self.mode == "cancel" {
                        cancellation.cancel();
                        return Err(ProcessError::Cancelled);
                    }
                    if self.mode == "changed-inputs" {
                        fs::write(self.inputs_directory.join(".env"), "PG_PASSWORD=changed\n")
                            .unwrap();
                    }
                    json!({"instanceId":if self.mode=="missing-identity" {"invalid"} else {INSTANCE}})
                }
                _ => panic!("unexpected Docker operation: {args:?}"),
            };
            output.stdout = serde_json::to_vec(&value).unwrap();
            Ok(output)
        })
    }
}

#[tokio::test]
async fn registration_is_private_durable_idempotent_and_does_not_store_secrets() {
    let fixture = Fixture::new();
    let first = fixture.register().await.unwrap();
    let path = fixture.state.join(REGISTRATION);
    let bytes = fs::read(&path).unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("private-test-password"));
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(first.instance_id.to_string(), INSTANCE);
    assert_eq!(first.core_container_id, CORE);
    assert_eq!(first.database_container_id, DATABASE);
    assert_eq!(fixture.register().await.unwrap(), first);
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert!(
        fixture
            .runner
            .homes
            .lock()
            .unwrap()
            .iter()
            .all(|path| !path.exists())
    );
}

#[tokio::test]
async fn unsafe_ambiguous_and_changing_installations_never_write_registration() {
    for mode in [
        "replicas",
        "rootless",
        "shared-volume",
        "shared-network",
        "changed-daemon",
        "changed-inputs",
        "missing-identity",
        "cancel",
        "tool-failure",
    ] {
        let mut fixture = Fixture::new();
        fixture.runner.mode = mode;
        let error = fixture.register().await.unwrap_err();
        assert!(!error.to_string().contains("private-test-password"));
        assert!(!fixture.state.join(REGISTRATION).exists(), "{mode}");
        JournalDirectory::open(fixture.state.clone())
            .unwrap()
            .lock()
            .unwrap();
        assert!(
            fixture
                .runner
                .homes
                .lock()
                .unwrap()
                .iter()
                .all(|path| !path.exists())
        );
    }
}

#[tokio::test]
async fn different_existing_registration_and_orphaned_evidence_are_preserved() {
    let mut fixture = Fixture::new();
    fixture.register().await.unwrap();
    let original = fs::read(fixture.state.join(REGISTRATION)).unwrap();
    fixture.runner.core["Image"] = json!(format!("sha256:{}", "e".repeat(64)));
    assert!(matches!(
        fixture.register().await,
        Err(UpdateError::Conflict(_))
    ));
    assert_eq!(
        fs::read(fixture.state.join(REGISTRATION)).unwrap(),
        original
    );
    let fixture = Fixture::new();
    JournalDirectory::open(fixture.state.clone())
        .unwrap()
        .write("recovery.json", &json!({"phase":"RecoveryRequired"}))
        .unwrap();
    assert!(fixture.register().await.is_err());
    assert_eq!(fixture.runner.calls.load(Ordering::SeqCst), 0);
    assert!(fixture.state.join("recovery.json").exists());
}

#[tokio::test]
async fn lock_contention_and_corrupt_registration_fail_before_docker() {
    let fixture = Fixture::new();
    let state = JournalDirectory::open(fixture.state.clone()).unwrap();
    let lock = state.lock().unwrap();
    assert!(matches!(
        fixture.register().await,
        Err(UpdateError::Conflict(_))
    ));
    drop(lock);
    state.write(REGISTRATION, &"not a registration").unwrap();
    assert!(fixture.register().await.is_err());
    assert_eq!(fixture.runner.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn environment_ports_mounts_and_service_identity_must_match_the_invocation() {
    for mutation in 0..6 {
        let mut fixture = Fixture::new();
        match mutation {
            0 => fixture.runner.core["Config"]["Env"][0] = json!("PG_HOST=external"),
            1 => {
                fixture.runner.core["HostConfig"]["PortBindings"]["8000/tcp"][0]["HostPort"] =
                    json!("9999")
            }
            2 => fixture.runner.core["Mounts"][0]["Name"] = json!("wrong-volume"),
            3 => {
                fixture.runner.core["Config"]["Labels"]["com.docker.compose.project.working_dir"] =
                    json!("/wrong")
            }
            4 => {
                fixture.runner.core["NetworkSettings"]["Networks"]["citadel-test_default"]["NetworkID"] =
                    json!("wrong")
            }
            _ => fixture.runner.core["State"]["Health"]["Status"] = json!("unhealthy"),
        }
        assert!(fixture.register().await.is_err(), "{mutation}");
        assert!(!fixture.state.join(REGISTRATION).exists());
    }
}

#[tokio::test]
async fn unknown_installation_kinds_are_preserved_and_never_sent_to_docker() {
    let fixture = Fixture::new();
    let registration = fixture.register().await.unwrap();
    let mut value = serde_json::to_value(registration).unwrap();
    value["kind"] = json!("Kubernetes");
    let state = JournalDirectory::open(fixture.state.clone()).unwrap();
    state.write(REGISTRATION, &value).unwrap();
    let before = fs::read(fixture.state.join(REGISTRATION)).unwrap();
    let calls = fixture.runner.calls.load(Ordering::SeqCst);
    assert!(fixture.register().await.is_err());
    assert_eq!(fixture.runner.calls.load(Ordering::SeqCst), calls);
    assert_eq!(fs::read(fixture.state.join(REGISTRATION)).unwrap(), before);
}
