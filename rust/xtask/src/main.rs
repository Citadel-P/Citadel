#![forbid(unsafe_code)]
#![recursion_limit = "512"]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Stdio};

use clap::{Parser, Subcommand};
use serde::Deserialize;
use sha2::{Digest, Sha256};

mod database_gen;
mod openapi_gen;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Manage the Rust-owned database schema and migration artifacts.
    Database {
        #[command(subcommand)]
        command: DatabaseCommand,
    },
    /// Generate deterministic full/public OpenAPI and foundation frontend artifacts.
    Openapi {
        /// Exit non-zero when generated API artifacts differ from source control.
        #[arg(long)]
        check: bool,
    },
    /// Validate the pinned Docker schema and regenerate the Phase 0 subset.
    Docker {
        /// Exit non-zero when regeneration changes the checked-in file.
        #[arg(long)]
        check: bool,
    },
}

#[derive(Subcommand)]
enum DatabaseCommand {
    /// Verify the Rust schema, migration catalog, and checksums without .NET inputs.
    Verify,
    /// Recreate the one-time Rust-v1 baseline from the frozen accepted .NET input.
    ImportBaseline,
}

#[derive(Deserialize)]
struct SchemaSource {
    path: String,
    sha256: String,
    api_version: String,
}

struct OperationSpec {
    operation_id: &'static str,
    method: &'static str,
    path: &'static str,
    versioned: bool,
    streaming: bool,
}

const OPERATIONS: &[OperationSpec] = &[
    OperationSpec {
        operation_id: "SystemPing",
        method: "get",
        path: "/_ping",
        versioned: false,
        streaming: false,
    },
    OperationSpec {
        operation_id: "SystemVersion",
        method: "get",
        path: "/version",
        versioned: false,
        streaming: false,
    },
    OperationSpec {
        operation_id: "SystemInfo",
        method: "get",
        path: "/info",
        versioned: true,
        streaming: false,
    },
    OperationSpec {
        operation_id: "ContainerList",
        method: "get",
        path: "/containers/json",
        versioned: true,
        streaming: false,
    },
    OperationSpec {
        operation_id: "ContainerInspect",
        method: "get",
        path: "/containers/{id}/json",
        versioned: true,
        streaming: false,
    },
    OperationSpec {
        operation_id: "ContainerCreate",
        method: "post",
        path: "/containers/create",
        versioned: true,
        streaming: false,
    },
    OperationSpec {
        operation_id: "ContainerStart",
        method: "post",
        path: "/containers/{id}/start",
        versioned: true,
        streaming: false,
    },
    OperationSpec {
        operation_id: "ContainerDelete",
        method: "delete",
        path: "/containers/{id}",
        versioned: true,
        streaming: false,
    },
    OperationSpec {
        operation_id: "SystemEvents",
        method: "get",
        path: "/events",
        versioned: true,
        streaming: true,
    },
    OperationSpec {
        operation_id: "ContainerStats",
        method: "get",
        path: "/containers/{id}/stats",
        versioned: true,
        streaming: true,
    },
    OperationSpec {
        operation_id: "SwarmInspect",
        method: "get",
        path: "/swarm",
        versioned: true,
        streaming: false,
    },
    OperationSpec {
        operation_id: "ImageList",
        method: "get",
        path: "/images/json",
        versioned: true,
        streaming: false,
    },
    OperationSpec {
        operation_id: "ImageCreate",
        method: "post",
        path: "/images/create",
        versioned: true,
        streaming: true,
    },
    OperationSpec {
        operation_id: "VolumeList",
        method: "get",
        path: "/volumes",
        versioned: true,
        streaming: false,
    },
    OperationSpec {
        operation_id: "VolumeInspect",
        method: "get",
        path: "/volumes/{name}",
        versioned: true,
        streaming: false,
    },
    OperationSpec {
        operation_id: "VolumeCreate",
        method: "post",
        path: "/volumes/create",
        versioned: true,
        streaming: false,
    },
    OperationSpec {
        operation_id: "VolumeDelete",
        method: "delete",
        path: "/volumes/{name}",
        versioned: true,
        streaming: false,
    },
    OperationSpec {
        operation_id: "NetworkList",
        method: "get",
        path: "/networks",
        versioned: true,
        streaming: false,
    },
    OperationSpec {
        operation_id: "NetworkInspect",
        method: "get",
        path: "/networks/{id}",
        versioned: true,
        streaming: false,
    },
    OperationSpec {
        operation_id: "NetworkCreate",
        method: "post",
        path: "/networks/create",
        versioned: true,
        streaming: false,
    },
    OperationSpec {
        operation_id: "NetworkDelete",
        method: "delete",
        path: "/networks/{id}",
        versioned: true,
        streaming: false,
    },
    OperationSpec {
        operation_id: "NodeList",
        method: "get",
        path: "/nodes",
        versioned: true,
        streaming: false,
    },
    OperationSpec {
        operation_id: "ServiceList",
        method: "get",
        path: "/services",
        versioned: true,
        streaming: false,
    },
    OperationSpec {
        operation_id: "TaskList",
        method: "get",
        path: "/tasks",
        versioned: true,
        streaming: false,
    },
    OperationSpec {
        operation_id: "SecretList",
        method: "get",
        path: "/secrets",
        versioned: true,
        streaming: false,
    },
    OperationSpec {
        operation_id: "ConfigList",
        method: "get",
        path: "/configs",
        versioned: true,
        streaming: false,
    },
];

