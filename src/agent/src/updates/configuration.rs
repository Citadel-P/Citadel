use serde_json::{Value, json};

pub(crate) fn validate_image(image: &str) -> Result<(), &'static str> {
    if !image
        .strip_prefix("ghcr.io/citadel-p/citadel.agent@sha256:")
        .is_some_and(|digest| {
            digest.len() == 64
                && digest
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
    {
        return Err("An immutable official Agent image is required.");
    }
    Ok(())
}
/// Recreate only the installed Docker profile. Never copy credentials into logs or requests.
pub(crate) fn replacement_config(
    source: &Value,
    image: &str,
    operation: &str,
) -> Result<Value, &'static str> {
    validate_image(image)?;
    let config = &source["Config"];
    if !matches!(
        config["Labels"]["com.citadel.system-role"].as_str(),
        Some("agent" | "edge-agent")
    ) || config["Labels"]["com.citadel.system"] != "true"
        || config["Labels"]
            .get("com.docker.swarm.service.id")
            .is_some()
        || source["State"]["Running"] != true
        || source["HostConfig"]["AutoRemove"] == true
        || !matches!(
            source["HostConfig"]["NetworkMode"].as_str(),
            Some("default" | "bridge" | "host")
        )
        || source["HostConfig"]["VolumesFrom"]
            .as_array()
            .is_some_and(|v| !v.is_empty())
    {
        return Err(
            "Manual update required: this is not a supported standalone Docker Agent installation.",
        );
    }
    let id = source["Id"]
        .as_str()
        .ok_or("Agent container identity is missing.")?;
    if config["Hostname"]
        .as_str()
        .is_some_and(|h| !h.is_empty() && !id.starts_with(h))
    {
        return Err("Manual update required: custom Agent hostnames are not supported.");
    }
    let mounts = source["Mounts"]
        .as_array()
        .ok_or("Agent mounts are unavailable.")?;
    if !mounts.iter().any(|m| {
        m["Destination"] == "/var/run/docker.sock"
            && m["Source"] == "/var/run/docker.sock"
            && m["Type"] == "bind"
            && m["RW"] == true
    }) {
        return Err("Manual update required: the standard local Docker socket mount is required.");
    }
    let env = config["Env"]
        .as_array()
        .ok_or("Agent environment is unavailable.")?;
    let value = |key: &str| {
        env.iter().filter_map(Value::as_str).find_map(|entry| {
            entry
                .split_once('=')
                .filter(|(name, _)| *name == key)
                .map(|(_, value)| value)
        })
    };
    if value("CITADEL_AGENT_MODE").is_some_and(|mode| mode.eq_ignore_ascii_case("edge")) {
        if !mounts.iter().any(|m| {
            m["Destination"] == "/app/data"
                && m["RW"] == true
                && matches!(m["Type"].as_str(), Some("bind" | "volume"))
        }) {
            return Err(
                "Manual update required: Edge identity must be stored in a persistent /app/data mount.",
            );
        }
        for key in ["CITADEL_EDGE_AGENT_KEY_PATH", "CITADEL_EDGE_IDENTITY_PATH"] {
            if let Some(path) = value(key) {
                // Only the standard directory is supported, including Build Pool filenames.
                let Some(file) = path.strip_prefix("/app/data/") else {
                    return Err(
                        "Manual update required: Edge key and identity paths must be files in /app/data.",
                    );
                };
                if file.is_empty() || file == "." || file == ".." || file.contains('/') {
                    return Err(
                        "Manual update required: Edge key and identity paths must be files in /app/data.",
                    );
                }
            }
        }
        // A nested mount could shadow the durable identity directory or a file within it.
        if mounts.iter().any(|m| {
            m["Destination"]
                .as_str()
                .is_some_and(|p| p.starts_with("/app/data/"))
        }) {
            return Err("Manual update required: nested Edge identity mounts are unsupported.");
        }
    }
    if source["HostConfig"]["Mounts"]
        .as_array()
        .is_some_and(|mounts| {
            mounts.iter().any(|m| {
                m["VolumeOptions"]
                    .as_object()
                    .is_some_and(|o| !o.is_empty())
                    || m["BindOptions"]
                        .as_object()
                        .is_some_and(|o| o.keys().any(|key| key != "Propagation"))
            })
        })
    {
        return Err(
            "Manual update required: specialized mount options must be preserved by the installation owner.",
        );
    }
    let mut result = config.clone();
    result["Image"] = json!(image);
    result["Hostname"] = json!("");
    result["Labels"]["com.citadel.agent-update"] = json!(operation);
    let mut host = source["HostConfig"].clone();
    // Docker records an ephemeral request as port 0; preserve the assigned host port.
    if let Some(ports) = source["NetworkSettings"]["Ports"].as_object() {
        for (port, bindings) in ports {
            if bindings.is_array() {
                host["PortBindings"][port] = bindings.clone();
            }
        }
    }
    let mut binds = Vec::new();
    for mount in mounts {
        let source = match mount["Type"].as_str() {
            Some("bind") => mount["Source"].as_str(),
            Some("volume") => mount["Name"].as_str(),
            Some("tmpfs") => continue,
            _ => return Err("Manual update required: unsupported Agent storage mount."),
        }
        .ok_or("Agent storage identity is missing.")?;
        let destination = mount["Destination"]
            .as_str()
            .ok_or("Agent mount destination is missing.")?;
        if source.contains(':') || destination.contains(':') {
            return Err("Manual update required: unsupported Agent mount path.");
        }
        let mode = if mount["RW"] == true { "rw" } else { "ro" };
        let propagation = mount["Propagation"]
            .as_str()
            .filter(|v| !v.is_empty())
            .unwrap_or("rprivate");
        let mut options: Vec<&str> = mount["Mode"]
            .as_str()
            .unwrap_or("")
            .split(',')
            .filter(|v| !v.is_empty())
            .collect();
        if !options.iter().any(|v| matches!(*v, "ro" | "rw")) {
            options.push(mode);
        }
        if mount["Type"] == "bind" && !options.contains(&propagation) {
            options.push(propagation);
        }
        let options = options.join(",");
        binds.push(format!("{source}:{destination}:{options}"));
    }
    host["Binds"] = json!(binds);
    host.as_object_mut()
        .ok_or("Agent host configuration is missing.")?
        .remove("Mounts");
    result["HostConfig"] = host;
    Ok(result)
}
