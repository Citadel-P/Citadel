use crate::{BuildError, BuildLog, BuildStore};
use chrono::{DateTime, Utc};
use futures_util::future::BoxFuture;
use serde::Serialize;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
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
}

pub struct NoopBuildLogSink;
impl BuildLogSink for NoopBuildLogSink {
    fn append<'a>(&'a self, _: &'a str, _: &'a str) -> BoxFuture<'a, Result<(), BuildError>> {
        Box::pin(async { Ok(()) })
    }
}

pub(crate) struct PersistedBuildLogs {
    store: Arc<dyn BuildStore>,
    run: Uuid,
    notifier: Option<BuildLogNotifier>,
    remaining: AtomicUsize,
}

impl PersistedBuildLogs {
    pub fn new(store: Arc<dyn BuildStore>, run: Uuid, notifier: Option<BuildLogNotifier>) -> Self {
        Self {
            store,
            run,
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
        Box::pin(async move {
            if !matches!(stream, "stdout" | "stderr" | "system") {
                return Err(BuildError::Validation("Invalid Build log stream.".into()));
            }
            // A noisy child must still be drained after the durable log budget
            // is exhausted. Count metadata as well as message bytes.
            let mut remaining = message;
            while !remaining.is_empty() {
                let mut end = remaining.len().min(8192);
                while !remaining.is_char_boundary(end) {
                    end -= 1;
                }
                let part = &remaining[..end];
                remaining = &remaining[end..];
                if self
                    .remaining
                    .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |left| {
                        left.checked_sub(part.len() + 128)
                    })
                    .is_err()
                {
                    return Ok(());
                }
                let log = BuildLog {
                    stream: stream.into(),
                    message: part.replace('\0', ""),
                };
                let entry = self.store.append_log(self.run, &log).await?;
                if let Some(notify) = &self.notifier {
                    notify(self.run, entry);
                }
            }
            Ok(())
        })
    }
}
