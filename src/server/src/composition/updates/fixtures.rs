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

pub(crate) fn source() -> Arc<dyn ReleaseSource> {
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
    Arc::new(Catalog)
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
