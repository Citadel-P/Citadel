use super::super::*;
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
    if networks.is_empty() {
        return Ok(result);
    }
    let saved = sqlx::query(concat!(
        include_str!("network_saved_owners.sql"),
        include_str!("../../swarm_services/effective_health.sql"),
        include_str!("network_usage_owners_tail.sql"),
        include_str!("network_saved.sql")
    ))
    .bind(actor.value())
    .bind(administrator)
    .bind(platform)
    .fetch_all(pool)
    .await
    .map_err(storage)?;
    let saved_count = saved.len();
    let saved: Vec<_> = saved
        .into_iter()
        .take(2000)
        .map(saved_reference)
        .collect::<Result<_, _>>()?;
    // Serialize only the admitted ID prefix, without first copying all endpoints.
    for (batch_index, batch) in networks.chunks(100).enumerate() {
        let bounded: Vec<_> = batch.iter().map(BoundedNetwork::from).collect();
        let rows = sqlx::query(concat!(
            include_str!("network_usage_owners.sql"),
            include_str!("../../swarm_services/effective_health.sql"),
            include_str!("network_usage_owners_tail.sql"),
            include_str!("network_usage.sql")
        ))
        .bind(actor.value())
        .bind(administrator)
        .bind(platform)
        .bind(sqlx::types::Json(&bounded))
        .fetch_all(pool)
        .await
        .map_err(storage)?;
        let scan_truncated = rows.len() + saved_count > 2000;
        let remaining = 2000usize.saturating_sub(rows.len());
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
        }
        for (entry, references, project) in saved.iter().take(remaining) {
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
                    let mut entry = (*entry).clone();
                    entry.reference = Some(reference.unwrap_or(&network.name).clone());
                    push(&mut result[batch_index * 100 + index], entry);
                }
            }
        }
    }
    Ok(result)
}

#[derive(serde::Serialize)]
struct BoundedNetwork<'a> {
    id: &'a str,
    scope: &'a str,
    docker_node_id: &'a Option<String>,
    container_ids: &'a [String],
}
impl<'a> From<&'a NetworkUsageQuery> for BoundedNetwork<'a> {
    fn from(value: &'a NetworkUsageQuery) -> Self {
        Self {
            id: &value.id,
            scope: &value.scope,
            docker_node_id: &value.docker_node_id,
            container_ids: &value.container_ids[..value.container_ids.len().min(500)],
        }
    }
}
type SavedReference = (NetworkUsageEntry, Vec<String>, Option<String>);
fn saved_reference(row: PgRow) -> Result<SavedReference, AuthorizedReadError> {
    let kind = match row.try_get::<i32, _>("kind").map_err(storage)? {
        1 => Some(NetworkWorkloadKind::Deployment),
        2 => Some(NetworkWorkloadKind::Stack),
        20 => Some(NetworkWorkloadKind::SwarmService),
        _ => None,
    };
    let entry = NetworkUsageEntry {
        docker_service_id: None,
        service_name: None,
        observed_at: None,
        is_stale: false,
        relationship: NetworkRelationship::SavedReference,
        container_id: None,
        docker_container_id: None,
        docker_node_id: None,
        container_name: None,
        container_state: None,
        workload_kind: kind,
        workload_id: row.try_get("ownerid").map_err(storage)?,
        workload_name: row.try_get("ownername").map_err(storage)?,
        workload_status: row.try_get("ownerstatus").map_err(storage)?,
        workload_processing: row.try_get("ownerprocessing").map_err(storage)?,
        reference: None,
    };
    let mut references: Vec<String> =
        serde_json::from_value(row.try_get("refs").map_err(storage)?).unwrap_or_default();
    // Deployments do not expose host/none/shared namespace modes. An
    // omitted or empty network list uses Docker's default bridge.
    if entry.workload_kind == Some(NetworkWorkloadKind::Deployment) && references.is_empty() {
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
    Ok((entry, references, project))
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn network_payload_bounds_endpoints_before_serializing() {
        let input = NetworkUsageQuery {
            id: "id".into(),
            name: "name".into(),
            scope: "local".into(),
            docker_node_id: None,
            container_ids: (0..10_000).map(|i| i.to_string()).collect(),
            stack_project: None,
        };
        let bounded = BoundedNetwork::from(&input);
        assert_eq!(bounded.container_ids.len(), 500);
        assert_eq!(bounded.container_ids.as_ptr(), input.container_ids.as_ptr());
        let encoded = serde_json::to_value(&bounded).unwrap();
        assert_eq!(encoded["container_ids"].as_array().unwrap().len(), 500);
        assert!(encoded.get("name").is_none());
    }
}
