//! Resolve a known installer invocation without trusting ambient Compose settings.
use super::super::files::validate_absolute;
use super::super::registration::{fingerprint_inputs, identifier, read_input};
use citadel_execution::{ProcessError, ProcessLimits, ProcessRequest, ProcessRunner};
use citadel_instance::environment;
use citadel_updates::UpdateError;
use serde::Serialize;
use serde_json::Value;
use std::{collections::HashSet, fs, os::unix::fs::PermissionsExt, path::PathBuf, time::Duration};
use tokio_util::sync::CancellationToken;

/// Local operator input, never an HTTP request. Paths are the actual host paths;
/// a container running this check must mount the inputs at those same paths.
pub struct InstallerComposeInputs {
    pub directory: PathBuf,
    pub project: String,
    pub compose_files: Vec<PathBuf>,
}

/// Configuration evidence only: this does not establish live runtime identity,
/// registration, a safe database transition or permission to replace Core.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckedInstallerCompose {
    pub project: String,
    pub configuration_sha256: String,
    pub core_image: String,
    pub core_volume: String,
    pub database_volume: String,
    pub network: String,
}

impl InstallerComposeInputs {
    pub(in crate::external::updates) fn capture(
        &self,
    ) -> Result<Vec<(PathBuf, Vec<u8>)>, UpdateError> {
        validate_absolute(&self.directory)?;
        if self.directory == std::path::Path::new("/")
            || self.project.len() > 63
            || !self
                .project
                .bytes()
                .next()
                .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
            || !self
                .project
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_-".contains(&b))
        {
            return Err(unsupported(
                "Use a canonical installation directory and an explicit lowercase Compose project name.",
            ));
        }
        if self.compose_files.is_empty() || self.compose_files.len() > 4 {
            return Err(unsupported(
                "Supply one to four Compose files in their original invocation order.",
            ));
        }
        let mut paths = HashSet::new();
        let mut captured = Vec::new();
        for path in &self.compose_files {
            validate_absolute(path)?;
            if path.parent() != Some(self.directory.as_path())
                || !matches!(
                    path.extension().and_then(|v| v.to_str()),
                    Some("yml" | "yaml")
                )
                || !paths.insert(path)
            {
                return Err(unsupported(
                    "Compose files must be distinct YAML files directly inside the installation directory.",
                ));
            }
            let bytes = read_input(path, false)?;
            // Validate the dependency boundary before Compose can read any file
            // outside the captured snapshot or resolve a remote include.
            super::profile::validate_input(&bytes)?;
            captured.push((path.clone(), bytes));
        }
        let environment = self.directory.join(".env");
        let bytes = read_input(&environment, true)?;
        validate_environment(&bytes, &self.project)?;
        captured.push((environment, bytes));
        Ok(captured)
    }

    pub async fn check(
        &self,
        process: &dyn ProcessRunner,
        cancellation: &CancellationToken,
    ) -> Result<CheckedInstallerCompose, UpdateError> {
        self.resolve(process, cancellation)
            .await
            .map(|(checked, _)| checked)
    }

    pub(in crate::external::updates) async fn resolve(
        &self,
        process: &dyn ProcessRunner,
        cancellation: &CancellationToken,
    ) -> Result<(CheckedInstallerCompose, Value), UpdateError> {
        if cancellation.is_cancelled() {
            return Err(UpdateError::Cancelled);
        }
        let captured = self.capture()?;
        let fingerprint = fingerprint_inputs(&self.project, &captured);
        let config = self
            .resolve_snapshot(&captured, process, cancellation)
            .await?;
        let checked = self.validate_resolved(&config, fingerprint.clone())?;
        if fingerprint_inputs(&self.project, &self.capture()?) != fingerprint {
            return Err(UpdateError::Conflict(
                "Installation inputs changed during validation; run the check again.".into(),
            ));
        }
        Ok((checked, config))
    }

    pub(super) async fn resolve_snapshot(
        &self,
        captured: &[(PathBuf, Vec<u8>)],
        process: &dyn ProcessRunner,
        cancellation: &CancellationToken,
    ) -> Result<Value, UpdateError> {
        // Copy validated bytes, not paths, so Compose cannot follow a changed
        // input or discover unrelated files between validation and execution.
        let snapshot = tempfile::Builder::new()
            .prefix("citadel-compose-check-")
            .permissions(fs::Permissions::from_mode(0o700))
            .tempdir()
            .map_err(|_| resolution_failed())?;
        for (path, bytes) in captured {
            let destination = snapshot
                .path()
                .join(path.file_name().ok_or_else(resolution_failed)?);
            let mut file = fs::OpenOptions::new();
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            file.write(true)
                .create_new(true)
                .mode(0o600)
                .open(destination)
                .and_then(|mut file| file.write_all(bytes))
                .map_err(|_| resolution_failed())?;
        }
        let home = snapshot.path().join("home");
        fs::create_dir(&home).map_err(|_| resolution_failed())?;
        let mut arguments: Vec<std::ffi::OsString> = vec![
            "--host".into(),
            "unix:///var/run/docker.sock".into(),
            "compose".into(),
            "--project-name".into(),
            self.project.clone().into(),
            "--project-directory".into(),
            snapshot.path().into(),
            "--env-file".into(),
            snapshot.path().join(".env").into(),
        ];
        for (path, _) in captured.iter().take(self.compose_files.len()) {
            arguments.push("--file".into());
            arguments.push(
                snapshot
                    .path()
                    .join(path.file_name().ok_or_else(resolution_failed)?)
                    .into(),
            );
        }
        arguments.extend(["config".into(), "--format".into(), "json".into()]);
        let request = ProcessRequest::new("docker")
            .args(arguments)
            .current_dir(snapshot.path())
            .env("HOME", &home)
            .env("DOCKER_CONFIG", &home)
            .limits(ProcessLimits {
                timeout: Duration::from_secs(30),
                maximum_stdout_bytes: 1024 * 1024,
                maximum_stderr_bytes: 16 * 1024,
                ..ProcessLimits::default()
            });
        // Await cancellation cleanup before deleting the private input snapshot.
        // Compose output may contain secrets; never expose its error text.
        let output = process
            .run(request, cancellation)
            .await
            .map_err(|error| match error {
                ProcessError::Cancelled => UpdateError::Cancelled,
                _ => resolution_failed(),
            })?;
        if cancellation.is_cancelled() {
            return Err(UpdateError::Cancelled);
        }
        if !output.succeeded() || output.stdout_truncated || output.stderr_truncated {
            return Err(resolution_failed());
        }
        let config: Value =
            serde_json::from_slice(&output.stdout).map_err(|_| resolution_failed())?;
        Ok(config)
    }

