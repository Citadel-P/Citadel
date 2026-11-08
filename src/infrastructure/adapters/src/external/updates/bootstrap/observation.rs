use super::super::InstallerComposeInputs;
use super::super::docker::{LocalDocker, SOCKET, failed, refuse, single};
use super::super::files::validate_absolute;
use citadel_updates::{UpdateError, is_sha256};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};

#[derive(PartialEq, Eq)]
pub(super) struct Observation {
    pub(super) daemon: String,
    pub(super) core: String,
    pub(super) database: String,
    pub(super) fingerprint: String,
    pub(super) retained_fingerprint: String,
    pub(super) core_image: String,
    pub(super) core_started_at: String,
}

/// Target process liveness is distinct from application readiness during maintenance.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum CoreState {
    Healthy,
    Stopped,
    TargetRunning,
    AcceptedTargetRunning,
}

pub(super) async fn observe(
    docker: &LocalDocker<'_>,
    inputs: &InstallerComposeInputs,
    config: &Value,
    checked: &super::super::CheckedInstallerCompose,
    state: &Path,
) -> Result<Observation, UpdateError> {
    observe_with_helper(
        docker,
        inputs,
        config,
        checked,
        state,
        None,
        CoreState::Healthy,
    )
    .await
}

pub(super) async fn observe_with_helper(
    docker: &LocalDocker<'_>,
    inputs: &InstallerComposeInputs,
    config: &Value,
    checked: &super::super::CheckedInstallerCompose,
    state: &Path,
    helper: Option<&str>,
    core_state: CoreState,
) -> Result<Observation, UpdateError> {
    let stopped = core_state == CoreState::Stopped;
    let info = docker.json(&["info", "--format", "{{json .}}"]).await?;
    if info["OSType"] != "linux"
        || info["ID"].as_str().is_none_or(str::is_empty)
        || info["SecurityOptions"]
            .as_array()
            .is_none_or(|options| options.iter().any(|v| v.as_str().is_none()))
    {
        return Err(refuse("Registration requires a local Linux Docker daemon."));
    }
    let socket = super::super::layout::compose_socket(config)?;
    let rootless = super::super::layout::rootless_socket(Path::new(socket))?;
    let daemon_rootless = info["SecurityOptions"]
        .as_array()
        .ok_or_else(failed)?
        .iter()
        .any(|v| v.as_str() == Some("name=rootless"));
    if rootless != daemon_rootless {
        return Err(refuse(
            "The registered Docker socket does not match the daemon rootless mode.",
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
            if role == "server" {
                core_state
            } else {
                CoreState::Healthy
            },
        )?;
    }
    let host_source = if rootless { docker_root } else { "/" };
    let host_target = if rootless {
        format!("/host{docker_root}")
    } else {
        "/host".into()
    };
    validate_mounts(
        core,
        &[
            ("volume", &checked.core_volume, "/app/data", true),
            ("bind", socket, SOCKET, true),
            ("bind", host_source, &host_target, false),
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
        let mut expected = vec![id.to_owned()];
        if role == "core"
            && let Some(helper) = helper
        {
            expected.push(helper.to_owned());
        }
        expected.sort();
        if docker.containers(&format!("volume={name}")).await? != expected {
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
            || networks.get(&checked.network).is_none_or(|v| {
                v["NetworkID"] != network_id
                    && !(stopped && container["Id"] == core["Id"] && v["NetworkID"] == "")
            })
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
    let mut expected_members = if stopped {
        vec![database["Id"].as_str().ok_or_else(failed)?]
    } else {
        ids.iter().map(String::as_str).collect()
    };
    if let Some(helper) = helper.filter(|id| members.contains_key(*id)) {
        let raw = docker.json(&["container", "inspect", helper]).await?;
        if single(&raw)?["HostConfig"]["NetworkMode"] == checked.network {
            expected_members.push(helper);
        }
    }
    if members.len() != expected_members.len()
        || expected_members.iter().any(|id| !members.contains_key(*id))
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
    let retained = json!({"daemon": info["ID"], "database": stable_container(database),
        "databaseStartedAt": started_at(database)?, "databaseRestarts": database["RestartCount"].as_u64().ok_or_else(failed)?,
        "volumes": volumes, "networkId": network_id});
    let retained_fingerprint = Sha256::digest(serde_json::to_vec(&retained).map_err(|_| failed())?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(Observation {
        daemon: info["ID"].as_str().ok_or_else(failed)?.into(),
        core: core["Id"].as_str().ok_or_else(failed)?.into(),
        database: database["Id"].as_str().ok_or_else(failed)?.into(),
        fingerprint,
        retained_fingerprint,
        core_image: core["Image"].as_str().ok_or_else(failed)?.into(),
        core_started_at: started_at(core)?.into(),
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
    state: CoreState,
) -> Result<(), UpdateError> {
    let labels = &actual["Config"]["Labels"];
    let files = inputs
        .compose_files
        .iter()
        .map(|p| p.to_str().ok_or_else(failed))
        .collect::<Result<Vec<_>, _>>()?
        .join(",");
    let healthy = if state == CoreState::Stopped {
        actual["State"]["Running"] == false
            && actual["State"]["Status"] == "exited"
            && actual["State"]["ExitCode"] == 0
            && actual["State"]["OOMKilled"] == false
            && actual["State"]["Error"] == ""
    } else if matches!(
        state,
        CoreState::TargetRunning | CoreState::AcceptedTargetRunning
    ) {
        // A maintenance listener deliberately fails readiness. Do not require a
        // healthy probe to identify the target, or infer completion from one.
        actual["State"]["Running"] == true
            && actual["State"]["Status"] == "running"
            && actual["State"]["OOMKilled"] == false
            && actual["State"]["Error"] == ""
            && actual["RestartCount"]
                .as_u64()
                .is_some_and(|count| state == CoreState::AcceptedTargetRunning || count == 0)
            && matches!(
                actual["State"]["Health"]["Status"].as_str(),
                Some("starting" | "healthy" | "unhealthy")
            )
    } else {
        actual["State"]["Running"] == true && actual["State"]["Health"]["Status"] == "healthy"
    };
    if !healthy
        || actual["State"]["Paused"] != false
        || actual["State"]["Restarting"] != false
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

fn started_at(container: &Value) -> Result<&str, UpdateError> {
    let value = container["State"]["StartedAt"]
        .as_str()
        .ok_or_else(failed)?;
    if chrono::DateTime::parse_from_rfc3339(value).is_err() || value.starts_with("0001-") {
        return Err(failed());
    }
    Ok(value)
}
