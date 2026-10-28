//! The same bounded Deno sandbox is used on Core and Agents.
use crate::runs::{logs::*, progress};
use crate::{ScriptProgress, ScriptRunResult};
use citadel_execution::{
    OutputLimitPolicy, ProcessError, ProcessLimits, ProcessRequest, ProcessRunner,
};
use std::{ffi::OsString, path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub struct ScriptSandbox {
    pub process: Arc<dyn ProcessRunner>,
    pub workspace: Arc<dyn crate::workspace::ScriptWorkspacePort>,
    pub deno_path: OsString,
    pub work_root: PathBuf,
    pub cache_directory: Option<PathBuf>,
    pub allow_net: Option<String>,
}

// No Debug: source and token contain run credentials.
pub struct ScriptExecution {
    pub run_id: Uuid,
    pub source: String,
    pub token: String,
    pub args_json: String,
    pub base_url: String,
    pub timeout_seconds: i32,
    pub maximum_log_bytes: usize,
}

impl ScriptSandbox {
    pub async fn execute(
        &self,
        input: ScriptExecution,
        cancellation: &CancellationToken,
        progress: Option<&broadcast::Sender<ScriptProgress>>,
    ) -> ScriptRunResult {
        let directory = self.work_root.join(input.run_id.to_string());
        let mut workspace = match self
            .workspace
            .prepare(&directory, self.cache_directory.as_deref(), &input.source)
            .await
        {
            Ok(workspace) => workspace,
            Err(error) => return ScriptRunResult::failed(None, error),
        };
        let directory = workspace.directory();
        let script = workspace.entrypoint();
        let mut args = vec![
            OsString::from("run"),
            OsString::from("--no-prompt"),
            // Do not discover operator/parent-workspace configuration or locks.
            OsString::from("--no-config"),
            OsString::from("--no-lock"),
            OsString::from("--no-npm"),
            OsString::from(format!("--allow-read={}", directory.display())),
            OsString::from(format!("--allow-write={}", directory.display())),
            OsString::from("--allow-env=NO_COLOR,DENO_DIR"),
        ];
        let default_net = allow_net_authority(&input.base_url);
        let allow_net = self.allow_net.as_deref().unwrap_or(&default_net).trim();
        if !allow_net.is_empty() {
            args.push(format!("--allow-net={allow_net}").into());
            // Module imports have a separate default allowlist in Deno.
            args.push(format!("--allow-import={allow_net}").into());
        } else {
            args.push("--deny-import".into());
        }
        args.push(script.as_os_str().to_owned());
        let mut request = ProcessRequest::new(self.deno_path.clone())
            .args(args)
            .current_dir(directory)
            .env("HOME", directory.as_os_str())
            .env("TMPDIR", directory.as_os_str())
            .env("DENO_DIR", directory.join("deno-cache").as_os_str())
            .env("NO_COLOR", "1")
            .env("DENO_TLS_CA_STORE", "mozilla,system")
            .env("CITADEL_SCRIPT_ARGS", &input.args_json)
            .limits(ProcessLimits {
                timeout: Duration::from_secs(input.timeout_seconds as u64),
                maximum_stdout_bytes: input.maximum_log_bytes,
                maximum_stderr_bytes: input.maximum_log_bytes,
                output_limit_policy: OutputLimitPolicy::Truncate,
            });
        if let Some(cache) = &self.cache_directory {
            request = request.env("DENO_DIR", cache.as_os_str());
        }
        let output = if let Some(sender) = progress {
            let (chunks, mut receiver) = mpsc::channel(8);
            let consume = async {
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                while let Some(chunk) = receiver.recv().await {
                    let chunk: citadel_execution::ProcessChunk = chunk;
                    let pending = if chunk.stream == "stderr" {
                        &mut stderr
                    } else {
                        &mut stdout
                    };
                    pending.extend_from_slice(&chunk.bytes);
                    if let Some(end) = pending.iter().rposition(|byte| *byte == b'\n') {
                        let text = redact_run_logs(
                            &String::from_utf8_lossy(&pending[..=end]),
                            &input.token,
                        );
                        pending.drain(..=end);
                        progress::send(
                            sender,
                            ScriptProgress {
                                run_id: Some(input.run_id),
                                stream: Some(text),
                                ..Default::default()
                            },
                            cancellation,
                        )
                        .await;
                    }
                }
                for pending in [stdout, stderr] {
                    if !pending.is_empty() {
                        let text =
                            redact_run_logs(&String::from_utf8_lossy(&pending), &input.token);
                        progress::send(
                            sender,
                            ScriptProgress {
                                run_id: Some(input.run_id),
                                stream: Some(text),
                                ..Default::default()
                            },
                            cancellation,
                        )
                        .await;
                    }
                }
            };
            let (output, ()) = tokio::join!(
                self.process.run(request.output(chunks), cancellation),
                consume
            );
            output
        } else {
            self.process.run(request, cancellation).await
        };
        let _ = workspace.cleanup().await;
        match output {
            Ok(output) => {
                let logs = redact_run_logs(
                    &joined_logs(
                        &output.stdout,
                        &output.stderr,
                        output.stdout_truncated || output.stderr_truncated,
                    ),
                    &input.token,
                );
                if output.succeeded() {
                    ScriptRunResult::success(output.exit_code, logs)
                } else {
                    let mut result = ScriptRunResult::failed(output.exit_code, logs);
                    result.error = Some(format!(
                        "Deno exited with code {}.",
                        output
                            .exit_code
                            .map_or_else(|| "unknown".into(), |code| code.to_string())
                    ));
                    result
                }
            }
            Err(ProcessError::Cancelled) => ScriptRunResult::cancelled(),
            Err(ProcessError::Timeout(_)) => ScriptRunResult::timed_out(),
            Err(error) => ScriptRunResult::failed(None, error.to_string()),
        }
    }
}
