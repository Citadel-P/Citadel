//! First-install registration using the same protected inputs and runtime checks as the CLI.
use super::super::{
    InstallerComposeInputs, JournalDirectory, RegisteredComposeInstallation,
    docker::{LocalDocker, failed, single},
    files::open_secure,
};
use citadel_execution::ProcessRunner;
use citadel_updates::UpdateError;
use serde::{Deserialize, Serialize};
use std::{
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;

pub const AUTOMATIC_STATE_MOUNT: &str = "/var/lib/citadel-updates";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomaticComposeSetup {
    pub inputs: InstallerComposeInputs,
    pub state_directory: PathBuf,
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PendingRegistration {
    automatic_setup: AutomaticComposeSetup,
    format: u32,
}

/// Read Docker's actual host paths, never PWD or a browser-supplied directory.
pub async fn discover_automatic_setup(
    process: &dyn ProcessRunner,
    cancellation: &CancellationToken,
) -> Result<AutomaticComposeSetup, UpdateError> {
    let hostname = std::fs::read_to_string("/proc/sys/kernel/hostname").map_err(|_| failed())?;
    let hostname = hostname.trim();
    if hostname.len() != 12 || !hostname.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(failed());
    }
    let home = tempfile::tempdir().map_err(|_| failed())?;
    let docker = LocalDocker {
        process,
        cancellation,
        home: home.path(),
        deadline: Instant::now() + Duration::from_secs(30),
    };
    let raw = docker.json(&["container", "inspect", hostname]).await?;
    discover(single(&raw)?, hostname)
}

pub(super) fn discover(
    core: &serde_json::Value,
    hostname: &str,
) -> Result<AutomaticComposeSetup, UpdateError> {
    if core["Id"]
        .as_str()
        .is_none_or(|id| !citadel_updates::is_sha256(id) || !id.starts_with(hostname))
        || core["Config"]["Hostname"] != hostname
        || core["State"]["Running"] != true
    {
        return Err(failed());
    }
    let labels = &core["Config"]["Labels"];
    if labels["com.docker.compose.service"] != "server"
        || labels["com.docker.compose.oneoff"] != "False"
    {
        return Err(failed());
    }
    let directory = PathBuf::from(
        labels["com.docker.compose.project.working_dir"]
            .as_str()
            .ok_or_else(failed)?,
    );
    let mounts = core["Mounts"].as_array().ok_or_else(failed)?;
    let mut states = mounts
        .iter()
        .filter(|m| m["Destination"] == AUTOMATIC_STATE_MOUNT);
    let state = states.next().ok_or_else(failed)?;
    let state_directory = PathBuf::from(state["Source"].as_str().ok_or_else(failed)?);
    if states.next().is_some()
        || state["Type"] != "bind"
        || state["RW"] != false
        || state_directory != directory.join(".citadel-updates")
    {
        return Err(failed());
    }
    let inputs = InstallerComposeInputs {
        directory,
        project: labels["com.docker.compose.project"]
            .as_str()
            .ok_or_else(failed)?
            .into(),
        compose_files: labels["com.docker.compose.project.config_files"]
            .as_str()
            .ok_or_else(failed)?
            .split(',')
            .map(PathBuf::from)
            .collect(),
    };
    super::super::files::validate_absolute(&inputs.directory)?;
    for path in &inputs.compose_files {
        super::super::files::validate_absolute(path)?;
    }
    Ok(AutomaticComposeSetup {
        inputs,
        state_directory,
    })
}

pub(in crate::external::updates) async fn prepare_automatic_setup(
    setup: &AutomaticComposeSetup,
    allow_initialize: bool,
    _process: &dyn ProcessRunner,
    cancellation: &CancellationToken,
) -> Result<Option<RegisteredComposeInstallation>, UpdateError> {
    if cancellation.is_cancelled() {
        return Err(UpdateError::Cancelled);
    }
    // Compose creates a bind directory with 0755. Tighten only an empty, owned
    // directory on an uninitialized database; never repair existing evidence.
    let directory = open_secure(&setup.state_directory, true)?;
    let metadata = directory.metadata().map_err(|_| failed())?;
    if metadata.uid() != rustix::process::geteuid().as_raw() {
        return Err(failed());
    }
    if metadata.mode() & 0o077 != 0 {
        let mut entries = rustix::fs::Dir::read_from(&directory).map_err(|_| failed())?;
        if !allow_initialize
            || entries.any(|e| {
                e.map_or(true, |e| {
                    ![b".".as_slice(), b".."].contains(&e.file_name().to_bytes())
                })
            })
        {
            return Err(failed());
        }
        directory
            .set_permissions(std::fs::Permissions::from_mode(0o700))
            .map_err(|_| failed())?;
    }
    let state = JournalDirectory::open(setup.state_directory.clone())?;
    let saved = state.read_optional::<serde_json::Value>("installation.json")?;
    let _lock = if saved.is_some() {
        state.existing_lock()?
    } else {
        if !allow_initialize {
            return Err(UpdateError::Conflict("Update registration is missing from an initialized installation. Preserve update state and use the explicit setup or recovery command.".into()));
        }
        state.lock()?
    };
    if state.read_optional::<serde_json::Value>("installation.json")? != saved {
        return Err(failed());
    }
    if let Some(value) = saved {
        if value.get("automatic_setup").is_none() {
            drop(_lock);
            return super::super::check_startup_registration(&setup.state_directory).map(Some);
        }
        let pending: PendingRegistration = serde_json::from_value(value).map_err(|_| failed())?;
        if pending.automatic_setup != *setup || pending.format != 1 {
            return Err(failed());
        }
        state.require_registration_only()?;
        state.sync()?;
        return Ok(None);
    }
    state.require_unregistered()?;
    if cancellation.is_cancelled() {
        return Err(UpdateError::Cancelled);
    }
    state.create(
        "installation.json",
        &PendingRegistration {
            automatic_setup: setup.clone(),
            format: 1,
        },
    )?;
    Ok(None)
}

pub(in crate::external::updates) async fn complete_automatic_setup(
    setup: &AutomaticComposeSetup,
    process: &dyn ProcessRunner,
    cancellation: &CancellationToken,
) -> Result<RegisteredComposeInstallation, UpdateError> {
    let state = JournalDirectory::open(setup.state_directory.clone())?;
    let lock = state.existing_lock()?;
    let value: serde_json::Value = state.read("installation.json")?;
    if value.get("automatic_setup").is_none() {
        let registered: RegisteredComposeInstallation =
            serde_json::from_value(value).map_err(|_| failed())?;
        if registered.directory != setup.inputs.directory
            || registered.project != setup.inputs.project
            || registered.compose_files != setup.inputs.compose_files
            || registered.state_directory != setup.state_directory
        {
            return Err(failed());
        }
        state.sync()?;
        drop(lock);
        return super::check_registered_compose_installation(
            &setup.state_directory,
            process,
            cancellation,
        )
        .await
        .map(|c| c.registration);
    }
    let pending: PendingRegistration = serde_json::from_value(value).map_err(|_| failed())?;
    if pending.automatic_setup != *setup || pending.format != 1 {
        return Err(failed());
    }
    state.require_registration_only()?;
    let (registration, _) = super::inspection::inspect_installation(
        &setup.inputs,
        setup.state_directory.clone(),
        uuid::Uuid::now_v7(),
        process,
        cancellation,
        true,
    )
    .await?;
    if state.read::<PendingRegistration>("installation.json")? != pending
        || cancellation.is_cancelled()
    {
        return Err(failed());
    }
    state.verify_lock(&lock)?;
    state.require_registration_only()?;
    // One atomic transition: a missing completed registration can never look
    // like a fresh pending setup, including after a crash.
    state.write("installation.json", &registration)?;
    Ok(registration)
}
