//! Durable Edge enrollment state. Invalid files fail closed and remain untouched.
use crate::config::{EdgeConfig, EdgeProfile};
use citadel_contracts::citadel::edge::v1::SessionAccepted;
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;
use zeroize::Zeroizing;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(super) struct Identity {
    pub platform_id: Uuid,
    pub resource_type: String,
    #[serde(default)]
    pub resource_id: Uuid,
    pub agent_id: Uuid,
    pub agent_fingerprint: String,
    #[serde(default)]
    pub enrollment_token_fingerprint: Option<String>,
}

pub(super) struct State {
    pub key: SigningKey,
    pub identity: Option<Identity>,
    // The fingerprint belongs to the credential actually sent for enrollment.
    pub token_fingerprint: Option<String>,
}

pub(super) fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

pub(super) fn fingerprint(bytes: &[u8]) -> String {
    let hex: String = Sha256::digest(bytes)
        .iter()
        .map(|v| format!("{v:02x}"))
        .collect();
    format!("SHA256:{hex}")
}

pub(super) fn credential(config: &EdgeConfig) -> io::Result<Option<Zeroizing<String>>> {
    match &config.profile {
        EdgeProfile::SwarmNode(node) => {
            // Docker Swarm rotates the mounted bootstrap credential independently.
            let Some(bytes) = read(&node.bootstrap_file, 64 * 1024)? else {
                return Ok(None);
            };
            let text = std::str::from_utf8(&bytes)
                .map_err(|_| invalid("Invalid Edge bootstrap credential encoding"))?;
            Ok((!text.trim().is_empty()).then(|| Zeroizing::new(text.trim().to_owned())))
        }
        _ => Ok(config.enrollment_token.clone()),
    }
}

impl State {
    pub fn load(config: &EdgeConfig) -> io::Result<Self> {
        if config.key_path == config.identity_path {
            return Err(invalid("Edge key and identity paths must differ"));
        }
        let mut identity = read(&config.identity_path, 16 * 1024)?
            .map(|bytes| {
                serde_json::from_slice::<Identity>(&bytes).map_err(|_| {
                    invalid("Persisted Edge identity is corrupt; restore or remove it explicitly")
                })
            })
            .transpose()?;
        if let Some(value) = &identity {
            validate_identity(value, &config.profile)?;
        }
        let token_fingerprint = credential(config)?
            .as_ref()
            .map(|v| fingerprint(v.as_bytes()));
        // Validate both files before any reset, so a corrupt key is never silently replaced.
        let key = load_key(&config.key_path)?;
        if let Some(value) = &identity {
            let key = key
                .as_ref()
                .ok_or_else(|| invalid("Persisted Edge identity has no private key"))?;
            if value.agent_fingerprint != fingerprint(key.verifying_key().as_bytes()) {
                return Err(invalid(
                    "Persisted Edge identity does not match its private key",
                ));
            }
        }
        let changed = !matches!(config.profile, EdgeProfile::SwarmNode(_))
            && identity
                .as_ref()
                .and_then(|v| v.enrollment_token_fingerprint.as_ref())
                .zip(token_fingerprint.as_ref())
                .is_some_and(|(old, new)| !old.is_empty() && old != new);
        let key = if changed {
            clear(config)?;
            identity = None;
            create_key(&config.key_path)?
        } else if let Some(key) = key {
            key
        } else {
            if identity.is_none() && token_fingerprint.is_none() {
                return Err(invalid(
                    "Edge requires a persisted identity or an enrollment credential",
                ));
            }
            create_key(&config.key_path)?
        };
        if identity.is_none() && token_fingerprint.is_none() {
            return Err(invalid(
                "Edge requires a persisted identity or an enrollment credential",
            ));
        }
        Ok(Self {
            key,
            identity,
            token_fingerprint,
        })
    }

