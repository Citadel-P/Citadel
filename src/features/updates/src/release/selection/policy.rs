use crate::ReleaseNotice;
use crate::UpdateError;
use semver::Version;

#[derive(Debug, Clone)]
pub struct Release {
    pub tag_name: String,
    pub draft: bool,
    pub prerelease: bool,
}

pub fn is_development(version: &Version) -> bool {
    version.pre.as_str().starts_with("dev.")
}

pub fn available_update(
    current: &str,
    release: Release,
) -> Result<Option<ReleaseNotice>, UpdateError> {
    let current = Version::parse(current.trim_start_matches('v'))
        .map_err(|_| UpdateError::Validation("Invalid installed release version.".into()))?;
    let latest = Version::parse(
        release
            .tag_name
            .strip_prefix('v')
            .unwrap_or(&release.tag_name),
    )
    .map_err(|_| UpdateError::Validation("Invalid candidate release version.".into()))?;
    let same_channel = if is_development(&current) {
        release.prerelease && is_development(&latest)
    } else {
        !release.prerelease && latest.pre.is_empty()
    };
    if release.draft || !same_channel || !latest.cmp_precedence(&current).is_gt() {
        return Ok(None);
    }
    Ok(Some(ReleaseNotice {
        version: latest.to_string(),
        // Never trust URLs supplied by release metadata.
        release_url: format!(
            "https://github.com/Citadel-P/Citadel/releases/tag/{}",
            release.tag_name
        ),
    }))
}

pub fn development_update(
    current: &str,
    releases: Vec<Release>,
) -> Result<Option<ReleaseNotice>, UpdateError> {
    Version::parse(current.trim_start_matches('v'))
        .map_err(|_| UpdateError::Validation("Invalid installed release version.".into()))?;
    // GitHub orders by creation time, which need not match version precedence.
    // Ignore unrelated/non-version tags without hiding a valid newer release.
    Ok(releases
        .into_iter()
        .filter_map(|release| available_update(current, release).ok().flatten())
        .filter_map(|update| Version::parse(&update.version).ok().map(|v| (v, update)))
        .max_by(|(a, _), (b, _)| a.cmp_precedence(b))
        .map(|(_, update)| update))
}
