//! Deno execution is owned by the Agent, with capacity retained through cleanup.
use super::Runtime;
use citadel_contracts::citadel::scripts::v1::{
    ScriptAttempt, ScriptCleanup, ScriptEvent, ScriptHost, ScriptRequest,
    script_service_server::ScriptService,
};
use citadel_scripts::sandbox::{ScriptExecution, ScriptSandbox};
use futures_util::stream::BoxStream;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;
use tonic::{Request, Response, Status};
use uuid::Uuid;

#[derive(Clone)]
pub(super) struct Admission(Arc<Mutex<Ledger>>);
struct Ledger {
    limit: usize,
    heavy: Arc<Semaphore>,
    runs: HashMap<Uuid, Run>,
}
struct Run {
    run_id: Uuid,
    generation: u64,
    cancel: CancellationToken,
    completed: bool,
    _heavy: OwnedSemaphorePermit,
}
fn ids(attempt: &ScriptAttempt) -> Result<(Uuid, Uuid), Status> {
    let parse = |s: &str| Uuid::parse_str(s).ok().filter(|id| !id.is_nil());
    match (parse(&attempt.run_id), parse(&attempt.attempt_id)) {
        (Some(run), Some(attempt)) if attempt != run => Ok((run, attempt)),
        _ => Err(Status::invalid_argument("Invalid scripts attempt identity")),
    }
}
impl Admission {
    pub(super) fn new(limit: usize, heavy: Arc<Semaphore>) -> Self {
        Self(Arc::new(Mutex::new(Ledger {
            limit,
            heavy,
            runs: HashMap::new(),
        })))
    }
    pub(super) fn limit(&self) -> usize {
        self.0.lock().map_or(0, |ledger| ledger.limit)
    }
    pub(super) fn reserve(
        &self,
        attempt: &ScriptAttempt,
        cancel: CancellationToken,
    ) -> Result<bool, Status> {
        let (run_id, id) = ids(attempt)?;
        let mut ledger = self
            .0
            .lock()
            .map_err(|_| Status::internal("Admission unavailable"))?;
        // Duplicate submission is ambiguous, never a definitive capacity refusal.
        if ledger.runs.contains_key(&id) || ledger.runs.values().any(|r| r.run_id == run_id) {
            return Err(Status::already_exists("Script attempt is already admitted"));
        }
        if ledger.runs.len() >= ledger.limit {
            return Ok(false);
        }
        let Ok(heavy) = ledger.heavy.clone().try_acquire_owned() else {
            return Ok(false);
        };
        ledger.runs.insert(
            id,
            Run {
                run_id,
                generation: attempt.generation,
                cancel,
                completed: false,
                _heavy: heavy,
            },
        );
        Ok(true)
    }
    fn completed(&self, attempt: &ScriptAttempt) {
        if let Ok(mut ledger) = self.0.lock()
            && let Ok((run, id)) = ids(attempt)
            && let Some(entry) = ledger.runs.get_mut(&id)
            && entry.run_id == run
            && entry.generation == attempt.generation
        {
            entry.completed = true;
        }
    }
    fn cleanup(&self, attempt: &ScriptAttempt) -> Result<bool, Status> {
        let (run, id) = ids(attempt)?;
        let mut ledger = self
            .0
            .lock()
            .map_err(|_| Status::internal("Admission unavailable"))?;
        if let Some(entry) = ledger.runs.get(&id) {
            if entry.run_id != run || entry.generation != attempt.generation {
                return Err(Status::failed_precondition("Script ownership changed"));
            }
            if !entry.completed {
                entry.cancel.cancel();
                return Ok(false);
            }
            ledger.runs.remove(&id);
        }
        Ok(true)
    }
}

