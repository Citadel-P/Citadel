use super::*;
use crate::connectors::routing::containers::Runtime;
use citadel_platforms::containers::{ContainerInspectionPort, ContainerTarget};

#[derive(Default)]
pub(super) struct WorkloadStorage {
    pub items: Vec<BackupSourceItem>,
    pub warnings: Vec<String>,
    pub services: BTreeSet<String>,
}

impl PostgresBackupSourcePlanner {
    pub(super) async fn include_bind_mounts(
        &self,
        plan: BackupSourcePlan,
        kind: BackupPreviewKind,
        id: Uuid,
        platform: Uuid,
        strict: bool,
        cancellation: &tokio_util::sync::CancellationToken,
    ) -> Result<BackupSourcePlan, BackupError> {
        let (binds, warnings) = self
            .workload_bind_mounts(kind, id, platform, strict, cancellation)
            .await?;
        finish_storage_plan(plan, binds, warnings, strict)
    }

    pub(super) async fn workload_bind_mounts(
        &self,
        kind: BackupPreviewKind,
        id: Uuid,
        platform: Uuid,
        strict: bool,
        cancellation: &tokio_util::sync::CancellationToken,
    ) -> Result<(Vec<BackupSourceItem>, Vec<String>), BackupError> {
        let discovery = self
            .workload_mounts(kind, id, platform, strict, cancellation)
            .await?;
        Ok((
            discovery
                .items
                .into_iter()
                .filter(|item| item.host_path.is_some())
                .collect(),
            discovery.warnings,
        ))
    }

    pub(super) async fn standalone_stack_storage(
        &self,
        id: Uuid,
        platform: Uuid,
        name: &str,
        strict: bool,
        cancellation: &tokio_util::sync::CancellationToken,
    ) -> Result<BackupSourcePlan, BackupError> {
        let WorkloadStorage {
            items,
            mut warnings,
            services,
        } = self
            .workload_mounts(BackupPreviewKind::Stack, id, platform, strict, cancellation)
            .await?;
        if !items.is_empty() {
            // Compare against the applied release, never a moving Git branch.
            let expected = self.stack_service_names(id, cancellation).await?;
            let missing = expected.difference(&services).cloned().collect::<Vec<_>>();
            if !missing.is_empty() {
                let message = format!(
                    "Storage discovery is incomplete: Stack services are missing from synchronized containers: {}. Apply the Stack and synchronize its inventory before backing up.",
                    missing.join(", ")
                );
                if strict {
                    return Err(BackupError::Validation(message));
                }
                warnings.push(message);
            }
        }
        if items.is_empty() {
            let message = "No supported volumes or bind mounts were resolved for this resource.";
            if strict {
                return Err(BackupError::Validation(message.into()));
            }
            warnings.push(message.into());
        }
        Ok(BackupSourcePlan {
            display_name: format!("Stack {name} ({} storage targets)", items.len()),
            items,
            warnings,
            local_directory: None,
        })
    }

    async fn stack_service_names(
        &self,
        id: Uuid,
        cancellation: &tokio_util::sync::CancellationToken,
    ) -> Result<BTreeSet<String>, BackupError> {
        let row = sqlx::query("SELECT r.spec,r.source FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE s.id=$1")
            .bind(id).fetch_optional(&self.pool).await.map_err(storage)?.ok_or(BackupError::NotFound)?;
        let mut spec = StackSpec::from_storage_value(row.try_get("spec").map_err(storage)?)
            .map_err(storage)?;
        if let StackSpec::Git { commit_sha, .. } = &mut spec {
            let source = row
                .try_get::<Option<Value>, _>("source")
                .map_err(storage)?
                .map(citadel_stacks::StackReleaseSource::from_storage_value)
                .transpose()
                .map_err(storage)?;
            let commit = source.map(|source| source.resolved_commit_sha).filter(|commit| !commit.trim().is_empty())
                .ok_or_else(|| BackupError::Validation("The applied Git commit is unavailable. Reapply the Stack before backing up its storage.".into()))?;
            *commit_sha = Some(commit);
        }
        let files = self.stack_compose_files(&spec, cancellation).await?;
        let model = parse_compose(&files).map_err(|e| BackupError::Validation(e.to_string()))?;
        Ok(model
            .services
            .into_iter()
            .map(|service| service.name)
            .collect())
    }

