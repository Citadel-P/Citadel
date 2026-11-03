use super::super::files::open_secure;
use base64::{Engine, engine::general_purpose::STANDARD};
use citadel_updates::UpdateError;
use serde::{Deserialize, Serialize};
use std::{io::Read, os::unix::fs::MetadataExt, path::Path};
use zeroize::Zeroizing;

const MAX_KEY_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryKeySource {
    DataVolume,
    Environment,
}

/// Availability and encoding checks only, not a backup or proof that stored
/// ciphertext can be decrypted. No key bytes or key fingerprints leave Core.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckedRecoveryKeys {
    pub jwt: RecoveryKeySource,
    pub secrets_encryption: RecoveryKeySource,
    pub agent_signing: RecoveryKeySource,
}

pub fn check_recovery_keys(
    data_root: &Path,
    jwt: Option<&str>,
    encryption: Option<&str>,
) -> Result<CheckedRecoveryKeys, UpdateError> {
    let (jwt_source, jwt) = load(data_root, "jwtsecret", jwt)?;
    // Match startup: JWT is UTF-8 key material, not decoded Base64.
    if std::str::from_utf8(&jwt).is_err() || jwt.len() < 32 {
        return Err(unavailable());
    }
    let (encryption_source, encryption) = load(data_root, "secret-encryption-key", encryption)?;
    let encoded = std::str::from_utf8(&encryption).map_err(|_| unavailable())?;
    let decoded = Zeroizing::new(STANDARD.decode(encoded.trim()).map_err(|_| unavailable())?);
    if decoded.len() != 32 {
        return Err(unavailable());
    }
    // Core's local connector persists a raw 32-byte Ed25519 seed. Remote Core
    // Agent configurations are outside the qualified Compose installation profile.
    let signing = read_key(&data_root.join("agent/signing-key"))?;
    if signing.len() != 32 {
        return Err(unavailable());
    }
    Ok(CheckedRecoveryKeys {
        jwt: jwt_source,
        secrets_encryption: encryption_source,
        agent_signing: RecoveryKeySource::DataVolume,
    })
}

fn load(
    root: &Path,
    file: &str,
    configured: Option<&str>,
) -> Result<(RecoveryKeySource, Zeroizing<Vec<u8>>), UpdateError> {
    match configured.filter(|value| !value.trim().is_empty()) {
        Some(value) if value.len() <= MAX_KEY_BYTES => Ok((
            RecoveryKeySource::Environment,
            Zeroizing::new(value.as_bytes().to_vec()),
        )),
        Some(_) => Err(unavailable()),
        None => Ok((RecoveryKeySource::DataVolume, read_key(&root.join(file))?)),
    }
}

fn read_key(path: &Path) -> Result<Zeroizing<Vec<u8>>, UpdateError> {
    let mut file = open_secure(path, false).map_err(|_| unavailable())?;
    let before = file.metadata().map_err(|_| unavailable())?;
    if before.mode() & 0o077 != 0 || before.len() > MAX_KEY_BYTES as u64 {
        return Err(unavailable());
    }
    let mut bytes = Zeroizing::new(Vec::new());
    file.by_ref()
        .take(MAX_KEY_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| unavailable())?;
    let after = open_secure(path, false)
        .and_then(|file| file.metadata().map_err(|_| unavailable()))
        .map_err(|_| unavailable())?;
    if bytes.len() > MAX_KEY_BYTES
        || before.dev() != after.dev()
        || before.ino() != after.ino()
        || before.len() != after.len()
        || before.mtime() != after.mtime()
        || before.mtime_nsec() != after.mtime_nsec()
        || before.ctime() != after.ctime()
        || before.ctime_nsec() != after.ctime_nsec()
        || before.mode() != after.mode()
    {
        return Err(unavailable());
    }
    Ok(bytes)
}

fn unavailable() -> UpdateError {
    UpdateError::Unavailable(
        "Recovery keys are missing, malformed, changed or unsafe. Preserve the original JWT, secret-encryption and Agent signing keys before updating.".into(),
    )
}
