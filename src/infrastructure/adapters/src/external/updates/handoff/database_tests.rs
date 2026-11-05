use super::super::{bootstrap::fixtures::DATABASE, docker::LocalDocker};
use super::{
    database_eligibility::check_database_source,
    tests::{HELPER, Runner},
    verification,
};
use citadel_execution::{ProcessError, ProcessOutput, ProcessRequest, ProcessRunner};
use citadel_updates::DatabaseUpdatePlan;
use futures_util::future::BoxFuture;
use serde_json::{Value, json};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;

struct DatabaseRunner {
    base: Arc<Runner>,
    facts: Value,
    helper: Option<Value>,
    alpine: bool,
    stopped: AtomicBool,
    edit: Option<std::path::PathBuf>,
    submissions: AtomicUsize,
}
impl DatabaseRunner {
    fn new(base: Arc<Runner>) -> Self {
        Self {
            base,
            facts: json!({"version":180001,"data":"/var/lib/postgresql/18/docker","recovery":false,"slots":0,"replicas":0,"subscriptions":0,"tablespaces":0}),
            helper: None,
            alpine: true,
            stopped: AtomicBool::new(false),
            edit: None,
            submissions: AtomicUsize::new(0),
        }
    }
}
fn output(value: Value) -> ProcessOutput {
    ProcessOutput {
        exit_code: Some(0),
        stdout: serde_json::to_vec(&value).unwrap(),
        stderr: vec![],
        stdout_truncated: false,
        stderr_truncated: false,
    }
}
impl ProcessRunner for DatabaseRunner {
    fn run<'a>(
        &'a self,
        request: ProcessRequest,
        token: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ProcessOutput, ProcessError>> {
        Box::pin(async move {
            let args: Vec<_> = request
                .arguments
                .iter()
                .map(|a| a.to_str().unwrap())
                .collect();
            if args.ends_with(&["container", "inspect", HELPER]) {
                if let Some(helper) = &self.helper {
                    return Ok(output(json!([helper])));
                }
            } else if args.ends_with(&["container", "inspect", DATABASE]) {
                let mut db = self.base.fixture.runner.database.clone();
                db["Config"]["Env"] = json!(["PGDATA=/var/lib/postgresql/18/docker"]);
                db["State"]["Running"] = json!(!self.stopped.load(Ordering::SeqCst));
                db["State"]["ExitCode"] = json!(0);
                db["State"]["OOMKilled"] = json!(false);
                db["State"]["Error"] = json!("");
                return Ok(output(json!([db])));
            } else if args.get(2) == Some(&"volume") && args.len() == 6 {
                return Ok(output(json!([{ "Name":args[4] }, { "Name":args[5] }])));
            } else if args.contains(&"exec") && args.contains(&DATABASE) {
                if args.last().unwrap().contains("pg_replication_slots") {
                    return Ok(output(self.facts.clone()));
                }
                assert!(args.last().unwrap().contains("/etc/alpine-release"));
                let mut out = output(Value::Null);
                out.exit_code = Some(if self.alpine { 0 } else { 1 });
                return Ok(out);
            } else if args.contains(&"stop") && args.contains(&DATABASE) {
                assert!(!self.stopped.swap(true, Ordering::SeqCst));
                if let Some(path) = &self.edit {
                    use std::io::Write;
                    std::fs::OpenOptions::new()
                        .append(true)
                        .open(path)
                        .unwrap()
                        .write_all(b"\n# concurrent administrator edit\n")
                        .unwrap();
                }
                return Ok(output(Value::Null));
            } else if args.contains(&"compose") && args.contains(&"up") {
                self.submissions.fetch_add(1, Ordering::SeqCst);
                return Err(ProcessError::Timeout(Duration::from_secs(300)));
            }
            self.base.run(request, token).await
        })
    }
}
fn plan() -> DatabaseUpdatePlan {
    super::super::compose::database_update(180001)
        .unwrap()
        .unwrap()
}

