//! Cache ownership across real workspace and sandbox execution boundaries; no database.
use citadel_adapters::filesystem::script_workspace::LocalScriptWorkspace;
use citadel_execution::{ProcessError, ProcessOutput, ProcessRequest, ProcessRunner};
use citadel_runtime::DynamicTasks;
use citadel_scripts::{
    ScriptRunStatus,
    sandbox::{ScriptExecution, ScriptSandbox},
};
use futures_util::future::BoxFuture;
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

struct Fixture {
    root: tempfile::TempDir,
    tasks: DynamicTasks,
}
impl Fixture {
    fn new() -> Self {
        Self {
            root: tempfile::tempdir().unwrap(),
            tasks: DynamicTasks::new(CancellationToken::new()),
        }
    }
    fn sandbox(
        &self,
        process: Arc<dyn ProcessRunner>,
        deno_path: impl Into<std::ffi::OsString>,
    ) -> ScriptSandbox {
        ScriptSandbox {
            process,
            workspace: Arc::new(LocalScriptWorkspace::new(self.tasks.clone())),
            deno_path: deno_path.into(),
            work_root: self.root.path().join("runs"),
            allow_net: None,
        }
    }
    fn assert_empty(&self) {
        assert_eq!(
            std::fs::read_dir(self.root.path().join("runs"))
                .unwrap()
                .count(),
            0
        );
        assert_eq!(
            std::fs::read_dir(self.root.path()).unwrap().count(),
            1,
            "no cache outside run directories"
        );
    }
}
fn input(source: &str) -> ScriptExecution {
    ScriptExecution {
        run_id: Uuid::now_v7(),
        source: source.into(),
        token: "fixture-secret".into(),
        args_json: "{}".into(),
        base_url: "http://127.0.0.1:1".into(),
        timeout_seconds: 10,
        maximum_log_bytes: 1024,
    }
}
struct CacheWriter {
    status: ScriptRunStatus,
    started: tokio::sync::Notify,
    wait: bool,
}
impl ProcessRunner for CacheWriter {
    fn run<'a>(
        &'a self,
        request: ProcessRequest,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ProcessOutput, ProcessError>> {
        Box::pin(async move {
            let directory = request.current_directory.as_ref().unwrap();
            let cache = PathBuf::from(
                &request
                    .environment
                    .iter()
                    .find(|(key, _)| key == "DENO_DIR")
                    .unwrap()
                    .1,
            );
            assert_eq!(cache, directory.join("deno-cache"));
            tokio::fs::create_dir_all(cache.join("gen")).await.unwrap();
            tokio::fs::write(cache.join("gen/module.js"), "cached generated module")
                .await
                .unwrap();
            self.started.notify_one();
            if self.wait {
                cancellation.cancelled().await;
            }
            match self.status {
                ScriptRunStatus::Cancelled => Err(ProcessError::Cancelled),
                ScriptRunStatus::TimedOut => Err(ProcessError::Timeout(Duration::from_secs(1))),
                _ => Ok(ProcessOutput {
                    exit_code: Some(if self.status == ScriptRunStatus::Succeeded {
                        0
                    } else {
                        1
                    }),
                    stdout: vec![],
                    stderr: vec![],
                    stdout_truncated: false,
                    stderr_truncated: false,
                }),
            }
        })
    }
}

#[tokio::test]
async fn cache_is_removed_after_success_failure_timeout_and_cancellation() {
    let fixture = Fixture::new();
    for status in [
        ScriptRunStatus::Succeeded,
        ScriptRunStatus::Failed,
        ScriptRunStatus::TimedOut,
        ScriptRunStatus::Cancelled,
    ] {
        let sandbox = fixture.sandbox(
            Arc::new(CacheWriter {
                status,
                started: tokio::sync::Notify::new(),
                wait: false,
            }),
            "unused",
        );
        let result = sandbox
            .execute(input("fixture source"), &CancellationToken::new(), None)
            .await;
        assert_eq!(result.status, status);
        fixture.assert_empty();
    }
}

#[tokio::test]
async fn abandoning_execution_keeps_cache_cleanup_owned() {
    let fixture = Fixture::new();
    let runner = Arc::new(CacheWriter {
        status: ScriptRunStatus::Cancelled,
        started: tokio::sync::Notify::new(),
        wait: true,
    });
    let sandbox = fixture.sandbox(runner.clone(), "unused");
    let cancellation = CancellationToken::new();
    {
        let execution = sandbox.execute(input("fixture source"), &cancellation, None);
        tokio::pin!(execution);
        tokio::select! {
            result = &mut execution => panic!("unexpected completion: {:?}", result.status),
            () = runner.started.notified() => {},
            () = tokio::time::sleep(Duration::from_secs(5)) => panic!("fixture did not start"),
        }
    }
    fixture.tasks.drain(Duration::from_secs(5)).await.unwrap();
    fixture.assert_empty();
}