const MODEL_FIELDS: &[(&str, &[&str])] = &[
    (
        "SystemVersion",
        &[
            "Version",
            "ApiVersion",
            "MinAPIVersion",
            "GitCommit",
            "Os",
            "Arch",
        ],
    ),
    (
        "SystemInfo",
        &[
            "ID",
            "Containers",
            "ContainersRunning",
            "ContainersPaused",
            "ContainersStopped",
            "Images",
            "DockerRootDir",
            "NCPU",
            "MemTotal",
            "OperatingSystem",
            "OSType",
            "Architecture",
            "Swarm",
        ],
    ),
    (
        "SwarmInfo",
        &[
            "NodeID",
            "NodeAddr",
            "LocalNodeState",
            "ControlAvailable",
            "Error",
            "RemoteManagers",
            "Nodes",
            "Managers",
            "Cluster",
        ],
    ),
    ("PeerNode", &["NodeID", "Addr"]),
    (
        "ContainerSummary",
        &[
            "Id", "Names", "Image", "ImageID", "Created", "Labels", "State", "Status", "Ports",
        ],
    ),
    (
        "ContainerInspectResponse",
        &[
            "Id", "Created", "Path", "Args", "State", "Image", "Name", "Config",
        ],
    ),
    (
        "ContainerState",
        &[
            "Status",
            "Running",
            "Paused",
            "Restarting",
            "OOMKilled",
            "Dead",
            "Pid",
            "ExitCode",
            "Error",
            "StartedAt",
            "FinishedAt",
            "Health",
        ],
    ),
    ("Health", &["Status"]),
    ("ContainerConfig", &["Env", "Image", "Labels"]),
    ("EventActor", &["ID", "Attributes"]),
    (
        "EventMessage",
        &["Type", "Action", "Actor", "scope", "time", "timeNano"],
    ),
    (
        "ContainerStatsResponse",
        &[
            "name",
            "id",
            "read",
            "preread",
            "cpu_stats",
            "precpu_stats",
            "memory_stats",
            "networks",
        ],
    ),
    (
        "ContainerCPUStats",
        &["cpu_usage", "system_cpu_usage", "online_cpus"],
    ),
    ("ContainerCPUUsage", &["total_usage", "percpu_usage"]),
    ("ContainerMemoryStats", &["usage", "stats", "limit"]),
    (
        "ContainerNetworkStats",
        &[
            "rx_bytes",
            "rx_packets",
            "rx_errors",
            "rx_dropped",
            "tx_bytes",
            "tx_packets",
            "tx_errors",
            "tx_dropped",
        ],
    ),
    (
        "ClusterInfo",
        &[
            "ID",
            "Version",
            "CreatedAt",
            "UpdatedAt",
            "RootRotationInProgress",
        ],
    ),
    ("ObjectVersion", &["Index"]),
    ("JoinTokens", &["Worker", "Manager"]),
    (
        "ImageSummary",
        &[
            "Id",
            "RepoTags",
            "RepoDigests",
            "Created",
            "Size",
            "Labels",
            "Containers",
        ],
    ),
    ("VolumeListResponse", &["Volumes", "Warnings"]),
    (
        "VolumeCreateOptions",
        &["Name", "Driver", "DriverOpts", "Labels"],
    ),
    (
        "Volume",
        &[
            "Name",
            "Driver",
            "Mountpoint",
            "CreatedAt",
            "Status",
            "Labels",
            "Scope",
            "Options",
        ],
    ),
    (
        "Network",
        &[
            "Name",
            "Id",
            "Created",
            "Scope",
            "Driver",
            "EnableIPv4",
            "EnableIPv6",
            "Internal",
            "Attachable",
            "Ingress",
            "ConfigOnly",
            "Options",
            "Labels",
            "Containers",
            "Peers",
        ],
    ),
    (
        "Node",
        &[
            "ID",
            "Version",
            "CreatedAt",
            "UpdatedAt",
            "Spec",
            "Description",
            "Status",
            "ManagerStatus",
        ],
    ),
    (
        "Service",
        &[
            "ID",
            "Version",
            "CreatedAt",
            "UpdatedAt",
            "Spec",
            "Endpoint",
            "UpdateStatus",
            "ServiceStatus",
        ],
    ),
    (
        "Task",
        &[
            "ID",
            "Version",
            "CreatedAt",
            "UpdatedAt",
            "Name",
            "Spec",
            "ServiceID",
            "Slot",
            "NodeID",
            "Status",
            "DesiredState",
        ],
    ),
    (
        "Secret",
        &["ID", "Version", "CreatedAt", "UpdatedAt", "Spec"],
    ),
    (
        "Config",
        &["ID", "Version", "CreatedAt", "UpdatedAt", "Spec"],
    ),
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::Database { command } => match command {
            DatabaseCommand::Verify => database_gen::verify(),
            DatabaseCommand::ImportBaseline => database_gen::import_baseline(),
        },
        Command::Openapi { check } => openapi_gen::generate(check),
        Command::Docker { check } => generate_docker(check),
    }
}

