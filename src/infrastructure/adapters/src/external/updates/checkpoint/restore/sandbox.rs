//! Own only a disposable, network-isolated PostgreSQL container and its tmpfs data.
use super::super::super::docker::{LocalDocker, single};
use super::evidence::failed;
use citadel_updates::{UpdateError, is_sha256};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{path::PathBuf, time::Duration};
use uuid::Uuid;

const LABEL: &str = "com.citadel.checkpoint-restore";
pub(super) const DATABASE: &str = "citadel_checkpoint_verify";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Sandbox {
    pub nonce: Uuid,
    pub image: String,
    pub dump: PathBuf,
    pub maximum_bytes: u64,
}
impl Sandbox {
    pub fn name(&self) -> String {
        format!("citadel-checkpoint-{}", self.nonce.simple())
    }

    pub async fn create(&self, docker: &LocalDocker<'_>) -> Result<String, UpdateError> {
        let path = self.dump.to_str().ok_or_else(failed)?.replace('"', "\"\"");
        let mount = format!("type=bind,\"source={path}\",destination=/checkpoint.dump,readonly");
        let data = format!("/var/lib/postgresql:rw,size={}", self.maximum_bytes);
        let memory = (self.maximum_bytes + 256 * 1024 * 1024).to_string();
        let label = format!("{LABEL}={}", self.nonce);
        let output = docker
            .run(&[
                "container",
                "create",
                "--name",
                &self.name(),
                "--label",
                &label,
                "--pull=never",
                "--network=none",
                "--restart=no",
                "--read-only",
                "--no-healthcheck",
                "--log-driver=none",
                "--user=0:0",
                "--pids-limit=128",
                "--cpus=1",
                "--memory",
                &memory,
                "--memory-swap",
                &memory,
                "--tmpfs",
                &data,
                "--tmpfs",
                "/var/run/postgresql:rw,size=16777216",
                "--mount",
                &mount,
                "--env",
                "POSTGRES_HOST_AUTH_METHOD=trust",
                "--env",
                "PGDATA=/var/lib/postgresql/data",
                "--env",
                "PGHOST=/var/run/postgresql",
                "--env",
                "POSTGRES_USER=postgres",
                "--env",
                &format!("POSTGRES_DB={DATABASE}"),
                "--entrypoint=/usr/local/bin/docker-entrypoint.sh",
                &self.image,
                "postgres",
                "-c",
                "listen_addresses=",
                "-c",
                "unix_socket_directories=/var/run/postgresql",
            ])
            .await?;
        let id = std::str::from_utf8(&output)
            .map(str::trim)
            .map_err(|_| failed())?;
        if !is_sha256(id) {
            return Err(failed());
        }
        let actual = self.inspect(docker, id).await?;
        if actual["State"]["Status"] != "created" || actual["State"]["Running"] != false {
            return Err(failed());
        }
        Ok(id.into())
    }

    pub async fn inspect(&self, docker: &LocalDocker<'_>, id: &str) -> Result<Value, UpdateError> {
        let raw = docker.json(&["container", "inspect", id]).await?;
        let value = single(&raw)?;
        let mounts = value["Mounts"].as_array().ok_or_else(failed)?;
        // Docker includes tmpfs entries in Mounts on some versions; they must
        // be exactly the two disposable paths, never volumes or extra binds.
        if mounts.iter().filter(|m| m["Type"] == "bind").count() != 1
            || mounts.iter().any(|m| match m["Type"].as_str() {
                Some("bind") => {
                    m["Source"] != self.dump.to_str().unwrap_or_default()
                        || m["Destination"] != "/checkpoint.dump"
                        || m["RW"] != false
                }
                Some("tmpfs") => !["/var/lib/postgresql", "/var/run/postgresql"]
                    .iter()
                    .any(|p| m["Destination"] == *p),
                _ => true,
            })
            || value["Id"] != id
            || value["Name"] != format!("/{}", self.name())
            || value["Image"] != self.image
            || value["Config"]["Image"] != self.image
            || value["Config"]["Labels"][LABEL] != self.nonce.to_string()
            || value["HostConfig"]["NetworkMode"] != "none"
            || value["NetworkSettings"]["Networks"]
                .as_object()
                .is_none_or(|v| v.keys().any(|name| name != "none"))
            || value["HostConfig"]["PublishAllPorts"] != false
            || !(value["HostConfig"]["PortBindings"].is_null()
                || value["HostConfig"]["PortBindings"]
                    .as_object()
                    .is_some_and(|v| v.is_empty()))
            || value["HostConfig"]["Memory"] != self.maximum_bytes + 256 * 1024 * 1024
            || value["HostConfig"]["MemorySwap"] != self.maximum_bytes + 256 * 1024 * 1024
            || value["HostConfig"]["NanoCpus"] != 1_000_000_000_u64
            || value["HostConfig"]["PidsLimit"] != 128
            || value["HostConfig"]["Privileged"] != false
            || value["HostConfig"]["ReadonlyRootfs"] != true
            || value["HostConfig"]["AutoRemove"] != false
            || value["HostConfig"]["RestartPolicy"]["Name"] != "no"
            || value["HostConfig"]["Tmpfs"]
                != json!({
                    "/var/lib/postgresql": format!("rw,size={}", self.maximum_bytes),
                    "/var/run/postgresql": "rw,size=16777216"
                })
            || value["Config"]["User"] != "0:0"
            || value["RestartCount"] != 0
            || value["Path"] != "/usr/local/bin/docker-entrypoint.sh"
            || value["Args"]
                != json!([
                    "postgres",
                    "-c",
                    "listen_addresses=",
                    "-c",
                    "unix_socket_directories=/var/run/postgresql"
                ])
        {
            return Err(failed());
        }
        Ok(value.clone())
    }

