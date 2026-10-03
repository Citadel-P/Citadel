use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use prost::Message;
use prost_types::FileDescriptorSet;

const INVENTORY: &str = include_str!("../../../../docs/agent-protocol.md");
const DIRECT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/direct.bin"));
const EDGE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/edge.bin"));

fn validate_operations(inventory: &str) -> Result<(), String> {
    let direct = FileDescriptorSet::decode(DIRECT).map_err(|error| error.to_string())?;
    let edge = FileDescriptorSet::decode(EDGE).map_err(|error| error.to_string())?;
    let mut methods = BTreeMap::new();
    for file in direct.file {
        for service in file.service {
            for method in service.method {
                let name = format!(
                    "{}.{}/{}",
                    file.package.as_deref().unwrap(),
                    service.name.as_deref().unwrap(),
                    method.name.as_deref().unwrap()
                );
                let shape = match (method.client_streaming(), method.server_streaming()) {
                    (false, false) => "unary",
                    (false, true) => "server",
                    (true, true) => "bidi",
                    (true, false) => "client",
                };
                methods.insert(
                    name,
                    (
                        method.input_type().trim_start_matches('.').to_owned(),
                        method.output_type().trim_start_matches('.').to_owned(),
                        shape,
                    ),
                );
            }
        }
    }
    let mut commands: BTreeMap<_, _> = edge
        .file
        .iter()
        .filter(|file| file.package() == "citadel.edge.v1")
        .flat_map(|file| &file.enum_type)
        .filter(|enumeration| enumeration.name() == "EdgeCommandKind")
        .flat_map(|enumeration| &enumeration.value)
        .filter(|value| value.number() != 0)
        .map(|value| (value.name().to_owned(), value.number().to_string()))
        .collect();
    if methods.is_empty() || commands.is_empty() {
        return Err("missing protocol descriptors".into());
    }
    let section = inventory
        .split_once("<!-- operations -->")
        .and_then(|(_, tail)| tail.split_once("<!-- /operations -->"))
        .ok_or("missing operation inventory")?
        .0;
    let adapters = Path::new(env!("CARGO_MANIFEST_DIR")).join("../adapters/src");
    for line in section
        .lines()
        .filter(|line| line.starts_with("| "))
        .skip(2)
    {
        let cells: Vec<_> = line.trim_matches('|').split('|').map(str::trim).collect();
        if cells.len() != 9 {
            return Err(format!("invalid inventory row: {line}"));
        }
        let Some((request, response, stream)) = methods.remove(cells[0]) else {
            return Err(format!("unknown or duplicate RPC: {}", cells[0]));
        };
        if (request.as_str(), response.as_str(), stream) != (cells[1], cells[2], cells[3]) {
            return Err(format!("RPC shape changed: {}", cells[0]));
        }
        if commands.remove(cells[4]).as_deref() != Some(cells[5]) {
            return Err(format!(
                "unknown, duplicate or renumbered command: {}",
                cells[4]
            ));
        }
        if !matches!(
            cells[6],
            "allowed" | "denied" | "helper-only" | "restore-only"
        ) {
            return Err(format!("missing Swarm policy: {}", cells[0]));
        }
        let (file, operation) = cells[7]
            .split_once(':')
            .ok_or("missing runtime reference")?;
        if !adapters.join(file).is_file() || operation.is_empty() {
            return Err(format!("invalid runtime reference: {}", cells[7]));
        }
        if cells[8].is_empty() || !inventory.contains(&format!("**{} —", cells[8])) {
            return Err(format!("undocumented gap: {}", cells[8]));
        }
    }
    if !methods.is_empty() || !commands.is_empty() {
        return Err(format!(
            "missing inventory: RPCs {:?}, commands {:?}",
            methods.keys().collect::<Vec<_>>(),
            commands.keys().collect::<Vec<_>>()
        ));
    }
    Ok(())
}

#[test]
fn every_rpc_and_edge_command_has_a_reviewed_protocol_entry() {
    validate_operations(INVENTORY).unwrap();
}

