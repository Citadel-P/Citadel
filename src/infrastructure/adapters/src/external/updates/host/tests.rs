use super::*;
use crate::external::updates::bootstrap::fixtures::{CORE, Fixture};
use citadel_execution::{ProcessError, ProcessOutput, ProcessRequest, ProcessRunner};
use futures_util::future::BoxFuture;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn database_status_reads_version_without_host_commands_or_installation_lock() {
    use citadel_updates::{UpdateError, UpdateInstallationRuntime};

    let fixture = Fixture::new();
    fixture.register().await.unwrap();
    let state = super::super::JournalDirectory::open(fixture.state.clone()).unwrap();
    let _owner = state.existing_lock().unwrap();
    let path = fixture.state.clone();
    let runner = Arc::new(Runner {
        fixture,
        response: Value::Null,
        mode: "normal",
        utility: Mutex::new(Value::Null),
        calls: Default::default(),
    });
    // A closed lazy pool cannot connect to any real database. Reaching its error
    // proves a status read did not fail first on the held installation lock.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://unused@localhost/unused")
        .unwrap();
    pool.close().await;
    let registered = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let runtime = DockerUpdateInstallation::new(runner.clone(), path, "0.1.1".into(), pool)
        .with_registration(registered.clone());
    let cancellation = CancellationToken::new();
    assert!(!runtime.registered());
    assert!(matches!(
        runtime.database_update(&cancellation).await,
        Err(UpdateError::Unavailable(_))
    ));
    assert!(matches!(
        runtime.source(&cancellation).await,
        Err(UpdateError::Conflict(_))
    ));
    assert!(runner.calls.lock().unwrap().is_empty());
    registered.store(true, std::sync::atomic::Ordering::Release);
    assert!(runtime.registered());
    assert!(matches!(
        runtime.database_update(&cancellation).await,
        Err(UpdateError::Storage(_))
    ));
    cancellation.cancel();
    assert!(matches!(
        runtime.database_update(&cancellation).await,
        Err(UpdateError::Cancelled)
    ));
    assert!(runner.calls.lock().unwrap().is_empty());
}

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
                    let mut core = self.fixture.runner.core.clone();
                    core["Config"]["Hostname"] = json!(hostname);
                    serde_json::to_vec(&vec![core]).unwrap()
                }
                ["image", "inspect", "--platform", "linux/amd64", digest] => {
                    let reference = if self.mode == "missing-source" {
                        "wrong"
                    } else {
                        digest
                    };
                    serde_json::to_vec(
                        &json!([{"RepoDigests":[reference], "Architecture":"amd64", "Os":"linux"}]),
                    )
                    .unwrap()
                }
                ["image", "inspect", digest] if digest.starts_with("ghcr.io/") => {
                    serde_json::to_vec(&json!([{"Id":format!("sha256:{}", "1".repeat(64))}]))
                        .unwrap()
                }
                ["image", "inspect", _] => {
                    let primary = format!("ghcr.io/citadel-p/citadel@sha256:{}", "f".repeat(64));
                    let mut refs = vec![primary];
                    if self.mode != "wrong-target" {
                        refs.push(format!(
                            "ghcr.io/citadel-p/citadel@sha256:{}",
                            "c".repeat(64)
                        ));
                    }
                    serde_json::to_vec(
                        &json!([{"RepoDigests":refs,"Architecture":"amd64","Os":"linux"}]),
                    )
                    .unwrap()
                }
                ["container", "create", ..] => {
                    assert!(args.contains(&"--network=none"));
                    assert!(args.contains(&"--read-only"));
                    assert!(args.contains(&"--tmpfs=/tmp:rw,noexec,nosuid,size=64m"));
                    assert!(args.contains(&"TMPDIR=/tmp"));
                    let name = args[args.iter().position(|arg| *arg == "--name").unwrap() + 1];
                    let label = args[args.iter().position(|arg| *arg == "--label").unwrap() + 1];
                    let nonce = label.strip_prefix("com.citadel.update-host=").unwrap();
                    let mounts: Vec<_> = [(self.fixture.inputs.directory.to_str().unwrap(), false), (self.fixture.state.to_str().unwrap(), true), ("/var/run/docker.sock", true)].into_iter().map(|(path,rw)| json!({"Type":"bind","Source":path,"Destination":path,"RW":rw})).chain([json!({"Type":"tmpfs","Destination":"/tmp","RW":true})]).collect();
                    let socket =
                        super::super::layout::container_socket(&self.fixture.runner.core).unwrap();
                    assert!(
                        args.contains(
                            &format!(
                                "type=bind,\"source={socket}\",\"destination=/var/run/docker.sock\""
                            )
                            .as_str()
                        )
                    );
                    let mut mounts = mounts;
                    mounts[2]["Source"] = json!(socket);
                    let mut utility = json!({"Id":"e".repeat(64), "Name":format!("/{name}"), "Image":args[args.len()-2], "Path":"/app/citadel-server", "Args":["update-host"], "RestartCount":0,
                        "Config":{"Hostname":"e".repeat(12), "User":"0:0", "Labels":{"com.citadel.update-host":nonce}},
                        "HostConfig":{"NetworkMode":"none","ReadonlyRootfs":true,"Privileged":false,"RestartPolicy":{"Name":"no"}},
                        "State":{"Running":false,"ExitCode":0,"OOMKilled":false,"Error":""}, "Mounts":mounts});
                    if self.mode == "foreign" {
                        utility["Config"]["Labels"]["com.citadel.update-host"] = json!("foreign");
                    }
                    *self.utility.lock().unwrap() = utility;
                    "e".repeat(64).into_bytes()
                }
                ["container", "inspect", id] if *id == "e".repeat(64) => {
                    serde_json::to_vec(&vec![self.utility.lock().unwrap().clone()]).unwrap()
                }
                ["container", "start", "--attach", "--interactive", id] => {
                    assert_eq!(*id, "e".repeat(64));
                    let command: HostUpdateRequest =
                        serde_json::from_slice(request.stdin.as_ref().unwrap()).unwrap();
                    assert!(matches!(
                        command,
                        HostUpdateRequest::Startup { .. }
                            | HostUpdateRequest::Finalize(_)
                            | HostUpdateRequest::Completed(_)
                    ));
                    if self.mode == "cancel" {
                        cancellation.cancel();
                        return Err(ProcessError::Cancelled);
                    }
                    if self.mode == "failed" {
                        return Ok(ProcessOutput {
                            exit_code: Some(1),
                            stdout: vec![],
                            stderr: b"private detail".to_vec(),
                            stdout_truncated: false,
                            stderr_truncated: false,
                        });
                    }
                    serde_json::to_vec(&self.response).unwrap()
                }
                ["container", "rm", "--force", id] => {
                    assert_eq!(*id, "e".repeat(64));
                    assert!(!cancellation.is_cancelled());
                    vec![]
                }
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
    for mode in ["normal", "rootless", "failed", "cancel", "foreign"] {
        let fixture = if mode == "rootless" {
            Fixture::rootless()
        } else {
            Fixture::new()
        };
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
        assert_eq!(
            result.is_ok(),
            matches!(mode, "normal" | "rootless"),
            "{mode}"
        );
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

#[tokio::test]
async fn target_cleanup_uses_the_source_executor_and_reports_the_actual_target() {
    let finalize = super::super::handoff::finalize_request().await;
    let operation = &finalize.completion.journal.operation;
    assert_ne!(
        operation.plan.source.catalog_sha256,
        operation.plan.target.catalog_sha256
    );
    for (request, mode) in [
        (
            HostUpdateRequest::Finalize(Box::new(finalize.clone())),
            "normal",
        ),
        (
            HostUpdateRequest::Completed(Box::new(operation.clone())),
            "normal",
        ),
        (
            HostUpdateRequest::Completed(Box::new(operation.clone())),
            "wrong-target",
        ),
        (
            HostUpdateRequest::Completed(Box::new(operation.clone())),
            "missing-source",
        ),
        (
            HostUpdateRequest::Completed(Box::new(operation.clone())),
            "failed",
        ),
    ] {
        let mut fixture = Fixture::new();
        fixture.state = operation.plan.checkpoint_directory.clone().into();
        let state = fixture.state.clone();
        let runner = Arc::new(Runner {
            fixture,
            mode,
            response: serde_json::to_value(HostUpdateResponse::Finalized(Box::new(
                finalize.completion.journal.clone(),
            )))
            .unwrap(),
            utility: Mutex::default(),
            calls: Mutex::default(),
        });
        let client = HostUpdateClient::new(runner.clone(), state);
        let result = client
            .exchange_from(request, &CancellationToken::new(), &CORE[..12])
            .await;
        let calls = runner.calls.lock().unwrap();
        if matches!(mode, "missing-source" | "wrong-target") {
            let reason = if mode == "missing-source" {
                "approved source Core image"
            } else {
                "approved update target"
            };
            assert!(
                matches!(result, Err(citadel_updates::UpdateError::Conflict(message)) if message.contains(reason))
            );
            assert!(
                !calls
                    .iter()
                    .any(|call| call.starts_with("container create"))
            );
            continue;
        }
        assert!(calls.iter().any(|call| call.starts_with("container create")
            && call.ends_with(&format!("sha256:{} update-host", "1".repeat(64)))));
        if mode == "failed" {
            assert!(
                matches!(result, Err(citadel_updates::UpdateError::Unavailable(message))
                if message.contains(&operation.id.to_string()) && message.contains("host cleanup") && !message.contains("private detail"))
            );
        } else {
            let (_, digest, architecture) = result.unwrap();
            assert_eq!(digest, operation.plan.target.image);
            assert_eq!(architecture, operation.plan.target.architecture);
        }
    }
}

#[tokio::test]
async fn changed_installer_inputs_reach_review_without_disclosing_the_environment() {
    let fixture = Fixture::new();
    fixture.register().await.unwrap();
    std::fs::write(
        fixture.inputs.directory.join(".env"),
        "PG_PASSWORD=redaction-sentinel\n",
    )
    .unwrap();
    let result = execute_host_update(
        HostUpdateRequest::Check {
            state_directory: fixture.state.clone(),
        },
        &fixture.runner,
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    let HostUpdateResponse::ReviewBlocked { reason } = result else {
        panic!("expected registration drift")
    };
    assert!(reason.contains("Registered installer inputs have changed"));
    assert!(!reason.contains("redaction-sentinel"));
}
