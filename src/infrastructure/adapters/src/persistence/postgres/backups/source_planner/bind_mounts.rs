use super::*;
use crate::connectors::routing::containers::Runtime;
use citadel_platforms::containers::{ContainerInspectionPort, ContainerTarget};

impl PostgresBackupSourcePlanner {
    pub(super) async fn include_bind_mounts(
        &self,
        mut plan: BackupSourcePlan,
        kind: BackupPreviewKind,
        id: Uuid,
        platform: Uuid,
        strict: bool,
        cancellation: &tokio_util::sync::CancellationToken,
    ) -> Result<BackupSourcePlan, BackupError> {
        let (binds, warnings) = self
            .workload_bind_mounts(kind, id, platform, strict, cancellation)
            .await?;
        if !binds.is_empty() {
            plan.display_name = format!(
                "{}; {} bind mount{}",
                plan.display_name,
                binds.len(),
                plural(binds.len())
            );
        }
        plan.items.extend(binds);
        plan.warnings.extend(warnings);
        if plan.items.len() > 1000 {
            return Err(BackupError::Validation(
                "Backup source exceeds the 1000 storage target limit.".into(),
            ));
        }
        if plan.items.is_empty() {
            if strict {
                return Err(BackupError::Validation(
                    "No supported volumes or bind mounts were resolved for this resource.".into(),
                ));
            }
            plan.warnings.push(
                "No supported volumes or bind mounts were resolved for this resource.".into(),
            );
        }
        Ok(plan)
    }