#[test]
fn inventory_gate_rejects_missing_duplicate_and_changed_operations() {
    let row = INVENTORY
        .lines()
        .find(|line| line.starts_with("| citadel."))
        .unwrap();
    assert!(validate_operations(&INVENTORY.replacen(row, "", 1)).is_err());
    assert!(validate_operations(&INVENTORY.replacen(row, &format!("{row}\n{row}"), 1)).is_err());
    assert!(validate_operations(&INVENTORY.replacen("| unary |", "| server |", 1)).is_err());
    assert!(validate_operations(&INVENTORY.replacen("| 10 |", "| 999 |", 1)).is_err());
    assert!(
        validate_operations(&INVENTORY.replacen("| CONTAINER_LIST |", "| UNKNOWN |", 1)).is_err()
    );
}

#[test]
fn edge_connection_remains_bidirectional_and_separate_from_commands() {
    let descriptors = FileDescriptorSet::decode(EDGE).unwrap();
    let services: Vec<_> = descriptors
        .file
        .iter()
        .flat_map(|file| &file.service)
        .collect();
    assert_eq!(services.len(), 1);
    assert_eq!(services[0].name(), "EdgeAgentService");
    assert_eq!(services[0].method.len(), 1);
    let connect = &services[0].method[0];
    assert_eq!(connect.name(), "Connect");
    assert_eq!(connect.input_type(), ".citadel.edge.v1.AgentEnvelope");
    assert_eq!(connect.output_type(), ".citadel.edge.v1.CoreEnvelope");
    assert!(connect.client_streaming() && connect.server_streaming());
    assert_eq!(citadel_contracts::EDGE_AGENT_PROTOCOL_VERSION, 2);
}

fn assignments(template: &str) -> BTreeMap<&str, &str> {
    let mut values = BTreeMap::new();
    for line in template.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line.split_once('=').expect("environment assignment");
        assert!(
            values.insert(key, value).is_none(),
            "duplicate variable {key}"
        );
    }
    values
}

#[test]
fn deployment_templates_preserve_transport_and_pool_selection() {
    let inbound = assignments(include_str!("../../../../.env.agent.example"));
    let build = assignments(include_str!("../../../../.env.build-agent.example"));
    assert_eq!(
        inbound, build,
        "inbound Build Pool uses the same Direct runtime"
    );
    assert_eq!(inbound.len(), 1);
    assert_eq!(
        inbound["HUB_PUBLIC_KEY"],
        "replace-with-citadel-core-hub-public-key"
    );

    let edge = assignments(include_str!("../../../../.env.edge.example"));
    let edge_build = assignments(include_str!("../../../../.env.edge-build-agent.example"));
    for values in [&edge, &edge_build] {
        assert_eq!(values["CITADEL_AGENT_MODE"], "edge");
        assert!(values["CITADEL_CORE_URL"].starts_with("https://citadel.example.com"));
        assert!(values["CITADEL_EDGE_ENROLLMENT_TOKEN"].starts_with("replace-with-"));
        assert!(!values.contains_key("HUB_PUBLIC_KEY"));
    }
    assert!(!edge.contains_key("CITADEL_EDGE_AGENT_PROFILE"));
    assert_eq!(edge_build["CITADEL_EDGE_AGENT_PROFILE"], "edge-build-agent");
    assert_eq!(
        edge_build["CITADEL_EDGE_ENROLLMENT_TOKEN"],
        "replace-with-build-edge-pool-enrollment-token"
    );
}

#[test]
fn template_settings_are_documented_including_commented_options() {
    let documented: BTreeSet<_> = INVENTORY
        .lines()
        .filter(|line| {
            line.starts_with("| CITADEL_")
                || line.starts_with("| HUB_")
                || line.starts_with("| DOCKER_")
        })
        .map(|line| line.split('|').nth(1).unwrap().trim())
        .collect();
    for template in [
        include_str!("../../../../.env.agent.example"),
        include_str!("../../../../.env.build-agent.example"),
        include_str!("../../../../.env.edge.example"),
        include_str!("../../../../.env.edge-build-agent.example"),
    ] {
        for line in template.lines() {
            if let Some((key, _)) = line.trim_start_matches('#').trim().split_once('=') {
                assert!(
                    documented.contains(key),
                    "undocumented template setting {key}"
                );
            }
        }
    }
}
