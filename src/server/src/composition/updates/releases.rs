//! Production release dependencies shared by HTTP review and the detached helper.
use citadel_adapters::external::updates::{GithubReleaseArtifacts, GithubReleaseSource};
use citadel_updates::{ReleaseArtifactSource, ReleaseSource, UpdateError};
use std::sync::Arc;

pub(crate) fn artifacts(
    process: Arc<citadel_processes::SystemProcess>,
) -> Result<Arc<dyn ReleaseArtifactSource>, UpdateError> {
    Ok(Arc::new(GithubReleaseArtifacts::new(process)?))
}

pub(crate) fn source() -> Result<Arc<dyn ReleaseSource>, UpdateError> {
    Ok(Arc::new(GithubReleaseSource::new()?))
}
