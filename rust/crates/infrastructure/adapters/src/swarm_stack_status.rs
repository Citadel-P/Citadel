//! Idle Swarm Stack observation. Applying releases are owned by the operation
//! worker; incomplete observations must not turn an interrupted release healthy.
use citadel_platforms::{RuntimeInventorySnapshot, RuntimeSwarmInventory, RuntimeSwarmService};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

pub(crate) async fn reconcile(
    tx: &mut Transaction<'_, Postgres>,
    snapshot: &RuntimeInventorySnapshot,
    swarm: &RuntimeSwarmInventory,
) -> Result<(), sqlx::Error> {
    // Use the ownership normalized in this transaction. Raw Docker labels
    // must not heal a release whose namespace failed ownership validation.
    let owners: std::collections::HashMap<String, Uuid> = sqlx::query_as(
        "SELECT dockerserviceid,stackid FROM swarmserviceprojections WHERE platformid=$1 AND ownership='CitadelStack' AND stackid IS NOT NULL AND NOT isstale",
    ).bind(snapshot.platform_id).fetch_all(&mut **tx).await?.into_iter().collect();
    let rows = sqlx::query("SELECT s.id,s.stacksource,s.stackupdatestate,r.id releaseid,r.status,r.spec,r.source FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE r.platformid=$1 AND s.controlstate='Idle' AND r.status IN ('Unknown','Healthy','Pending','Paused','Degraded','Stopped','TimedOut') ORDER BY s.id FOR UPDATE OF s,r")
        .bind(snapshot.platform_id).fetch_all(&mut **tx).await?;
    for row in rows {
        let stack_id: Uuid = row.try_get("id")?;
        let release_id: Uuid = row.try_get("releaseid")?;
        let old: String = row.try_get("status")?;
        let services: Vec<_> = swarm
            .services
            .iter()
            .filter(|service| {
                owners.get(&service.id) == Some(&stack_id)
                    && label(service, "com.citadel.managed", "true")
                    && label(service, "com.citadel.stack-id", &stack_id.to_string())
                    && label(service, "com.citadel.release-id", &release_id.to_string())
            })
            .collect();
        let Some(next) = next_status(&old, &services, &swarm.tasks) else {
            continue;
        };
        if next == old {
            continue;
        }
        if next == "Healthy"
            && matches!(old.as_str(), "Unknown" | "TimedOut")
            && !complete_resources(tx, release_id, &services, swarm).await?
        {
            continue;
        }
        if next == "Healthy"
            && matches!(old.as_str(), "Unknown" | "TimedOut")
            && row.try_get::<String, _>("stacksource")? == "Git"
        {
            // Rust materializes each operation from immutable Git blobs, with no
            // shared active worktree. Recovery still requires the exact commit
            // captured before dispatch; it must never resolve a moving branch.
            let source: Option<serde_json::Value> = row.try_get("source")?;
            let Some(commit) = source
                .as_ref()
                .and_then(|source| {
                    source
                        .get("ResolvedCommitSha")
                        .or_else(|| source.get("resolvedCommitSha"))
                })
                .and_then(serde_json::Value::as_str)
                .filter(|commit| !commit.is_empty())
            else {
                continue;
            };
            let mut update = citadel_stacks::StackUpdateState::from_storage_value(
                row.try_get("stackupdatestate")?,
            )
            .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
            if update.record_applied_commit(commit, snapshot.observed_at) {
                sqlx::query("UPDATE stacks SET stackupdatestate=$2 WHERE id=$1")
                    .bind(stack_id)
                    .bind(
                        update
                            .to_storage_value()
                            .map_err(|error| sqlx::Error::Protocol(error.to_string()))?,
                    )
                    .execute(&mut **tx)
                    .await?;
            }
        }
        sqlx::query("UPDATE stackreleases SET status=$2 WHERE id=$1")
            .bind(release_id)
            .bind(next)
            .execute(&mut **tx)
            .await?;
        sqlx::query("UPDATE stacks SET rowversion=rowversion+1 WHERE id=$1")
            .bind(stack_id)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

fn label(service: &RuntimeSwarmService, name: &str, expected: &str) -> bool {
    service
        .labels
        .get(name)
        .is_some_and(|value| value.eq_ignore_ascii_case(expected))
}

fn next_status<'a>(
    old: &str,
    services: &[&RuntimeSwarmService],
    tasks: &[citadel_platforms::RuntimeSwarmTask],
) -> Option<&'a str> {
    let counts = services
        .iter()
        .map(|service| {
            service
                .labels
                .get("com.citadel.stack-service-count")?
                .parse::<usize>()
                .ok()
        })
        .collect::<Option<Vec<_>>>()?;
    let count = *counts.first()?;
    if count == 0 || counts.iter().any(|value| *value != count) {
        return None;
    }
    let recoverable = matches!(old, "Unknown" | "TimedOut");
    if services.len() != count && recoverable {
        return None;
    }
    let next = if services.len() == count {
        citadel_platforms::jobs::stack_observed_status(services, tasks)
    } else {
        "Degraded"
    };
    if recoverable && !matches!(next, "Healthy" | "Failed") {
        return None;
    }
    Some(next)
}

