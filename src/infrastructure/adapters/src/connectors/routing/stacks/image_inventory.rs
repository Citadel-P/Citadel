//! Share image inventory only within one scheduled page, keyed by resolved daemon binding.
use super::{DockerClient, PlatformTarget, StackError, update_scanner::runtime_failure};
use crate::connectors::agent::execution::AgentExecutionClient;
use citadel_platforms::{ConnectorKind, RuntimeContainerSummary};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};
use tokio::sync::OnceCell;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(PartialEq)]
struct Binding {
    connector: ConnectorKind,
    address: String,
    node: Option<String>,
    session: Option<Uuid>,
}

impl Binding {
    fn resolve(
        target: &PlatformTarget,
        peer: Option<&AgentExecutionClient>,
    ) -> Result<Self, StackError> {
        Ok(Self {
            connector: target.connector,
            address: target.address.clone(),
            node: target.node_id.clone(),
            session: match peer {
                Some(AgentExecutionClient::Edge(s)) if s.is_closed() => {
                    return Err(StackError::Cancelled);
                }
                Some(AgentExecutionClient::Edge(s)) => Some(s.id),
                _ => None,
            },
        })
    }
}

#[derive(Default)]
pub(super) struct BatchObservations {
    standalone: Mutex<BTreeMap<Uuid, (Binding, Arc<StackImageInventory>)>>,
    swarm: Mutex<BTreeMap<Uuid, (Binding, Arc<SwarmImageInventory>)>>,
}

#[derive(Default)]
pub(super) struct SwarmImageInventory {
    pub(super) services: OnceCell<BTreeMap<String, Vec<citadel_platforms::RuntimeSwarmService>>>,
    pub(super) placement: OnceCell<(
        Vec<citadel_platforms::RuntimeSwarmNode>,
        Vec<citadel_platforms::RuntimeSwarmTask>,
    )>,
}

impl BatchObservations {
    pub(super) fn swarm(
        &self,
        target: &PlatformTarget,
        peer: Option<&AgentExecutionClient>,
    ) -> Result<Arc<SwarmImageInventory>, StackError> {
        let binding = Binding::resolve(target, peer)?;
        let mut entries = self.swarm.lock().unwrap();
        if let Some((_, value)) = entries.get(&target.id).filter(|(key, _)| *key == binding) {
            return Ok(value.clone());
        }
        let value = Arc::new(SwarmImageInventory::default());
        entries.insert(target.id, (binding, value.clone()));
        Ok(value)
    }
}