    /// Resolve physical volume names and host paths from owned, synchronized containers. Never infer
    /// a Compose-relative path on Core or fall back to a different Swarm node.
    pub(super) async fn workload_mounts(
        &self,
        kind: BackupPreviewKind,
        id: Uuid,
        platform: Uuid,
        strict: bool,
        cancellation: &tokio_util::sync::CancellationToken,
    ) -> Result<WorkloadStorage, BackupError> {
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
                "Storage discovery exceeds the 1000 Container limit.".into(),
            ));
        }
        if targets.is_empty() {
            let message = "No synchronized containers were available to discover storage. Apply the workload and synchronize its inventory.";
            return if strict {
                Err(BackupError::Validation(message.into()))
            } else {
                Ok(WorkloadStorage {
                    warnings: vec![message.into()],
                    ..Default::default()
                })
            };
        }
        if kind == "Deployment" && targets.len() > 1 {
            return Err(BackupError::Conflict(
                "Deployment Container ownership is ambiguous.".into(),
            ));
        }
        let mut warnings = BTreeSet::new();
        let mut complete_targets = Vec::new();
        for row in targets {
            if row.try_get::<bool, _>("incomplete").map_err(storage)? {
                let message = "Storage discovery is incomplete: an owned Container or current Task has stale or incomplete inventory. Synchronize the workload before backing up.";
                if strict {
                    return Err(BackupError::Validation(message.into()));
                }
                warnings.insert(message.into());
            } else {
                complete_targets.push(row);
            }
        }
        if complete_targets.is_empty() {
            return Ok(WorkloadStorage {
                warnings: warnings.into_iter().collect(),
                ..Default::default()
            });
        }
        let Some(router) = &self.runtime else {
            let message = "Storage discovery requires Container inspection.";
            return if strict {
                Err(BackupError::Validation(message.into()))
            } else {
                warnings.insert(message.into());
                Ok(WorkloadStorage {
                    warnings: warnings.into_iter().collect(),
                    ..Default::default()
                })
            };
        };
        let mut items = BTreeMap::new();
        let mut services = BTreeSet::new();
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
                    let message = "Storage discovery is incomplete: an owned Container could not be inspected.";
                    if strict {
                        return Err(BackupError::Validation(message.into()));
                    }
                    warnings.insert(message.into());
                    continue;
                }
            };
            if let Some(service) = document
                .get("config")
                .or_else(|| document.get("Config"))
                .and_then(|v| v.get("labels").or_else(|| v.get("Labels")))
                .and_then(|v| v.get("com.docker.compose.service"))
                .and_then(Value::as_str)
            {
                services.insert(service.to_owned());
            }
            let discovered = match supported_mounts(&document, platform, target.node_id.as_deref())
            {
                Ok(discovered) => discovered,
                Err(error) if strict => return Err(error),
                Err(error) => {
                    warnings.insert(error.to_string());
                    continue;
                }
            };
            warnings.extend(discovered.1);
            for item in discovered.0 {
                // Host paths and Docker volume names are different source identities.
                let key = (
                    item.docker_node_id.clone(),
                    item.host_path.clone(),
                    item.volume_name.clone(),
                );
                items.entry(key).or_insert(item);
                if items.len() > 1000 {
                    return Err(BackupError::Validation(
                        "Backup source exceeds the 1000 storage target limit.".into(),
                    ));
                }
            }
        }
        Ok(WorkloadStorage {
            items: items.into_values().collect(),
            warnings: warnings.into_iter().collect(),
            services,
        })
    }
}

pub(super) fn finish_storage_plan(
    mut plan: BackupSourcePlan,
    binds: Vec<BackupSourceItem>,
    warnings: Vec<String>,
    strict: bool,
) -> Result<BackupSourcePlan, BackupError> {
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
        plan.warnings
            .push("No supported volumes or bind mounts were resolved for this resource.".into());
    }
    Ok(plan)
}

pub(super) fn volume_names(document: &Value) -> Result<BTreeSet<String>, BackupError> {
    let mut names = BTreeSet::new();
    for mount in inspected_mounts(document)? {
        if optional_string(mount, "type").is_some_and(|kind| kind.eq_ignore_ascii_case("volume")) {
            let name = optional_string(mount, "name").ok_or_else(|| BackupError::Validation(
                "Storage discovery is incomplete: an inspected Docker volume has no physical name.".into()
            ))?;
            names.insert(name);
        }
    }
    Ok(names)
}

fn inspected_mounts(document: &Value) -> Result<&Vec<Value>, BackupError> {
    document.get("mounts").or_else(|| document.get("Mounts"))
        .and_then(Value::as_array)
        .ok_or_else(|| BackupError::Validation("Storage discovery is incomplete: Container inspection did not include mount information.".into()))
}