#[tokio::test]
async fn eligibility_reports_database_blockers_without_mutation() {
    let base = Runner::new().await;
    let home = tempfile::tempdir().unwrap();
    let token = CancellationToken::new();
    for (field, value, expected) in [
        ("slots", json!(1), "replication slots"),
        ("replicas", json!(1), "replicas"),
        ("subscriptions", json!(1), "subscriptions"),
        ("tablespaces", json!(1), "tablespaces"),
        ("recovery", json!(true), "recovery"),
        ("data", json!("/custom"), "PGDATA"),
        ("version", json!(180002), "version changed"),
        ("alpine", json!(false), "Alpine"),
        ("normal", Value::Null, ""),
    ] {
        let mut runner = DatabaseRunner::new(base.clone());
        runner.facts[field] = value;
        runner.alpine = field != "alpine";
        let docker = LocalDocker {
            process: &runner,
            cancellation: &token,
            home: home.path(),
            deadline: Instant::now() + Duration::from_secs(30),
        };
        let result = check_database_source(&docker, &base.intent.registration, &plan()).await;
        if field == "normal" {
            result.unwrap();
        } else {
            assert!(
                result.unwrap_err().to_string().contains(expected),
                "{field}"
            );
        }
        assert!(!runner.stopped.load(Ordering::SeqCst));
        assert_eq!(runner.submissions.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn helper_profile_is_stable_for_core_and_strict_for_database_updates() {
    let base = Runner::new().await;
    let home = tempfile::tempdir().unwrap();
    let token = CancellationToken::new();
    for database in [false, true] {
        for valid in [true, false] {
            let mut runner = DatabaseRunner::new(base.clone());
            let mut intent = base.intent.clone();
            let mut helper = base.helper();
            if database {
                intent.operation.plan.database = Some(plan());
                intent.operation.plan.target = intent.operation.plan.source.clone();
                helper["Config"]["Hostname"] = json!(&HELPER[..12]);
                helper["Config"]["Env"] = json!(["TMPDIR=/tmp"]);
                helper["HostConfig"]["Tmpfs"] = json!({"/tmp":"rw,noexec,nosuid,size=64m"});
                helper["HostConfig"]["NetworkMode"] = json!(intent.registration.network);
                helper["NetworkSettings"]["Networks"] =
                    json!({intent.registration.network.clone(): {}});
            }
            if !valid {
                helper["HostConfig"]["Tmpfs"] = if database {
                    Value::Null
                } else {
                    json!({"/tmp":"rw,noexec,nosuid,size=64m"})
                };
            }
            runner.helper = Some(helper);
            let docker = LocalDocker {
                process: &runner,
                cancellation: &token,
                home: home.path(),
                deadline: Instant::now() + Duration::from_secs(30),
            };
            let binding = super::evidence::HelperBinding {
                nonce: intent.nonce,
                helper_container_id: HELPER.into(),
            };
            assert_eq!(
                verification::helper(&docker, &intent, &binding)
                    .await
                    .is_ok(),
                valid,
                "database={database}, valid={valid}"
            );
        }
    }
}

#[tokio::test]
async fn edits_during_database_shutdown_prevent_compose_submission() {
    for environment in [Some(true), Some(false), None] {
        let (base_runner, mut helper, _checkpoint) =
            super::replacement_tests::ready("normal").await;
        helper
            .replace_core(&base_runner, &CancellationToken::new())
            .await
            .unwrap();
        let mut execution =
            super::ExecutionJournal::read_for(&helper.state, &helper.intent).unwrap();
        // Exercise the database mutation boundary using the real registered files
        // and durable replacement configuration from the common handoff fixture.
        helper.intent.operation.plan.database = Some(plan());
        helper.intent.operation.plan.target = helper.intent.operation.plan.source.clone();
        execution.journal.operation.plan = helper.intent.operation.plan.clone();
        for entry in &mut execution.journal.history {
            entry.plan = helper.intent.operation.plan.clone();
        }
        helper
            .state
            .write(&helper.intent.file("launch"), &helper.intent)
            .unwrap();
        let mut runner = DatabaseRunner::new(base_runner.base.clone());
        runner.edit = environment.map(|environment| {
            if environment {
                helper.intent.registration.environment_file.clone()
            } else {
                helper.intent.registration.compose_files[0].clone()
            }
        });
        let home = tempfile::tempdir().unwrap();
        let token = CancellationToken::new();
        let docker = LocalDocker {
            process: &runner,
            cancellation: &token,
            home: home.path(),
            deadline: Instant::now() + Duration::from_secs(30),
        };
        execution.replacement.as_mut().unwrap().database = Some(
            super::database::DatabaseReplacement::prepare(&docker, &helper.intent)
                .await
                .unwrap(),
        );
        execution.save(&helper.state, &helper.owner).unwrap();
        assert_eq!(
            super::ExecutionJournal::read_for(&helper.state, &helper.intent).unwrap(),
            execution
        );
        assert!(
            helper
                .replace_database(&docker, &mut execution)
                .await
                .is_err()
        );
        assert!(runner.stopped.load(Ordering::SeqCst));
        assert_eq!(
            runner.submissions.load(Ordering::SeqCst),
            usize::from(environment.is_none())
        );
    }
}
