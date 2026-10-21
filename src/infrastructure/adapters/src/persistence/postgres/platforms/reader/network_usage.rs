use super::*;
use citadel_platforms::network_usage::*;

pub(super) async fn read(
    pool: &PgPool,
    actor: ActorId,
    administrator: bool,
    platform: Uuid,
    networks: &[NetworkUsageQuery],
) -> Result<Vec<NetworkUsage>, AuthorizedReadError> {
    let mut result = vec![
        NetworkUsage {
            is_partial: true,
            ..Default::default()
        };
        networks.len()
    ];
    // Bound request materialization and response size. Never inspect Docker here.
    for (batch_index, batch) in networks.chunks(100).enumerate() {
        let mut bounded = serde_json::to_value(batch).map_err(storage)?;
        for network in bounded.as_array_mut().expect("serialized slice") {
            if let Some(ids) = network["container_ids"].as_array_mut() {
                ids.truncate(500);
            }
        }
        let rows = sqlx::query(concat!(
            include_str!("network_usage_owners.sql"),
            include_str!("../../swarm_services/effective_health.sql"),
            include_str!("network_usage_owners_tail.sql"),
            include_str!("network_usage.sql")
        ))
        .bind(actor.value())
        .bind(administrator)
        .bind(platform)
        .bind(bounded)
        .fetch_all(pool)
        .await
        .map_err(storage)?;
        let scan_truncated = rows.len() > 2000;
        for (i, network) in batch.iter().enumerate() {
            result[batch_index * 100 + i].truncated =
                scan_truncated || network.container_ids.len() > 500;
        }
        for row in rows.into_iter().take(2000) {
            let kind = match row.try_get::<Option<i32>, _>("kind").map_err(storage)? {
                Some(1) => Some(NetworkWorkloadKind::Deployment),
                Some(2) => Some(NetworkWorkloadKind::Stack),
                Some(20) => Some(NetworkWorkloadKind::SwarmService),
                _ => None,
            };
            let index: Option<i32> = row.try_get("network_index").map_err(storage)?;
            let entry = NetworkUsageEntry {
                docker_service_id: row.try_get("dockerserviceid").map_err(storage)?,
                service_name: row.try_get("servicename").map_err(storage)?,
                observed_at: row.try_get("observedat").map_err(storage)?,
                is_stale: row.try_get("isstale").map_err(storage)?,
                relationship: if row
                    .try_get::<Option<String>, _>("dockerserviceid")
                    .map_err(storage)?
                    .is_some()
                {
                    NetworkRelationship::ServiceNetwork
                } else if index.is_some() {
                    NetworkRelationship::Attachment
                } else {
                    NetworkRelationship::SavedReference
                },
                container_id: row.try_get("containerid").map_err(storage)?,
                docker_container_id: row.try_get("dockercontainerid").map_err(storage)?,
                docker_node_id: row.try_get("dockernodeid").map_err(storage)?,
                container_name: row.try_get("containername").map_err(storage)?,
                container_state: row.try_get("containerstate").map_err(storage)?,
                workload_kind: kind,
                workload_id: row.try_get("ownerid").map_err(storage)?,
                workload_name: row.try_get("ownername").map_err(storage)?,
                workload_status: row.try_get("ownerstatus").map_err(storage)?,
                workload_processing: row.try_get("ownerprocessing").map_err(storage)?,
                reference: None,
            };
            if let Some(index) = index {
                push(&mut result[batch_index * 100 + index as usize], entry);
                continue;
            }
            let mut references: Vec<String> =
                serde_json::from_value(row.try_get("refs").map_err(storage)?).unwrap_or_default();
            // Deployments do not expose host/none/shared namespace modes. An
            // omitted or empty network list uses Docker's default bridge.
            if entry.workload_kind == Some(NetworkWorkloadKind::Deployment) && references.is_empty()
            {
                references.push("bridge".into());
            }
            let project = if entry.workload_kind == Some(NetworkWorkloadKind::Stack) {
                row.try_get::<Option<String>, _>("projectname")
                    .map_err(storage)?
                    .or_else(|| {
                        entry
                            .workload_name
                            .as_deref()
                            .zip(entry.workload_id)
                            .map(|(name, id)| citadel_stacks::normalize_project_name(name, id))
                    })
            } else {
                None
            };
            if let Some(compose) = row
                .try_get::<Option<String>, _>("compose")
                .map_err(storage)?
                && let Ok(model) = citadel_stacks::parse_compose(&[compose])
            {
                references.extend(model.network_names.into_values());
            }
            for (index, network) in batch.iter().enumerate() {
                // Saved configuration has no worker-local identity. Do not match it
                // to a same-named local network on an arbitrary Swarm node.
                if network.docker_node_id.is_some() && network.scope != "swarm" {
                    continue;
                }
                // Explicit references and Docker's project identity describe
                // configured use, even without containers or running Swarm tasks.
                let reference = references.iter().find(|reference| {
                    !reference.contains('$')
                        && (*reference == &network.id || *reference == &network.name)
                });
                let project_matches = project.as_ref().is_some_and(|project| {
                    !project.is_empty() && network.stack_project.as_ref() == Some(project)
                });
                if reference.is_some() || project_matches {
                    let mut entry = entry.clone();
                    entry.reference = Some(reference.unwrap_or(&network.name).clone());
                    push(&mut result[batch_index * 100 + index], entry);
                }
            }
        }
    }
    Ok(result)
}

fn push(usage: &mut NetworkUsage, entry: NetworkUsageEntry) {
    if usage.entries.len() < 50 {
        usage.entries.push(entry);
    } else {
        usage.truncated = true;
    }
}

pub(super) async fn containers(
    pool: &PgPool,
    actor: ActorId,
    administrator: bool,
    platform: Uuid,
    container: Option<Uuid>,
) -> Result<Vec<NetworkContainer>, AuthorizedReadError> {
    sqlx::query(concat!(
        include_str!("network_usage_owners.sql"),
        include_str!("../../swarm_services/effective_health.sql"),
        include_str!("network_usage_owners_tail.sql"),
        include_str!("network_containers.sql")
    ))
    .bind(actor.value())
    .bind(administrator)
    .bind(platform)
    .bind(container)
    .fetch_all(pool)
    .await
    .map_err(storage)?
    .into_iter()
    .map(|row| {
        Ok(NetworkContainer {
            container_id: row.try_get("id").map_err(storage)?,
            docker_container_id: row.try_get("dockercontainerid").map_err(storage)?,
            docker_node_id: row.try_get("dockernodeid").map_err(storage)?,
            container_name: row.try_get("name").map_err(storage)?,
            container_state: row.try_get("state").map_err(storage)?,
            is_stale: row.try_get("isstale").map_err(storage)?,
            workload_kind: match row.try_get::<Option<i32>, _>("kind").map_err(storage)? {
                Some(1) => Some(NetworkWorkloadKind::Deployment),
                Some(2) => Some(NetworkWorkloadKind::Stack),
                Some(20) => Some(NetworkWorkloadKind::SwarmService),
                _ => None,
            },
            workload_id: row.try_get("ownerid").map_err(storage)?,
            workload_name: row.try_get("ownername").map_err(storage)?,
            workload_status: row.try_get("ownerstatus").map_err(storage)?,
            workload_processing: row.try_get("ownerprocessing").map_err(storage)?,
        })
    })
    .collect()
}
