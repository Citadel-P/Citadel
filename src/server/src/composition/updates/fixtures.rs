//! Compiled only into test executables, never selectable by a production setting.
use citadel_updates::{
    Release, ReleaseArtifactSource, ReleaseChannel, ReleaseSource, UpdateError, UpgradeSource,
    VerifiedReleaseArtifact,
};
use futures_util::future::BoxFuture;
use serde::Deserialize;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[derive(Deserialize)]
struct Fixture {
    target: Artifact,
    sources: Vec<UpgradeSource>,
}

#[derive(Deserialize)]
struct Artifact {
    version: String,
    image: String,
}

fn fixture() -> Result<Fixture, UpdateError> {
    let bytes = std::fs::read("/app/upgrade-fixture.json")
        .map_err(|_| UpdateError::Unavailable("Missing private update fixture.".into()))?;
    serde_json::from_slice(&bytes)
        .map_err(|_| UpdateError::Unavailable("Invalid private update fixture.".into()))
}

pub(crate) fn artifacts(
    _: Arc<citadel_processes::SystemProcess>,
) -> Result<Arc<dyn ReleaseArtifactSource>, UpdateError> {
    Ok(Arc::new(fixture()?))
}

pub(crate) fn source() -> Result<Arc<dyn ReleaseSource>, UpdateError> {
    struct Catalog;
    impl ReleaseSource for Catalog {
        fn releases(&self, _: ReleaseChannel) -> BoxFuture<'_, Result<Vec<Release>, UpdateError>> {
            Box::pin(async {
                Ok(vec![Release {
                    tag_name: format!("v{}", fixture()?.target.version),
                    draft: false,
                    prerelease: true,
                }])
            })
        }
    }
    Ok(Arc::new(Catalog))
}

pub(crate) fn sources() -> Result<Vec<UpgradeSource>, UpdateError> {
    Ok(fixture()?.sources)
}

impl ReleaseArtifactSource for Fixture {
    fn verify<'a>(
        &'a self,
        version: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<VerifiedReleaseArtifact, UpdateError>> {
        Box::pin(async move {
            let root = std::path::Path::new("/app/data");
            pause_at("preparation", root, cancellation).await?;
            if root.join("update-fixture-fail-preparation").exists() {
                return Err(UpdateError::Unavailable(
                    "Injected fixture preparation failure.".into(),
                ));
            }
            if cancellation.is_cancelled() {
                return Err(UpdateError::Cancelled);
            }
            if version != self.target.version {
                return Err(UpdateError::Unavailable("Unknown fixture release.".into()));
            }
            Ok(VerifiedReleaseArtifact {
                version: self.target.version.clone(),
                image: self.target.image.clone(),
            })
        })
    }
}

/// Test-executable-only barriers for crash tests at durable acceptance boundaries.
/// Production never compiles this module or consults these files.
pub(crate) async fn pause_at(
    boundary: &str,
    root: &std::path::Path,
    cancellation: &CancellationToken,
) -> Result<(), UpdateError> {
    let pause = root.join(format!("update-fixture-pause-{boundary}"));
    if !pause.exists() {
        return Ok(());
    }
    if boundary != "preparation" {
        std::fs::write(
            root.join(format!("update-fixture-paused-{boundary}")),
            b"ready",
        )
        .map_err(|_| UpdateError::Unavailable("Cannot publish fixture barrier.".into()))?;
    } // Helper preparation has a read-only data mount; its journal is the barrier.

    tokio::time::timeout(std::time::Duration::from_secs(600), async {
        while pause.exists() {
            tokio::select! {
                () = cancellation.cancelled() => return Err(UpdateError::Cancelled),
                () = tokio::time::sleep(std::time::Duration::from_millis(50)) => {},
            }
        }
        Ok(())
    })
    .await
    .map_err(|_| UpdateError::Unavailable("Fixture barrier timed out.".into()))?
}