pub(super) fn supported_mounts(
    document: &Value,
    platform: Uuid,
    node: Option<&str>,
) -> Result<(Vec<BackupSourceItem>, Vec<String>), BackupError> {
    let mut items = volume_names(document)?
        .into_iter()
        .map(|name| BackupSourceItem::new(platform, name, node.map(str::to_owned), None))
        .collect::<Vec<_>>();
    let mut paths = BTreeSet::new();
    let mut warnings = Vec::new();
    for mount in inspected_mounts(document)? {
        let kind = optional_string(mount, "type").ok_or_else(|| {
            BackupError::Validation(
                "Storage discovery is incomplete: an inspected mount has no type.".into(),
            )
        })?;
        if !kind.eq_ignore_ascii_case("bind") {
            continue;
        }
        let path = mount
            .get("source")
            .or_else(|| mount.get("Source"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                BackupError::Validation(
                    "Storage discovery is incomplete: an inspected bind mount has no host path."
                        .into(),
                )
            })?;
        if citadel_backups::spec::validate_bind_path(path).is_ok() {
            paths.insert(path.to_owned());
        } else {
            warnings.push(format!(
                "Bind mount '{path}' is excluded because its host path is not supported for backup."
            ));
        }
    }
    for path in paths {
        let mut item =
            BackupSourceItem::new(platform, String::new(), node.map(str::to_owned), None);
        item.host_path = Some(path);
        items.push(item);
    }
    Ok((items, warnings))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovers_bind_paths_without_confusing_volume_paths_or_runtime_mounts() {
        let (items, warnings) = supported_mounts(
            &serde_json::json!({"Mounts": [
                {"Type":"bind", "Source":"/srv/data"},
                {"type":"bind", "source":"/srv/data"},
                {"Type":"bind", "Source":"/srv/config.json"},
                {"Type":"volume", "Name":"data", "Source":"/var/lib/docker/volumes/data/_data"},
                {"Type":"bind", "Source":"/var/run/docker.sock"},
                {"Type":"bind", "Source":"/"},
                {"Type":"tmpfs", "Destination":"/tmp"}
            ]}),
            Uuid::nil(),
            None,
        )
        .unwrap();
        assert_eq!(
            items
                .iter()
                .filter(|item| item.host_path.is_none())
                .map(|item| item.volume_name.as_str())
                .collect::<Vec<_>>(),
            vec!["data"]
        );
        assert_eq!(
            items
                .into_iter()
                .filter_map(|item| item.host_path)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["/srv/data".into(), "/srv/config.json".into()])
        );
        assert_eq!(warnings.len(), 2);
    }
    #[test]
    fn physical_volume_names_support_both_transport_casings_and_deduplicate() {
        for document in [
            serde_json::json!({"Mounts":[{"Type":"volume","Name":"actual_data"},{"Type":"volume","Name":"actual_data"},{"Type":"volume","Name":"anonymous"}]}),
            serde_json::json!({"mounts":[{"type":"volume","name":"actual_data"},{"type":"volume","name":"actual_data"},{"type":"volume","name":"anonymous"}]}),
        ] {
            let (items, warnings) =
                supported_mounts(&document, Uuid::nil(), Some("worker-a")).unwrap();
            assert!(warnings.is_empty());
            assert_eq!(items.len(), 2);
            assert!(
                items
                    .iter()
                    .all(|item| item.docker_node_id.as_deref() == Some("worker-a")
                        && item.host_path.is_none())
            );
            assert_eq!(
                volume_names(&document).unwrap(),
                BTreeSet::from(["actual_data".into(), "anonymous".into()])
            );
        }
    }

    #[test]
    fn missing_inspection_metadata_is_not_an_empty_successful_source() {
        for document in [
            serde_json::json!({}),
            serde_json::json!({"mounts":null}),
            serde_json::json!({"mounts":[{"type":"volume"}]}),
            serde_json::json!({"mounts":[{"type":"bind"}]}),
            serde_json::json!({"mounts":[{"source":"/srv/data"}]}),
        ] {
            assert!(matches!(
                supported_mounts(&document, Uuid::nil(), None),
                Err(BackupError::Validation(_))
            ));
        }
        assert!(
            volume_names(&serde_json::json!({"mounts":[]}))
                .unwrap()
                .is_empty()
        );
    }
}
