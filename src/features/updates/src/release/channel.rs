#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseChannel {
    Stable,
    Development,
}

impl ReleaseChannel {
    pub fn for_version(version: &str) -> Self {
        if semver::Version::parse(version.trim_start_matches('v'))
            .is_ok_and(|v| crate::is_development(&v))
        {
            Self::Development
        } else {
            Self::Stable
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Development => "development",
        }
    }
}
