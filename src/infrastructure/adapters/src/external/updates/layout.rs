//! Host paths are Docker-daemon paths; executors always use the fixed internal socket.
use super::{
    docker::{SOCKET, failed, refuse},
    files::validate_absolute,
};
use citadel_updates::UpdateError;
use serde_json::Value;
use std::path::Path;

pub(super) fn rootless_socket(path: &Path) -> Result<bool, UpdateError> {
    validate_absolute(path)?;
    if path == Path::new(SOCKET) {
        return Ok(false);
    }
    let uid = path
        .to_str()
        .and_then(|s| s.strip_prefix("/run/user/"))
        .and_then(|s| s.strip_suffix("/docker.sock"));
    if uid.is_some_and(|s| {
        s.parse::<u32>()
            .is_ok_and(|uid| uid != 0 && uid.to_string() == s)
    }) {
        Ok(true)
    } else {
        Err(refuse(
            "Updates require the local Docker socket or a rootless /run/user/<uid>/docker.sock socket.",
        ))
    }
}

pub(super) fn compose_socket(config: &Value) -> Result<&str, UpdateError> {
    let mounts = config["services"]["server"]["volumes"]
        .as_array()
        .ok_or_else(failed)?;
    let mut sockets = mounts.iter().filter(|m| m["target"] == SOCKET);
    let mount = sockets.next().ok_or_else(failed)?;
    let source = mount["source"].as_str().ok_or_else(failed)?;
    rootless_socket(Path::new(source))?;
    if sockets.next().is_some() || mount["type"] != "bind" || mount["read_only"] == true {
        return Err(refuse(
            "Core must mount its local Docker socket read-write at /var/run/docker.sock.",
        ));
    }
    Ok(source)
}

pub(super) fn container_socket(container: &Value) -> Result<&str, UpdateError> {
    let mounts = container["Mounts"].as_array().ok_or_else(failed)?;
    let mut sockets = mounts.iter().filter(|m| m["Destination"] == SOCKET);
    let mount = sockets.next().ok_or_else(failed)?;
    let source = mount["Source"].as_str().ok_or_else(failed)?;
    rootless_socket(Path::new(source))?;
    if sockets.next().is_some() || mount["Type"] != "bind" || mount["RW"] != true {
        return Err(refuse(
            "Core's Docker socket mount is ambiguous or read-only.",
        ));
    }
    Ok(source)
}