    pub(super) fn validate_resolved(
        &self,
        config: &Value,
        fingerprint: String,
    ) -> Result<CheckedInstallerCompose, UpdateError> {
        if config["name"] != self.project
            || config["services"].as_object().is_none_or(|v| v.len() != 2)
        {
            return Err(resolution_failed());
        }
        let core = &config["services"]["server"];
        let database = &config["services"]["pg_db"];
        let env = &core["environment"];
        if env[environment::PG_HOST] != "pg_db"
            || env[environment::PG_PORT] != "5432"
            || [
                environment::CITADEL_DATABASE_URL,
                environment::CITADEL_CORE_AGENT_ADDRESS,
                environment::CITADEL_CORE_AGENT_PRIVATE_KEY_PATH,
            ]
            .iter()
            .any(|key| env.get(*key).is_some())
            || env
                .get(environment::CITADEL_DOCKER_SOCKET)
                .is_some_and(|v| v != "/var/run/docker.sock")
            || env
                .get(environment::CITADEL_DATA_ROOT)
                .is_some_and(|v| v != "/app/data")
        {
            return Err(unsupported(
                "Updates require the packaged PostgreSQL service, local Docker socket and /app/data volume.",
            ));
        }
        for (key, pg_key) in [
            (environment::PG_USER, "POSTGRES_USER"),
            (environment::PG_PASSWORD, "POSTGRES_PASSWORD"),
            (environment::PG_DATABASE, "POSTGRES_DB"),
        ] {
            if env[key].as_str().is_none_or(str::is_empty)
                || env[key] != database["environment"][pg_key]
            {
                return Err(unsupported(
                    "Core and packaged PostgreSQL settings must agree; check the installer .env syntax.",
                ));
            }
        }
        let image = core["image"].as_str().ok_or_else(resolution_failed)?;
        if !official_image(image) {
            return Err(unsupported(
                "The installer must select an official Citadel Core image.",
            ));
        }
        super::profile::validate_resolved(config)?;
        let core_volume = named_resource(config, "volumes", "citadel_data")?;
        let database_volume = named_resource(config, "volumes", "postgres_data")?;
        let network = named_resource(config, "networks", "default")?;
        if core_volume == database_volume {
            return Err(resolution_failed());
        }
        Ok(CheckedInstallerCompose {
            project: self.project.clone(),
            configuration_sha256: fingerprint,
            core_image: image.into(),
            core_volume,
            database_volume,
            network,
        })
    }
}

pub(super) fn validate_environment(bytes: &[u8], project: &str) -> Result<(), UpdateError> {
    let text = std::str::from_utf8(bytes).map_err(|_| resolution_failed())?;
    let mut names = HashSet::new();
    for line in text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let Some((name, value)) = line.split_once('=') else {
            return Err(unsupported(
                "Installer .env must use one NAME=value assignment per line.",
            ));
        };
        if name.is_empty()
            || !name
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
            || name.as_bytes()[0].is_ascii_digit()
            || !names.insert(name)
            || value.contains('$')
        {
            return Err(unsupported(
                "Installer .env must use unique assignments without variable interpolation.",
            ));
        }
        if name.starts_with("COMPOSE_") && (name != "COMPOSE_PROJECT_NAME" || value != project) {
            return Err(unsupported(
                "Compose overrides in .env are unsupported; COMPOSE_PROJECT_NAME must match the supplied project.",
            ));
        }
    }
    Ok(())
}

fn official_image(image: &str) -> bool {
    [
        citadel_updates::CORE_IMAGE,
        "docker.io/citadelplane/citadel",
        "citadelplane/citadel",
    ]
    .iter()
    .any(|repository| {
        let Some(reference) = image.strip_prefix(repository) else {
            return false;
        };
        if let Some(digest) = reference.strip_prefix("@sha256:") {
            return citadel_updates::is_sha256(digest);
        }
        reference.strip_prefix(':').is_some_and(|tag| {
            !tag.is_empty()
                && tag.len() <= 128
                && !tag.starts_with(['.', '-'])
                && tag
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        })
    })
}

fn named_resource(config: &Value, kind: &str, key: &str) -> Result<String, UpdateError> {
    config[kind][key]["name"]
        .as_str()
        .filter(|v| identifier(v))
        .map(str::to_owned)
        .ok_or_else(resolution_failed)
}

pub(super) fn unsupported(message: &str) -> UpdateError {
    UpdateError::Unavailable(message.into())
}
fn resolution_failed() -> UpdateError {
    unsupported(
        "Cannot safely resolve the installer Compose configuration. Check the private .env and packaged Docker Compose tool.",
    )
}
