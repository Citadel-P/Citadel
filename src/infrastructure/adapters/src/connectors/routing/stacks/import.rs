use super::*;
use crate::connectors::containers::ownership::{Owner, owner};
use citadel_platforms::ContainerInventoryPort;
use std::collections::{BTreeMap, BTreeSet};

impl StackRuntimeRouter {
    pub(super) async fn orphaned_stack_owner(
        &self,
        owners: &[Option<Owner>],
    ) -> Result<Option<Uuid>, StackError> {
        let Some(Some(Owner::Stack(id))) = owners.iter().find(|owner| owner.is_some()) else {
            if owners.iter().any(Option::is_some) {
                return Err(StackError::Conflict(
                    "The project has unsupported Citadel ownership labels.".into(),
                ));
            }
            return Ok(None);
        };
        if owners.iter().any(|owner| *owner != Some(Owner::Stack(*id))) {
            return Err(StackError::Conflict(
                "The project contains mixed Citadel owners or unmanaged workloads.".into(),
            ));
        }
        if sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM stacks WHERE id=$1)")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(storage)?
        {
            return Err(StackError::Conflict(
                "The project is owned by an existing Citadel Stack.".into(),
            ));
        }
        Ok(Some(*id))
    }

    pub(super) async fn compose_import(
        &self,
        platform_id: Uuid,
        project: &str,
        cancel: &CancellationToken,
    ) -> Result<StackImportClaim, StackError> {
        let target = self.platform(platform_id).await?;
        let containers = if target.connector == citadel_platforms::ConnectorKind::Local {
            citadel_platforms::ContainerInventoryPort::list_containers(&self.docker, cancel).await
        } else {
            match self.agent_for(&target, cancel).await? {
                crate::connectors::agent::execution::AgentExecutionClient::Direct(agent) => {
                    agent.list_containers(cancel).await
                }
                crate::connectors::agent::execution::AgentExecutionClient::Edge(session) => {
                    crate::connectors::edge::EdgeRuntime { session }
                        .list_containers(cancel)
                        .await
                }
            }
        }
        .map_err(agent_error)?;
        let mut containers = containers
            .into_iter()
            .filter(|c| {
                c.labels
                    .get("com.docker.compose.project")
                    .is_some_and(|p| p == project)
            })
            .collect::<Vec<_>>();
        containers.sort_by(|a, b| a.id.cmp(&b.id));
        if containers.is_empty() {
            return Err(StackError::NotFound);
        }
        let rows = sqlx::query("SELECT dockercontainerid,issystem,isswarmtask,deploymentid,stackid,controlstate FROM containers WHERE platformid=$1 AND stack=$2")
            .bind(platform_id).bind(project).fetch_all(&self.pool).await.map_err(storage)?;
        let persisted = rows
            .iter()
            .map(|r| r.try_get::<String, _>("dockercontainerid").map_err(storage))
            .collect::<Result<BTreeSet<_>, _>>()?;
        if persisted != containers.iter().map(|c| c.id.clone()).collect() {
            return Err(StackError::Conflict(
                "Compose project membership changed. Refresh the Platform and retry.".into(),
            ));
        }
        for row in &rows {
            if row.try_get::<bool, _>("issystem").map_err(storage)?
                || row.try_get::<bool, _>("isswarmtask").map_err(storage)?
                || row
                    .try_get::<Option<Uuid>, _>("deploymentid")
                    .map_err(storage)?
                    .is_some()
                || row
                    .try_get::<Option<Uuid>, _>("stackid")
                    .map_err(storage)?
                    .is_some()
                || row
                    .try_get::<Option<String>, _>("controlstate")
                    .map_err(storage)?
                    .as_deref()
                    == Some("Processing")
            {
                return Err(StackError::Conflict(
                    "Compose project contains system, managed or processing containers.".into(),
                ));
            }
        }
        let mut owners = Vec::new();
        let mut grouped = BTreeMap::<String, ComposeProjectRuntimeService>::new();
        let mut hash = Sha256::new();
        for container in &containers {
            if container.is_system || container.is_swarm_task {
                return Err(StackError::Conflict(
                    "System or Swarm task containers cannot be imported as a Compose project."
                        .into(),
                ));
            }
            owners.push(
                owner(&container.labels).map_err(|message| StackError::Conflict(message.into()))?,
            );
            hash.update(serde_json::to_vec(&container.labels).map_err(runtime_io)?);
            let name = container
                .labels
                .get("com.docker.compose.service")
                .filter(|name| !name.is_empty())
                .ok_or_else(|| {
                    StackError::Conflict(
                        "Compose service labels are missing. Refresh the Platform and retry."
                            .into(),
                    )
                })?;
            let service =
                grouped
                    .entry(name.clone())
                    .or_insert_with(|| ComposeProjectRuntimeService {
                        name: name.clone(),
                        image: Some(container.image.clone()),
                        container_count: 0,
                        states: Vec::new(),
                    });
            service.container_count += 1;
            service.states.push(container.state.clone());
        }
        Ok(StackImportClaim {
            orphaned_owner_id: self.orphaned_stack_owner(&owners).await?,
            platform_id,
            platform_name: String::new(),
            project_name: project.into(),
            import_kind: StackImportKind::ComposeProject,
            runtime_fingerprint: hash
                .finalize()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
            service_names: grouped.keys().cloned().collect(),
            container_ids: containers.iter().map(|c| c.id.clone()).collect(),
            container_names: containers.iter().map(|c| c.name.clone()).collect(),
            services: grouped.into_values().collect(),
        })
    }
}
