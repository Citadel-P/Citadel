use super::*;
use crate::external::updates::bootstrap::fixtures::{CORE, Fixture};
use citadel_execution::{ProcessError, ProcessOutput, ProcessRequest, ProcessRunner};
use futures_util::future::BoxFuture;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

struct Runner {
    fixture: Fixture,
    response: Value,
    mode: &'static str,
    utility: Mutex<Value>,
    calls: Mutex<Vec<String>>,
}
impl ProcessRunner for Runner {
    fn run<'a>(
        &'a self,
        request: ProcessRequest,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ProcessOutput, ProcessError>> {
        Box::pin(async move {
            let args: Vec<_> = request.arguments[2..]
                .iter()
                .map(|arg| arg.to_str().unwrap())
                .collect();
            self.calls.lock().unwrap().push(args.join(" "));
            let bytes = match args.as_slice() {
                ["container", "inspect", hostname] if *hostname == &CORE[..12] => {
                    let mut core = self.fixture.runner.core.clone(); core["Config"]["Hostname"] = json!(hostname);
                    serde_json::to_vec(&vec![core]).unwrap()
                }
                ["image", "inspect", _] => serde_json::to_vec(&json!([{"RepoDigests": [format!("ghcr.io/citadel-p/citadel@sha256:{}", "a".repeat(64))], "Architecture":"amd64", "Os":"linux"}])).unwrap(),
                ["container", "create", ..] => {
                    assert!(args.contains(&"--network=none")); assert!(args.contains(&"--read-only"));
                    assert!(args.contains(&"--tmpfs=/tmp:rw,noexec,nosuid,size=64m"));
                    assert!(args.contains(&"TMPDIR=/tmp"));
                    let name = args[args.iter().position(|arg| *arg == "--name").unwrap()+1];
                    let label = args[args.iter().position(|arg| *arg == "--label").unwrap()+1];
                    let nonce = label.strip_prefix("com.citadel.update-host=").unwrap();
                    let mounts: Vec<_> = [(self.fixture.inputs.directory.to_str().unwrap(), false), (self.fixture.state.to_str().unwrap(), true), ("/var/run/docker.sock", true)].into_iter().map(|(path,rw)| json!({"Type":"bind","Source":path,"Destination":path,"RW":rw})).chain([json!({"Type":"tmpfs","Destination":"/tmp","RW":true})]).collect();
                    let mut utility = json!({"Id":"e".repeat(64), "Name":format!("/{name}"), "Image":self.fixture.runner.core["Image"], "Path":"/app/citadel-server", "Args":["update-host"], "RestartCount":0,
                        "Config":{"Hostname":"e".repeat(12), "User":"0:0", "Labels":{"com.citadel.update-host":nonce}},
                        "HostConfig":{"NetworkMode":"none","ReadonlyRootfs":true,"Privileged":false,"RestartPolicy":{"Name":"no"}},
                        "State":{"Running":false,"ExitCode":0,"OOMKilled":false,"Error":""}, "Mounts":mounts});
                    if self.mode == "foreign" { utility["Config"]["Labels"]["com.citadel.update-host"] = json!("foreign"); }
                    *self.utility.lock().unwrap() = utility;
                    "e".repeat(64).into_bytes()
                }
                ["container", "inspect", id] if *id == "e".repeat(64) => serde_json::to_vec(&vec![self.utility.lock().unwrap().clone()]).unwrap(),
                ["container", "start", "--attach", "--interactive", id] => {
                    assert_eq!(*id, "e".repeat(64));
                    let command: HostUpdateRequest = serde_json::from_slice(request.stdin.as_ref().unwrap()).unwrap();
                    assert!(matches!(command, HostUpdateRequest::Startup { .. }));
                    if self.mode == "cancel" { cancellation.cancel(); return Err(ProcessError::Cancelled); }
                    if self.mode == "failed" { return Ok(ProcessOutput { exit_code: Some(1), stdout: vec![], stderr: b"private detail".to_vec(), stdout_truncated: false, stderr_truncated: false }); }
                    serde_json::to_vec(&self.response).unwrap()
                }
                ["container", "rm", "--force", id] => { assert_eq!(*id, "e".repeat(64)); assert!(!cancellation.is_cancelled()); vec![] }
                _ => panic!("unexpected command {args:?}"),
            };
            Ok(ProcessOutput {
                exit_code: Some(0),
                stdout: bytes,
                stderr: vec![],
                stdout_truncated: false,
                stderr_truncated: false,
            })
        })
    }
}

#[tokio::test]
async fn utility_is_owned_isolated_and_cleaned_even_when_request_fails_or_cancels() {
    for mode in ["normal", "failed", "cancel", "foreign"] {
        let fixture = Fixture::new();
        let registration = fixture.register().await.unwrap();
        let response =
            serde_json::to_value(HostUpdateResponse::Registered(Box::new(registration))).unwrap();
        let state = fixture.state.clone();
        let runner = Arc::new(Runner {
            fixture,
            response,
            mode,
            utility: Mutex::new(Value::Null),
            calls: Default::default(),
        });
        let client = HostUpdateClient::new(runner.clone(), state.clone());
        let result = client
            .exchange_from(
                HostUpdateRequest::Startup {
                    state_directory: state,
                },
                &CancellationToken::new(),
                &CORE[..12],
            )
            .await;
        assert_eq!(result.is_ok(), mode == "normal", "{mode}");
        if let Err(error) = result {
            assert!(!error.to_string().contains("private detail"));
        }
        let calls = runner.calls.lock().unwrap();
        assert_eq!(
            calls
                .iter()
                .any(|call| call.starts_with("container rm --force")),
            mode != "foreign"
        );
        assert_eq!(
            calls.iter().any(|call| call.starts_with("container start")),
            mode != "foreign"
        );
    }
}
