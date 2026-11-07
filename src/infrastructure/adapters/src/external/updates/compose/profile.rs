//! Supported Docker Compose installation layout, independent of installer text.
//! This is an adapter policy, not the shared update protocol or a Compose schema.
use super::resolution::unsupported;
use citadel_updates::UpdateError;
use serde_json::Value;

const SERVICE_FIELDS: &[&str] = &[
    "image",
    "restart",
    "env_file",
    "environment",
    "depends_on",
    "volumes",
    "ports",
    "healthcheck",
    "stop_grace_period",
    "labels",
    "networks",
];

fn fields(value: &Value, allowed: &[&str]) -> Result<(), UpdateError> {
    let Some(map) = value.as_object() else {
        return Err(invalid());
    };
    if map.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> UpdateError {
    unsupported(
        "Compose configuration is outside the supported Docker Compose installation profile. External file dependencies and unrecognized fields are not supported.",
    )
}

pub(super) fn validate_input(bytes: &[u8]) -> Result<(), UpdateError> {
    // YAML Value rejects duplicate mapping keys. Only the Core volume override
    // is allowed: its complete resolved mount profile is validated afterwards.
    // Never expose parser diagnostics: source lines can contain credentials.
    let mut yaml: serde_yaml_ng::Value = serde_yaml_ng::from_slice(bytes).map_err(|_| invalid())?;
    if let Some(volumes) = yaml
        .get_mut("services")
        .and_then(|s| s.get_mut("server"))
        .and_then(|s| s.get_mut("volumes"))
        && let serde_yaml_ng::Value::Tagged(tagged) = volumes
        && tagged.tag == "!override"
        && tagged.value.is_sequence()
    {
        *volumes = tagged.value.clone();
    }
    if !untagged(&yaml) {
        return Err(invalid());
    }
    let value = serde_json::to_value(yaml).map_err(|_| invalid())?;
    fields(
        &value,
        &["name", "version", "services", "volumes", "networks"],
    )?;
    let services = value.get("services").ok_or_else(invalid)?;
    fields(services, &["server", "pg_db"])?;
    for service in services.as_object().ok_or_else(invalid)?.values() {
        fields(service, SERVICE_FIELDS)?;
        if let Some(env_files) = service.get("env_file") {
            // Only this captured private environment file may be opened by Compose.
            let files = env_files.as_array().ok_or_else(invalid)?;
            if files.len() != 1 {
                return Err(invalid());
            }
            let file = &files[0];
            fields(file, &["path", "format", "required"])?;
            if file["path"] != ".env"
                || file["format"] != "raw"
                || file.get("required").is_some_and(|v| v != true)
            {
                return Err(invalid());
            }
        }
    }
    for (kind, names, driver) in [
        ("volumes", &["citadel_data", "postgres_data"][..], "local"),
        ("networks", &["default"][..], "bridge"),
    ] {
        if let Some(resources) = value.get(kind) {
            fields(resources, names)?;
            for resource in resources.as_object().ok_or_else(invalid)?.values() {
                if resource.is_null() {
                    continue;
                }
                fields(resource, &["name", "driver", "labels"])?;
                if resource.get("driver").is_some_and(|v| v != driver) {
                    return Err(invalid());
                }
            }
        }
    }
    Ok(())
}

fn untagged(value: &serde_yaml_ng::Value) -> bool {
    use serde_yaml_ng::Value as Yaml;
    match value {
        Yaml::Tagged(_) => false,
        Yaml::Sequence(values) => values.iter().all(untagged),
        Yaml::Mapping(values) => values
            .iter()
            .all(|(key, value)| untagged(key) && untagged(value)),
        _ => true,
    }
}

pub(super) fn validate_resolved(config: &Value) -> Result<(), UpdateError> {
    let core = &config["services"]["server"];
    let database = &config["services"]["pg_db"];
    for service in [core, database] {
        if service["restart"] != "unless-stopped" {
            return Err(invalid());
        }
        fields(&service["networks"], &["default"])?;
        if service["networks"].as_object().is_none_or(|v| v.len() != 1) {
            return Err(invalid());
        }
        let health = &service["healthcheck"];
        if health["disable"] == true
            || health["test"].as_array().is_none_or(|test| {
                !matches!(
                    test.first().and_then(Value::as_str),
                    Some("CMD" | "CMD-SHELL")
                ) || test.len() < 2
            })
        {
            return Err(invalid());
        }
    }
    if core["depends_on"]["pg_db"]["condition"] != "service_healthy"
        || database["environment"]
            .get("PGDATA")
            .is_some_and(|path| path != "/var/lib/postgresql/18/docker")
        || database
            .get("ports")
            .is_some_and(|v| v.as_array().is_none_or(|v| !v.is_empty()))
    {
        return Err(invalid());
    }
    let image = database["image"].as_str().ok_or_else(invalid)?;
    if ![
        "postgres:",
        "postgres@sha256:",
        "docker.io/library/postgres:",
        "docker.io/library/postgres@sha256:",
    ]
    .iter()
    .any(|prefix| image.strip_prefix(prefix).is_some_and(|v| !v.is_empty()))
    {
        return Err(invalid());
    }
    let socket = super::super::layout::compose_socket(config)?;
    let rootless = super::super::layout::rootless_socket(std::path::Path::new(socket))?;
    let host_source = if rootless {
        core["volumes"]
            .as_array()
            .ok_or_else(invalid)?
            .iter()
            .find(|m| m["type"] == "bind" && m["source"] != socket)
            .and_then(|m| m["source"].as_str())
            .ok_or_else(invalid)?
    } else {
        "/"
    };
    super::super::files::validate_absolute(std::path::Path::new(host_source))?;
    if rootless && host_source == "/" {
        return Err(invalid());
    }
    let host_target = if rootless {
        format!("/host{host_source}")
    } else {
        "/host".into()
    };
    mounts(
        core,
        &[
            ("volume", "citadel_data", "/app/data", false),
            ("bind", socket, "/var/run/docker.sock", false),
            ("bind", host_source, &host_target, true),
        ],
    )?;
    mounts(
        database,
        &[("volume", "postgres_data", "/var/lib/postgresql", false)],
    )?;
    for (kind, names, driver) in [
        ("volumes", &["citadel_data", "postgres_data"][..], "local"),
        ("networks", &["default"][..], "bridge"),
    ] {
        fields(&config[kind], names)?;
        for name in names {
            let resource = &config[kind][name];
            fields(resource, &["name", "driver", "labels", "ipam"])?;
            if resource
                .get("ipam")
                .is_some_and(|v| v.as_object().is_none_or(|v| !v.is_empty()))
            {
                return Err(invalid());
            }
            if resource.get("driver").is_some_and(|v| v != driver) {
                return Err(invalid());
            }
        }
    }
    Ok(())
}

fn mounts(service: &Value, expected: &[(&str, &str, &str, bool)]) -> Result<(), UpdateError> {
    let mounts = service["volumes"].as_array().ok_or_else(invalid)?;
    if mounts.len() != expected.len() {
        return Err(invalid());
    }
    for (kind, source, target, read_only) in expected {
        let mount = mounts
            .iter()
            .find(|v| v["target"] == *target)
            .ok_or_else(invalid)?;
        fields(
            mount,
            &["type", "source", "target", "read_only", "bind", "volume"],
        )?;
        if mount["type"] != *kind
            || mount["source"] != *source
            || mount
                .get("read_only")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                != *read_only
        {
            return Err(invalid());
        }
        if let Some(bind) = mount.get("bind") {
            fields(bind, &["create_host_path"])?;
        }
        if let Some(volume) = mount.get("volume") {
            // No subpaths or alternate copy semantics for the persistent roots.
            fields(volume, &[])?;
        }
    }
    Ok(())
}
