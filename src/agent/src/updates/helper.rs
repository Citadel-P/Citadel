use serde_json::{Value, json};

/// Only known local Unix socket layouts, mounted at the Agent's standard endpoint.
/// The helper must not accidentally connect to the host's rootful Docker daemon.
pub(super) fn socket_source(source: &Value) -> Result<&str, &'static str> {
    const ERROR: &str =
        "Manual update required: a writable standard or rootless Docker socket bind is required.";
    let mounts = source["Mounts"].as_array().ok_or(ERROR)?;
    let mut sockets = mounts
        .iter()
        .filter(|m| m["Destination"] == "/var/run/docker.sock");
    let socket = sockets.next().ok_or(ERROR)?;
    if sockets.next().is_some() || socket["Type"] != "bind" || socket["RW"] != true {
        return Err(ERROR);
    }
    let path = socket["Source"].as_str().ok_or(ERROR)?;
    let rootless = path
        .strip_prefix("/run/user/")
        .and_then(|p| p.strip_suffix("/docker.sock"))
        .is_some_and(|uid| !uid.is_empty() && uid.bytes().all(|b| b.is_ascii_digit()));
    if path != "/var/run/docker.sock" && !rootless {
        return Err(ERROR);
    }
    // A mounted socket does not authorize updating a different, remotely selected engine.
    if source["Config"]["Env"].as_array().is_some_and(|env| {
        env.iter().any(|entry| {
            entry
                .as_str()
                .and_then(|e| e.strip_prefix("DOCKER_HOST="))
                .is_some_and(|value| {
                    !value.trim().is_empty() && value.trim() != "unix:///var/run/docker.sock"
                })
        })
    }) {
        return Err("Manual update required: the Agent must use its mounted local Docker socket.");
    }
    Ok(path)
}

pub(crate) fn helper_config(
    source: &Value,
    image: &str,
    operation: &str,
) -> Result<Value, &'static str> {
    super::replacement_config(source, image, operation)?;
    let socket = socket_source(source)?;
    Ok(json!({
        "Image": image, "Entrypoint": ["/app/citadel-agent"],
        "Cmd": ["replace-agent", source["Id"], source["Image"], image, operation],
        "Labels": {"com.citadel.system": "true", "com.citadel.system-role": "agent-updater", "com.citadel.agent-update": operation},
        "HostConfig": {
            "Binds": [format!("{socket}:/var/run/docker.sock:rw")], "NetworkMode": "none",
            "ReadonlyRootfs": true, "CapDrop": ["ALL"], "SecurityOpt": ["no-new-privileges"],
            "RestartPolicy": {"Name": "no"}
        }
    }))
}
