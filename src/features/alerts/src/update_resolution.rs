//! Verified workload state used to close only the update that an alert describes.
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct AppliedStackImage {
    pub service: String,
    pub image: String,
    pub digest: String,
}

#[derive(Debug, Clone)]
pub enum VerifiedUpdate {
    DeploymentImage(String),
    StackImages(Vec<AppliedStackImage>),
    StackGit(String),
}

pub fn is_update_available(alert_type: &str) -> bool {
    matches!(
        alert_type,
        "DeploymentImageUpdateAvailable" | "StackImageUpdateAvailable" | "StackGitUpdateAvailable"
    )
}

fn digest_matches(actual: &str, expected: Option<&str>) -> bool {
    !actual.trim().is_empty()
        && expected.is_some_and(|expected| {
            !expected.trim().is_empty() && citadel_primitives::image_digests_equal(actual, expected)
        })
}

impl VerifiedUpdate {
    pub const fn alert_type(&self) -> &'static str {
        match self {
            Self::DeploymentImage(_) => "DeploymentImageUpdateAvailable",
            Self::StackImages(_) => "StackImageUpdateAvailable",
            Self::StackGit(_) => "StackGitUpdateAvailable",
        }
    }

    pub fn satisfies(&self, info: &Value) -> bool {
        match self {
            Self::DeploymentImage(digest) => {
                digest_matches(digest, info.get("LatestImage").and_then(Value::as_str))
            }
            Self::StackGit(commit) => {
                !commit.trim().is_empty()
                    && info
                        .get("RemoteCommitSha")
                        .and_then(Value::as_str)
                        .is_some_and(|target| {
                            !target.trim().is_empty() && commit.eq_ignore_ascii_case(target)
                        })
            }
            Self::StackImages(images) => {
                info.get("Updates")
                    .and_then(Value::as_array)
                    .is_some_and(|updates| {
                        !updates.is_empty()
                            && updates.iter().all(|target| {
                                images.iter().any(|actual| {
                                    !actual.service.is_empty()
                                        && !actual.image.is_empty()
                                        && target.get("ServiceName").and_then(Value::as_str)
                                            == Some(actual.service.as_str())
                                        && target.get("ImageName").and_then(Value::as_str)
                                            == Some(actual.image.as_str())
                                        && digest_matches(
                                            &actual.digest,
                                            target.get("LatestDigest").and_then(Value::as_str),
                                        )
                                })
                            })
                    })
            }
        }
    }
}

#[cfg(test)]
mod tests;