fn generate_docker(check: bool) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask must be inside the workspace");
    let source_path = workspace.join("crates/adapters/src/docker/schema/source.json");
    let source: SchemaSource = serde_json::from_slice(&fs::read(&source_path)?)?;
    let schema_path = normalize(workspace.join(&source.path));
    let schema_bytes = fs::read(&schema_path)?;
    let actual_hash = hex_digest(&schema_bytes);
    if actual_hash != source.sha256 {
        return Err(format!(
            "Docker schema checksum mismatch for {}: expected {}, got {}",
            schema_path.display(),
            source.sha256,
            actual_hash
        )
        .into());
    }

    let text = std::str::from_utf8(&schema_bytes)?;
    validate_operations(text)?;
    validate_models(text)?;

    let generated = rustfmt(&render(&source.api_version, &source.sha256))?;
    let output = workspace.join("crates/adapters/src/docker/generated.rs");
    if check {
        let existing = fs::read_to_string(&output)?;
        if existing != generated {
            return Err(format!("{} is stale; run `cargo xtask docker`", output.display()).into());
        }
    } else {
        fs::write(&output, generated)?;
    }

    println!(
        "validated Docker API {} ({})",
        source.api_version, actual_hash
    );
    Ok(())
}

fn validate_operations(schema: &str) -> Result<(), Box<dyn std::error::Error>> {
    for operation in OPERATIONS {
        let path_block = indented_block(schema, &format!("  {}:", operation.path), 2)?;
        let method_block = indented_block(path_block, &format!("    {}:", operation.method), 4)?;
        let expected = format!("      operationId: \"{}\"", operation.operation_id);
        if !method_block.lines().any(|line| line.trim_end() == expected) {
            return Err(format!(
                "{} {} is missing {expected}",
                operation.method, operation.path
            )
            .into());
        }
    }
    Ok(())
}