    /// Resolve physical host paths from owned, synchronized containers. Never infer
    /// a Compose-relative path on Core or fall back to a different Swarm node.
    pub(super) async fn workload_bind_mounts(
        &self,
        kind: BackupPreviewKind,
        id: Uuid,
        platform: Uuid,
        strict: bool,
        cancellation: &tokio_util::sync::CancellationToken,
    ) -> Result<(Vec<BackupSourceItem>, Vec<String>), BackupError> {
        let kind = match kind {
            BackupPreviewKind::Deployment => "Deployment",
            BackupPreviewKind::Stack => "Stack",
            BackupPreviewKind::SwarmService => "SwarmService",
        };
        let targets = sqlx::query(r#"
            SELECT DISTINCT dockercontainerid,dockernodeid,projectionstalesince IS NOT NULL AS incomplete FROM containers
            WHERE platformid=$1
              AND (($3='Deployment' AND deploymentid=$2) OR ($3='Stack' AND stackid=$2 AND dockernodeid IS NULL))
            UNION
            SELECT t.dockercontainerid,t.dockernodeid,
                   (t.isstale OR s.isstale OR lower(t.state)<>'running' OR t.dockercontainerid IS NULL OR t.dockernodeid IS NULL) AS incomplete
            FROM swarmtaskprojections t
            JOIN swarmserviceprojections s USING(platformid,dockerserviceid)
            WHERE t.platformid=$1 AND lower(t.desiredstate)='running'
              AND (($3='SwarmService' AND (s.swarmserviceid=$2 OR s.dockerserviceid=(SELECT dockerserviceid FROM swarmservices WHERE id=$2))) OR ($3='Stack' AND s.stackid=$2))
            LIMIT 1001
        "#).bind(platform).bind(id).bind(kind).fetch_all(&self.pool).await.map_err(storage)?;
        if targets.len() > 1000 {
            return Err(BackupError::Validation(
                "Bind mount discovery exceeds the 1000 Container limit.".into(),
            ));
        }
        if targets.is_empty() {
            let message = "No synchronized containers were available to discover bind mounts. Apply the workload and synchronize its inventory.";
            return if strict {
                Err(BackupError::Validation(message.into()))
            } else {
                Ok((vec![], vec![message.into()]))
            };
        }
        let mut warnings = BTreeSet::new();
        let mut complete_targets = Vec::new();
        for row in targets {
            if row.try_get::<bool, _>("incomplete").map_err(storage)? {
                let message = "Bind mount discovery is incomplete: an owned Container or current Task has stale or incomplete inventory. Synchronize the workload before backing up.";
                if strict {
                    return Err(BackupError::Validation(message.into()));
                }
                warnings.insert(message.into());
            } else {
                complete_targets.push(row);
            }
        }
        if complete_targets.is_empty() {
            return Ok((vec![], warnings.into_iter().collect()));
        }
        let Some(router) = &self.runtime else {
            let message = "Bind mount discovery requires Container inspection.";
            return if strict {
                Err(BackupError::Validation(message.into()))
            } else {
                warnings.insert(message.into());
                Ok((vec![], warnings.into_iter().collect()))
            };
        };
        let mut items = BTreeMap::new();
        for row in complete_targets {
            let target = ContainerTarget {
                id: Uuid::nil(),
                platform_id: platform,
                docker_id: row.try_get("dockercontainerid").map_err(storage)?,
                node_id: row.try_get("dockernodeid").map_err(storage)?,
            };
            let inspected = async {
                match router.resolve(&target, cancellation).await? {
                    Runtime::Local(r) => r.inspection(&target.docker_id, cancellation).await,
                    Runtime::Agent(r) => r.inspection(&target.docker_id, cancellation).await,
                    Runtime::Edge(r) => r.inspection(&target.docker_id, cancellation).await,
                }
            }
            .await;
            let document = match inspected {
                Ok(document) => document,
                Err(_) => {
                    let message = "Bind mount discovery is incomplete: an owned Container could not be inspected.";
                    if strict {
                        return Err(BackupError::Validation(message.into()));
                    }
                    warnings.insert(message.into());
                    continue;
                }
            };
            let (paths, excluded) = supported_bind_paths(&document);
            warnings.extend(excluded);
            for path in paths {
                let key = (target.node_id.clone(), path.clone());
                items.entry(key).or_insert_with(|| {
                    let mut item = BackupSourceItem::new(
                        platform,
                        String::new(),
                        target.node_id.clone(),
                        None,
                    );
                    item.host_path = Some(path);
                    item
                });
                if items.len() > 1000 {
                    return Err(BackupError::Validation(
                        "Backup source exceeds the 1000 bind mount limit.".into(),
                    ));
                }
            }
        }
        Ok((
            items.into_values().collect(),
            warnings.into_iter().collect(),
        ))
    }
}

fn supported_bind_paths(document: &Value) -> (BTreeSet<String>, Vec<String>) {
    let mut paths = BTreeSet::new();
    let mut warnings = Vec::new();
    for mount in document
        .get("mounts")
        .or_else(|| document.get("Mounts"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if optional_string(mount, "type").is_none_or(|kind| !kind.eq_ignore_ascii_case("bind")) {
            continue;
        }
        let Some(path) = mount
            .get("source")
            .or_else(|| mount.get("Source"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        if citadel_backups::spec::validate_bind_path(path).is_ok() {
            paths.insert(path.to_owned());
        } else {
            warnings.push(format!(
                "Bind mount '{path}' is excluded because its host path is not supported for backup."
            ));
        }
    }
    (paths, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovers_bind_paths_without_confusing_volume_paths_or_runtime_mounts() {
        let (paths, warnings) = supported_bind_paths(&serde_json::json!({"Mounts": [
            {"Type":"bind", "Source":"/srv/data"},
            {"type":"bind", "source":"/srv/data"},
            {"Type":"bind", "Source":"/srv/config.json"},
            {"Type":"volume", "Name":"data", "Source":"/var/lib/docker/volumes/data/_data"},
            {"Type":"bind", "Source":"/var/run/docker.sock"},
            {"Type":"bind", "Source":"/"},
            {"Type":"tmpfs", "Destination":"/tmp"}
        ]}));
        assert_eq!(
            paths,
            BTreeSet::from(["/srv/data".into(), "/srv/config.json".into()])
        );
        assert_eq!(warnings.len(), 2);
    }
}
