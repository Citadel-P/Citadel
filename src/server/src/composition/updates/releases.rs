//! Production release dependencies shared by HTTP review and the detached helper.
use citadel_adapters::external::updates::{GithubReleaseArtifacts, GithubReleaseSource};
use citadel_updates::{ReleaseArtifactSource, ReleaseSource, UpdateError, UpgradeSource};
use std::sync::Arc;

pub(crate) fn artifacts(
    process: Arc<citadel_processes::SystemProcess>,
) -> Result<Arc<dyn ReleaseArtifactSource>, UpdateError> {
    Ok(Arc::new(GithubReleaseArtifacts::new(process)?))
}

pub(crate) fn source() -> Result<Arc<dyn ReleaseSource>, UpdateError> {
    Ok(Arc::new(GithubReleaseSource::new()?))
}

pub(crate) fn sources() -> Result<Vec<UpgradeSource>, UpdateError> {
    serde_json::from_str(include_str!("../../../upgrade-sources.json"))
        .map_err(|_| UpdateError::Unavailable("Invalid embedded upgrade source metadata.".into()))
}
