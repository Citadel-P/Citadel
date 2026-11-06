//! Read-only topology discovery, independent of registration and update ownership.
use super::docker::{LocalDocker, failed, single};
use citadel_execution::ProcessRunner;
use citadel_updates::{DatabaseInstallation, DatabaseInstallationRuntime, UpdateError, is_sha256};
use futures_util::future::BoxFuture;
use serde_json::Value;
use std::{
    os::unix::fs::PermissionsExt,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;

pub struct DockerDatabaseInstallation {
    process: Arc<dyn ProcessRunner>,
    pool: sqlx::PgPool,
}
impl DockerDatabaseInstallation {
    pub fn new(process: Arc<dyn ProcessRunner>, pool: sqlx::PgPool) -> Self {
        Self { process, pool }
    }

    async fn inspect(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<DatabaseInstallation, UpdateError> {
        let hostname =
            std::fs::read_to_string("/proc/sys/kernel/hostname").map_err(|_| failed())?;
        let hostname = hostname.trim();
        if hostname.len() != 12 || !hostname.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Ok(DatabaseInstallation::Unknown);
        }
        let home = tempfile::Builder::new()
            .prefix("citadel-database-status-")
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir()
            .map_err(|_| failed())?;
        let docker = LocalDocker {
            process: self.process.as_ref(),
            cancellation,
            home: home.path(),
            deadline: Instant::now() + Duration::from_secs(5),
        };
        let raw = docker.json(&["container", "inspect", hostname]).await?;
        let core = single(&raw)?;
        if core["Id"]
            .as_str()
            .is_none_or(|id| !is_sha256(id) || !id.starts_with(hostname))
            || core["Config"]["Hostname"] != hostname
            || core["State"]["Running"] != true
        {
            return Ok(DatabaseInstallation::Unknown);
        }
        let labels = &core["Config"]["Labels"];
        let Some(project) = labels["com.docker.compose.project"]
            .as_str()
            .filter(|p| !p.is_empty())
        else {
            return Ok(DatabaseInstallation::Unknown);
        };
        if labels["com.docker.compose.service"]
            .as_str()
            .is_none_or(str::is_empty)
        {
            return Ok(DatabaseInstallation::Unknown);
        }
        let ids = docker
            .containers(&format!("label=com.docker.compose.project={project}"))
            .await?;
        // Bound work even for a custom project. Failure to inspect never implies external.
        if ids.is_empty() || ids.len() > 16 {
            return Ok(DatabaseInstallation::Unknown);
        }
        let mut args = vec!["container", "inspect"];
        args.extend(ids.iter().map(String::as_str));
        let containers = docker.json(&args).await?;
        let containers = containers.as_array().ok_or_else(failed)?;
        if containers.len() != ids.len()
            || ids
                .iter()
                .any(|id| !containers.iter().any(|c| c["Id"] == *id))
        {
            return Ok(DatabaseInstallation::Unknown);
        }
        let endpoint = crate::persistence::postgres::updates::database_endpoint(&self.pool).await?;
        Ok(classify(core, containers, endpoint.as_deref()))
    }
}
impl DatabaseInstallationRuntime for DockerDatabaseInstallation {
    fn database_installation<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, DatabaseInstallation> {
        Box::pin(async move {
            self.inspect(cancellation)
                .await
                .unwrap_or(DatabaseInstallation::Unknown)
        })
    }
}

fn classify(core: &Value, containers: &[Value], address: Option<&str>) -> DatabaseInstallation {
    let Some(address) = address
        .and_then(|s| s.parse::<std::net::IpAddr>().ok())
        .map(|ip| ip.to_canonical())
    else {
        return DatabaseInstallation::Unknown;
    };
    let Some(networks) = core["NetworkSettings"]["Networks"].as_object() else {
        return DatabaseInstallation::Unknown;
    };
    if networks.is_empty() || core["HostConfig"]["NetworkMode"] == "host" {
        return DatabaseInstallation::Unknown;
    }
    let project = &core["Config"]["Labels"]["com.docker.compose.project"];
    // The connected server's address identifies the database, not its service
    // name or image. A custom Compose layout can be bundled but ineligible for updates.
    let mut matches = 0;
    for db in containers
        .iter()
        .filter(|c| c["Config"]["Labels"]["com.docker.compose.project"] == *project)
    {
        if db["HostConfig"]["NetworkMode"] == "host" {
            return DatabaseInstallation::Unknown;
        }
        let Some(db_networks) = db["NetworkSettings"]["Networks"].as_object() else {
            return DatabaseInstallation::Unknown;
        };
        if networks.values().any(|network| {
            db_networks.values().any(|db_network| {
                network["NetworkID"]
                    .as_str()
                    .is_some_and(|id| !id.is_empty())
                    && network["NetworkID"] == db_network["NetworkID"]
                    && ["IPAddress", "GlobalIPv6Address"].iter().any(|field| {
                        db_network[*field]
                            .as_str()
                            .and_then(|s| s.parse::<std::net::IpAddr>().ok())
                            .is_some_and(|ip| ip.to_canonical() == address)
                    })
            })
        }) {
            matches += 1;
            continue;
        }
        if db_networks.values().any(|network| {
            ["IPAddress", "GlobalIPv6Address"].iter().any(|field| {
                network[*field]
                    .as_str()
                    .and_then(|s| s.parse::<std::net::IpAddr>().ok())
                    .is_some_and(|ip| ip.to_canonical() == address)
            })
        }) {
            return DatabaseInstallation::Unknown;
        }
    }
    match matches {
        0 => DatabaseInstallation::External,
        1 => DatabaseInstallation::Bundled,
        _ => DatabaseInstallation::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn topology_uses_the_connected_endpoint_not_service_names_or_container_order() {
        let core = json!({"Config":{"Labels":{"com.docker.compose.project":"citadel", "com.docker.compose.service":"core"}}, "NetworkSettings":{"Networks":{"default":{"NetworkID":"bridge", "IPAddress":"172.18.0.3"}}}});
        let db = json!({"Config":{"Labels":{"com.docker.compose.project":"citadel", "com.docker.compose.service":"postgres"}}, "NetworkSettings":{"Networks":{"default":{"NetworkID":"bridge", "IPAddress":"172.18.0.2"}}}});
        let mut unrelated = db.clone();
        unrelated["Config"]["Labels"]["com.docker.compose.service"] = json!("pg_db");
        unrelated["NetworkSettings"]["Networks"]["default"]["IPAddress"] = json!("172.18.0.4");
        for containers in [
            vec![core.clone(), db.clone(), unrelated.clone()],
            vec![unrelated.clone(), db.clone(), core.clone()],
        ] {
            assert_eq!(
                classify(&core, &containers, Some("172.18.0.2")),
                DatabaseInstallation::Bundled
            );
            assert_eq!(
                classify(&core, &containers, Some("192.0.2.2")),
                DatabaseInstallation::External
            );
        }
        // An address collision on a different network cannot establish identity.
        let mut collision = db.clone();
        collision["NetworkSettings"]["Networks"]["default"]["NetworkID"] = json!("other");
        for containers in [vec![db.clone(), collision.clone()], vec![collision, db]] {
            assert_eq!(
                classify(&core, &containers, Some("172.18.0.2")),
                DatabaseInstallation::Unknown
            );
        }
    }
    #[test]
    fn only_the_connected_database_in_cores_compose_project_is_bundled() {
        let core = json!({"Config":{"Labels":{"com.docker.compose.project":"citadel"}}, "NetworkSettings":{"Networks":{"default":{"NetworkID":"bridge"}}}});
        let db = json!({"Config":{"Labels":{"com.docker.compose.project":"citadel", "com.docker.compose.service":"pg_db"}}, "NetworkSettings":{"Networks":{"default":{"NetworkID":"bridge", "IPAddress":"172.18.0.2", "GlobalIPv6Address":"fd00::2"}}}});
        assert_eq!(
            classify(&core, std::slice::from_ref(&db), Some("172.18.0.2")),
            DatabaseInstallation::Bundled
        );
        assert_eq!(
            classify(&core, std::slice::from_ref(&db), Some("fd00::2")),
            DatabaseInstallation::Bundled
        );
        assert_eq!(
            classify(&core, std::slice::from_ref(&db), Some("192.0.2.2")),
            DatabaseInstallation::External
        );
        assert_eq!(
            classify(&core, &[], Some("172.18.0.2")),
            DatabaseInstallation::External
        );
        assert_eq!(
            classify(&core, std::slice::from_ref(&db), None),
            DatabaseInstallation::Unknown
        );
        assert_eq!(
            classify(&core, &[db.clone(), db.clone()], Some("172.18.0.2")),
            DatabaseInstallation::Unknown
        );
        assert_eq!(
            classify(&core, std::slice::from_ref(&db), Some("fd00:0:0:0:0:0:0:2")),
            DatabaseInstallation::Bundled
        );
        let mut host = db.clone();
        host["HostConfig"]["NetworkMode"] = json!("host");
        assert_eq!(
            classify(&core, &[host], Some("127.0.0.1")),
            DatabaseInstallation::Unknown
        );
        let mut foreign = db;
        foreign["Config"]["Labels"]["com.docker.compose.project"] = json!("foreign");
        assert_eq!(
            classify(&core, &[foreign], Some("172.18.0.2")),
            DatabaseInstallation::External
        );
    }
}
