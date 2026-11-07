use super::super::InstallerComposeInputs;
use super::super::docker::{LocalDocker, SOCKET, failed, refuse, single};
use citadel_updates::{UpdateError, is_sha256};
use serde_json::Value;
use std::{fs, path::Path};

pub(super) async fn verify_executor(
    docker: &LocalDocker<'_>,
    inputs: &InstallerComposeInputs,
    state: &Path,
    socket: &Path,
) -> Result<(), UpdateError> {
    if !Path::new("/.dockerenv").exists() {
        return Ok(());
    }
    let hostname = fs::read_to_string("/proc/sys/kernel/hostname").map_err(|_| failed())?;
    let hostname = hostname.trim();
    if hostname.len() != 12 || !hostname.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(refuse(
            "Run registration with Docker's default container hostname.",
        ));
    }
    let raw = docker.json(&["container", "inspect", hostname]).await?;
    let executor = single(&raw)?;
    validate_executor(executor, hostname, inputs, state, socket)
}

pub(in crate::external::updates) fn validate_executor(
    executor: &Value,
    hostname: &str,
    inputs: &InstallerComposeInputs,
    state: &Path,
    socket: &Path,
) -> Result<(), UpdateError> {
    if executor["Id"]
        .as_str()
        .is_none_or(|id| !is_sha256(id) || !id.starts_with(hostname))
        || executor["Config"]["Hostname"] != hostname
    {
        return Err(refuse(
            "Cannot establish updater executor identity on the local daemon.",
        ));
    }
    validate_executor_mounts(executor, inputs, state, socket, false)
}

pub(in crate::external::updates) fn validate_executor_mounts(
    executor: &Value,
    inputs: &InstallerComposeInputs,
    state: &Path,
    socket: &Path,
    writable_inputs: bool,
) -> Result<(), UpdateError> {
    let mounts = executor["Mounts"].as_array().ok_or_else(failed)?;
    let mounts: Vec<_> = mounts
        .iter()
        .filter(|mount| {
            !(mount["Type"] == "tmpfs" && mount["Destination"] == "/tmp" && mount["RW"] == true)
        })
        .collect();
    if mounts.len() != 3 {
        return Err(refuse(
            "Updater executor must mount only installer inputs, update state and the Docker socket.",
        ));
    }
    super::super::layout::rootless_socket(socket)?;
    for (path, target, writable) in [
        (
            inputs.directory.as_path(),
            inputs.directory.as_path(),
            writable_inputs,
        ),
        (state, state, true),
        (socket, Path::new(SOCKET), true),
    ] {
        if !mounts.iter().any(|m| {
            m["Type"] == "bind"
                && m["Source"].as_str() == path.to_str()
                && m["Destination"].as_str() == target.to_str()
                && m["RW"] == writable
        }) {
            return Err(refuse(
                "Updater mounts must match their required access modes and exact host paths.",
            ));
        }
    }
    Ok(())
}
