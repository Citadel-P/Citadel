use serde_json::{Map, Value, json};

/// Recreate dynamic endpoints by immutable network identity, never by a reusable name.
pub(super) fn preserve_networks(source: &Value, target: &mut Value) -> Result<(), &'static str> {
    const ERROR: &str =
        "Manual update required: Agent network attachments cannot be safely recreated.";
    let mode = source["HostConfig"]["NetworkMode"].as_str().ok_or(ERROR)?;
    if mode.is_empty() || mode.starts_with("container:") || mode == "none" {
        return Err(ERROR);
    }
    let networks = source["NetworkSettings"]["Networks"].as_object();
    if matches!(mode, "default" | "bridge" | "host") {
        // Do not silently drop additional attachments on a legacy default-network profile.
        if networks
            .is_some_and(|networks| networks.keys().any(|name| name != "bridge" && name != mode))
        {
            return Err(ERROR);
        }
        return Ok(());
    }
    let networks = networks.filter(|n| !n.is_empty()).ok_or(ERROR)?;
    let mut endpoints = Map::new();
    let mut primary = None;
    let source_id = source["Id"].as_str().ok_or(ERROR)?;
    for (name, endpoint) in networks {
        let id = endpoint["NetworkID"]
            .as_str()
            .filter(|id| id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or(ERROR)?;
        if name == "bridge" || name == "host" || name == "none" {
            return Err(ERROR);
        }
        if name == mode || id == mode {
            primary = Some(id);
        }
        // The source is retained for recovery. Static addresses cannot be assigned to both.
        if endpoint["IPAMConfig"]
            .as_object()
            .is_some_and(|c| !c.is_empty())
            || endpoint["Links"]
                .as_array()
                .is_some_and(|links| !links.is_empty())
            || source["Config"]["MacAddress"]
                .as_str()
                .is_some_and(|mac| !mac.is_empty())
        {
            return Err(
                "Manual update required: fixed addresses and legacy network links need an explicit handover.",
            );
        }
        let mut settings = Map::new();
        settings.insert("NetworkID".into(), json!(id));
        if let Some(aliases) = endpoint["Aliases"].as_array() {
            let mut preserved = Vec::new();
            for alias in aliases {
                let alias = alias.as_str().ok_or(ERROR)?;
                // Old Docker versions include the source's short ID as an alias.
                if alias != source_id && alias != &source_id[..12.min(source_id.len())] {
                    preserved.push(alias);
                }
            }
            settings.insert("Aliases".into(), json!(preserved));
        }
        for key in ["DriverOpts", "GwPriority"] {
            if let Some(value) = endpoint.get(key).filter(|v| !v.is_null()) {
                settings.insert(key.into(), value.clone());
            }
        }
        if endpoints
            .insert(id.into(), Value::Object(settings))
            .is_some()
        {
            return Err(ERROR);
        }
    }
    target["HostConfig"]["NetworkMode"] = json!(primary.ok_or(ERROR)?);
    target["NetworkingConfig"] = json!({"EndpointsConfig": endpoints});
    Ok(())
}
