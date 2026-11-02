//! Local-operator registration. No update, container replacement or database write.
use super::files::validate_absolute;
use super::registration::fingerprint_inputs;
use super::{InstallerComposeInputs, JournalDirectory, RegisteredComposeInstallation};
use citadel_execution::{ProcessError, ProcessLimits, ProcessRequest, ProcessRunner};
use citadel_updates::{INSTALLER_FORMAT, UpdateError, is_sha256};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const SOCKET: &str = "/var/run/docker.sock";
const REGISTRATION: &str = "installation.json";

/// Paths come from the local operator's original invocation, never from HTTP.
/// The state directory must already exist with the intended executor's ownership.
pub async fn register_compose_installation(
    inputs: &InstallerComposeInputs,
    state_directory: PathBuf,
    process: &dyn ProcessRunner,
    cancellation: &CancellationToken,
) -> Result<RegisteredComposeInstallation, UpdateError> {
    validate_absolute(&state_directory)?;
    if state_directory == inputs.directory || state_directory == Path::new("/") {
        return Err(refuse("Use a dedicated private update state directory."));
    }
    let state = JournalDirectory::open(state_directory.clone())?;
    let _lock = state.lock()?;
    // Invalid or unreadable existing evidence is never treated as missing.
    let existing: Option<RegisteredComposeInstallation> = state.read_optional(REGISTRATION)?;
    if let Some(existing) = &existing {
        existing.validate()?;
    }
    if existing.is_none() {
        state.require_unregistered()?;
    }
    let (checked, config) = inputs.resolve(process, cancellation).await?;
    let home = tempfile::Builder::new()
        .prefix("citadel-register-")
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()
        .map_err(|_| failed())?;
    let docker = LocalDocker {
        process,
        cancellation,
        home: home.path(),
        deadline: Instant::now() + Duration::from_secs(120),
    };
    verify_executor(&docker, inputs, &state_directory).await?;
    let first = observe(&docker, inputs, &config, &checked, &state_directory).await?;
    let identity = docker.json(&["exec", &first.core, "/app/citadel-server", "instance-identity-json"]).await
        .map_err(|error| match error { UpdateError::Cancelled => error, _ => refuse("Core must support instance-identity-json and have an initialized, reachable Citadel database.") })?;
    let instance_id = identity["instanceId"]
        .as_str()
        .and_then(|id| Uuid::parse_str(id).ok())
        .filter(|id| !id.is_nil())
        .ok_or_else(failed)?;
    // Bind only stable observations; a concurrent replacement or input edit
    // invalidates this attempt. Preflight must repeat these checks before use.
    let second = observe(&docker, inputs, &config, &checked, &state_directory).await?;
    if first != second
        || fingerprint_inputs(&inputs.project, &inputs.capture()?) != checked.configuration_sha256
    {
        return Err(UpdateError::Conflict(
            "Installation changed during registration; retry after it is stable.".into(),
        ));
    }
    if cancellation.is_cancelled() {
        return Err(UpdateError::Cancelled);
    }
    let registration = RegisteredComposeInstallation {
        format: INSTALLER_FORMAT,
        kind: citadel_updates::InstallationKind::DockerCompose,
        id: existing.as_ref().map(|r| r.id).unwrap_or_else(Uuid::now_v7),
        instance_id,
        directory: inputs.directory.clone(),
        project: inputs.project.clone(),
        compose_files: inputs.compose_files.clone(),
        environment_file: inputs.directory.join(".env"),
        docker_socket: SOCKET.into(),
        daemon_id: first.daemon,
        core_container_id: first.core,
        database_container_id: first.database,
        core_volume: checked.core_volume,
        database_volume: checked.database_volume,
        network: checked.network,
        state_directory,
        executor_uid: rustix::process::geteuid().as_raw(),
        executor_gid: rustix::process::getegid().as_raw(),
        configuration_sha256: checked.configuration_sha256,
        runtime_sha256: first.fingerprint,
    };
    registration.validate()?;
    state.verify_location()?;
    if let Some(existing) = existing {
        if existing != registration {
            return Err(UpdateError::Conflict("A different installation is already registered. Existing registration and recovery state were preserved.".into()));
        }
        // A previous create may have published the entry before directory fsync
        // failed. An idempotent acknowledgement must make that entry durable.
        state.sync()?;
        return Ok(existing);
    }
    state.create(REGISTRATION, &registration)?;
    Ok(registration)
}