fn validate_models(schema: &str) -> Result<(), Box<dyn std::error::Error>> {
    for (model, fields) in MODEL_FIELDS {
        let model_block = indented_block(schema, &format!("  {model}:"), 2)?;
        let properties = indented_block(model_block, "    properties:", 4)
            .map_err(|error| format!("Docker definition {model}: {error}"))?;
        for property in *fields {
            let expected = format!("      {property}:");
            if !properties.lines().any(|line| line.trim_end() == expected) {
                return Err(format!(
                    "Docker definition {model} is missing required field {property}"
                )
                .into());
            }
        }
    }
    Ok(())
}

fn indented_block<'a>(
    source: &'a str,
    marker: &str,
    indentation: usize,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    let start = source
        .match_indices(marker)
        .map(|(offset, _)| offset)
        .find(|offset| *offset == 0 || source.as_bytes().get(offset - 1) == Some(&b'\n'))
        .ok_or_else(|| format!("Docker schema is missing {marker}"))?;
    let body_start = source[start..]
        .find('\n')
        .map(|offset| start + offset + 1)
        .unwrap_or(source.len());
    let mut end = source.len();
    let prefix = " ".repeat(indentation);
    for (offset, _) in source[body_start..].match_indices('\n') {
        let next_start = body_start + offset + 1;
        let next_end = source[next_start..]
            .find('\n')
            .map(|value| next_start + value)
            .unwrap_or(source.len());
        let next = source[next_start..next_end].trim_end_matches('\r');
        if !next.is_empty() && next.starts_with(&prefix) && !next[indentation..].starts_with(' ') {
            end = next_start;
            break;
        }
    }
    Ok(&source[body_start..end])
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn normalize(path: PathBuf) -> PathBuf {
    path.components().collect()
}

