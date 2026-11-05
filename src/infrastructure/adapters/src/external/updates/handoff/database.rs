//! PostgreSQL execution within the existing owned instance-update handoff.
use super::super::{
    RegisteredComposeInstallation,
    docker::{LocalDocker, single},
};
use super::{
    ExecutionJournal, OwnedUpdateHelper,
    database_eligibility::{check_database_source, env, facts},
    evidence::{HelperLaunchIntent, conflict},
    verification,
};
use citadel_execution::ProcessRunner;
use citadel_updates::UpdateError;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct DatabaseReplacement {
    infrastructure_sha256: String,
    container_id: Option<String>,
    runtime_sha256: Option<String>,
}

impl DatabaseReplacement {
    pub(super) async fn prepare(
        docker: &LocalDocker<'_>,
        intent: &HelperLaunchIntent,
    ) -> Result<Self, UpdateError> {
        check_database_source(
            docker,
            &intent.registration,
            intent
                .operation
                .plan
                .database
                .as_ref()
                .ok_or_else(conflict)?,
        )
        .await?;
        Ok(Self {
            infrastructure_sha256: infrastructure(docker, &intent.registration).await?,
            container_id: None,
            runtime_sha256: None,
        })
    }
    pub(super) fn registration(
        &self,
        original: &RegisteredComposeInstallation,
    ) -> Result<RegisteredComposeInstallation, UpdateError> {
        let mut registration = original.clone();
        registration.database_container_id = self
            .container_id
            .clone()
            .filter(|id| citadel_updates::is_sha256(id))
            .ok_or_else(conflict)?;
        Ok(registration)
    }
    pub(super) async fn verify(
        &self,
        docker: &LocalDocker<'_>,
        intent: &HelperLaunchIntent,
    ) -> Result<(), UpdateError> {
        let reg = self.registration(&intent.registration)?;
        let database = verification::container(docker, &reg.database_container_id).await?;
        verification::database_running(&database)?;
        if self.runtime_sha256.as_ref() != Some(&runtime_hash(&database)?)
            || infrastructure(docker, &reg).await? != self.infrastructure_sha256
        {
            return Err(conflict());
        }
        Ok(())
    }
}

impl ExecutionJournal {
    pub(super) fn target_registration(
        &self,
        intent: &HelperLaunchIntent,
    ) -> Result<RegisteredComposeInstallation, UpdateError> {
        match (
            &intent.operation.plan.database,
            self.replacement.as_ref().and_then(|r| r.database.as_ref()),
        ) {
            (None, None) => Ok(intent.registration.clone()),
            (Some(_), Some(database)) => database.registration(&intent.registration),
            _ => Err(conflict()),
        }
    }
}

impl OwnedUpdateHelper {
    pub(super) async fn prepare_database(
        &self,
        process: &dyn ProcessRunner,
        cancellation: &CancellationToken,
    ) -> Result<(), UpdateError> {
        let home = super::runtime::private_home()?;
        let docker = LocalDocker {
            process,
            cancellation,
            home: home.path(),
            deadline: Instant::now() + Duration::from_secs(600),
        };
        check_database_source(
            &docker,
            &self.intent.registration,
            self.intent
                .operation
                .plan
                .database
                .as_ref()
                .ok_or_else(conflict)?,
        )
        .await?;
        let plan = self
            .intent
            .operation
            .plan
            .database
            .as_ref()
            .ok_or_else(conflict)?;
        // Pull the qualified digest while Core is still serving. Never fetch a tag
        // or install packages after stopping the database.
        if docker
            .json(&["image", "inspect", &plan.target_image])
            .await
            .is_err()
        {
            docker
                .run_with_timeout(
                    &[
                        "pull",
                        "--platform",
                        &format!("linux/{}", self.intent.operation.plan.source.architecture),
                        &plan.target_image,
                    ],
                    Duration::from_secs(500),
                )
                .await?;
        }
        let image = docker
            .json(&["image", "inspect", &plan.target_image])
            .await?;
        let image = single(&image)?;
        if image["Architecture"] != self.intent.operation.plan.source.architecture
            || image["Os"] != "linux"
        {
            return Err(conflict());
        }
        Ok(())
    }

