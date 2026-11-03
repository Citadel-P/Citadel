use super::super::docker::SOCKET;
use super::super::{InstallerComposeInputs, JournalDirectory, RegisteredComposeInstallation};
use super::registration::REGISTRATION;
use super::*;
use citadel_execution::ProcessOutput;
use citadel_execution::{ProcessError, ProcessRequest, ProcessRunner};
use citadel_updates::UpdateError;
use futures_util::future::BoxFuture;
use serde_json::{Value, json};
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub(in crate::external::updates) const CORE: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub(in crate::external::updates) const DATABASE: &str =
    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
pub(in crate::external::updates) const NETWORK: &str =
    "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
pub(in crate::external::updates) const IMAGE: &str = "ghcr.io/citadel-p/citadel:v0.1.1";
pub(in crate::external::updates) const INSTANCE: &str = "01900000-0000-7000-8000-000000000001";

pub(in crate::external::updates) struct Fixture {
    pub(in crate::external::updates) _root: tempfile::TempDir,
    pub(in crate::external::updates) inputs: InstallerComposeInputs,
    pub(in crate::external::updates) state: PathBuf,
    pub(in crate::external::updates) runner: Runner,
}
impl Fixture {
    pub(in crate::external::updates) fn new() -> Self {
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
                "State":{"Running":true,"Paused":false,"Restarting":false,"Health":{"Status":"healthy"},"StartedAt":"2026-11-03T00:00:00Z"}, "RestartCount":0,
                "Config":{"Hostname": &id[..12],"Image":service["image"],"Env": service["environment"].as_object().unwrap().iter().map(|(k,v)|format!("{k}={}",v.as_str().unwrap())).collect::<Vec<_>>(),
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
    pub(in crate::external::updates) async fn register(
        &self,
    ) -> Result<RegisteredComposeInstallation, UpdateError> {
        register_compose_installation(
            &self.inputs,
            self.state.clone(),
            &self.runner,
            &CancellationToken::new(),
        )
        .await
    }
    pub(in crate::external::updates) async fn check(
        &self,
    ) -> Result<CheckedUpdateInstallation, UpdateError> {
        check_registered_compose_installation(&self.state, &self.runner, &CancellationToken::new())
            .await
    }
}

pub(in crate::external::updates) struct Runner {
    pub(in crate::external::updates) config: Value,
    pub(in crate::external::updates) core: Value,
    pub(in crate::external::updates) database: Value,
    pub(in crate::external::updates) inputs_directory: PathBuf,
    pub(in crate::external::updates) state: PathBuf,
    pub(in crate::external::updates) mode: &'static str,
    pub(in crate::external::updates) calls: AtomicUsize,
    pub(in crate::external::updates) observations: AtomicUsize,
    pub(in crate::external::updates) homes: Mutex<Vec<PathBuf>>,
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
                [
                    "exec",
                    id,
                    "/app/citadel-server",
                    "instance-identity-json",
                    tail @ ..,
                ] => {
                    assert_eq!(*id, CORE);
                    if self.mode == "cancel" {
                        cancellation.cancel();
                        return Err(ProcessError::Cancelled);
                    }
                    if self.mode == "changed-inputs" {
                        fs::write(self.inputs_directory.join(".env"), "PG_PASSWORD=changed\n")
                            .unwrap();
                    }
                    if self.mode == "evidence-during-check" {
                        fs::write(self.state.join("recovery.json"), b"preserve recovery").unwrap();
                    }
                    if self.mode == "registration-during-check" {
                        let state = JournalDirectory::open(self.state.clone()).unwrap();
                        let mut registration: RegisteredComposeInstallation =
                            state.read(REGISTRATION).unwrap();
                        registration.instance_id = Uuid::now_v7();
                        state.write(REGISTRATION, &registration).unwrap();
                    }
                    let instance = match self.mode {
                        "missing-identity" => "invalid",
                        "different-instance" => "01900000-0000-7000-8000-000000000099",
                        _ => INSTANCE,
                    };
                    if tail.is_empty() {
                        json!({"instanceId":instance})
                    } else {
                        assert_eq!(tail, &["--verify-migrations"]);
                        if self.mode == "invalid-catalog" {
                            return Err(ProcessError::Spawn(std::io::Error::other(
                                "private detail",
                            )));
                        }
                        json!({"instanceId": instance, "catalogSha256": "a".repeat(64), "databaseBytes": 1024})
                    }
                }
                [
                    "exec",
                    "--user",
                    "65532:0",
                    id,
                    "/app/citadel-server",
                    "check-update-recovery-keys",
                ] => {
                    assert_eq!(*id, CORE);
                    if self.mode == "recovery-key-failure" {
                        return Err(ProcessError::Spawn(std::io::Error::other(
                            "private key detail",
                        )));
                    }
                    if self.mode == "key-cancel" {
                        cancellation.cancel();
                        return Err(ProcessError::Cancelled);
                    }
                    let source = |setting| {
                        if self.config["services"]["server"]["environment"][setting]
                            .as_str()
                            .is_some_and(|v| !v.trim().is_empty())
                        {
                            "Environment"
                        } else {
                            "DataVolume"
                        }
                    };
                    json!({
                        "jwt": if self.mode == "unregistered-key" { "Environment" } else { source("CITADEL_JWT_KEY") },
                        "secretsEncryption": source("CITADEL_SECRETS_ENCRYPTION_KEY"),
                        "agentSigning": if self.mode == "invalid-key-report" { "Unknown" } else if self.mode == "external-agent-key" { "Environment" } else { "DataVolume" }
                    })
                }
                _ => panic!("unexpected Docker operation: {args:?}"),
            };
            output.stdout = serde_json::to_vec(&value).unwrap();
            Ok(output)
        })
    }
}
