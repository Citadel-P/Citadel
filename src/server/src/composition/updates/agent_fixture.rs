//! Private release source for the browser/acceptance executable only.
//! All authenticated transport, replacement and reconnect checks remain real.
use citadel_updates::*;
use futures_util::future::BoxFuture;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub(super) struct Runtime(pub Arc<dyn AgentUpdateRuntime>);

impl AgentUpdateRuntime for Runtime {
    fn release<'a>(
        &'a self,
        version: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<AgentUpdateRelease, UpdateError>> {
        Box::pin(async move {
            if cancel.is_cancelled() {
                return Err(UpdateError::Cancelled);
            }
            #[derive(serde::Deserialize)]
            struct Fixture {
                agent: Option<AgentUpdateRelease>,
            }
            let bytes = std::fs::read("/app/upgrade-fixture.json")
                .map_err(|_| UpdateError::Unavailable("Missing private update fixture.".into()))?;
            let fixture: Fixture = serde_json::from_slice(&bytes)
                .map_err(|_| UpdateError::Unavailable("Invalid private update fixture.".into()))?;
            fixture
                .agent
                .filter(|r| r.version == version)
                .ok_or_else(|| {
                    UpdateError::Unavailable("No Agent release in the private fixture.".into())
                })
        })
    }

    fn targets<'a>(
        &'a self,
        selected: Option<&'a [String]>,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<AgentUpdateTarget>, UpdateError>> {
        self.0.targets(selected, cancel)
    }

    fn install<'a>(
        &'a self,
        target: &'a AgentUpdateTarget,
        release: &'a AgentUpdateRelease,
        operation: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        self.0.install(target, release, operation, cancel)
    }

    fn verify<'a>(
        &'a self,
        target: &'a AgentUpdateTarget,
        release: &'a AgentUpdateRelease,
        operation: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<bool, UpdateError>> {
        self.0.verify(target, release, operation, cancel)
    }

    fn resume<'a>(
        &'a self,
        target: &'a AgentUpdateTarget,
        operation: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        self.0.resume(target, operation, cancel)
    }
}