struct LocalDocker<'a> {
    process: &'a dyn ProcessRunner,
    cancellation: &'a CancellationToken,
    home: &'a Path,
    deadline: Instant,
}
impl LocalDocker<'_> {
    async fn json(&self, arguments: &[&str]) -> Result<Value, UpdateError> {
        serde_json::from_slice(&self.run(arguments).await?).map_err(|_| failed())
    }
    async fn containers(&self, filter: &str) -> Result<Vec<String>, UpdateError> {
        let bytes = self
            .run(&[
                "container",
                "ls",
                "--all",
                "--no-trunc",
                "--filter",
                filter,
                "--format",
                "{{json .ID}}",
            ])
            .await?;
        let text = std::str::from_utf8(&bytes).map_err(|_| failed())?;
        let mut ids = text
            .lines()
            .map(|line| serde_json::from_str::<String>(line).map_err(|_| failed()))
            .collect::<Result<Vec<_>, _>>()?;
        if ids.iter().any(|id| !is_sha256(id)) {
            return Err(failed());
        }
        ids.sort();
        Ok(ids)
    }
    async fn run(&self, arguments: &[&str]) -> Result<Vec<u8>, UpdateError> {
        if self.cancellation.is_cancelled() {
            return Err(UpdateError::Cancelled);
        }
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(refuse("Installation inspection timed out."));
        }
        let request = ProcessRequest::new("docker")
            .args(
                ["--host", "unix:///var/run/docker.sock"]
                    .into_iter()
                    .chain(arguments.iter().copied()),
            )
            .env("HOME", self.home)
            .env("DOCKER_CONFIG", self.home)
            .limits(ProcessLimits {
                timeout: remaining.min(Duration::from_secs(15)),
                maximum_stdout_bytes: 1024 * 1024,
                maximum_stderr_bytes: 16 * 1024,
                ..ProcessLimits::default()
            });
        let output =
            self.process
                .run(request, self.cancellation)
                .await
                .map_err(|error| match error {
                    ProcessError::Cancelled => UpdateError::Cancelled,
                    _ => failed(),
                })?;
        if !output.succeeded() || output.stdout_truncated || output.stderr_truncated {
            return Err(failed());
        }
        Ok(output.stdout)
    }
}

#[derive(PartialEq, Eq)]
struct Observation {
    daemon: String,
    core: String,
    database: String,
    fingerprint: String,
}

