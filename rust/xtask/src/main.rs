#![forbid(unsafe_code)]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Stdio};

use clap::{Parser, Subcommand};
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate the pinned Docker schema and regenerate the Phase 0 subset.
    GenerateDocker {
        /// Exit non-zero when regeneration changes the checked-in file.
        #[arg(long)]
        check: bool,
    },
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
        ],
    ),
    (
        "ContainerSummary",
        &[
            "Id", "Names", "Image", "ImageID", "Created", "Labels", "State", "Status",
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
        ],
    ),
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
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::GenerateDocker { check } => generate_docker(check),
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
            return Err(format!(
                "{} is stale; run `cargo xtask generate-docker`",
                output.display()
            )
            .into());
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
        let properties = indented_block(model_block, "    properties:", 4)?;
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
        .find(marker)
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
        r#"// @generated by `cargo xtask generate-docker`; do not edit.
// Source Docker Engine API: {api_version}; SHA-256: {checksum}

use std::collections::HashMap;
use serde::{{Deserialize, Serialize}};

pub const SCHEMA_API_VERSION: &str = "{api_version}";
pub const SCHEMA_SHA256: &str = "{checksum}";

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