fn rustfmt(source: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut child = ProcessCommand::new("rustfmt")
        .args(["--edition", "2024", "--emit", "stdout"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("rustfmt stdin is unavailable")?
        .write_all(source.as_bytes())?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(format!("rustfmt failed with {}", output.status).into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn render(api_version: &str, checksum: &str) -> String {
    let endpoints = OPERATIONS
        .iter()
        .map(|operation| format!(
            "pub const {}: Endpoint = Endpoint {{ operation_id: \"{}\", method: \"{}\", path: \"{}\", versioned: {}, streaming: {} }};",
            screaming_snake(operation.operation_id),
            operation.operation_id,
            operation.method.to_ascii_uppercase(),
            operation.path,
            operation.versioned,
            operation.streaming,
        ))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"// @generated by `cargo xtask docker`; do not edit.
// Source Docker Engine API: {api_version}; SHA-256: {checksum}

use std::collections::HashMap;
use serde::{{Deserialize, Serialize}};
use serde_json::Value;

pub const SCHEMA_API_VERSION: &str = "{api_version}";
pub const SCHEMA_SHA256: &str = "{checksum}";

// Docker emits null for some optional collection fields even when its OpenAPI
// schema declares an array or object. Treating null as empty preserves the
// generated collection type while accepting the daemon's wire representation.
fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{{
    Option::<T>::deserialize(deserializer).map(Option::unwrap_or_default)
}}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Endpoint {{
    pub operation_id: &'static str,
    pub method: &'static str,
    pub path: &'static str,
    pub versioned: bool,
    pub streaming: bool,
}}

{endpoints}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DockerVersion {{
    #[serde(rename = "Version", default)] pub version: String,
    #[serde(rename = "ApiVersion", default)] pub api_version: String,
    #[serde(rename = "MinAPIVersion", default)] pub min_api_version: String,
    #[serde(rename = "GitCommit", default)] pub git_commit: String,
    #[serde(rename = "Os", default)] pub os: String,
    #[serde(rename = "Arch", default)] pub arch: String,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DockerInfo {{
    #[serde(rename = "ID", default)] pub id: String,
    #[serde(rename = "Containers", default)] pub containers: u64,
    #[serde(rename = "ContainersRunning", default)] pub containers_running: u64,
    #[serde(rename = "ContainersPaused", default)] pub containers_paused: u64,
    #[serde(rename = "ContainersStopped", default)] pub containers_stopped: u64,
    #[serde(rename = "Images", default)] pub images: u64,
    #[serde(rename = "DockerRootDir", default)] pub docker_root_dir: String,
    #[serde(rename = "NCPU", default)] pub cpu_count: u32,
    #[serde(rename = "MemTotal", default)] pub memory_total: u64,
    #[serde(rename = "OperatingSystem", default)] pub operating_system: String,
    #[serde(rename = "OSType", default)] pub os_type: String,
    #[serde(rename = "Architecture", default)] pub architecture: String,
    #[serde(rename = "Swarm", default)] pub swarm: Option<DockerSwarmInfo>,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DockerSwarmInfo {{
    #[serde(rename = "NodeID", default)] pub node_id: String,
    #[serde(rename = "NodeAddr", default)] pub node_addr: String,
    #[serde(rename = "LocalNodeState", default)] pub local_node_state: String,
    #[serde(rename = "ControlAvailable", default)] pub control_available: bool,
    #[serde(rename = "Error", default)] pub error: String,
    #[serde(rename = "RemoteManagers", default, deserialize_with = "deserialize_null_default")] pub remote_managers: Vec<DockerPeerNode>,
    #[serde(rename = "Nodes", default)] pub nodes: i64,
    #[serde(rename = "Managers", default)] pub managers: i64,
    #[serde(rename = "Cluster", default)] pub cluster: Option<DockerClusterInfo>,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DockerPeerNode {{
    #[serde(rename = "NodeID", default)] pub node_id: String,
    #[serde(rename = "Addr", default)] pub address: String,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DockerClusterInfo {{
    #[serde(rename = "ID", default)] pub id: String,
    #[serde(rename = "CreatedAt", default)] pub created_at: String,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ContainerSummary {{
    #[serde(rename = "Id", default)] pub id: String,
    #[serde(rename = "Names", default)] pub names: Vec<String>,
    #[serde(rename = "Image", default)] pub image: String,
    #[serde(rename = "ImageID", default)] pub image_id: String,
    #[serde(rename = "Created", default)] pub created: i64,
    #[serde(rename = "Labels", default)] pub labels: HashMap<String, String>,
    #[serde(rename = "State", default)] pub state: String,
    #[serde(rename = "Status", default)] pub status: String,
    #[serde(rename = "Ports", default)] pub ports: Value,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ContainerInspect {{
    #[serde(rename = "Id", default)] pub id: String,
    #[serde(rename = "Created", default)] pub created: String,
    #[serde(rename = "Path", default)] pub path: String,
    #[serde(rename = "Args", default)] pub args: Vec<String>,
    #[serde(rename = "State", default)] pub state: ContainerState,
    #[serde(rename = "Image", default)] pub image: String,
    #[serde(rename = "Name", default)] pub name: String,
    #[serde(rename = "Config", default)] pub config: ContainerConfig,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ContainerState {{
    #[serde(rename = "Status", default)] pub status: String,
    #[serde(rename = "Running", default)] pub running: bool,
    #[serde(rename = "Paused", default)] pub paused: bool,
    #[serde(rename = "Restarting", default)] pub restarting: bool,
    #[serde(rename = "OOMKilled", default)] pub oom_killed: bool,
    #[serde(rename = "Dead", default)] pub dead: bool,
    #[serde(rename = "Pid", default)] pub pid: i64,
    #[serde(rename = "ExitCode", default)] pub exit_code: i64,
    #[serde(rename = "Error", default)] pub error: String,
    #[serde(rename = "StartedAt", default)] pub started_at: String,
    #[serde(rename = "FinishedAt", default)] pub finished_at: String,
    #[serde(rename = "Health", default)] pub health: Option<ContainerHealth>,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ContainerHealth {{
    #[serde(rename = "Status", default)] pub status: String,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ContainerConfig {{
    #[serde(rename = "Env", default)] pub environment: Vec<String>,
    #[serde(rename = "Image", default)] pub image: String,
    #[serde(rename = "Labels", default)] pub labels: HashMap<String, String>,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DockerEventActor {{
    #[serde(rename = "ID", default)] pub id: String,
    #[serde(rename = "Attributes", default)] pub attributes: HashMap<String, String>,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DockerEvent {{
    #[serde(rename = "Type", default)] pub resource_type: String,
    #[serde(rename = "Action", default)] pub action: String,
    #[serde(rename = "Actor", default)] pub actor: DockerEventActor,
    #[serde(default)] pub scope: String,
    #[serde(default)] pub time: i64,
    #[serde(rename = "timeNano", default)] pub time_nano: i64,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ContainerStats {{
    #[serde(default)] pub name: String,
    #[serde(default)] pub id: String,
    #[serde(default)] pub read: String,
    #[serde(default)] pub preread: String,
    #[serde(default)] pub cpu_stats: CpuStats,
    #[serde(default)] pub precpu_stats: CpuStats,
    #[serde(default)] pub memory_stats: MemoryStats,
    #[serde(default)] pub networks: HashMap<String, NetworkStats>,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CpuStats {{
    #[serde(default)] pub cpu_usage: CpuUsage,
    #[serde(default)] pub system_cpu_usage: u64,
    #[serde(default)] pub online_cpus: u32,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CpuUsage {{
    #[serde(default)] pub total_usage: u64,
    #[serde(default)] pub percpu_usage: Vec<u64>,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MemoryStats {{
    #[serde(default)] pub usage: u64,
    #[serde(default)] pub stats: HashMap<String, u64>,
    #[serde(default)] pub limit: u64,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct NetworkStats {{
    #[serde(default)] pub rx_bytes: u64,
    #[serde(default)] pub rx_packets: u64,
    #[serde(default)] pub rx_errors: u64,
    #[serde(default)] pub rx_dropped: u64,
    #[serde(default)] pub tx_bytes: u64,
    #[serde(default)] pub tx_packets: u64,
    #[serde(default)] pub tx_errors: u64,
    #[serde(default)] pub tx_dropped: u64,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SwarmInspect {{
    #[serde(rename = "ID", default)] pub id: String,
    #[serde(rename = "Version", default)] pub version: ObjectVersion,
    #[serde(rename = "CreatedAt", default)] pub created_at: String,
    #[serde(rename = "UpdatedAt", default)] pub updated_at: String,
    #[serde(rename = "RootRotationInProgress", default)] pub root_rotation_in_progress: bool,
    #[serde(rename = "JoinTokens", default)] pub join_tokens: JoinTokens,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ObjectVersion {{
    #[serde(rename = "Index", default)] pub index: u64,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct JoinTokens {{
    #[serde(rename = "Worker", default)] pub worker: String,
    #[serde(rename = "Manager", default)] pub manager: String,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ImageSummary {{
    #[serde(rename = "Id", default)] pub id: String,
    #[serde(rename = "RepoTags", default)] pub repo_tags: Vec<String>,
    #[serde(rename = "RepoDigests", default)] pub repo_digests: Vec<String>,
    #[serde(rename = "Created", default)] pub created: i64,
    #[serde(rename = "Size", default)] pub size: i64,
    #[serde(rename = "Labels", default)] pub labels: HashMap<String, String>,
    #[serde(rename = "Containers", default)] pub containers: i64,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct VolumeListResponse {{
    #[serde(rename = "Volumes", default, deserialize_with = "deserialize_null_default")] pub volumes: Vec<DockerVolume>,
    #[serde(rename = "Warnings", default, deserialize_with = "deserialize_null_default")] pub warnings: Vec<String>,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DockerVolume {{
    #[serde(rename = "Name", default)] pub name: String,
    #[serde(rename = "Driver", default)] pub driver: String,
    #[serde(rename = "Mountpoint", default)] pub mountpoint: String,
    #[serde(rename = "CreatedAt", default)] pub created_at: String,
    #[serde(rename = "Status", default, deserialize_with = "deserialize_null_default")] pub status: HashMap<String, Value>,
    #[serde(rename = "Labels", default, deserialize_with = "deserialize_null_default")] pub labels: HashMap<String, String>,
    #[serde(rename = "Scope", default)] pub scope: String,
    #[serde(rename = "ClusterVolume", default)] pub cluster_volume: Option<Value>,
    #[serde(rename = "Options", default, deserialize_with = "deserialize_null_default")] pub options: HashMap<String, String>,
    #[serde(rename = "UsageData", default)] pub usage_data: Option<Value>,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct VolumeCreateOptions {{
    #[serde(rename = "Name", default)] pub name: String,
    #[serde(rename = "Driver", default)] pub driver: String,
    #[serde(rename = "DriverOpts", default)] pub driver_options: HashMap<String, String>,
    #[serde(rename = "Labels", default)] pub labels: HashMap<String, String>,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DockerNetwork {{
    #[serde(rename = "Name", default)] pub name: String,
    #[serde(rename = "Id", default)] pub id: String,
    #[serde(rename = "Created", default)] pub created: String,
    #[serde(rename = "Scope", default)] pub scope: String,
    #[serde(rename = "Driver", default)] pub driver: String,
    #[serde(rename = "EnableIPv4", default)] pub enable_ipv4: bool,
    #[serde(rename = "EnableIPv6", default)] pub enable_ipv6: bool,
    #[serde(rename = "IPAM", default)] pub ipam: Option<Value>,
    #[serde(rename = "Internal", default)] pub internal: bool,
    #[serde(rename = "Attachable", default)] pub attachable: bool,
    #[serde(rename = "Ingress", default)] pub ingress: bool,
    #[serde(rename = "ConfigFrom", default)] pub config_from: Option<Value>,
    #[serde(rename = "ConfigOnly", default)] pub config_only: bool,
    #[serde(rename = "Containers", default, deserialize_with = "deserialize_null_default")] pub containers: HashMap<String, Value>,
    #[serde(rename = "Peers", default, deserialize_with = "deserialize_null_default")] pub peers: Vec<Value>,
    #[serde(rename = "Options", default)] pub options: HashMap<String, String>,
    #[serde(rename = "Labels", default)] pub labels: HashMap<String, String>,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct NetworkCreateRequest {{
    #[serde(rename = "Name")] pub name: String,
    #[serde(rename = "Driver", skip_serializing_if = "Option::is_none")] pub driver: Option<String>,
    #[serde(rename = "Scope", skip_serializing_if = "Option::is_none")] pub scope: Option<String>,
    #[serde(rename = "Internal", skip_serializing_if = "Option::is_none")] pub internal: Option<bool>,
    #[serde(rename = "Attachable", skip_serializing_if = "Option::is_none")] pub attachable: Option<bool>,
    #[serde(rename = "Ingress", skip_serializing_if = "Option::is_none")] pub ingress: Option<bool>,
    #[serde(rename = "EnableIPv6", skip_serializing_if = "Option::is_none")] pub enable_ipv6: Option<bool>,
    #[serde(rename = "EnableIPv4", skip_serializing_if = "Option::is_none")] pub enable_ipv4: Option<bool>,
    #[serde(rename = "ConfigOnly", skip_serializing_if = "Option::is_none")] pub config_only: Option<bool>,
    #[serde(rename = "IPAM", skip_serializing_if = "Option::is_none")] pub ipam: Option<Value>,
    #[serde(rename = "ConfigFrom", skip_serializing_if = "Option::is_none")] pub config_from: Option<Value>,
    #[serde(rename = "Labels", default)] pub labels: HashMap<String, String>,
    #[serde(rename = "Options", default)] pub options: HashMap<String, String>,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct NetworkCreateResponse {{
    #[serde(rename = "Id", default)] pub id: String,
    #[serde(rename = "Warning", default)] pub warning: String,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SwarmNode {{
    #[serde(rename = "ID", default)] pub id: String,
    #[serde(rename = "Version", default)] pub version: ObjectVersion,
    #[serde(rename = "CreatedAt", default)] pub created_at: String,
    #[serde(rename = "UpdatedAt", default)] pub updated_at: String,
    #[serde(rename = "Spec", default)] pub spec: Value,
    #[serde(rename = "Description", default)] pub description: Value,
    #[serde(rename = "Status", default)] pub status: Value,
    #[serde(rename = "ManagerStatus", default)] pub manager_status: Value,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SwarmService {{
    #[serde(rename = "ID", default)] pub id: String,
    #[serde(rename = "Version", default)] pub version: ObjectVersion,
    #[serde(rename = "CreatedAt", default)] pub created_at: String,
    #[serde(rename = "UpdatedAt", default)] pub updated_at: String,
    #[serde(rename = "Spec", default)] pub spec: Value,
    #[serde(rename = "Endpoint", default)] pub endpoint: Value,
    #[serde(rename = "UpdateStatus", default)] pub update_status: Value,
    #[serde(rename = "ServiceStatus", default)] pub service_status: Value,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SwarmTask {{
    #[serde(rename = "ID", default)] pub id: String,
    #[serde(rename = "Version", default)] pub version: ObjectVersion,
    #[serde(rename = "CreatedAt", default)] pub created_at: String,
    #[serde(rename = "UpdatedAt", default)] pub updated_at: String,
    #[serde(rename = "Name", default)] pub name: String,
    #[serde(rename = "Spec", default)] pub spec: Value,
    #[serde(rename = "ServiceID", default)] pub service_id: String,
    #[serde(rename = "Slot", default)] pub slot: Option<i32>,
    #[serde(rename = "NodeID", default)] pub node_id: String,
    #[serde(rename = "Status", default)] pub status: Value,
    #[serde(rename = "DesiredState", default)] pub desired_state: String,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SwarmSecret {{
    #[serde(rename = "ID", default)] pub id: String,
    #[serde(rename = "Version", default)] pub version: ObjectVersion,
    #[serde(rename = "CreatedAt", default)] pub created_at: String,
    #[serde(rename = "UpdatedAt", default)] pub updated_at: String,
    #[serde(rename = "Spec", default)] pub spec: Value,
}}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SwarmConfig {{
    #[serde(rename = "ID", default)] pub id: String,
    #[serde(rename = "Version", default)] pub version: ObjectVersion,
    #[serde(rename = "CreatedAt", default)] pub created_at: String,
    #[serde(rename = "UpdatedAt", default)] pub updated_at: String,
    #[serde(rename = "Spec", default)] pub spec: Value,
}}
"#
    )
}

fn screaming_snake(value: &str) -> String {
    let mut output = String::new();
    for (index, character) in value.chars().enumerate() {
        if index > 0 && character.is_ascii_uppercase() {
            output.push('_');
        }
        output.push(character.to_ascii_uppercase());
    }
    output
}