async fn observe(
    docker: &LocalDocker<'_>,
    inputs: &InstallerComposeInputs,
    config: &Value,
    checked: &super::CheckedInstallerCompose,
    state: &Path,
) -> Result<Observation, UpdateError> {
    let info = docker.json(&["info", "--format", "{{json .}}"]).await?;
    if info["OSType"] != "linux"
        || info["ID"].as_str().is_none_or(str::is_empty)
        || info["SecurityOptions"].as_array().is_none_or(|options| {
            options
                .iter()
                .any(|v| v.as_str().is_none_or(|v| v.contains("rootless")))
        })
    {
        return Err(refuse(
            "Registration requires a rootful local Linux Docker daemon.",
        ));
    }
    let docker_root = info["DockerRootDir"].as_str().ok_or_else(failed)?;
    validate_absolute(Path::new(docker_root))?;
    if state.starts_with(docker_root) {
        return Err(refuse(
            "Update state must be stored outside Docker's data directory.",
        ));
    }
    let project_filter = format!("label=com.docker.compose.project={}", inputs.project);
    let ids = docker.containers(&project_filter).await?;
    if ids.len() != 2 {
        return Err(refuse(
            "The registered project must contain exactly one Core and one packaged PostgreSQL container.",
        ));
    }
    let mut services = BTreeMap::new();
    for id in &ids {
        let raw = docker.json(&["container", "inspect", id]).await?;
        let inspected = single(&raw)?;
        if inspected["Id"] != *id {
            return Err(failed());
        }
        let role = inspected["Config"]["Labels"]["com.docker.compose.service"]
            .as_str()
            .ok_or_else(failed)?;
        if !["server", "pg_db"].contains(&role)
            || services
                .insert(role.to_owned(), inspected.clone())
                .is_some()
        {
            return Err(refuse(
                "The registered project has ambiguous Core or database services.",
            ));
        }
    }
    let core = services.get("server").ok_or_else(failed)?;
    let database = services.get("pg_db").ok_or_else(failed)?;
    for (role, actual) in [("server", core), ("pg_db", database)] {
        validate_service(
            inputs,
            &config["services"][role],
            actual,
            role,
            &checked.network,
        )?;
    }
    validate_mounts(
        core,
        &[
            ("volume", &checked.core_volume, "/app/data", true),
            ("bind", SOCKET, SOCKET, true),
            ("bind", "/", "/host", false),
        ],
    )?;
    validate_mounts(
        database,
        &[(
            "volume",
            &checked.database_volume,
            "/var/lib/postgresql",
            true,
        )],
    )?;
    let mut volumes = Vec::new();
    for (name, role, container) in [
        (&checked.core_volume, "core", core),
        (&checked.database_volume, "database", database),
    ] {
        let id = container["Id"].as_str().ok_or_else(failed)?;
        if docker.containers(&format!("volume={name}")).await? != [id] {
            return Err(refuse(
                "A Citadel persistent volume is shared with another container; registration is ambiguous.",
            ));
        }
        let raw = docker.json(&["volume", "inspect", name]).await?;
        let volume = single(&raw)?;
        let mountpoint = volume["Mountpoint"].as_str().ok_or_else(failed)?;
        validate_absolute(Path::new(mountpoint))?;
        if volume["Name"] != *name
            || volume["Driver"] != "local"
            || volume["Scope"] != "local"
            || volume["Labels"]["com.citadel.system"] != "true"
            || volume["Labels"]["com.citadel.system-role"] != role
            || volume
                .get("Options")
                .is_some_and(|v| !v.is_null() && v.as_object().is_none_or(|v| !v.is_empty()))
            || state.starts_with(mountpoint)
        {
            return Err(refuse(
                "Updates require distinct local Citadel volumes and update state outside those volumes.",
            ));
        }
        volumes.push(volume.clone());
    }
    let raw = docker
        .json(&["network", "inspect", &checked.network])
        .await?;
    let network = single(&raw)?;
    let network_id = network["Id"]
        .as_str()
        .filter(|id| is_sha256(id))
        .ok_or_else(failed)?;
    if network["Name"] != checked.network
        || network["Driver"] != "bridge"
        || network["Scope"] != "local"
        || network["Labels"]["com.docker.compose.project"] != inputs.project
    {
        return Err(refuse(
            "The installation must use its registered local Compose bridge network.",
        ));
    }
    for container in [core, database] {
        let networks = container["NetworkSettings"]["Networks"]
            .as_object()
            .ok_or_else(failed)?;
        if networks.len() != 1
            || networks
                .get(&checked.network)
                .is_none_or(|v| v["NetworkID"] != network_id)
        {
            return Err(refuse(
                "Running Citadel network attachments differ from the supplied installation.",
            ));
        }
    }
    let members = network["Containers"].as_object().ok_or_else(failed)?;
    let aliases = |container: &Value| -> Result<Vec<String>, UpdateError> {
        container["NetworkSettings"]["Networks"][&checked.network]["Aliases"]
            .as_array()
            .ok_or_else(failed)?
            .iter()
            .map(|v| v.as_str().map(str::to_owned).ok_or_else(failed))
            .collect()
    };
    if members.len() != 2
        || ids.iter().any(|id| !members.contains_key(id))
        || !aliases(database)?.iter().any(|alias| alias == "pg_db")
        || aliases(core)?.iter().any(|alias| alias == "pg_db")
    {
        return Err(refuse(
            "The packaged database network identity is ambiguous.",
        ));
    }
    // Ignore health samples, restart counters and IP addresses. Capture immutable
    // container/image identity and configuration, volume identity and network ID.
    let stable = json!({"daemon": info["ID"], "core": stable_container(core), "database": stable_container(database), "volumes": volumes, "networkId": network_id});
    let fingerprint = Sha256::digest(serde_json::to_vec(&stable).map_err(|_| failed())?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(Observation {
        daemon: info["ID"].as_str().ok_or_else(failed)?.into(),
        core: core["Id"].as_str().ok_or_else(failed)?.into(),
        database: database["Id"].as_str().ok_or_else(failed)?.into(),
        fingerprint,
    })
}

fn stable_container(container: &Value) -> Value {
    json!({"id": container["Id"], "image": container["Image"], "config": container["Config"], "hostConfig": container["HostConfig"], "mounts": container["Mounts"]})
}

fn validate_service(
    inputs: &InstallerComposeInputs,
    expected: &Value,
    actual: &Value,
    role: &str,
    network: &str,
) -> Result<(), UpdateError> {
    let labels = &actual["Config"]["Labels"];
    let files = inputs
        .compose_files
        .iter()
        .map(|p| p.to_str().ok_or_else(failed))
        .collect::<Result<Vec<_>, _>>()?
        .join(",");
    if actual["State"]["Running"] != true
        || actual["State"]["Paused"] != false
        || actual["State"]["Restarting"] != false
        || actual["State"]["Health"]["Status"] != "healthy"
        || actual["Config"]["Image"] != expected["image"]
        || labels["com.docker.compose.project"] != inputs.project
        || labels["com.docker.compose.service"] != role
        || labels["com.docker.compose.oneoff"] != "False"
        || labels["com.docker.compose.container-number"] != "1"
        || labels["com.docker.compose.project.working_dir"].as_str() != inputs.directory.to_str()
        || labels["com.docker.compose.project.config_files"] != files
        || labels.get("com.docker.swarm.service.id").is_some()
        || actual["HostConfig"]["Privileged"] != false
        || actual["HostConfig"]["NetworkMode"] != network
        || actual["HostConfig"]["RestartPolicy"]["Name"] != "unless-stopped"
        || ["ExtraHosts", "Links"].iter().any(|key| {
            actual["HostConfig"].get(*key).is_some_and(|value| {
                !value.is_null() && value.as_array().is_none_or(|v| !v.is_empty())
            })
        })
    {
        return Err(refuse(
            "Running Citadel services do not match the supplied Compose invocation or are not stable.",
        ));
    }
    let mut environment = BTreeMap::new();
    for entry in actual["Config"]["Env"].as_array().ok_or_else(failed)? {
        let (key, value) = entry
            .as_str()
            .and_then(|v| v.split_once('='))
            .ok_or_else(failed)?;
        if environment.insert(key, value).is_some() {
            return Err(failed());
        }
    }
    for (key, value) in expected["environment"].as_object().ok_or_else(failed)? {
        if value.as_str() != environment.get(key.as_str()).copied() {
            return Err(refuse(
                "Running service environment differs from the private installer inputs.",
            ));
        }
    }
    if role == "server" {
        for key in [
            "CITADEL_DATABASE_URL",
            "CITADEL_CORE_AGENT_ADDRESS",
            "CITADEL_CORE_AGENT_PRIVATE_KEY_PATH",
        ] {
            if environment.contains_key(key) {
                return Err(refuse(
                    "Core has an unsupported database or Agent override.",
                ));
            }
        }
        for (key, value) in [
            ("CITADEL_DATA_ROOT", "/app/data"),
            ("CITADEL_DOCKER_SOCKET", SOCKET),
        ] {
            if environment.get(key).is_some_and(|actual| *actual != value) {
                return Err(refuse(
                    "Core has an unsupported persistence or Docker endpoint override.",
                ));
            }
        }
    }
    let mut ports: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    if let Some(expected_ports) = expected.get("ports") {
        for port in expected_ports.as_array().ok_or_else(failed)? {
            let key = format!(
                "{}/{}",
                port["target"].as_u64().ok_or_else(failed)?,
                port["protocol"].as_str().ok_or_else(failed)?
            );
            ports
                .entry(key)
                .or_default()
                .push(json!({"HostIp": port["host_ip"], "HostPort": port["published"]}));
        }
    }
    let expected_ports = serde_json::to_value(ports).map_err(|_| failed())?;
    let actual_ports = &actual["HostConfig"]["PortBindings"];
    if *actual_ports != expected_ports && !(actual_ports.is_null() && expected_ports == json!({})) {
        return Err(refuse(
            "Running service ports differ from the supplied Compose configuration.",
        ));
    }
    Ok(())
}

fn validate_mounts(
    container: &Value,
    expected: &[(&str, &str, &str, bool)],
) -> Result<(), UpdateError> {
    let mounts = container["Mounts"].as_array().ok_or_else(failed)?;
    if mounts.len() != expected.len()
        || expected.iter().any(|(kind, source, target, writable)| {
            !mounts.iter().any(|mount| {
                mount["Type"] == *kind
                    && mount[if *kind == "volume" { "Name" } else { "Source" }] == *source
                    && mount["Destination"] == *target
                    && mount["RW"] == *writable
            })
        })
    {
        return Err(refuse(
            "Running Citadel storage mounts differ from the registered installer profile.",
        ));
    }
    Ok(())
}

async fn verify_executor(
    docker: &LocalDocker<'_>,
    inputs: &InstallerComposeInputs,
    state: &Path,
) -> Result<(), UpdateError> {
    if !Path::new("/.dockerenv").exists() {
        return Ok(());
    }
    let hostname = fs::read_to_string("/proc/sys/kernel/hostname").map_err(|_| failed())?;
    let hostname = hostname.trim();
    if hostname.len() != 12 || !hostname.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(refuse(
            "Run registration with Docker's default container hostname.",
        ));
    }
    let raw = docker.json(&["container", "inspect", hostname]).await?;
    let executor = single(&raw)?;
    validate_executor(executor, hostname, inputs, state)
}