    pub fn accept(&mut self, config: &EdgeConfig, accepted: &SessionAccepted) -> io::Result<()> {
        let uuid = |s: &str| {
            Uuid::parse_str(s).map_err(|_| invalid("Core accepted an invalid Edge identity"))
        };
        if uuid(&accepted.session_id)?.is_nil() {
            return Err(invalid("Core accepted an empty Edge session"));
        }
        let resource_type = match accepted.resource_type {
            0 => "Platform",
            1 => "BuildAgentPool",
            _ => return Err(invalid("Core accepted an unknown Edge resource type")),
        };
        let value = Identity {
            platform_id: uuid(&accepted.platform_id)?,
            resource_id: uuid(if accepted.resource_id.is_empty() {
                &accepted.platform_id
            } else {
                &accepted.resource_id
            })?,
            resource_type: resource_type.into(),
            agent_id: uuid(&accepted.agent_id)?,
            agent_fingerprint: fingerprint(self.key.verifying_key().as_bytes()),
            enrollment_token_fingerprint: self.token_fingerprint.clone().or_else(|| {
                self.identity
                    .as_ref()
                    .and_then(|v| v.enrollment_token_fingerprint.clone())
            }),
        };
        validate_identity(&value, &config.profile)?;
        if let EdgeProfile::SwarmNode(node) = &config.profile
            && accepted.node_id != node.node_id
        {
            return Err(invalid("Core accepted a different Swarm node"));
        }
        if let Some(old) = &self.identity
            && (old.platform_id != value.platform_id
                || old.agent_id != value.agent_id
                || target_id(old) != value.resource_id
                || old.resource_type != value.resource_type)
        {
            return Err(invalid(
                "Core changed the persisted Edge identity during reconnect",
            ));
        }
        let bytes =
            serde_json::to_vec(&value).map_err(|_| invalid("Could not encode Edge identity"))?;
        atomic_write(&config.identity_path, &bytes, false)?;
        self.identity = Some(value);
        Ok(())
    }

    pub fn rejected(&mut self, config: &EdgeConfig, reason: &str) -> io::Result<()> {
        let token = credential(config)?;
        if self.identity.is_some()
            && token.is_some()
            && (!matches!(config.profile, EdgeProfile::SwarmNode(_))
                || reason.to_ascii_lowercase().contains("revoked"))
        {
            clear(config)?;
            self.key = create_key(&config.key_path)?;
            self.identity = None;
            self.token_fingerprint = token.as_ref().map(|v| fingerprint(v.as_bytes()));
            tracing::info!("Core rejected persisted Edge identity; retrying enrollment");
        }
        Ok(())
    }
}

pub(super) fn target_id(value: &Identity) -> Uuid {
    if value.resource_id.is_nil() {
        value.platform_id
    } else {
        value.resource_id
    }
}
fn validate_identity(value: &Identity, profile: &EdgeProfile) -> io::Result<()> {
    let valid = !value.agent_id.is_nil()
        && !target_id(value).is_nil()
        && match profile {
            EdgeProfile::BuildPool => {
                value.resource_type == "BuildAgentPool" && value.platform_id.is_nil()
            }
            EdgeProfile::Ordinary => {
                value.resource_type == "Platform" && value.platform_id == target_id(value)
            }
            EdgeProfile::SwarmNode(node) => {
                value.resource_type == "Platform"
                    && value.platform_id == target_id(value)
                    && Uuid::parse_str(&node.platform_id).ok() == Some(value.platform_id)
            }
        };
    if valid {
        Ok(())
    } else {
        Err(invalid(
            "Persisted or accepted Edge target does not match the configured profile",
        ))
    }
}
fn clear(config: &EdgeConfig) -> io::Result<()> {
    for path in [&config.identity_path, &config.key_path] {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
fn read(path: &Path, limit: u64) -> io::Result<Option<Zeroizing<Vec<u8>>>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(v) => v,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    if !metadata.is_file() || metadata.len() > limit {
        return Err(invalid("Invalid Edge state file type or size"));
    }
    let mut bytes = Zeroizing::new(Vec::new());
    fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(invalid("Edge state file exceeds its size limit"));
    }
    Ok(Some(bytes))
}
fn load_key(path: &Path) -> io::Result<Option<SigningKey>> {
    let Some(bytes) = read(path, 32)? else {
        return Ok(None);
    };
    let seed: &[u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| invalid("Persisted Edge key is corrupt; restore or remove it explicitly"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(Some(SigningKey::from_bytes(seed)))
}
fn create_key(path: &Path) -> io::Result<SigningKey> {
    let mut seed = Zeroizing::new([0; 32]);
    getrandom::fill(seed.as_mut()).map_err(|_| io::Error::other("Edge key generation failed"))?;
    match atomic_write(path, seed.as_ref(), true) {
        Ok(()) => Ok(SigningKey::from_bytes(&seed)),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            load_key(path)?.ok_or_else(|| invalid("Edge key disappeared during creation"))
        }
        Err(e) => Err(e),
    }
}
struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn atomic_write(path: &Path, bytes: &[u8], create: bool) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|v| !v.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let temporary = Temporary(parent.join(format!(".edge-state-{}.tmp", Uuid::now_v7())));
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary.0)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    if create {
        fs::hard_link(&temporary.0, path)?;
    } else {
        fs::rename(&temporary.0, path)?;
    }
    fs::File::open(parent)?.sync_all()?;
    Ok(())
}
