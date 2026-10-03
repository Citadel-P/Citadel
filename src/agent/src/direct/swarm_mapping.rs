use super::{strings, text};
use citadel_adapters::connectors::docker::projection as d;
use citadel_contracts::citadel::swarm::v1::*;
use prost_types::Timestamp;
use serde_json::Value;
pub(super) fn time(v: &str) -> Option<Timestamp> {
    chrono::DateTime::parse_from_rfc3339(v)
        .ok()
        .map(|t| Timestamp {
            seconds: t.timestamp(),
            nanos: t.timestamp_subsec_nanos() as i32,
        })
}
pub(super) fn node(v: d::SwarmNode, tasks: &[d::SwarmTask]) -> SwarmNodeMessage {
    let current: Vec<_> = tasks
        .iter()
        .filter(|t| t.node_id == v.id && t.desired_state.eq_ignore_ascii_case("running"))
        .collect();
    let hostname = node_name(&v);
    SwarmNodeMessage {
        id: v.id,
        version_index: v.version.index,
        hostname,
        role: enum_name(&text(&v.spec, "Role"), &["Worker", "Manager"]),
        is_leader: v.manager_status["Leader"].as_bool().unwrap_or(false),
        reachability: enum_name(
            &text(&v.manager_status, "Reachability"),
            &["Reachable", "Unreachable"],
        ),
        status: enum_name(
            &text(&v.status, "State"),
            &["Down", "Ready", "Disconnected"],
        ),
        status_message: text(&v.status, "Message"),
        availability: enum_name(
            &text(&v.spec, "Availability"),
            &["Active", "Pause", "Drain"],
        ),
        engine_version: text(&v.description["Engine"], "EngineVersion"),
        operating_system: text(&v.description["Platform"], "OS"),
        architecture: text(&v.description["Platform"], "Architecture"),
        address: text(&v.status, "Addr"),
        labels: strings(&v.spec["Labels"]),
        running_task_count: current
            .iter()
            .filter(|t| t.status["State"].as_str() == Some("running"))
            .count() as i32,
        desired_task_count: current.len() as i32,
        created_at: time(&v.created_at),
        updated_at: time(&v.updated_at),
    }
}
pub(super) fn task(v: d::SwarmTask) -> SwarmTaskMessage {
    SwarmTaskMessage {
        id: v.id,
        version_index: v.version.index,
        name: v.name,
        service_id: v.service_id,
        slot: v.slot,
        node_id: v.node_id,
        desired_state: task_state(&v.desired_state),
        state: task_state(&text(&v.status, "State")),
        status_message: text(&v.status, "Message"),
        error: text(&v.status, "Err"),
        image: text(&v.spec["ContainerSpec"], "Image"),
        ports: v.status["PortStatus"]["Ports"]
            .as_array()
            .into_iter()
            .flatten()
            .map(port)
            .collect(),
        status_timestamp: time(&text(&v.status, "Timestamp")),
        created_at: time(&v.created_at),
        updated_at: time(&v.updated_at),
        container_id: text(&v.status["ContainerStatus"], "ContainerID"),
    }
}
fn enum_name(value: &str, names: &[&str]) -> String {
    names
        .iter()
        .find(|name| name.eq_ignore_ascii_case(value))
        .copied()
        .unwrap_or("Unknown")
        .into()
}
fn task_state(value: &str) -> String {
    enum_name(
        value,
        &[
            "New",
            "Allocated",
            "Pending",
            "Assigned",
            "Accepted",
            "Preparing",
            "Ready",
            "Starting",
            "Running",
            "Complete",
            "Shutdown",
            "Failed",
            "Rejected",
            "Remove",
            "Orphaned",
        ],
    )
}
pub(super) fn node_name(v: &d::SwarmNode) -> String {
    v.description["Hostname"]
        .as_str()
        .or_else(|| v.spec["Name"].as_str())
        .unwrap_or_default()
        .into()
}
fn port(v: &Value) -> String {
    let target = v["TargetPort"]
        .as_u64()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "?".into());
    let protocol = v["Protocol"].as_str().unwrap_or("tcp");
    match v["PublishedPort"].as_u64() {
        Some(published) => format!(
            "{published}:{target}/{protocol} ({})",
            v["PublishMode"].as_str().unwrap_or("ingress")
        ),
        None => format!("{target}/{protocol}"),
    }
}
pub(super) fn network(v: d::DockerNetwork) -> SwarmNetworkMessage {
    SwarmNetworkMessage {
        id: v.id,
        name: v.name,
        scope: v.scope,
        driver: v.driver,
        is_attachable: v.attachable,
        is_internal: v.internal,
        is_ingress: v.ingress,
        is_encrypted: v.options.contains_key("encrypted"),
        enable_ipv6: v.enable_ipv6,
        subnets: v.ipam.unwrap_or_default()["Config"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v["Subnet"].as_str().map(str::to_owned))
            .collect(),
        labels: v.labels,
        created_at: time(&v.created),
    }
}
pub(super) fn secret(v: d::SwarmSecret) -> SwarmSecretMessage {
    SwarmSecretMessage {
        id: v.id,
        version_index: v.version.index,
        name: text(&v.spec, "Name"),
        driver: text(&v.spec["Driver"], "Name"),
        labels: strings(&v.spec["Labels"]),
        created_at: time(&v.created_at),
        updated_at: time(&v.updated_at),
    }
}
pub(super) fn config(v: d::SwarmConfig) -> SwarmConfigMessage {
    SwarmConfigMessage {
        id: v.id,
        version_index: v.version.index,
        name: text(&v.spec, "Name"),
        templating_driver: text(&v.spec["Templating"], "Name"),
        labels: strings(&v.spec["Labels"]),
        created_at: time(&v.created_at),
        updated_at: time(&v.updated_at),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wire_state_names_and_port_modes_are_preserved() {
        assert_eq!(task_state("running"), "Running");
        assert_eq!(task_state("not-a-state"), "Unknown");
        assert_eq!(enum_name("manager", &["Manager", "Worker"]), "Manager");
        assert_eq!(
            port(&serde_json::json!({"TargetPort":80,"PublishedPort":8080,"PublishMode":"host"})),
            "8080:80/tcp (host)"
        );
        assert_eq!(port(&serde_json::json!({"TargetPort":80})), "80/tcp");
    }
}