fn validate_executor(
    executor: &Value,
    hostname: &str,
    inputs: &InstallerComposeInputs,
    state: &Path,
) -> Result<(), UpdateError> {
    if executor["Id"]
        .as_str()
        .is_none_or(|id| !is_sha256(id) || !id.starts_with(hostname))
        || executor["Config"]["Hostname"] != hostname
    {
        return Err(refuse(
            "Cannot establish registration executor identity on the local daemon.",
        ));
    }
    let mounts = executor["Mounts"].as_array().ok_or_else(failed)?;
    if mounts.len() != 3 {
        return Err(refuse(
            "Registration executor must mount only installer inputs, update state and the Docker socket.",
        ));
    }
    for (path, writable) in [
        (inputs.directory.as_path(), false),
        (state, true),
        (Path::new(SOCKET), true),
    ] {
        if !mounts.iter().any(|m| {
            m["Type"] == "bind"
                && m["Source"].as_str() == path.to_str()
                && m["Destination"].as_str() == path.to_str()
                && m["RW"] == writable
        }) {
            return Err(refuse(
                "Mount installer inputs read-only, update state read-write and the Docker socket at their exact host paths.",
            ));
        }
    }
    Ok(())
}

fn single(value: &Value) -> Result<&Value, UpdateError> {
    value
        .as_array()
        .filter(|v| v.len() == 1)
        .map(|v| &v[0])
        .ok_or_else(failed)
}
fn refuse(message: &str) -> UpdateError {
    UpdateError::Unavailable(message.into())
}
fn failed() -> UpdateError {
    refuse(
        "Cannot safely inspect the local Citadel installation; Docker output was withheld because it may contain secrets.",
    )
}

#[cfg(test)]
mod tests;
