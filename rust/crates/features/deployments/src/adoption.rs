//! Non-mutating Docker-to-Deployment draft mapping.
use crate::DeploymentDetails;
use crate::DeploymentError;
use crate::DeploymentSpec;
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Clone)]
pub struct AdoptContainer {
    pub name: String,
    pub description: Option<String>,
    pub spec: DeploymentSpec,
    pub preview_fingerprint: String,
    pub tag_ids: Vec<Uuid>,
    pub import_sensitive_environment_as_secrets: bool,
}

#[derive(Clone)]
pub struct AdoptionSource {
    pub id: Uuid,
    pub docker_container_id: String,
    pub name: String,
    pub platform_id: Uuid,
    pub platform_name: String,
    pub state: String,
}

#[derive(Clone)]
pub struct AdoptionIssue {
    pub code: String,
    pub message: String,
    pub severity: &'static str,
    pub field_path: Option<String>,
}

pub struct ContainerAdoptionDraft {
    pub source: AdoptionSource,
    pub draft: AdoptionDeploymentDraft,
    pub issues: Vec<AdoptionIssue>,
    pub preview_fingerprint: String,
    pub can_import_sensitive_environment_values: bool,
}

pub struct AdoptionDeploymentDraft {
    pub name: String,
    pub platform_id: Uuid,
    pub description: Option<String>,
    pub spec: DeploymentSpec,
    pub tag_ids: Vec<Uuid>,
}

pub trait ContainerAdoptionPort: Send + Sync {
    fn draft<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ContainerAdoptionDraft, DeploymentError>>;
    fn adopt<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        input: AdoptContainer,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<DeploymentDetails, DeploymentError>>;
}

// Docker and generated protobuf inspection documents spell acronyms differently.
// Only field names are compared here; dictionary keys/values remain untouched.
pub fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    value
        .as_object()
        .and_then(|map| {
            map.iter()
                .find(|(key, _)| {
                    key.bytes()
                        .filter(|c| *c != b'_')
                        .map(|c| c.to_ascii_lowercase())
                        .eq(name
                            .bytes()
                            .filter(|c| *c != b'_')
                            .map(|c| c.to_ascii_lowercase()))
                })
                .map(|(_, value)| value)
        })
        .unwrap_or(&Value::Null)
}
pub fn text<'a>(value: &'a Value, name: &str) -> &'a str {
    field(value, name).as_str().unwrap_or("")
}
pub fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}
fn nonempty(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Array(v) => !v.is_empty(),
        Value::Object(v) => !v.is_empty(),
        Value::String(v) => !v.is_empty(),
        Value::Bool(v) => *v,
        Value::Number(v) => v.as_i64().is_some_and(|n| n != 0),
    }
}
pub use citadel_primitives::is_sensitive_environment_name as sensitive;
pub fn valid_binding_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .bytes()
            .enumerate()
            .all(|(i, c)| c == b'_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit()))
}
pub fn sensitive_values(inspection: &Value) -> BTreeMap<String, String> {
    strings(field(field(inspection, "Config"), "Env"))
        .into_iter()
        .filter_map(|entry| {
            let (name, value) = entry.split_once('=')?;
            (sensitive(name)
                && valid_binding_name(name)
                && !value.is_empty()
                && value != "********")
                .then(|| (name.to_owned(), value.to_owned()))
        })
        .collect()
}

pub fn has_resolved_sensitive_environment(
    environment: &[String],
    name: &str,
    available_bindings: &[String],
) -> bool {
    let Some(entry) = environment
        .iter()
        .find(|entry| entry.split_once('=').map_or(entry.as_str(), |(key, _)| key) == name)
    else {
        return false;
    };
    let Some((_, mut value)) = entry.split_once('=') else {
        return available_bindings.iter().any(|binding| binding == name);
    };
    if value.trim().is_empty() || value == "********" {
        return false;
    }
    while let Some((_, remainder)) = value.split_once("${") {
        let Some((reference, rest)) = remainder.split_once('}') else {
            break;
        };
        if !reference.trim().is_empty()
            && !available_bindings
                .iter()
                .any(|binding| binding == reference)
        {
            return false;
        }
        value = rest;
    }
    true
}

