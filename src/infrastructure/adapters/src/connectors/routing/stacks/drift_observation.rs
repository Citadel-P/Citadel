use super::*;
use crate::connectors::agent::execution::AgentExecutionClient;
use std::collections::{BTreeMap, BTreeSet};

impl StackRuntimeRouter {
    pub(super) async fn group_snapshot(
        &self,
        id: Uuid,
        projects: &BTreeSet<String>,
        cancel: &CancellationToken,
    ) -> Result<Option<BTreeMap<String, StackRuntimeSnapshot>>, StackError> {
        let target = self.platform(id).await?;
        let generation = self.docker.daemon_generation();
        let label = "com.docker.compose.project";
        let start = tokio::time::Instant::now();
        let peer = if target.connector == citadel_platforms::ConnectorKind::Local {
            None
        } else {
            Some(self.agent_for(&target, cancel).await?)
        };
        // The extra item is a truncation sentinel. A partial list cannot prove absence.
        const MAX_CONTAINERS: i32 = 4096;
        let limit = Some(MAX_CONTAINERS + 1);
        let list = async {
            match &peer {
                Some(peer) => match peer
                    .list_containers_by_label_limit(label, limit, cancel)
                    .await
                {
                    Ok(rows) => Ok(Some(rows)),
                    Err(error)
                        if error.kind == citadel_platforms::RuntimeErrorKind::ResourceExhausted =>
                    {
                        Ok(None)
                    }
                    Err(error) => Err(agent_error(error)),
                },
                None => {
                    let filters = serde_json::json!({"label": [label]}).to_string();
                    match self
                        .docker
                        .list_containers_filtered_limit(true, Some(&filters), limit)
                        .await
                    {
                        Ok(rows) => Ok(Some(
                            rows.into_iter()
                                .map(citadel_docker::client::runtime::map_container)
                                .collect(),
                        )),
                        Err(citadel_docker::client::DockerError::ResponseTooLarge { .. }) => {
                            Ok(None)
                        }
                        Err(error) => Err(runtime_io(error)),
                    }
                }
            }
        };
        let rows = tokio::select! {
            biased;
            () = cancel.cancelled() => return Err(StackError::Cancelled),
            rows = list => rows?,
        };
        let Some(rows) = rows.filter(|rows| rows.len() <= MAX_CONTAINERS as usize) else {
            return Ok(None);
        };
        let mut selected: BTreeMap<&str, Vec<_>> =
            projects.iter().map(|p| (p.as_str(), vec![])).collect();
        for row in rows {
            if let Some(group) = row
                .labels
                .get(label)
                .and_then(|p| selected.get_mut(p.as_str()))
            {
                group.push(row);
            }
        }
        let mut output = BTreeMap::new();
        let mut bytes = 0;
        for (project, rows) in selected {
            let snapshot = compose_snapshot(rows, project);
            bytes += snapshot
                .containers
                .iter()
                .map(|c| {
                    128 + c.docker_container_id.capacity()
                        + c.service_name.capacity()
                        + c.state.capacity()
                })
                .sum::<usize>();
            if bytes > 2 * 1024 * 1024 {
                return Ok(None);
            }
            output.insert(project.to_owned(), snapshot);
        }
        // Discard the whole capture if its route changed during the request.
        let current = self.platform(id).await?;
        if current.address != target.address
            || current.node_id != target.node_id
            || current.connector != target.connector
            || peer.is_none() && generation != self.docker.daemon_generation()
            || start.elapsed() > std::time::Duration::from_secs(5)
        {
            return Ok(None);
        }
        if let Some(original) = peer {
            let active = self.agent_for(&current, cancel).await?;
            let same = match (original, active) {
                (AgentExecutionClient::Direct(a), AgentExecutionClient::Direct(b)) => {
                    std::sync::Arc::ptr_eq(&a, &b)
                }
                (AgentExecutionClient::Edge(a), AgentExecutionClient::Edge(b)) => {
                    a.id == b.id && !a.is_closed()
                }
                _ => false,
            };
            if !same || start.elapsed() > std::time::Duration::from_secs(5) {
                return Ok(None);
            }
        }
        Ok(Some(output))
    }
}