    pub async fn restore(
        &self,
        docker: &LocalDocker<'_>,
        id: &str,
        timeout: Duration,
    ) -> Result<Vec<u8>, UpdateError> {
        docker.run(&["container", "start", id]).await?;
        loop {
            let actual = self.inspect(docker, id).await?;
            if actual["State"]["Running"] != true
                || actual["State"]["Paused"] != false
                || actual["State"]["Restarting"] != false
                || actual["State"]["OOMKilled"] != false
            {
                return Err(failed());
            }
            // The image entrypoint temporarily starts PostgreSQL during initdb.
            // Wait for its final exec before probing, so restoration cannot race
            // the initialization server's planned shutdown.
            let command = docker.run(&["exec", id, "cat", "/proc/1/comm"]).await?;
            if command == b"postgres\n"
                && docker
                    .run(&[
                        "exec",
                        id,
                        "pg_isready",
                        "-h",
                        "/var/run/postgresql",
                        "-U",
                        "postgres",
                        "-d",
                        DATABASE,
                    ])
                    .await
                    .is_ok()
            {
                break;
            }
            tokio::select! {
                biased;
                () = docker.cancellation.cancelled() => return Err(UpdateError::Cancelled),
                () = tokio::time::sleep(Duration::from_millis(250)) => {}
            }
        }
        docker
            .run_with_timeout(
                &[
                    "exec",
                    id,
                    "pg_restore",
                    "--host=/var/run/postgresql",
                    "--username=postgres",
                    "--dbname",
                    DATABASE,
                    "--format=custom",
                    "--no-owner",
                    "--no-privileges",
                    "--single-transaction",
                    "--exit-on-error",
                    "/checkpoint.dump",
                ],
                timeout,
            )
            .await?;
        let output = docker
            .run(&[
                "exec",
                id,
                "psql",
                "--no-psqlrc",
                "--host=/var/run/postgresql",
                "--username=postgres",
                "--dbname",
                DATABASE,
                "--tuples-only",
                "--no-align",
                "--set=ON_ERROR_STOP=1",
                "--command",
                crate::persistence::postgres::updates::CHECKPOINT_PROBE_SQL,
            ])
            .await?;
        let after = self.inspect(docker, id).await?;
        if after["State"]["Running"] != true || after["State"]["OOMKilled"] != false {
            return Err(failed());
        }
        Ok(output)
    }

    pub async fn cleanup(&self, docker: &LocalDocker<'_>) -> Result<(), UpdateError> {
        let ids = docker
            .containers(&format!("name=^/{}$", self.name()))
            .await?;
        if ids.len() > 1 {
            return Err(failed());
        }
        if let Some(id) = ids.first() {
            self.inspect(docker, id).await?;
            docker
                .run_with_timeout(&["container", "rm", "--force", id], Duration::from_secs(30))
                .await?;
        }
        if !docker
            .containers(&format!("name=^/{}$", self.name()))
            .await?
            .is_empty()
        {
            return Err(failed());
        }
        Ok(())
    }
}