pub fn draft_name(name: &str) -> String {
    let name: String = name
        .trim()
        .trim_start_matches('/')
        .chars()
        .take(64)
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let name = name.trim_matches(['-', '_']);
    if name.is_empty() {
        "app".into()
    } else if name.len() < 3 {
        format!("{name}-app")
    } else {
        name.into()
    }
}
fn issue(
    issues: &mut Vec<AdoptionIssue>,
    code: &str,
    message: impl Into<String>,
    blocker: bool,
    path: Option<&str>,
) {
    issues.push(AdoptionIssue {
        code: code.into(),
        message: message.into(),
        severity: if blocker { "Blocker" } else { "Warning" },
        field_path: path.map(str::to_owned),
    });
}

pub fn map_draft(
    source: AdoptionSource,
    inspection: &Value,
    image_id: Option<Uuid>,
    image_defaults: Option<&Value>,
    fingerprint: String,
    name: String,
) -> Result<ContainerAdoptionDraft, DeploymentError> {
    let config = field(inspection, "Config");
    let host = field(inspection, "HostConfig");
    if config.is_null() || host.is_null() {
        return Err(DeploymentError::Conflict(
            "Container inspection is incomplete.".into(),
        ));
    }
    let mut issues = Vec::new();
    let labels = field(config, "Labels")
        .as_object()
        .cloned()
        .unwrap_or_default();
    for key in labels.keys() {
        let lower = key.to_ascii_lowercase();
        if lower.starts_with("com.citadel.") || lower.starts_with("x-citadel.") {
            issue(
                &mut issues,
                "CITADEL_OWNERSHIP_LABEL",
                "Container has existing Citadel ownership labels.",
                true,
                Some("spec.labels"),
            );
        }
    }
    if labels
        .get("com.docker.compose.project")
        .is_some_and(nonempty)
    {
        issue(
            &mut issues,
            "COMPOSE_PROJECT_CONTAINER",
            "This container belongs to a Compose project. Import the project as a Stack instead.",
            true,
            None,
        );
    }
    for key in [
        "Privileged",
        "ReadonlyRootfs",
        "CapAdd",
        "CapDrop",
        "SecurityOpt",
        "Tmpfs",
        "GroupAdd",
        "Sysctls",
        "StorageOpt",
        "VolumesFrom",
        "Links",
        "PublishAllPorts",
        "ContainerIDFile",
        "PidMode",
        "UTSMode",
        "UsernsMode",
        "Cgroup",
        "AutoRemove",
        "Dns",
        "DnsOptions",
        "DnsSearch",
        "ExtraHosts",
        "Ulimits",
        "Devices",
        "DeviceRequests",
    ] {
        if nonempty(field(host, key)) {
            issue(
                &mut issues,
                "UNSUPPORTED_CONFIGURATION",
                format!("Custom {key} settings are not represented by deployments."),
                true,
                None,
            );
        }
    }
    for (key, allowed) in [
        ("IpcMode", &["", "private"][..]),
        ("CgroupnsMode", &["", "private", "host"][..]),
        ("Runtime", &["", "runc"][..]),
        ("Isolation", &["", "default"][..]),
    ] {
        if !allowed.contains(&text(host, key)) {
            issue(
                &mut issues,
                "UNSUPPORTED_CONFIGURATION",
                format!("Custom {key} is not represented by deployments."),
                true,
                None,
            );
        }
    }
    let network_mode = text(host, "NetworkMode");
    if matches!(network_mode, "host" | "none") || network_mode.starts_with("container:") {
        issue(
            &mut issues,
            "NETWORK_MODE",
            format!("Network mode '{network_mode}' is not represented by deployments."),
            true,
            None,
        );
    }
    for (key, defaults) in [
        ("ShmSize", &[0, 67_108_864][..]),
        ("CpuPeriod", &[0, 100_000][..]),
        ("OomScoreAdj", &[0][..]),
        ("MemorySwappiness", &[-1][..]),
    ] {
        if field(host, key)
            .as_i64()
            .is_some_and(|n| !defaults.contains(&n))
        {
            issue(
                &mut issues,
                "RESOURCE_LIMIT",
                format!("Custom {key} is not represented by deployments."),
                true,
                None,
            );
        }
    }
    for key in [
        "MemoryReservation",
        "PidsLimit",
        "IOMaximumBandwidth",
        "CpuPercent",
        "CpuCount",
        "KernelMemoryTCP",
        "CpuQuota",
        "CpuShares",
        "CpusetCpus",
        "CpusetMems",
        "BlkioWeight",
    ] {
        if nonempty(field(host, key)) && field(host, key).as_i64() != Some(-1) {
            issue(
                &mut issues,
                "RESOURCE_LIMIT",
                format!("Custom {key} is not represented by deployments."),
                true,
                None,
            );
        }
    }
    let memory = field(host, "Memory").as_i64().unwrap_or(0);
    let swap = field(host, "MemorySwap").as_i64().unwrap_or(0);
    if swap != 0 && (memory <= 0 || Some(swap) != memory.checked_mul(2)) {
        issue(
            &mut issues,
            "MEMORY_CONFIGURATION",
            "Custom memory swap settings are not represented by deployments.",
            true,
            None,
        );
    }
    let restart = field(host, "RestartPolicy");
    let restart_name = text(restart, "Name").to_ascii_lowercase();
    let policy = match restart_name.as_str() {
        "" | "no" | "empty" => "No",
        "always" => "Always",
        "unless-stopped" | "unlessstopped" => "UnlessStopped",
        "on-failure" | "onfailure" => "OnFailure",
        _ => {
            issue(
                &mut issues,
                "RESTART_POLICY_NOT_SUPPORTED",
                "Unsupported restart policy.",
                true,
                None,
            );
            "No"
        }
    };
    if policy == "OnFailure" && field(restart, "MaximumRetryCount").as_i64().unwrap_or(0) > 0 {
        issue(
            &mut issues,
            "RESTART_RETRY_LIMIT",
            "Restart retry limits are not represented by deployments.",
            true,
            None,
        );
    }
    let stop_signal = text(config, "StopSignal").to_ascii_uppercase();
    if !matches!(
        stop_signal.as_str(),
        "" | "SIGTERM" | "SIGKILL" | "SIGINT" | "SIGQUIT"
    ) {
        issue(
            &mut issues,
            "STOP_SIGNAL_NOT_SUPPORTED",
            "Unsupported stop signal.",
            true,
            None,
        );
    }
    if let Some(defaults) = image_defaults {
        for key in ["User", "Entrypoint", "WorkingDir"] {
            let matches = if key == "Entrypoint" {
                strings(field(config, key)) == strings(field(defaults, key))
            } else if key == "User" {
                normalize_user(text(config, "User")) == normalize_user(text(defaults, "User"))
            } else {
                text(config, key) == text(defaults, key)
            };
            if !matches {
                issue(
                    &mut issues,
                    "IMAGE_OVERRIDE_NOT_PRESERVED",
                    format!("The container {key} override differs from the selected image."),
                    true,
                    None,
                );
            }
        }
    } else if ["User", "Entrypoint", "WorkingDir"]
        .iter()
        .any(|key| nonempty(field(config, key)))
    {
        issue(
            &mut issues,
            "IMAGE_DEFAULTS_UNAVAILABLE",
            "Citadel could not verify whether process settings are inherited from the image.",
            true,
            None,
        );
    }
    if image_id.is_none() {
        issue(
            &mut issues,
            "SOURCE_IMAGE_UNAVAILABLE",
            "The original image is unavailable. Select an appropriate replacement image for future Apply operations.",
            false,
            Some("spec.image.imageId"),
        );
    }
    let log = field(host, "LogConfig");
    if nonempty(field(log, "Config"))
        || !matches!(text(log, "Type"), "" | "json-file" | "local" | "JsonFile")
    {
        issue(
            &mut issues,
            "LOGGING_NOT_PRESERVED",
            "Custom logging settings will not be preserved by Apply.",
            false,
            None,
        );
    }

    let mut ports = Vec::new();
    if let Some(bindings) = field(host, "PortBindings").as_object() {
        for (port, bindings) in bindings {
            let list = bindings
                .as_array()
                .or_else(|| field(bindings, "HostPortBinding").as_array());
            let mut mapped = false;
            for binding in list.into_iter().flatten() {
                let host_port = text(binding, "HostPort");
                if host_port.is_empty() {
                    continue;
                }
                let ip = text(binding, "HostIP");
                if !matches!(ip, "" | "0.0.0.0" | "::") {
                    issue(
                        &mut issues,
                        "HOST_IP_NOT_PRESERVED",
                        "A published port's host IP is not represented by deployments.",
                        false,
                        Some("spec.ports"),
                    );
                }
                ports.push(format!("{host_port}:{port}"));
                mapped = true;
            }
            if !mapped {
                ports.push(port.clone());
            }
        }
    }
    let mut volumes = Vec::new();
    for mount in field(inspection, "Mounts").as_array().into_iter().flatten() {
        let kind = text(mount, "Type");
        let target = text(mount, "Destination");
        if !matches!(kind, "volume" | "bind") {
            issue(
                &mut issues,
                "TMPFS_NOT_SUPPORTED",
                "Unsupported mount type.",
                true,
                Some("spec.volumes"),
            );
            continue;
        }
        let source = if kind == "volume" {
            text(mount, "Name")
        } else {
            text(mount, "Source")
        };
        if source.is_empty() || target.is_empty() || source.contains(':') || target.contains(':') {
            issue(
                &mut issues,
                "UNSUPPORTED_MOUNT",
                "Mount paths cannot be represented by deployments.",
                true,
                Some("spec.volumes"),
            );
            continue;
        }
        volumes.push(format!(
            "{source}:{target}{}",
            if field(mount, "RW").as_bool() == Some(false) {
                ":ro"
            } else {
                ""
            }
        ));
        if !matches!(text(mount, "Propagation"), "" | "rprivate") {
            issue(
                &mut issues,
                "MOUNT_PROPAGATION_NOT_PRESERVED",
                "Custom mount propagation is not represented by deployments.",
                true,
                Some("spec.volumes"),
            );
        }
    }
    let network_settings = field(field(inspection, "NetworkSettings"), "Networks");
    let mut networks: Vec<_> = if let Some(map) = network_settings.as_object() {
        map.keys().cloned().collect()
    } else {
        network_settings
            .as_array()
            .into_iter()
            .flatten()
            .map(|entry| text(entry, "Key").to_owned())
            .filter(|key| !key.is_empty())
            .collect()
    };
    networks.sort();
    networks.dedup();
    let values = sensitive_values(inspection);
    let environment = strings(field(config, "Env"));
    let names: Vec<_> = environment
        .iter()
        .map(|entry| {
            entry
                .split_once('=')
                .map_or(entry.as_str(), |(name, _)| name)
        })
        .filter(|name| sensitive(name))
        .collect();
    let can_import = !names.is_empty() && names.iter().all(|name| values.contains_key(*name));
    let environment: Vec<_> = environment
        .iter()
        .map(|entry| {
            let name = entry
                .split_once('=')
                .map_or(entry.as_str(), |(name, _)| name);
            if sensitive(name) {
                issue(
                    &mut issues,
                    "SENSITIVE_ENVIRONMENT_VALUE_REQUIRED",
                    format!(
                        "Enter a value or binding for sensitive environment variable '{name}'."
                    ),
                    false,
                    Some(&format!("spec.environmentVariables.{name}")),
                );
                if can_import {
                    format!("{name}=${{{name}}}")
                } else {
                    format!("{name}=")
                }
            } else {
                entry.clone()
            }
        })
        .collect();
    let labels: BTreeMap<_, _> = labels
        .into_iter()
        .filter(|(key, _)| {
            !key.starts_with("com.docker.compose.") && !key.starts_with("com.citadel.")
        })
        .collect();
    let spec:DeploymentSpec=serde_json::from_value(json!({
        "image":{"$type":"Local","imageId":image_id.map(|id|id.to_string()).unwrap_or_default()},"updateBehavior":"Disabled",
        "lifeCycleSpec":{"stopTimeout":field(config,"StopTimeout"),"stopSignal":if matches!(stop_signal.as_str(),"SIGTERM"|"SIGKILL"|"SIGINT"|"SIGQUIT") {Some(stop_signal)} else {None},"restartPolicy":policy},
        "resourceSpec":{"nanoCpus":field(host,"NanoCpus").as_i64().filter(|n|*n>0).map(|n|n as f64/1_000_000_000.0),"memoryLimit":(memory>0).then_some(memory as f64/1048576.0)},
        "labels":labels,"ports":ports,"volumes":volumes,"networks":networks,"command":strings(field(config,"Cmd")),"environmentVariables":environment
    })).map_err(|_|DeploymentError::Validation("Container configuration cannot be represented as a Deployment.".into()))?;
    Ok(ContainerAdoptionDraft {
        draft: AdoptionDeploymentDraft {
            name,
            platform_id: source.platform_id,
            description: Some(format!("Adopted from Docker container {}.", source.name)),
            spec,
            tag_ids: vec![],
        },
        source,
        issues,
        preview_fingerprint: fingerprint,
        can_import_sensitive_environment_values: can_import,
    })
}

fn normalize_user(user: &str) -> &str {
    if matches!(user.trim(), "root" | "0") {
        ""
    } else {
        user.trim()
    }
}
