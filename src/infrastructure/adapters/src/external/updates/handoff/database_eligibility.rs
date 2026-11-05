//! Read-only database eligibility, shared by review and detached execution.
use super::super::{InstallerComposeInputs, RegisteredComposeInstallation, docker::LocalDocker};
use super::{evidence::conflict, verification};
use citadel_execution::ProcessRunner;
use citadel_updates::{DatabaseUpdatePlan, UpdateError, UpdatePlan};
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

pub(in crate::external::updates) async fn check_database_eligibility(
    plan: &UpdatePlan,
    process: &dyn ProcessRunner,
    cancellation: &CancellationToken,
) -> Result<(), UpdateError> {
    let checked = super::super::check_registered_compose_installation(
        std::path::Path::new(&plan.checkpoint_directory),
        process,
        cancellation,
    )
    .await?;
    let reg = &checked.registration;
    if reg.id != plan.registration_id
        || reg.instance_id != plan.instance_id
        || reg.configuration_sha256 != plan.configuration_sha256
    {
        return Err(conflict());
    }
    let database = plan.database.as_ref().ok_or_else(conflict)?;
    let inputs = InstallerComposeInputs {
        directory: reg.directory.clone(),
        project: reg.project.clone(),
        compose_files: reg.compose_files.clone(),
    };
    // Resolve both configurations without publishing or pulling an image.
    inputs
        .prepare_database_replacement(database, process, cancellation)
        .await?;
    let home = super::runtime::private_home()?;
    let docker = LocalDocker {
        process,
        cancellation,
        home: home.path(),
        deadline: Instant::now() + Duration::from_secs(60),
    };
    check_database_source(&docker, reg, database).await?;
    Ok(())
}

pub(super) async fn check_database_source(
    docker: &LocalDocker<'_>,
    registration: &RegisteredComposeInstallation,
    plan: &DatabaseUpdatePlan,
) -> Result<Value, UpdateError> {
    super::super::compose::validate_database_update(plan)?;
    let database = verification::container(docker, &registration.database_container_id).await?;
    verification::database_running(&database)?;
    let source_data = format!("/var/lib/postgresql/{}/docker", plan.source_version / 10000);
    let value = facts(docker, &registration.database_container_id).await?;
    for (field, expected, message) in [
        (
            "version",
            serde_json::json!(plan.source_version),
            "PostgreSQL version changed since review.",
        ),
        (
            "recovery",
            serde_json::json!(false),
            "PostgreSQL is in recovery; replica updates are not supported.",
        ),
        (
            "slots",
            serde_json::json!(0),
            "PostgreSQL replication slots must be removed before an in-app update.",
        ),
        (
            "replicas",
            serde_json::json!(0),
            "PostgreSQL has connected replicas; in-app updates are not supported.",
        ),
        (
            "subscriptions",
            serde_json::json!(0),
            "PostgreSQL subscriptions are not supported by in-app updates.",
        ),
        (
            "tablespaces",
            serde_json::json!(0),
            "PostgreSQL external tablespaces are not supported by in-app updates.",
        ),
    ] {
        if value[field] != expected {
            return Err(UpdateError::Conflict(message.into()));
        }
    }
    if value["data"] != source_data || env(&database, "PGDATA")? != source_data {
        return Err(UpdateError::Conflict(
            "PostgreSQL must use the supported versioned PGDATA directory.".into(),
        ));
    }
    // Only Alpine's packaged cluster (UID 70) is qualified by this executor.
    docker
        .run(&[
            "exec",
            &registration.database_container_id,
            "sh",
            "-ec",
            "test -f /etc/alpine-release; test \"$(stat -c %u \"$PGDATA\")\" = 70",
        ])
        .await
        .map_err(|error| match error {
            UpdateError::Cancelled => UpdateError::Cancelled,
            _ => UpdateError::Conflict(
                "PostgreSQL must use an Alpine cluster owned by UID 70.".into(),
            ),
        })?;
    Ok(database)
}
pub(super) async fn facts(docker: &LocalDocker<'_>, id: &str) -> Result<Value, UpdateError> {
    let bytes = docker.run(&["exec", id, "sh", "-ec",
        "exec psql -X -A -t -v ON_ERROR_STOP=1 -U \"$POSTGRES_USER\" -d \"$POSTGRES_DB\" -c \"SELECT json_build_object('version', current_setting('server_version_num')::int, 'data', current_setting('data_directory'), 'recovery', pg_is_in_recovery(), 'slots', (SELECT count(*) FROM pg_replication_slots), 'replicas', (SELECT count(*) FROM pg_stat_replication), 'subscriptions', (SELECT count(*) FROM pg_subscription), 'tablespaces', (SELECT count(*) FROM pg_tablespace WHERE spcname NOT IN ('pg_default', 'pg_global')))\""]).await?;
    serde_json::from_slice(&bytes).map_err(|_| conflict())
}
pub(super) fn env<'a>(container: &'a Value, key: &str) -> Result<&'a str, UpdateError> {
    container["Config"]["Env"]
        .as_array()
        .ok_or_else(conflict)?
        .iter()
        .filter_map(Value::as_str)
        .find_map(|v| {
            v.split_once('=')
                .filter(|(name, _)| *name == key)
                .map(|(_, value)| value)
        })
        .ok_or_else(conflict)
}