#[tonic::async_trait]
impl ScriptService for Runtime {
    type RunStream = BoxStream<'static, Result<ScriptEvent, Status>>;
    async fn check_host(&self, _: Request<()>) -> Result<Response<ScriptHost>, Status> {
        let daemon_id = self.docker.info().await.map_err(super::docker_error)?.id;
        let (slots, occupied) = {
            let ledger = self
                .scripts
                .0
                .lock()
                .map_err(|_| Status::internal("Admission unavailable"))?;
            (ledger.limit as u32, ledger.runs.len() as u32)
        };
        Ok(Response::new(ScriptHost {
            daemon_id,
            authority_id: self.builds.authority()?,
            generation: self.builds.generation()?,
            slots,
            occupied,
            heavy_slots: self.heavy_capacity as u32,
        }))
    }
    async fn reconcile(
        &self,
        request: Request<ScriptAttempt>,
    ) -> Result<Response<ScriptCleanup>, Status> {
        let attempt = request.into_inner();
        ids(&attempt)?;
        if attempt.authority_id != self.builds.authority()? {
            return Ok(Response::new(ScriptCleanup { confirmed: false }));
        }
        // Fence a delayed opening before confirming that no attempt exists.
        self.builds.fence_generation(attempt.generation)?;
        Ok(Response::new(ScriptCleanup {
            confirmed: self.scripts.cleanup(&attempt)?,
        }))
    }
    async fn run(
        &self,
        request: Request<ScriptRequest>,
    ) -> Result<Response<Self::RunStream>, Status> {
        let request = request.into_inner();
        let attempt = request
            .attempt
            .as_ref()
            .ok_or_else(|| Status::invalid_argument("Missing attempt"))?
            .clone();
        let (_, attempt_id) = ids(&attempt)?;
        if request.source.is_empty()
            || request.source.len() > 1024 * 1024
            || request.token.is_empty()
            || request.token.len() > 16384
            || request.args_json.len() > 64 * 1024
            || !(1..=86400).contains(&request.timeout_seconds)
            || !(1024..=4 * 1024 * 1024).contains(&request.maximum_log_bytes)
            || !serde_json::from_str::<serde_json::Value>(&request.args_json)
                .is_ok_and(|v| v.is_object())
        {
            return Err(Status::invalid_argument("Invalid script execution limits"));
        }
        let origin = url::Url::parse(&request.core_url)
            .map_err(|_| Status::invalid_argument("Invalid Core origin"))?;
        if origin.scheme() != "https"
            || origin.host_str().is_none()
            || !origin.username().is_empty()
            || origin.password().is_some()
            || origin.query().is_some()
            || origin.fragment().is_some()
            || origin.path() != "/"
        {
            return Err(Status::invalid_argument(
                "Remote scripts requires a secure Core origin",
            ));
        }
        let cancel = self.shutdown.child_token();
        if !self
            .builds
            .reserve_scripts(&self.scripts, &attempt, cancel.clone())?
        {
            return Ok(Response::new(Box::pin(futures_util::stream::once(async {
                Ok(ScriptEvent {
                    not_started: true,
                    ..Default::default()
                })
            }))));
        }
        let guard = cancel.clone().drop_guard();
        let (sender, mut receiver) = tokio::sync::mpsc::channel(16);
        let ledger = self.scripts.clone();
        let sandbox = ScriptSandbox {
            process: Arc::new(citadel_processes::SystemProcess),
            workspace: Arc::new(
                citadel_scripts_runtime::script_workspace::LocalScriptWorkspace::new(
                    self.tasks.clone(),
                ),
            ),
            deno_path: self.deno_path.clone(),
            work_root: self.script_root.clone(),
            cache_directory: None,
            allow_net: None,
        };
        let task_attempt = attempt.clone();
        if !self.tasks.spawn("scripts.execute", async move {
            let (progress, mut output) = tokio::sync::broadcast::channel(32);
            let script = ScriptExecution {
                run_id: attempt_id,
                source: request.source,
                token: request.token,
                args_json: request.args_json,
                base_url: request.core_url,
                timeout_seconds: request.timeout_seconds as i32,
                maximum_log_bytes: request.maximum_log_bytes as usize,
            };
            let operation = sandbox.execute(script, &cancel, Some(&progress));
            tokio::pin!(operation);
            let result = loop {
                tokio::select! {
                    result = &mut operation => break result,
                    item = output.recv() => {
                        if let Ok(item) = item { let _ = sender.try_send(ScriptEvent {
                            stream: item.stream.unwrap_or_default(), ..Default::default()
                        }); }
                    }
                }
            };
            // execute returns after reaping the process and attempting workspace cleanup.
            // Failed directory removal remains owned by the workspace cleanup task.
            ledger.completed(&task_attempt);
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                sender.send(ScriptEvent {
                    completed: true,
                    status: result.status.as_str().into(),
                    exit_code: result.exit_code,
                    logs: result.logs,
                    error: result.error,
                    ..Default::default()
                }),
            )
            .await;
            Ok::<(), std::io::Error>(())
        }) {
            self.scripts.completed(&attempt);
            return Err(Status::unavailable("Agent is shutting down"));
        }
        Ok(Response::new(Box::pin(async_stream::stream! {
            let _guard = guard;
            while let Some(item) = receiver.recv().await { yield Ok(item); }
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::direct::build_admission::BuildAdmission;
    fn attempt(builds: &BuildAdmission) -> ScriptAttempt {
        ScriptAttempt {
            run_id: Uuid::now_v7().to_string(),
            attempt_id: Uuid::now_v7().to_string(),
            authority_id: builds.authority().unwrap(),
            generation: builds.generation().unwrap(),
        }
    }
    #[test]
    fn duplicate_is_ambiguous_and_capacity_is_held_until_reaped_and_confirmed() {
        let heavy = Arc::new(Semaphore::new(1));
        let builds = BuildAdmission::with_heavy(2, heavy.clone());
        let ledger = Admission::new(4, heavy.clone());
        let first = attempt(&builds);
        let cancel = CancellationToken::new();
        assert!(
            builds
                .reserve_scripts(&ledger, &first, cancel.clone())
                .unwrap()
        );
        assert_eq!(
            builds
                .reserve_scripts(&ledger, &first, cancel.clone())
                .unwrap_err()
                .code(),
            tonic::Code::AlreadyExists
        );
        assert!(
            !builds
                .reserve_scripts(&ledger, &attempt(&builds), CancellationToken::new())
                .unwrap()
        );
        assert!(
            builds
                .build(
                    &Uuid::now_v7().to_string(),
                    &Uuid::now_v7().to_string(),
                    vec![],
                    &builds.authority().unwrap(),
                    1
                )
                .unwrap()
                .is_err()
        );
        assert!(!ledger.cleanup(&first).unwrap());
        assert!(cancel.is_cancelled());
        assert_eq!(
            heavy.available_permits(),
            0,
            "cancellation is not cleanup proof"
        );
        ledger.completed(&first);
        assert_eq!(
            heavy.available_permits(),
            0,
            "completed work waits for receipt"
        );
        builds.fence_generation(first.generation).unwrap();
        assert!(ledger.cleanup(&first).unwrap());
        assert_eq!(heavy.available_permits(), 1);
        assert!(
            !builds
                .reserve_scripts(&ledger, &first, CancellationToken::new())
                .unwrap(),
            "old attempt cannot reenter after receipt"
        );
    }
    #[test]
    fn cleanup_and_admission_are_fenced_together() {
        for _ in 0..100 {
            let heavy = Arc::new(Semaphore::new(2));
            let builds = BuildAdmission::with_heavy(2, heavy.clone());
            let ledger = Admission::new(2, heavy);
            let request = attempt(&builds);
            let barrier = std::sync::Barrier::new(2);
            std::thread::scope(|scope| {
                scope.spawn(|| {
                    barrier.wait();
                    let _ = builds
                        .reserve_scripts(&ledger, &request, CancellationToken::new())
                        .unwrap();
                });
                barrier.wait();
                builds.fence_generation(request.generation).unwrap();
                let confirmed = ledger.cleanup(&request).unwrap();
                if !confirmed {
                    ledger.completed(&request);
                    assert!(ledger.cleanup(&request).unwrap());
                }
            });
            assert!(ledger.0.lock().unwrap().runs.is_empty());
            let mut wrong = attempt(&builds);
            wrong.authority_id = Uuid::now_v7().to_string();
            assert!(
                !builds
                    .reserve_scripts(&ledger, &wrong, CancellationToken::new())
                    .unwrap()
            );
        }
    }
}

#[cfg(test)]
mod execution_tests {
    use super::*;
    use futures_util::StreamExt;
    #[tokio::test]
    #[ignore = "requires CITADEL_DENO_PATH"]
    async fn actual_deno_is_bounded_redacted_sandboxed_and_reaped_after_stream_loss() {
        let shutdown = CancellationToken::new();
        let docker = citadel_docker::client::DockerClient::with_endpoint(
            "http://127.0.0.1:1".parse().unwrap(),
            std::time::Duration::from_secs(1),
            "/unused",
        )
        .unwrap();
        let mut runtime = Runtime::new(docker, shutdown.clone(), None);
        runtime.script_root =
            std::env::temp_dir().join(format!("citadel-agent-scripts-{}", Uuid::now_v7()));
        runtime.deno_path = std::env::var_os("CITADEL_DENO_PATH").unwrap();
        let request = |source: &str| ScriptRequest {
            attempt: Some(ScriptAttempt {
                run_id: Uuid::now_v7().to_string(),
                attempt_id: Uuid::now_v7().to_string(),
                authority_id: runtime.builds.authority().unwrap(),
                generation: runtime.builds.generation().unwrap(),
            }),
            source: source.into(),
            token: "fixture-secret".into(),
            args_json: "{}".into(),
            core_url: "https://core.invalid".into(),
            timeout_seconds: 10,
            maximum_log_bytes: 4096,
        };
        for origin in [
            "http://core.invalid",
            "https://user:pass@core.invalid",
            "https://core.invalid/path",
        ] {
            let mut bad = request("console.log('no')");
            bad.core_url = origin.into();
            assert!(runtime.run(Request::new(bad)).await.is_err());
        }
        for (source, status) in [
            ("console.log('fixture-secret');", "Succeeded"),
            (
                "if ((await Deno.permissions.request({name: 'net', host: 'core.invalid:5432'})).state !== 'denied') throw new Error('Other Core ports must be denied');",
                "Succeeded",
            ),
            (
                "if ((await Deno.permissions.request({name: 'import', host: 'deno.land'})).state !== 'denied') throw new Error('Public module imports must be denied');",
                "Succeeded",
            ),
            ("await Deno.readTextFile('/etc/passwd');", "Failed"),
            ("await new Deno.Command('/bin/sh').output();", "Failed"),
            ("await fetch('https://example.invalid');", "Failed"),
        ] {
            let req = request(source);
            let attempt = req.attempt.clone().unwrap();
            let mut stream = runtime.run(Request::new(req)).await.unwrap().into_inner();
            let mut complete = false;
            while let Some(event) = stream.next().await {
                let event = event.unwrap();
                assert!(!event.stream.contains("fixture-secret"));
                assert!(!event.logs.contains("fixture-secret"));
                if event.completed {
                    assert_eq!(event.status, status, "{source}: {}", event.logs);
                    complete = true;
                }
            }
            assert!(complete);
            assert!(
                runtime
                    .reconcile(Request::new(attempt))
                    .await
                    .unwrap()
                    .into_inner()
                    .confirmed
            );
        }
        let req = request("console.log('started'); await new Promise(r=>setTimeout(r,60000));");
        let attempt = req.attempt.clone().unwrap();
        let mut stream = runtime.run(Request::new(req)).await.unwrap().into_inner();
        assert!(
            stream
                .next()
                .await
                .unwrap()
                .unwrap()
                .stream
                .contains("started")
        );
        drop(stream);
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                if runtime
                    .reconcile(Request::new(attempt.clone()))
                    .await
                    .unwrap()
                    .into_inner()
                    .confirmed
                {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(runtime.heavy.available_permits(), 2);
        assert!(
            tokio::fs::read_dir(&runtime.script_root)
                .await
                .unwrap()
                .next_entry()
                .await
                .unwrap()
                .is_none()
        );
        shutdown.cancel();
        tokio::fs::remove_dir_all(&runtime.script_root)
            .await
            .unwrap();
    }
}