#[tokio::test]
#[ignore = "requires CITADEL_DENO_PATH; only disposable filesystem workspaces, no database or Docker daemon"]
async fn actual_deno_cache_does_not_accumulate_across_repeated_or_concurrent_runs() {
    let fixture = Fixture::new();
    let sandbox = fixture.sandbox(
        Arc::new(citadel_processes::SystemProcess),
        std::env::var_os("CITADEL_DENO_PATH").unwrap(),
    );
    let source = "const cache = Deno.env.get('DENO_DIR'); if (cache !== Deno.cwd() + '/deno-cache') throw Error('cache escaped run'); await Deno.mkdir(cache, {recursive:true}); await Deno.writeTextFile(cache + '/probe', 'generated'); console.log('ready');";
    let cancellation = CancellationToken::new();
    for _ in 0..3 {
        assert_eq!(
            sandbox
                .execute(input(source), &cancellation, None)
                .await
                .status,
            ScriptRunStatus::Succeeded
        );
        fixture.assert_empty();
    }
    let failed = format!("{source} throw Error('expected fixture failure');");
    let (ok, failed) = tokio::join!(
        sandbox.execute(input(source), &cancellation, None),
        sandbox.execute(input(&failed), &cancellation, None)
    );
    assert_eq!(ok.status, ScriptRunStatus::Succeeded);
    assert_eq!(failed.status, ScriptRunStatus::Failed);
    fixture.assert_empty();

    let waiting = format!("{source} await new Promise(() => setInterval(() => {{}}, 1000));");
    let mut timed = input(&waiting);
    timed.timeout_seconds = 1;
    assert_eq!(
        sandbox.execute(timed, &cancellation, None).await.status,
        ScriptRunStatus::TimedOut
    );
    fixture.assert_empty();

    let run = input(&waiting);
    let marker = sandbox
        .work_root
        .join(run.run_id.to_string())
        .join("deno-cache/probe");
    let cancel = async {
        tokio::time::timeout(Duration::from_secs(5), async {
            while !marker.exists() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        cancellation.cancel();
    };
    let (result, ()) = tokio::join!(sandbox.execute(run, &cancellation, None), cancel);
    assert_eq!(result.status, ScriptRunStatus::Cancelled);
    fixture.assert_empty();
}

struct ChunkedOutput;
impl ProcessRunner for ChunkedOutput {
    fn run<'a>(
        &'a self,
        request: ProcessRequest,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ProcessOutput, ProcessError>> {
        Box::pin(async move {
            let sender = request.output.unwrap();
            let mut stdout = Vec::new();
            // A long unterminated line, then a token and UTF-8 character split
            // across transport chunks. Only complete lines may be redacted.
            let mut pieces = vec![vec![b'x'; 8192]; 32];
            pieces.extend([
                b" fixture-".to_vec(),
                b"secret \xf0\x9f".to_vec(),
                b"\x98\x80\nnext fixture-".to_vec(),
                b"secret".to_vec(),
            ]);
            for bytes in pieces {
                stdout.extend_from_slice(&bytes);
                sender
                    .send(citadel_execution::ProcessChunk {
                        stream: "stdout",
                        bytes,
                    })
                    .await
                    .unwrap();
            }
            Ok(ProcessOutput {
                exit_code: Some(0),
                stdout,
                stderr: vec![],
                stdout_truncated: false,
                stderr_truncated: false,
            })
        })
    }
}

#[tokio::test]
async fn long_progress_lines_preserve_split_utf8_and_redaction() {
    let fixture = Fixture::new();
    let sandbox = fixture.sandbox(Arc::new(ChunkedOutput), "unused");
    let (progress, mut receiver) = tokio::sync::broadcast::channel(8);
    let mut execution = input("fixture source");
    execution.maximum_log_bytes = 512 * 1024;
    let result = sandbox
        .execute(execution, &CancellationToken::new(), Some(&progress))
        .await;
    assert_eq!(result.status, ScriptRunStatus::Succeeded);
    let mut streamed = String::new();
    while let Ok(event) = receiver.try_recv() {
        streamed.push_str(event.stream.as_deref().unwrap_or_default());
    }
    assert_eq!(
        streamed,
        format!("{} [redacted] 😀\nnext [redacted]", "x".repeat(8192 * 32))
    );
    assert!(!streamed.contains("fixture-secret"));
    fixture.assert_empty();
}
