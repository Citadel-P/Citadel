//! Prove the actual installer changes only the Core image before downtime.
use super::super::registration::fingerprint_inputs;
use super::{InstallerComposeInputs, publication::PreparedCoreReplacement};
use citadel_execution::ProcessRunner;
use citadel_updates::{UpdateError, official_image_digest};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use zeroize::{Zeroize, Zeroizing};

/// Configuration evidence only. It does not authorize shutdown or replacement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckedCoreReplacement {
    pub source_configuration_sha256: String,
    pub target_configuration_sha256: String,
    pub target_image: String,
}

impl InstallerComposeInputs {
    /// Resolve both versions from private snapshots. Preserve the original files,
    /// their order and every setting except services.server.image.
    pub async fn check_replacement(
        &self,
        target_image: &str,
        process: &dyn ProcessRunner,
        cancellation: &CancellationToken,
    ) -> Result<CheckedCoreReplacement, UpdateError> {
        Ok(self
            .prepare_replacement(target_image, process, cancellation)
            .await?
            .checked)
    }

    pub(in crate::external::updates) async fn prepare_replacement(
        &self,
        target_image: &str,
        process: &dyn ProcessRunner,
        cancellation: &CancellationToken,
    ) -> Result<PreparedCoreReplacement, UpdateError> {
        if !official_image_digest(target_image) {
            return Err(unsupported());
        }
        if cancellation.is_cancelled() {
            return Err(UpdateError::Cancelled);
        }
        super::publication::check_environment(self)?;
        let mut captured = self.capture()?;
        // Capture owns credentials. Wipe the snapshots' memory after either outcome.
        let result = async {
            let source_hash = fingerprint_inputs(&self.project, &captured);
            let source = self.resolve_snapshot(&captured, process, cancellation).await?;
            self.validate_resolved(&source, source_hash.clone())?;
            let original = &captured.last().ok_or_else(unsupported)?.1;
            let environment = replace_image(original, target_image)?;
            captured.last_mut().ok_or_else(unsupported)?.1.zeroize();
            captured.last_mut().ok_or_else(unsupported)?.1 = environment.to_vec();
            let target_hash = fingerprint_inputs(&self.project, &captured);
            let mut target = self.resolve_snapshot(&captured, process, cancellation).await?;
            let checked = self.validate_resolved(&target, target_hash.clone())?;
            if checked.core_image != target_image || source["services"]["server"]["image"] == target_image {
                return Err(unsupported());
            }
            target["services"]["server"]["image"] = source["services"]["server"]["image"].clone();
            // The installer passes .env through to Core as raw environment. This
            // selector changes alongside image, but must not affect any other key.
            let source_selector = source["services"]["server"]["environment"].get("CITADEL_IMAGE");
            if let Some(environment) = target["services"]["server"]["environment"].as_object_mut()
                && environment.get("CITADEL_IMAGE").is_some_and(|value| value == target_image) {
                    match source_selector {
                        Some(value) => { environment.insert("CITADEL_IMAGE".into(), value.clone()); }
                        None => { environment.remove("CITADEL_IMAGE"); }
                    }
            }
            if source != target {
                return Err(unsupported());
            }
            if cancellation.is_cancelled() {
                return Err(UpdateError::Cancelled);
            }
            if fingerprint_inputs(&self.project, &self.capture()?) != source_hash {
                return Err(UpdateError::Conflict("Installer inputs changed during replacement preflight. Review the update again.".into()));
            }
            Ok(PreparedCoreReplacement {
                checked: CheckedCoreReplacement {
                    source_configuration_sha256: source_hash,
                    target_configuration_sha256: target_hash,
                    target_image: target_image.into(),
                },
                environment,
            })
        }.await;
        for (_, bytes) in &mut captured {
            bytes.zeroize();
        }
        result
    }
}

fn replace_image(original: &[u8], image: &str) -> Result<Zeroizing<Vec<u8>>, UpdateError> {
    let text = std::str::from_utf8(original).map_err(|_| unsupported())?;
    let mut result = Zeroizing::new(Vec::with_capacity(original.len() + image.len() + 32));
    let mut found = false;
    // capture() has already rejected duplicate assignments and interpolation.
    // Keep comments, all unrelated values, and CRLF/LF choices byte-for-byte.
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed
            .split_once('=')
            .is_some_and(|(name, _)| name == "CITADEL_IMAGE")
        {
            if found {
                return Err(unsupported());
            }
            found = true;
            result.extend_from_slice(b"CITADEL_IMAGE=");
            result.extend_from_slice(image.as_bytes());
            if line.ends_with("\r\n") {
                result.extend_from_slice(b"\r\n");
            } else if line.ends_with('\n') {
                result.push(b'\n');
            }
        } else {
            result.extend_from_slice(line.as_bytes());
        }
    }
    if !found {
        if !result.is_empty() && !result.ends_with(b"\n") {
            result.push(b'\n');
        }
        result.extend_from_slice(b"CITADEL_IMAGE=");
        result.extend_from_slice(image.as_bytes());
        result.push(b'\n');
    }
    if result.len() > 1024 * 1024 {
        return Err(unsupported());
    }
    Ok(result)
}

fn unsupported() -> UpdateError {
    UpdateError::Unavailable("The registered Compose inputs must use CITADEL_IMAGE to change only the Core image. PostgreSQL, volumes, network, credentials and other settings must remain unchanged.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_preserves_comments_credentials_and_line_endings() {
        let image = format!("ghcr.io/citadel-p/citadel@sha256:{}", "a".repeat(64));
        for (input, expected) in [
            (
                "# private\r\nPG_PASSWORD='keep # exactly'\r\nCITADEL_IMAGE=old\r\nX=y",
                format!(
                    "# private\r\nPG_PASSWORD='keep # exactly'\r\nCITADEL_IMAGE={image}\r\nX=y"
                ),
            ),
            (
                "PG_PASSWORD=keep",
                format!("PG_PASSWORD=keep\nCITADEL_IMAGE={image}\n"),
            ),
            ("CITADEL_IMAGE=old", format!("CITADEL_IMAGE={image}")),
        ] {
            assert_eq!(
                &*replace_image(input.as_bytes(), &image).unwrap(),
                expected.as_bytes()
            );
        }
        assert!(replace_image(b"CITADEL_IMAGE=a\nCITADEL_IMAGE=b\n", &image).is_err());
    }
}
