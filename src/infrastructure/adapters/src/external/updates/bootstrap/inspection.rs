use super::super::docker::{LocalDocker, failed, refuse};
use super::super::registration::fingerprint_inputs;
use super::super::{InstallerComposeInputs, RegisteredComposeInstallation};
use super::executor::verify_executor;
use super::observation::observe;
use citadel_execution::ProcessRunner;
use citadel_instance::environment;
use citadel_updates::{INSTALLER_FORMAT, UpdateError};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub(super) struct SourcePreflight {
    pub database: citadel_updates::UpdateDatabaseState,
    pub recovery_keys: super::super::CheckedRecoveryKeys,
}

pub(super) async fn inspect_installation(
    inputs: &InstallerComposeInputs,
    state_directory: PathBuf,
    registration_id: Uuid,
    process: &dyn ProcessRunner,
    cancellation: &CancellationToken,
    preflight: bool,
) -> Result<(RegisteredComposeInstallation, Option<SourcePreflight>), UpdateError> {
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
    let socket = super::super::layout::compose_socket(&config)?;
    verify_executor(
        &docker,
        inputs,
        &state_directory,
        std::path::Path::new(socket),
    )
    .await?;
    let first = observe(&docker, inputs, &config, &checked, &state_directory).await?;
    let mut command = vec![
        "exec",
        &first.core,
        "/app/citadel-server",
        "instance-identity-json",
    ];
    if preflight {
        command.push("--verify-migrations");
    }
    let identity = docker.json(&command).await.map_err(|error| match error {
        UpdateError::Cancelled => error,
        _ if preflight => refuse("Core must have initialized identity, a fully applied matching migration catalog and no owned update."),
        _ => refuse("Core must support instance-identity-json and have an initialized, reachable Citadel database."),
    })?;
    let source = if preflight {
        let database: citadel_updates::UpdateDatabaseState =
            serde_json::from_value(identity.clone()).map_err(|_| failed())?;
        if !citadel_updates::is_sha256(&database.catalog_sha256) || database.database_bytes == 0 {
            return Err(failed());
        }
        // Match the packaged entrypoint's Core UID/GID; Docker exec otherwise
        // inherits the image's root user and would not test Core's key access.
        let recovery_keys = docker.json(&[
            "exec", "--user", "65532:0", &first.core, "/app/citadel-server", "check-update-recovery-keys",
        ]).await.map_err(|error| match error {
            UpdateError::Cancelled => error,
            _ => refuse("Core recovery keys are missing, malformed or unsafe; preserve the original keys before updating."),
        })?;
        let recovery_keys: super::super::CheckedRecoveryKeys =
            serde_json::from_value(recovery_keys).map_err(|_| failed())?;
        check_key_sources(&recovery_keys, &config)?;
        Some(SourcePreflight {
            database,
            recovery_keys,
        })
    } else {
        None
    };
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
        id: registration_id,
        instance_id,
        directory: inputs.directory.clone(),
        project: inputs.project.clone(),
        compose_files: inputs.compose_files.clone(),
        environment_file: inputs.directory.join(".env"),
        docker_socket: socket.into(),
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
    Ok((registration, source))
}

fn check_key_sources(
    keys: &super::super::CheckedRecoveryKeys,
    config: &serde_json::Value,
) -> Result<(), UpdateError> {
    use super::super::RecoveryKeySource;
    for (setting, source) in [
        (environment::CITADEL_JWT_KEY, keys.jwt),
        (
            environment::CITADEL_SECRETS_ENCRYPTION_KEY,
            keys.secrets_encryption,
        ),
    ] {
        let external = config["services"]["server"]["environment"][setting]
            .as_str()
            .is_some_and(|value| !value.trim().is_empty());
        let expected = if external {
            RecoveryKeySource::Environment
        } else {
            RecoveryKeySource::DataVolume
        };
        if source != expected {
            return Err(refuse(
                "Recovery-key sources differ from the protected installer inputs. Preserve the original external keys in those inputs before updating.",
            ));
        }
    }
    if keys.agent_signing != RecoveryKeySource::DataVolume {
        return Err(refuse(
            "The Agent signing key must be retained in the registered Core data volume.",
        ));
    }
    Ok(())
}
