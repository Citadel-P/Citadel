//! Owned temporary caches for backup subprocesses.
use super::*;
use citadel_scripts_runtime::workspace_cleanup::DirectoryCleanup;

impl ResticOperation<'_> {
    pub(super) async fn run_observed(
        &self,
        mut request: ProcessRequest,
        cancellation: &CancellationToken,
    ) -> Result<citadel_execution::ProcessOutput, citadel_execution::ProcessError> {
        if let Some(settings) = &self.executor.settings {
            tokio::fs::create_dir_all(&settings.working_directory)
                .await
                .map_err(citadel_execution::ProcessError::Io)?;
            if request.current_directory.is_none() {
                request.current_directory = Some(settings.working_directory.clone());
            }
        }
        // Restic cache and Docker CLI configuration belong to this invocation,
        // never to the Core account's home directory.
        let root = request
            .current_directory
            .clone()
            .unwrap_or_else(std::env::temp_dir)
            .join(format!("citadel-backup-process-{}", Uuid::now_v7()));
        let reservation = DirectoryCleanup::reserve(&self.executor.cleanup_tasks)
            .map_err(citadel_execution::ProcessError::Io)?;
        let path = root.clone();
        let mut cleanup = tokio::task::spawn_blocking(move || {
            let mut directory = std::fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                directory.mode(0o700);
            }
            directory.create(&path)?;
            Ok::<_, std::io::Error>(DirectoryCleanup::new(path, reservation))
        })
        .await
        .map_err(|error| citadel_execution::ProcessError::Io(std::io::Error::other(error)))?
        .map_err(citadel_execution::ProcessError::Io)?;
        request = request
            .env("HOME", root.as_os_str())
            .env("TMPDIR", root.as_os_str())
            .env("RESTIC_CACHE_DIR", root.join("restic-cache").as_os_str())
            .env("DOCKER_CONFIG", root.join("docker").as_os_str());
        if request.program == self.executor.docker {
            for key in ["DOCKER_HOST", "DOCKER_TLS_VERIFY", "DOCKER_CERT_PATH"] {
                if !request.environment.iter().any(|(name, _)| name == key)
                    && let Some(value) = std::env::var_os(key)
                {
                    request = request.env(key, value);
                }
            }
        }
        let result = if let Some(progress) = &self.progress {
            let (sender, receiver) = tokio::sync::mpsc::channel(32);
            let (result, ()) = tokio::join!(
                run(request.output(sender), cancellation),
                observe_output(
                    receiver,
                    progress,
                    self.executor.log_line_limit(),
                    self.executor.maximum_output
                )
            );
            result
        } else {
            run(request, cancellation).await
        };
        if let Err(error) = cleanup.cleanup().await {
            // Cache cleanup cannot turn a completed mutation into retryable work.
            // The guard retains tracked cleanup ownership for another attempt.
            tracing::warn!(%error, "Backup process workspace cleanup failed");
        }
        result
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    struct NoSecrets;
    impl BackupSecretResolver for NoSecrets {
        fn resolve(
            &self,
            _: Uuid,
            _: Uuid,
        ) -> BoxFuture<'_, Result<zeroize::Zeroizing<String>, BackupError>> {
            panic!("process fixture must not resolve secrets")
        }
    }
    fn executor(
        root: &std::path::Path,
        tasks: citadel_runtime::DynamicTasks,
    ) -> DockerResticBackupExecutor {
        DockerResticBackupExecutor::new(
            tasks,
            "unused-docker",
            "unused-image",
            Arc::new(NoSecrets),
            1024,
            sqlx::postgres::PgPoolOptions::new()
                .connect_lazy("postgres://unused@127.0.0.1:1/unused")
                .unwrap(),
        )
        .with_settings(super::super::super::settings::BackupExecutionSettings {
            working_directory: root.into(),
            allowed_core_paths: vec![],
            default_timeout: Duration::from_secs(5),
            maximum_log_line_bytes: 1024,
        })
    }
    fn operation(
        executor: &DockerResticBackupExecutor,
        ready: Option<Arc<tokio::sync::Notify>>,
    ) -> ResticOperation<'_> {
        let progress = ready.map(|ready| {
            Arc::new(move |_: BackupLog| ready.notify_one()) as Arc<dyn Fn(BackupLog) + Send + Sync>
        });
        ResticOperation::new(
            executor,
            OperationOwner {
                repository: Uuid::now_v7(),
                run: Uuid::now_v7(),
                attempt: Uuid::now_v7(),
            },
            false,
            progress,
        )
    }
    fn request(tail: &str) -> ProcessRequest {
        ProcessRequest::new("/bin/sh").args(["-c", &format!("mkdir -p \"$RESTIC_CACHE_DIR\"; printf cached > \"$RESTIC_CACHE_DIR/fixture\"; printf 'ready\\n'; {tail}")])
    }
    fn assert_empty(root: &std::path::Path) {
        assert_eq!(std::fs::read_dir(root).unwrap().count(), 0);
    }
    #[tokio::test]
    async fn backup_process_cache_cleanup_covers_success_failure_and_cancellation() {
        let root = tempfile::tempdir().unwrap();
        let tasks = citadel_runtime::DynamicTasks::new(CancellationToken::new());
        let executor = executor(root.path(), tasks.clone());
        for code in [0, 7] {
            let output = operation(&executor, None)
                .run_observed(request(&format!("exit {code}")), &CancellationToken::new())
                .await
                .unwrap();
            assert_eq!(output.exit_code, Some(code));
            assert_empty(root.path());
            assert_eq!(tasks.active(), 0);
        }
        let ready = Arc::new(tokio::sync::Notify::new());
        let operation = operation(&executor, Some(ready.clone()));
        let cancellation = CancellationToken::new();
        let cancel = async {
            ready.notified().await;
            cancellation.cancel();
        };
        let (result, ()) = tokio::time::timeout(Duration::from_secs(5), async {
            tokio::join!(
                operation.run_observed(request("sleep 30"), &cancellation),
                cancel
            )
        })
        .await
        .unwrap();
        assert!(matches!(
            result,
            Err(citadel_execution::ProcessError::Cancelled)
        ));
        assert_empty(root.path());
        tasks.drain(Duration::from_secs(5)).await.unwrap();
    }
    #[tokio::test]
    async fn dropped_backup_process_keeps_cache_cleanup_tracked() {
        let root = tempfile::tempdir().unwrap();
        let tasks = citadel_runtime::DynamicTasks::new(CancellationToken::new());
        let executor = executor(root.path(), tasks.clone());
        let ready = Arc::new(tokio::sync::Notify::new());
        let operation = operation(&executor, Some(ready.clone()));
        let cancellation = CancellationToken::new();
        {
            let execution = operation.run_observed(request("sleep 30"), &cancellation);
            tokio::pin!(execution);
            tokio::select! {
                _ = &mut execution => panic!("fixture exited early"),
                () = ready.notified() => {},
                () = tokio::time::sleep(Duration::from_secs(5)) => panic!("fixture did not start"),
            }
            assert_eq!(tasks.active(), 1);
        }
        tasks.drain(Duration::from_secs(5)).await.unwrap();
        assert_empty(root.path());
    }
    #[tokio::test]
    async fn shutdown_refuses_new_backup_process_workspaces() {
        let root = tempfile::tempdir().unwrap();
        let tasks = citadel_runtime::DynamicTasks::new(CancellationToken::new());
        let executor = executor(root.path(), tasks.clone());
        tasks.drain(Duration::from_secs(1)).await.unwrap();
        let result = operation(&executor, None)
            .run_observed(request("exit 0"), &CancellationToken::new())
            .await;
        assert!(matches!(
            result,
            Err(citadel_execution::ProcessError::Io(_))
        ));
        assert_empty(root.path());
    }
}