#[derive(Default)]
pub(super) struct StackImageInventory {
    owned: BTreeMap<(String, String), BTreeSet<String>>,
    legacy: BTreeMap<(String, String), BTreeSet<String>>,
    pub(super) images: BTreeMap<String, Vec<String>>,
}
impl StackImageInventory {
    pub(super) fn service_images(
        &self,
        stack: &str,
        project: &str,
        service: &str,
    ) -> impl Iterator<Item = &String> {
        self.owned
            .get(&(stack.into(), service.into()))
            .into_iter()
            .flatten()
            .chain(
                self.legacy
                    .get(&(project.into(), service.into()))
                    .into_iter()
                    .flatten(),
            )
    }
    pub(super) async fn observe(
        docker: &DockerClient,
        target: &PlatformTarget,
        peer: Option<&AgentExecutionClient>,
        batch: Option<&BatchObservations>,
        manual_scope: Option<(&str, &str)>,
        cancel: &CancellationToken,
    ) -> Result<Arc<Self>, StackError> {
        let binding = Binding::resolve(target, peer)?;
        if let Some(batch) = batch
            && let Some((_, inventory)) = batch
                .standalone
                .lock()
                .unwrap()
                .get(&target.id)
                .filter(|(key, _)| *key == binding)
        {
            return Ok(inventory.clone());
        }
        let list = |label: Option<String>| async move {
            if let Some(peer) = peer {
                let result = match label {
                    Some(label) => peer.list_containers_by_label(&label, cancel).await,
                    None => peer.list_containers(cancel).await,
                };
                result.map_err(runtime_failure)
            } else {
                let filters = label.map(|label| serde_json::json!({"label":[label]}).to_string());
                Ok(docker
                    .list_containers_filtered(true, filters.as_deref())
                    .await
                    .map_err(runtime_failure)?
                    .into_iter()
                    .map(citadel_docker::client::runtime::map_container)
                    .collect())
            }
        };
        let inventory = async {
            let containers = if let Some((stack, project)) = manual_scope {
                let mut containers = list(Some(format!("com.citadel.stack-id={stack}"))).await?;
                containers
                    .extend(list(Some(format!("com.docker.compose.project={project}"))).await?);
                containers
            } else {
                list(None).await?
            };
            let images = if let Some(peer) = peer {
                peer.image_digests(cancel).await.map_err(runtime_failure)?
            } else {
                docker
                    .image_digest_catalog()
                    .await
                    .map_err(runtime_failure)?
            };
            let mut inventory = Self {
                images,
                ..Default::default()
            };
            for container in containers {
                inventory.insert(container);
            }
            Ok(Arc::new(inventory))
        };
        let inventory = tokio::select! {biased; ()=cancel.cancelled()=>return Err(StackError::Cancelled),result=inventory=>result}?;
        if let Some(batch) = batch {
            batch
                .standalone
                .lock()
                .unwrap()
                .insert(target.id, (binding, Arc::clone(&inventory)));
        }
        Ok(inventory)
    }
    fn insert(&mut self, container: RuntimeContainerSummary) {
        let mut labels = container.labels;
        let Some(service) = labels.remove("com.docker.compose.service") else {
            return;
        };
        if let Some(owner) = labels.remove("com.citadel.stack-id") {
            self.owned
                .entry((owner, service))
                .or_default()
                .insert(container.image_id);
        } else if !labels.contains_key("com.citadel.deployment-id")
            && let Some(project) = labels.remove("com.docker.compose.project")
        {
            self.legacy
                .entry((project, service))
                .or_default()
                .insert(container.image_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn swarm_inventory_is_lazy_and_scoped_to_the_page_and_connection() {
        let page = BatchObservations::default();
        let mut target = PlatformTarget {
            id: Uuid::now_v7(),
            connector: ConnectorKind::Local,
            address: "first".into(),
            node_id: None,
            status: citadel_primitives::PlatformStatus::Online,
        };
        let first = page.swarm(&target, None).unwrap();
        first
            .services
            .get_or_init(|| async { BTreeMap::new() })
            .await;
        let second = page.swarm(&target, None).unwrap();
        assert!(second.services.initialized());
        assert!(!second.placement.initialized());
        assert!(
            !BatchObservations::default()
                .swarm(&target, None)
                .unwrap()
                .services
                .initialized()
        );
        target.address = "replacement".into();
        assert!(!page.swarm(&target, None).unwrap().services.initialized());
    }
    #[test]
    fn stopped_members_keep_explicit_ownership_and_deduplicate_legacy_matches() {
        let mut inventory = StackImageInventory::default();
        for (image, owner, project, deployment) in [
            ("owned", Some("one"), "misleading", None),
            ("foreign", Some("two"), "shared", None),
            ("legacy", None, "shared", None),
            ("legacy", None, "shared", None),
            ("deployment", None, "shared", Some("other")),
        ] {
            let mut labels = BTreeMap::from([
                ("com.docker.compose.service".into(), "web".into()),
                ("com.docker.compose.project".into(), project.into()),
            ]);
            if let Some(id) = owner {
                labels.insert("com.citadel.stack-id".into(), id.into());
            }
            if let Some(id) = deployment {
                labels.insert("com.citadel.deployment-id".into(), id.into());
            }
            inventory.insert(RuntimeContainerSummary {
                labels,
                image_id: image.into(),
                state: "exited".into(),
                ..Default::default()
            });
        }
        assert_eq!(
            inventory
                .service_images("one", "shared", "web")
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["owned", "legacy"]
        );
    }
}