    pub(super) async fn replace_database(
        &self,
        docker: &LocalDocker<'_>,
        execution: &mut ExecutionJournal,
    ) -> Result<(), UpdateError> {
        let reg = &self.intent.registration;
        let plan = self
            .intent
            .operation
            .plan
            .database
            .as_ref()
            .ok_or_else(conflict)?;
        let receipt = execution
            .replacement
            .as_ref()
            .and_then(|r| r.database.as_ref())
            .ok_or_else(conflict)?;
        if receipt.container_id.is_some()
            || infrastructure(docker, reg).await? != receipt.infrastructure_sha256
        {
            return Err(conflict());
        }
        let old = check_database_source(
            docker,
            &self.intent.registration,
            self.intent
                .operation
                .plan
                .database
                .as_ref()
                .ok_or_else(conflict)?,
        )
        .await?;
        // Replacement intent and checkpoint are already durable. Never replay any
        // mutation below after a timeout or process restart.
        docker
            .run_with_timeout(
                &[
                    "container",
                    "stop",
                    "--signal",
                    "SIGTERM",
                    "--timeout",
                    "-1",
                    &reg.database_container_id,
                ],
                Duration::from_secs(300),
            )
            .await?;
        let stopped = verification::container(docker, &reg.database_container_id).await?;
        if stopped["State"]["Running"] != false
            || stopped["State"]["ExitCode"] != 0
            || stopped["State"]["OOMKilled"] != false
            || stopped["State"]["Error"] != ""
            || stopped["Image"] != old["Image"]
            || stopped["Config"] != old["Config"]
            || stopped["State"]["StartedAt"] != old["State"]["StartedAt"]
        {
            return Err(conflict());
        }
        if infrastructure(docker, reg).await? != receipt.infrastructure_sha256 {
            return Err(conflict());
        }
        // Stopping PostgreSQL can take minutes. Revalidate the approved files
        // at the mutation boundary, not only before waiting for shutdown.
        self.state.verify_lock(&self.owner)?;
        if reg.fingerprint(std::path::Path::new("/"))?
            != execution
                .replacement
                .as_ref()
                .ok_or_else(conflict)?
                .configuration
                .target_configuration_sha256
            || ExecutionJournal::read_for(&self.state, &self.intent)? != *execution
        {
            return Err(conflict());
        }
        compose(
            docker,
            reg,
            &[
                "up",
                "--detach",
                "--no-deps",
                "--no-build",
                "--pull",
                "never",
                "--force-recreate",
                "pg_db",
            ],
        )
        .await?;
        let ids = docker
            .containers(&format!("label=com.docker.compose.project={}", reg.project))
            .await?;
        let mut target = None;
        for id in ids {
            let container = verification::container(docker, &id).await?;
            if container["Config"]["Labels"]["com.docker.compose.service"] == "pg_db"
                && target.replace(container).is_some()
            {
                return Err(conflict());
            }
        }
        let mut database = target.ok_or_else(conflict)?;
        let id = database["Id"].as_str().ok_or_else(conflict)?.to_owned();
        if id == reg.database_container_id
            || database["Config"]["Image"] != plan.target_image
            || env(&database, "PGDATA")? != "/var/lib/postgresql/18/docker"
        {
            return Err(conflict());
        }
        let target_image = docker
            .json(&["image", "inspect", &plan.target_image])
            .await?;
        if database["Image"] != single(&target_image)?["Id"] {
            return Err(conflict());
        }
        let deadline = Instant::now() + Duration::from_secs(120);
        while verification::database_running(&database).is_err() {
            if Instant::now() >= deadline || docker.cancellation.is_cancelled() {
                return Err(conflict());
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
            database = verification::container(docker, &id).await?;
        }
        if facts(docker, &id).await?["version"] != plan.target_version {
            return Err(conflict());
        }
        if infrastructure(docker, reg).await? != receipt.infrastructure_sha256 {
            return Err(conflict());
        }
        self.state.verify_lock(&self.owner)?;
        if ExecutionJournal::read_for(&self.state, &self.intent)? != *execution {
            return Err(conflict());
        }
        let receipt = execution
            .replacement
            .as_mut()
            .and_then(|r| r.database.as_mut())
            .ok_or_else(conflict)?;
        receipt.container_id = Some(id);
        receipt.runtime_sha256 = Some(runtime_hash(&database)?);
        execution.save(&self.state, &self.owner)
    }
}

fn hash(value: &Value) -> Result<String, UpdateError> {
    Ok(
        Sha256::digest(serde_json::to_vec(value).map_err(|_| conflict())?)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    )
}
fn runtime_hash(container: &Value) -> Result<String, UpdateError> {
    hash(
        &json!({"id":container["Id"], "image":container["Image"], "config":container["Config"], "host":container["HostConfig"], "mounts":container["Mounts"], "started":container["State"]["StartedAt"], "restarts":container["RestartCount"]}),
    )
}
async fn infrastructure(
    docker: &LocalDocker<'_>,
    reg: &RegisteredComposeInstallation,
) -> Result<String, UpdateError> {
    let volumes = docker
        .json(&["volume", "inspect", &reg.core_volume, &reg.database_volume])
        .await?;
    let network = docker.json(&["network", "inspect", &reg.network]).await?;
    let network = single(&network)?;
    let daemon = docker.json(&["info", "--format", "{{json .}}"]).await?;
    if daemon["ID"] != reg.daemon_id {
        return Err(conflict());
    }
    hash(
        &json!({"volumes": volumes, "network":network["Id"], "driver":network["Driver"], "options":network["Options"], "ipam":network["IPAM"], "daemon":daemon["ID"]}),
    )
}
async fn compose(
    docker: &LocalDocker<'_>,
    reg: &RegisteredComposeInstallation,
    command: &[&str],
) -> Result<(), UpdateError> {
    let mut args = vec![
        "compose",
        "--project-name",
        &reg.project,
        "--project-directory",
        reg.directory.to_str().ok_or_else(conflict)?,
        "--env-file",
        reg.environment_file.to_str().ok_or_else(conflict)?,
    ];
    for file in &reg.compose_files {
        args.extend(["--file", file.to_str().ok_or_else(conflict)?]);
    }
    args.extend(command);
    docker
        .run_with_timeout(&args, Duration::from_secs(300))
        .await
        .map(|_| ())
}
