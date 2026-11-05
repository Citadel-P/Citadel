//! Private checkpoint key identities; never included in public update views or logs.
use super::super::{JournalDirectory, PreparedUpdateCheckpoint, recovery_keys::read_key};
use super::evidence::conflict;
use base64::{Engine, engine::general_purpose::STANDARD};
use citadel_updates::UpdateError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TargetRecoveryKeys {
    jwt: Option<String>,
    encryption: Option<String>,
    agent_signing: String,
}

impl std::fmt::Debug for TargetRecoveryKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TargetRecoveryKeys([REDACTED])")
    }
}

impl TargetRecoveryKeys {
    pub(super) fn from_checkpoint(
        state: &JournalDirectory,
        receipt: &PreparedUpdateCheckpoint,
    ) -> Result<Self, UpdateError> {
        let stage = JournalDirectory::open(
            state
                .path()
                .join(format!("checkpoint-{}", receipt.operation_id)),
        )?;
        if receipt.bundle != stage.path().join(receipt.operation_id.simple().to_string())
            || stage.read::<PreparedUpdateCheckpoint>("checkpoint.json")? != *receipt
        {
            return Err(conflict());
        }
        // Read private, bounded checksum metadata. The checkpoint receipt already
        // bound its exact bytes before replacement; never trust a fresh manifest.
        let bytes = read_key(&receipt.bundle.join("checksums.json"))?;
        if digest(&bytes) != receipt.bundle_checksums_sha256 {
            return Err(conflict());
        }
        let sums: BTreeMap<String, String> =
            serde_json::from_slice(&bytes).map_err(|_| conflict())?;
        let external = &receipt.required_external_configuration;
        if external.len() > 2
            || external.iter().any(|name| {
                !matches!(
                    name.as_str(),
                    "CITADEL_JWT_KEY" | "CITADEL_SECRETS_ENCRYPTION_KEY"
                )
            })
            || external
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != external.len()
        {
            return Err(conflict());
        }
        let key = |name: &str, setting: &str| -> Result<Option<String>, UpdateError> {
            let path = format!("recovery/{name}");
            let expected = sums.get(&path);
            if external.iter().any(|name| name == setting) {
                if expected.is_some() {
                    return Err(conflict());
                }
                return Ok(None);
            }
            let expected = expected.ok_or_else(conflict)?;
            if !citadel_updates::is_sha256(expected)
                || digest(&read_key(&receipt.bundle.join(&path))?) != *expected
            {
                return Err(conflict());
            }
            Ok(Some(expected.clone()))
        };
        Ok(Self {
            jwt: key("jwtsecret", "CITADEL_JWT_KEY")?,
            encryption: key("secret-encryption-key", "CITADEL_SECRETS_ENCRYPTION_KEY")?,
            agent_signing: key("agent/signing-key", "")?.ok_or_else(conflict)?,
        })
    }

    /// Compare the retained files and the keys actually loaded by this Core.
    /// Environment-backed keys are bound by the verified unchanged Compose
    /// environment. File-backed keys must still equal the checkpoint, even when
    /// there are no encrypted rows that could reveal a replacement key.
    pub fn verify_loaded(
        &self,
        root: &Path,
        jwt: &[u8],
        encryption: &[u8],
        jwt_external: bool,
        encryption_external: bool,
    ) -> Result<(), UpdateError> {
        if jwt_external != self.jwt.is_none() || encryption_external != self.encryption.is_none() {
            return Err(conflict());
        }
        if let Some(expected) = &self.jwt {
            let bytes = read_key(&root.join("jwtsecret"))?;
            if digest(&bytes) != *expected || bytes.as_slice() != jwt {
                return Err(conflict());
            }
        }
        if let Some(expected) = &self.encryption {
            let bytes = read_key(&root.join("secret-encryption-key"))?;
            let encoded = std::str::from_utf8(&bytes).map_err(|_| conflict())?;
            let decoded =
                zeroize::Zeroizing::new(STANDARD.decode(encoded.trim()).map_err(|_| conflict())?);
            if digest(&bytes) != *expected || decoded.as_slice() != encryption {
                return Err(conflict());
            }
        }
        let signing = read_key(&root.join("agent/signing-key"))?;
        if signing.len() != 32 || digest(&signing) != self.agent_signing {
            return Err(conflict());
        }
        Ok(())
    }
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn checkpoint_key_identity_rejects_replacement_and_stale_loaded_material() {
        let root = tempfile::Builder::new()
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir()
            .unwrap();
        std::fs::create_dir(root.path().join("agent")).unwrap();
        let jwt = b"original-jwt-material-at-least-32-bytes";
        let encryption = [7; 32];
        let encoded = STANDARD.encode(encryption);
        let signing = [8; 32];
        let keys = TargetRecoveryKeys {
            jwt: Some(digest(jwt)),
            encryption: Some(digest(encoded.as_bytes())),
            agent_signing: digest(&signing),
        };
        let write = |name: &str, bytes: &[u8]| {
            let path = root.path().join(name);
            std::fs::write(&path, bytes).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        };
        for (name, bytes) in [
            ("jwtsecret", jwt.as_slice()),
            ("secret-encryption-key", encoded.as_bytes()),
            ("agent/signing-key", signing.as_slice()),
        ] {
            write(name, bytes);
        }
        keys.verify_loaded(root.path(), jwt, &encryption, false, false)
            .unwrap();
        assert!(
            keys.verify_loaded(
                root.path(),
                b"a different but well-formed 32-byte JWT key",
                &encryption,
                false,
                false
            )
            .is_err()
        );
        assert!(
            keys.verify_loaded(root.path(), jwt, &[9; 32], false, false)
                .is_err()
        );
        assert!(
            keys.verify_loaded(root.path(), jwt, &encryption, true, false)
                .is_err()
        );
        for (name, original, replacement) in [
            (
                "jwtsecret",
                jwt.as_slice(),
                b"another-valid-32-byte-key-material".as_slice(),
            ),
            (
                "secret-encryption-key",
                encoded.as_bytes(),
                STANDARD.encode([9; 32]).as_bytes(),
            ),
            ("agent/signing-key", signing.as_slice(), [9; 32].as_slice()),
        ] {
            write(name, replacement);
            assert!(
                keys.verify_loaded(root.path(), jwt, &encryption, false, false)
                    .is_err(),
                "{name}"
            );
            write(name, original);
        }
        keys.verify_loaded(root.path(), jwt, &encryption, false, false)
            .unwrap();
        assert_eq!(format!("{keys:?}"), "TargetRecoveryKeys([REDACTED])");
    }
}
