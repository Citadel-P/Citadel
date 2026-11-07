use super::files::{mapped_path, open_secure, read_bounded, unavailable_input, validate_absolute};
use citadel_updates::{INSTALLER_FORMAT, InstallationKind, UpdateError, is_sha256};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const MAX_INPUT: u64 = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegisteredComposeInstallation {
    pub format: u32,
    pub kind: InstallationKind,
    pub id: Uuid,
    pub instance_id: Uuid,
    pub directory: PathBuf,
    pub project: String,
    pub compose_files: Vec<PathBuf>,
    pub environment_file: PathBuf,
    pub docker_socket: PathBuf,
    pub daemon_id: String,
    pub core_container_id: String,
    pub database_container_id: String,
    pub core_volume: String,
    pub database_volume: String,
    pub network: String,
    pub state_directory: PathBuf,
    pub executor_uid: u32,
    pub executor_gid: u32,
    pub configuration_sha256: String,
    pub runtime_sha256: String,
}

impl RegisteredComposeInstallation {
    pub fn validate(&self) -> Result<(), UpdateError> {
        if self.format != INSTALLER_FORMAT
            || self.kind != InstallationKind::DockerCompose
            || self.id.is_nil()
            || self.instance_id.is_nil()
            || !identifier(&self.project)
            || !identifier(&self.core_volume)
            || !identifier(&self.database_volume)
            || !identifier(&self.network)
            || self.daemon_id.is_empty()
            || self.core_container_id.is_empty()
            || self.database_container_id.is_empty()
            || self.core_container_id == self.database_container_id
            || self.compose_files.is_empty()
            || self.compose_files.len() > 4
            || !is_sha256(&self.configuration_sha256)
            || !is_sha256(&self.runtime_sha256)
        {
            return Err(UpdateError::Validation(
                "Invalid installation registration.".into(),
            ));
        }
        super::layout::rootless_socket(&self.docker_socket)?;
        for path in [&self.directory, &self.state_directory, &self.docker_socket] {
            validate_absolute(path)?;
        }
        if self.directory == Path::new("/")
            || self.state_directory == Path::new("/")
            || self.state_directory == self.directory
            || self.core_volume == self.database_volume
        {
            return Err(UpdateError::Validation(
                "Installation paths or volume identities are ambiguous.".into(),
            ));
        }
        for path in self.compose_files.iter().chain([&self.environment_file]) {
            validate_absolute(path)?;
            if !path.starts_with(&self.directory) || path == &self.directory {
                return Err(UpdateError::Validation(
                    "Compose inputs must belong to the registered installation directory.".into(),
                ));
            }
        }
        if self
            .environment_file
            .file_name()
            .is_none_or(|n| n != ".env")
        {
            return Err(UpdateError::Unavailable(
                "Only the registered installer .env is supported.".into(),
            ));
        }
        Ok(())
    }

    /// Revalidate canonical paths through a read-only host mapping in Core, or
    /// directly in the helper. Neither side resolves arbitrary request paths.
    pub fn fingerprint(&self, host_root: &Path) -> Result<String, UpdateError> {
        self.validate()?;
        let mut inputs = Vec::new();
        for input in self.compose_files.iter().chain([&self.environment_file]) {
            let path = mapped_path(host_root, input)?;
            inputs.push((
                input.clone(),
                read_input(&path, input == &self.environment_file)?,
            ));
        }
        Ok(fingerprint_inputs(&self.project, &inputs))
    }
}

pub(super) fn read_input(path: &Path, private: bool) -> Result<Vec<u8>, UpdateError> {
    use std::os::unix::fs::MetadataExt;
    let file = open_secure(path, false)?;
    if private && file.metadata().map_err(|_| unavailable_input())?.mode() & 0o077 != 0 {
        return Err(UpdateError::Unavailable(
            "The installer .env must be private (0600).".into(),
        ));
    }
    read_bounded(file, MAX_INPUT)
}

pub(super) fn fingerprint_inputs<B: AsRef<[u8]>>(project: &str, inputs: &[(PathBuf, B)]) -> String {
    let mut hash = Sha256::new();
    hash.update(project.as_bytes());
    for (path, bytes) in inputs {
        let bytes = bytes.as_ref();
        hash.update((path.as_os_str().len() as u64).to_be_bytes());
        hash.update(path.as_os_str().as_encoded_bytes());
        hash.update((bytes.len() as u64).to_be_bytes());
        hash.update(bytes);
    }
    hash.finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(super) fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        && !value.starts_with('-')
}
