//! Durable, bounded fan-out after a successful Build. Consumer failure never
//! rewrites the outcome of the Build which produced the image.
use crate::{BuildEntitlements, BuildError, BuildLogEntry};
use citadel_domain::LicenseCapability;
use futures_util::future::BoxFuture;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildConsumerType {
    Deployment,
    Stack,
}
impl BuildConsumerType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Deployment => "Deployment",
            Self::Stack => "Stack",
        }
    }
}
#[derive(Debug, Clone)]
pub struct BuildConsumerClaim {
    pub id: Uuid,
    pub build_run_id: Uuid,
    pub resource_id: Uuid,
    pub resource_type: BuildConsumerType,
    pub expected_version: Option<i64>,
    pub redeploy: bool,
    pub service_names: Vec<String>,
}
pub trait BuildCompletionRepository: Send + Sync {
    fn claim_next(&self) -> BoxFuture<'_, Result<Option<BuildConsumerClaim>, BuildError>>;
    fn complete<'a>(
        &'a self,
        claim: &'a BuildConsumerClaim,
        message: &'a str,
    ) -> BoxFuture<'a, Result<Option<BuildLogEntry>, BuildError>>;
    fn recover(&self) -> BoxFuture<'_, Result<(), BuildError>>;
}
pub trait BuildConsumerRuntime: Send + Sync {
    fn apply<'a>(&'a self, claim: &'a BuildConsumerClaim) -> BoxFuture<'a, Result<(), BuildError>>;
    fn changed(&self, resource: BuildConsumerType, id: Uuid);
    fn log(&self, _entry: BuildLogEntry) {}
}
pub struct BuildCompletionService {
    store: Arc<dyn BuildCompletionRepository>,
    runtime: Arc<dyn BuildConsumerRuntime>,
    entitlements: Arc<dyn BuildEntitlements>,
}
impl BuildCompletionService {
    pub fn new(
        store: Arc<dyn BuildCompletionRepository>,
        runtime: Arc<dyn BuildConsumerRuntime>,
        entitlements: Arc<dyn BuildEntitlements>,
    ) -> Self {
        Self {
            store,
            runtime,
            entitlements,
        }
    }
    pub async fn process_batch(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<usize, BuildError> {
        self.store.recover().await?;
        let mut processed = 0;
        for _ in 0..25 {
            if cancellation.is_cancelled() {
                break;
            }
            let Some(claim) = self.store.claim_next().await? else {
                break;
            };
            let message = if claim.expected_version.is_none() {
                "Build consumer no longer uses this Build Project; skipped."
            } else {
                self.runtime.changed(claim.resource_type, claim.resource_id);
                if !claim.redeploy {
                    "Resolved Build image updated; Apply remains manual."
                } else if !self
                    .entitlements
                    .enabled(LicenseCapability::AutomatedOperations)
                    .await?
                {
                    "Resolved Build image updated; automatic Apply requires an active license."
                } else {
                    // Existing Apply owns its operation after dispatch. Cancellation
                    // leaves this receipt for recovery, never replays the mutation.
                    match tokio::select! {
                        () = cancellation.cancelled() => return Ok(processed),
                        result = self.runtime.apply(&claim) => result,
                    } {
                        Ok(()) => "Resolved Build image updated and automatic Apply completed.",
                        Err(_) => {
                            "Automatic Apply did not complete. Review the resource activity before retrying."
                        }
                    }
                }
            };
            if let Some(entry) = self.store.complete(&claim, message).await? {
                self.runtime.log(entry);
            }
            processed += 1;
        }
        Ok(processed)
    }
}

#[cfg(test)]
#[path = "completion_tests.rs"]
mod tests;
