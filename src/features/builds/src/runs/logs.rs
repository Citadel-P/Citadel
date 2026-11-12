use crate::BuildError;
use crate::BuildLog;
use crate::BuildRepository;
use chrono::{DateTime, Utc};
use futures_util::future::BoxFuture;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct BuildLogEntry {
    pub id: Uuid,
    pub build_run_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub stream: String,
    pub message: String,
}

pub type BuildLogNotifier = Arc<dyn Fn(Uuid, BuildLogEntry) + Send + Sync>;

/// Accepts only redacted output. Implementations persist before notifying clients.
pub trait BuildLogSink: Send + Sync {
    fn append<'a>(
        &'a self,
        stream: &'a str,
        message: &'a str,
    ) -> BoxFuture<'a, Result<(), BuildError>>;
    fn append_batch<'a>(&'a self, logs: &'a [BuildLog]) -> BoxFuture<'a, Result<(), BuildError>> {
        Box::pin(async move {
            for log in logs {
                self.append(&log.stream, &log.message).await?;
            }
            Ok(())
        })
    }
}

pub struct NoopBuildLogSink;
impl BuildLogSink for NoopBuildLogSink {
    fn append<'a>(&'a self, _: &'a str, _: &'a str) -> BoxFuture<'a, Result<(), BuildError>> {
        Box::pin(async { Ok(()) })
    }
}

pub(crate) struct PersistedBuildLogs {
    store: Arc<dyn BuildRepository>,
    run: Uuid,
    attempt_id: Uuid,
    notifier: Option<BuildLogNotifier>,
    remaining: AtomicUsize,
}

impl PersistedBuildLogs {
    pub fn new(
        store: Arc<dyn BuildRepository>,
        run: Uuid,
        attempt_id: Uuid,
        notifier: Option<BuildLogNotifier>,
    ) -> Self {
        Self {
            store,
            run,
            attempt_id,
            notifier,
            remaining: AtomicUsize::new(4 * 1024 * 1024),
        }
    }
}

impl BuildLogSink for PersistedBuildLogs {
    fn append<'a>(
        &'a self,
        stream: &'a str,
        message: &'a str,
    ) -> BoxFuture<'a, Result<(), BuildError>> {
        Box::pin(async move { self.append_parts(&[(stream, message)]).await })
    }
    fn append_batch<'a>(&'a self, logs: &'a [BuildLog]) -> BoxFuture<'a, Result<(), BuildError>> {
        Box::pin(async move {
            let parts: Vec<_> = logs
                .iter()
                .map(|log| (log.stream.as_str(), log.message.as_str()))
                .collect();
            self.append_parts(&parts).await
        })
    }
}
impl PersistedBuildLogs {
    async fn append_parts(&self, logs: &[(&str, &str)]) -> Result<(), BuildError> {
        if logs
            .iter()
            .any(|(stream, _)| !matches!(*stream, "stdout" | "stderr" | "system"))
        {
            return Err(BuildError::Validation("Invalid Build log stream.".into()));
        }
        let mut batch = Vec::new();
        let mut bytes = 0;
        for &(stream, mut remaining) in logs {
            while !remaining.is_empty() {
                let end = remaining.floor_char_boundary(8192.min(remaining.len()));
                let part = &remaining[..end];
                remaining = &remaining[end..];
                let cost = part.len() + 128;
                if self
                    .remaining
                    .try_update(Ordering::Relaxed, Ordering::Relaxed, |left| {
                        left.checked_sub(cost)
                    })
                    .is_err()
                {
                    return self.persist(&batch).await;
                }
                if bytes + cost > 64 * 1024 || batch.len() >= 128 {
                    self.persist(&batch).await?;
                    batch.clear();
                    bytes = 0;
                }
                batch.push(BuildLog {
                    stream: stream.into(),
                    message: part.replace('\0', ""),
                });
                bytes += cost;
            }
        }
        self.persist(&batch).await
    }
    async fn persist(&self, logs: &[BuildLog]) -> Result<(), BuildError> {
        if logs.is_empty() {
            return Ok(());
        }
        let entries = self
            .store
            .append_logs(self.run, self.attempt_id, logs)
            .await?;
        if let Some(notify) = &self.notifier {
            for entry in entries {
                notify(self.run, entry);
            }
        }
        Ok(())
    }
}
