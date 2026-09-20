//! Existing Docker inspection transports projected into Citadel's inspection JSON.
use citadel_contracts::citadel::{
    containers::v1::InspectContainerRequest,
    shared_models::v1::{ContainerStateType, InspectContainerResponse},
};
use citadel_platforms::{
    RuntimeCapabilityError, RuntimeErrorKind, containers::ContainerInspectionPort,
};
use futures_util::future::BoxFuture;
use serde_json::{Map, Value};
use tokio_util::sync::CancellationToken;

use crate::{agent::AgentClient, docker::DockerClient, edge::EdgeRuntime};

impl ContainerInspectionPort for DockerClient {
    fn inspection<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Value, RuntimeCapabilityError>> {
        Box::pin(async move {
            let document = tokio::select! {
                biased;
                () = cancel.cancelled() => return Err(RuntimeCapabilityError::new(RuntimeErrorKind::Cancelled, "operation cancelled", false)),
                result = self.inspect_container_document(id) => result.map_err(crate::docker::runtime::normalize_docker_error)?,
            };
            map_inspection(document, id)
        })
    }
}
impl ContainerInspectionPort for AgentClient {
    fn inspection<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Value, RuntimeCapabilityError>> {
        Box::pin(async move { map_agent(self.inspect_container(id, cancel).await?, id) })
    }
}
impl ContainerInspectionPort for EdgeRuntime {
    fn inspection<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Value, RuntimeCapabilityError>> {
        Box::pin(async move {
            let response = crate::agent_execution::unary(
                &self.session,
                citadel_contracts::citadel::edge::v1::EdgeCommandKind::ContainerInspect,
                InspectContainerRequest {
                    container_id: id.into(),
                },
                cancel,
            )
            .await?;
            map_agent(response, id)
        })
    }
}
fn map_agent(
    response: InspectContainerResponse,
    id: &str,
) -> Result<Value, RuntimeCapabilityError> {
    let document = serde_json::to_value(response)
        .map_err(|_| invalid("Invalid Agent inspection response."))?;
    map_inspection(document, id)
}
fn invalid(message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Conflict, message, false)
}

/// Preserve dictionary keys (labels, paths, port names, network names), unlike
/// indiscriminately camel-casing every JSON key. Agent/protobuf containers and
/// Docker responses have the same information but different collection wrappers.
pub fn map_inspection(document: Value, expected_id: &str) -> Result<Value, RuntimeCapabilityError> {
    let mut result = normalize(document, "");
    if result["id"].as_str() != Some(expected_id) {
        return Err(invalid(
            "Container identity changed. Refresh inventory before inspecting.",
        ));
    }
    if let Some(environment) = result
        .pointer_mut("/config/env")
        .and_then(Value::as_array_mut)
    {
        for entry in environment {
            if let Some((name, value)) = entry.as_str().and_then(|value| value.split_once('='))
                && !value.is_empty()
                && citadel_primitives::is_sensitive_environment_name(name)
            {
                *entry = Value::String(format!("{name}=********"));
            }
        }
    }
    if let Some(status) = result.pointer_mut("/state/status") {
        if let Some(value) = status.as_i64() {
            *status = Value::String(
                i32::try_from(value)
                    .ok()
                    .and_then(|value| ContainerStateType::try_from(value).ok())
                    .unwrap_or(ContainerStateType::Unknown)
                    .as_str_name()
                    .into(),
            );
        } else if let Some(value) = status.as_str() {
            let mut chars = value.chars();
            *status = Value::String(chars.next().map_or_else(String::new, |first| {
                first.to_ascii_uppercase().to_string() + chars.as_str()
            }));
        }
    }
    for key in ["args", "execIDs", "mounts"] {
        if result[key].is_null() {
            result[key] = Value::Array(vec![]);
        }
    }
    Ok(result)
}

fn normalize(value: Value, context: &str) -> Value {
    match value {
        Value::Array(items) if context == "networks" => Value::Object(
            items
                .into_iter()
                .filter_map(|mut entry| {
                    let key = entry.get_mut("key")?.take().as_str()?.to_owned();
                    let value = entry.get_mut("value")?.take();
                    Some((key, normalize(value, "endpoint")))
                })
                .collect(),
        ),
        Value::Array(items) => {
            Value::Array(items.into_iter().map(|item| normalize(item, "")).collect())
        }
        Value::Object(map)
            if matches!(
                context,
                "labels"
                    | "annotations"
                    | "storageOpt"
                    | "tmpfs"
                    | "sysctls"
                    | "options"
                    | "driverOpts"
                    | "data"
                    | "logOptions"
            ) =>
        {
            Value::Object(map)
        }
        Value::Object(map) if matches!(context, "exposedPorts" | "volumes") => {
            Value::Array(map.into_iter().map(|(key, _)| Value::String(key)).collect())
        }
        Value::Object(map) if matches!(context, "ports" | "portBindings" | "networks") => {
            Value::Object(
                map.into_iter()
                    .map(|(name, mut value)| {
                        if context != "networks"
                            && let Some(bindings) = value.get_mut("host_port_binding")
                        {
                            value = bindings.take();
                        }
                        (name, normalize(value, ""))
                    })
                    .collect(),
            )
        }
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| {
                    let key = field_name(&key);
                    let child = if context == "logConfig" && key == "config" {
                        "logOptions"
                    } else {
                        &key
                    };
                    let value = normalize(value, child);
                    (key, value)
                })
                .collect::<Map<_, _>>(),
        ),
        value => value,
    }
}

fn field_name(name: &str) -> String {
    // Acronyms whose .NET property spelling differs from protobuf snake_case.
    const ACRONYMS: &[&str] = &[
        "execIDs",
        "containerIDFile",
        "sandboxID",
        "endpointID",
        "networkID",
        "hostIP",
        "dnsNames",
        "linkLocalIPs",
        "secondaryIPAddresses",
        "secondaryIPv6Addresses",
        "globalIPv6Address",
        "globalIPv6PrefixLen",
        "linkLocalIPv6Address",
        "linkLocalIPv6PrefixLen",
        "kernelMemoryTCP",
        "oomKilled",
        "rw",
        "tty",
        "ipAddress",
        "ipPrefixLen",
        "ipv6Gateway",
        "ipv4Address",
        "ipv6Address",
        "ipamConfig",
        "dns",
        "dnsOptions",
        "dnsSearch",
    ];
    if let Some(key) = ACRONYMS.iter().find(|key| {
        key.bytes().map(|b| b.to_ascii_lowercase()).eq(name
            .bytes()
            .filter(|b| *b != b'_')
            .map(|b| b.to_ascii_lowercase()))
    }) {
        return (*key).into();
    }
    let mut chars = name.chars();
    let mut result = chars
        .next()
        .map_or_else(String::new, |first| first.to_ascii_lowercase().to_string());
    let mut uppercase = false;
    for ch in chars {
        if ch == '_' {
            uppercase = true;
        } else {
            result.push(if uppercase {
                ch.to_ascii_uppercase()
            } else {
                ch
            });
            uppercase = false;
        }
    }
    result
}