async fn complete_resources(
    tx: &mut Transaction<'_, Postgres>,
    release: Uuid,
    services: &[&RuntimeSwarmService],
    swarm: &RuntimeSwarmInventory,
) -> Result<bool, sqlx::Error> {
    let namespaces: std::collections::HashSet<_> = services
        .iter()
        .filter_map(|s| s.labels.get("com.docker.stack.namespace"))
        .filter(|value| !value.trim().is_empty())
        .collect();
    if namespaces.is_empty() {
        return Ok(false);
    }
    // A missing resource in the daemon response is incomplete evidence, even if
    // its ownership cannot be determined until the next successful refresh.
    if services.iter().any(|service| {
        service
            .secret_ids
            .iter()
            .any(|id| !swarm.secrets.iter().any(|r| &r.id == id))
            || service
                .config_ids
                .iter()
                .any(|id| !swarm.configs.iter().any(|r| &r.id == id))
    }) {
        return Ok(false);
    }
    let retained: Vec<(String, String)> = sqlx::query_as("SELECT kind,dockerresourceid FROM stackreleaseswarmresources WHERE stackreleaseid=$1 AND json_array_length(mounts)>0")
        .bind(release).fetch_all(&mut **tx).await?;
    let required_secrets = swarm
        .secrets
        .iter()
        .filter(|resource| {
            resource
                .labels
                .get("com.docker.stack.namespace")
                .is_some_and(|v| namespaces.contains(v))
                && services.iter().any(|s| s.secret_ids.contains(&resource.id))
        })
        .map(|resource| ("Secret", &resource.id));
    let required_configs = swarm
        .configs
        .iter()
        .filter(|resource| {
            resource
                .labels
                .get("com.docker.stack.namespace")
                .is_some_and(|v| namespaces.contains(v))
                && services.iter().any(|s| s.config_ids.contains(&resource.id))
        })
        .map(|resource| ("Config", &resource.id));
    Ok(required_secrets
        .chain(required_configs)
        .all(|(kind, id)| retained.iter().any(|(k, i)| k == kind && i == id)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interrupted_stack_waits_for_every_owned_service_to_converge() {
        let mut service = RuntimeSwarmService {
            desired_task_count: 1,
            running_task_count: 1,
            ..Default::default()
        };
        service
            .labels
            .insert("com.citadel.stack-service-count".into(), "2".into());
        assert_eq!(next_status("TimedOut", &[&service], &[]), None);
        assert_eq!(next_status("Healthy", &[&service], &[]), Some("Degraded"));
        let mut second = service.clone();
        second.running_task_count = 0;
        assert_eq!(next_status("Unknown", &[&service, &second], &[]), None);
        second.running_task_count = 1;
        assert_eq!(
            next_status("TimedOut", &[&service, &second], &[]),
            Some("Healthy")
        );
        second
            .labels
            .insert("com.citadel.stack-service-count".into(), "3".into());
        assert_eq!(next_status("Healthy", &[&service, &second], &[]), None);
    }
}
