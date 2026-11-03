//! Disposable container execution, ownership verification and cleanup.
use super::docker::{Docker, failed};
use citadel_updates::{UpdateError, UpgradeDescriptor, is_sha256};
use serde_json::Value;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub(super) const OWNER: &str = "com.citadel.update-inspection";

pub(super) struct Probe<'a, 'b> {
    pub(super) docker: &'a Docker<'b>,
    pub(super) name: &'a str,
    pub(super) nonce: &'a str,
    pub(super) label: &'a str,
    pub(super) image: &'a str,
}

impl Probe<'_, '_> {
    pub(super) async fn read(
        &self,
        platform: &str,
        cancellation: &CancellationToken,
    ) -> Result<UpgradeDescriptor, UpdateError> {
        // Finish this bounded create even if cancellation arrives, then clean up
        // without starting it. Killing the CLI mid-create makes ownership ambiguous.
        let created = self
            .docker
            .run(
                &[
                    "create",
                    "--name",
                    self.name,
                    "--label",
                    self.label,
                    "--platform",
                    platform,
                    "--pull=never",
                    "--network=none",
                    "--read-only",
                    "--cap-drop=ALL",
                    "--security-opt=no-new-privileges",
                    "--user=65532:65532",
                    "--env=TOKIO_WORKER_THREADS=2",
                    "--pids-limit=32",
                    "--memory=128m",
                    "--cpus=1",
                    "--restart=no",
                    "--log-driver=none",
                    "--no-healthcheck",
                    "--entrypoint=/app/citadel-server",
                    self.image,
                    "upgrade-descriptor-json",
                ],
                Duration::from_secs(30),
                &CancellationToken::new(),
            )
            .await
            .map_err(|_| UpdateError::Unavailable(format!(
                "Creation of inspection container {} was not confirmed. Inspect this exact name for a late Docker create after cleanup.",
                self.name,
            )))?;
        let id = std::str::from_utf8(&created)
            .map(str::trim)
            .map_err(|_| failed())?;
        if !is_sha256(id) {
            return Err(failed());
        }
        self.inspect_owned(id, cancellation).await?;
        let bytes = self
            .docker
            .run(
                &["start", "--attach", id],
                Duration::from_secs(30),
                cancellation,
            )
            .await?;
        let container = self.inspect_owned(id, cancellation).await?;
        if container["State"]["Running"] != false
            || container["State"]["ExitCode"] != 0
            || container["State"]["OOMKilled"] != false
            || container["State"]["Error"] != ""
            || container["State"]["Status"] != "exited"
        {
            return Err(failed());
        }
        serde_json::from_slice(&bytes).map_err(|_| failed())
    }

    async fn inspect_owned(
        &self,
        id: &str,
        cancellation: &CancellationToken,
    ) -> Result<Value, UpdateError> {
        let value: Value = serde_json::from_slice(
            &self
                .docker
                .run(
                    &["container", "inspect", "--format", "{{json .}}", id],
                    Duration::from_secs(15),
                    cancellation,
                )
                .await?,
        )
        .map_err(|_| failed())?;
        if value["Id"] != id
            || value["Name"] != format!("/{}", self.name)
            || value["Image"] != self.image
            || value["Config"]["Labels"][OWNER] != self.nonce
        {
            return Err(failed());
        }
        Ok(value)
    }

    pub(super) async fn cleanup(&self) -> Result<(), UpdateError> {
        let cancellation = CancellationToken::new();
        let output = self
            .docker
            .run(
                &[
                    "container",
                    "ls",
                    "--all",
                    "--no-trunc",
                    "--filter",
                    &format!("label={}", self.label),
                    "--filter",
                    &format!("name=^/{}$", self.name),
                    "--format",
                    "{{json .}}",
                ],
                Duration::from_secs(15),
                &cancellation,
            )
            .await?;
        let text = std::str::from_utf8(&output).map_err(|_| failed())?;
        let rows: Vec<_> = text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();
        if rows.len() > 1 {
            return Err(failed());
        }
        if let Some(row) = rows.first() {
            let row: Value = serde_json::from_str(row).map_err(|_| failed())?;
            let id = row["ID"]
                .as_str()
                .filter(|id| is_sha256(id))
                .ok_or_else(failed)?;
            if row["Names"] != self.name {
                return Err(failed());
            }
            self.inspect_owned(id, &cancellation).await?;
            self.docker
                .run(
                    &["rm", "--force", "--volumes", id],
                    Duration::from_secs(30),
                    &cancellation,
                )
                .await?;
        }
        Ok(())
    }
}
