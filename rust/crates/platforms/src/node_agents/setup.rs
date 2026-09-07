//! Install/repair/upgrade share one idempotent system-Service workflow.
use super::lifecycle::*;
use crate::{
    RuntimeCapabilityError, RuntimeInventorySnapshot, RuntimePlatformInfo, RuntimeSwarmService,
};
use citadel_domain::ActorId;
use futures_util::future::BoxFuture;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::Duration,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Clone, Copy, Debug)]
pub enum SetupKind {
    Install,
    Repair,
    Upgrade,
}
impl SetupKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Install => "Install",
            Self::Repair => "Repair",
            Self::Upgrade => "Upgrade",
        }
    }
}

pub struct SetupOptions {
    pub core_url: String,
    pub image: String,
    pub ca_bundle: Option<Arc<[u8]>>,
}
pub struct AgentDistribution {
    pub digest: String,
    pub linux_architectures: BTreeSet<String>,
}
pub struct Bootstrap {
    pub id: Uuid,
    pub secret_name: String,
    pub token: zeroize::Zeroizing<Vec<u8>>,
}
pub struct SystemAgentSpec {
    pub name: String,
    pub image: String,
    pub environment: Vec<String>,
    pub manager_node_id: String,
    pub volume_name: String,
    pub secret_id: String,
    pub secret_name: String,
    pub ca_config_id: Option<String>,
    pub ca_config_name: Option<String>,
    pub architectures: Vec<String>,
    pub labels: BTreeMap<String, String>,
}
pub trait NodeAgentSetupStore: NodeAgentLifecycleStore {
    fn claim_setup(
        &self,
        actor: ActorId,
        id: Uuid,
        info: RuntimePlatformInfo,
        kind: SetupKind,
    ) -> BoxFuture<'_, Result<NodeAgentRemovalClaim, RuntimeCapabilityError>>;
    fn bootstrap<'a>(
        &'a self,
        claim: &'a NodeAgentRemovalClaim,
    ) -> BoxFuture<'a, Result<Bootstrap, RuntimeCapabilityError>>;
    fn secret_created<'a>(
        &'a self,
        claim: &'a NodeAgentRemovalClaim,
        bootstrap: Uuid,
        secret: &'a str,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>>;
    fn service_applied<'a>(
        &'a self,
        claim: &'a NodeAgentRemovalClaim,
        spec: &'a SystemAgentSpec,
        id: &'a str,
        reference: &'a str,
        digest: &'a str,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>>;
    fn finish_setup<'a>(
        &'a self,
        claim: &'a NodeAgentRemovalClaim,
        kind: SetupKind,
        error: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>>;
    fn persist_inventory(
        &self,
        snapshot: RuntimeInventorySnapshot,
    ) -> BoxFuture<'_, Result<(), RuntimeCapabilityError>>;
    fn promote_manager<'a>(
        &'a self,
        claim: &'a NodeAgentRemovalClaim,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>>;
    fn task_bindings<'a>(
        &'a self,
        claim: &'a NodeAgentRemovalClaim,
        service: &'a str,
    ) -> BoxFuture<'a, Result<Vec<(String, String)>, RuntimeCapabilityError>>;
}
pub trait NodeAgentSetupRuntime: NodeAgentLifecycleRuntime {
    fn snapshot<'a>(
        &'a self,
        claim: &'a NodeAgentRemovalClaim,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeInventorySnapshot, RuntimeCapabilityError>>;
    fn distribution<'a>(
        &'a self,
        claim: &'a NodeAgentRemovalClaim,
        image: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<AgentDistribution, RuntimeCapabilityError>>;
    fn create_material<'a>(
        &'a self,
        claim: &'a NodeAgentRemovalClaim,
        kind: NodeAgentResource,
        name: &'a str,
        data: &'a [u8],
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<String, RuntimeCapabilityError>>;
    fn apply_system<'a>(
        &'a self,
        claim: &'a NodeAgentRemovalClaim,
        spec: &'a SystemAgentSpec,
        current: Option<&'a RuntimeSwarmService>,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<String, RuntimeCapabilityError>>;
    fn connected(&self, platform: Uuid, node: &str) -> bool;
}

pub struct NodeAgentSetupService {
    pub store: Arc<dyn NodeAgentSetupStore>,
    pub runtime: Arc<dyn NodeAgentSetupRuntime>,
    pub changed: Arc<dyn Fn(Uuid) + Send + Sync>,
}
impl NodeAgentSetupService {
    pub async fn run(
        &self,
        actor: ActorId,
        id: Uuid,
        kind: SetupKind,
        options: SetupOptions,
        sender: mpsc::Sender<NodeAgentProgress>,
        cancel: CancellationToken,
    ) {
        if cancel.is_cancelled() {
            return;
        }
        if let Err(error) = options.validate() {
            let _ = sender.try_send(item(
                id,
                Uuid::nil(),
                "failed",
                &error.message,
                true,
                Some(error.message.clone()),
            ));
            return;
        }
        let start = async {
            let info = self.runtime.info(id, &cancel).await?;
            self.store.claim_setup(actor, id, info, kind).await
        };
        let claim_result = tokio::select! {
            () = cancel.cancelled() => return,
            result = tokio::time::timeout(Duration::from_secs(30), start) => {
                result.unwrap_or_else(|_| Err(fail("Node-agent validation timed out.")))
            }
        };
        let claim = match claim_result {
            Ok(claim) => claim,
            Err(error) => {
                let _ = sender.try_send(item(
                    id,
                    Uuid::nil(),
                    "failed",
                    &error.message,
                    true,
                    Some(error.message.clone()),
                ));
                return;
            }
        };
        (self.changed)(id);
        let result = tokio::select! {
            () = cancel.cancelled() => Err(fail("Node-agent setup canceled. Repair coverage can reconcile partial state.")),
            result = tokio::time::timeout(Duration::from_secs(300), self.execute(&claim, &options, &sender, &cancel)) => {
                result.unwrap_or_else(|_| Err(fail("Node-agent setup timed out. Repair coverage after checking missing nodes.")))
            }
        };
        let mut error = result.err().map(|e| e.message);
        if !matches!(
            tokio::time::timeout(
                Duration::from_secs(15),
                self.store.finish_setup(&claim, kind, error.as_deref())
            )
            .await,
            Ok(Ok(()))
        ) {
            error = Some(
                "Could not persist node-agent completion. Retry after the operation lease expires."
                    .into(),
            );
        }
        (self.changed)(id);
        let message = error.as_deref().unwrap_or("Node-agent coverage is ready.");
        let _ = tokio::time::timeout(
            Duration::from_secs(5),
            sender.send(item(
                id,
                claim.operation_id,
                if error.is_some() {
                    "failed"
                } else {
                    "completed"
                },
                message,
                true,
                error.clone(),
            )),
        )
        .await;
    }
    async fn execute(
        &self,
        claim: &NodeAgentRemovalClaim,
        options: &SetupOptions,
        sender: &mpsc::Sender<NodeAgentProgress>,
        cancel: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        let snapshot = self.runtime.snapshot(claim, cancel).await?;
        let swarm = snapshot
            .swarm
            .as_ref()
            .ok_or_else(|| fail("Swarm inventory is unavailable."))?;
        let eligible: Vec<_> = swarm
            .nodes
            .iter()
            .filter(|n| {
                n.id != claim.manager_node_id
                    && n.status.eq_ignore_ascii_case("ready")
                    && n.availability.eq_ignore_ascii_case("active")
                    && n.operating_system.eq_ignore_ascii_case("linux")
                    && matches!(architecture(&n.architecture).as_str(), "amd64" | "arm64")
            })
            .map(|n| (n.id.clone(), architecture(&n.architecture)))
            .collect();
        // A down worker is not evidence of complete coverage.
        if eligible.is_empty()
            && swarm.nodes.iter().any(|n| {
                n.id != claim.manager_node_id && n.availability.eq_ignore_ascii_case("active")
            })
        {
            return Err(fail(
                "No active satellite can be scheduled. Check node status and supported architectures.",
            ));
        }
        let name = format!("citadel-node-agent-{}", claim.platform_id.simple());
        let current = swarm
            .services
            .iter()
            .find(|s| Some(&s.id) == claim.service_id.as_ref())
            .or_else(|| swarm.services.iter().find(|s| s.name == name))
            .cloned();
        if current.as_ref().is_some_and(|s| !owned(&s.labels, claim)) {
            return Err(fail(
                "The node-agent Service has conflicting ownership labels.",
            ));
        }
        let configs = swarm.configs.clone();
        let secrets = swarm.secrets.clone();
        self.store.persist_inventory(snapshot).await?;
        self.store.promote_manager(claim).await?;
        self.runtime.disconnect(
            claim.platform_id,
            std::slice::from_ref(&claim.manager_node_id),
        );
        if eligible.is_empty() {
            return Ok(());
        }
        let required: BTreeSet<_> = eligible.iter().map(|(_, a)| a.clone()).collect();
        let distribution = self
            .runtime
            .distribution(claim, &options.image, cancel)
            .await?;
        let pinned = pin_image(&options.image, &distribution, &required)?;
        let _ = sender.try_send(item(
            claim.platform_id,
            claim.operation_id,
            "image",
            "Resolved an immutable Agent image supporting the eligible nodes.",
            false,
            None,
        ));
        let mut bootstrap = self.store.bootstrap(claim).await?;
        let secret = self
            .runtime
            .create_material(
                claim,
                NodeAgentResource::Secret,
                &bootstrap.secret_name,
                &bootstrap.token,
                cancel,
            )
            .await;
        bootstrap.token.fill(0);
        let secret = secret?;
        self.store
            .secret_created(claim, bootstrap.id, &secret)
            .await?;
        let mut ca_id = None;
        let mut ca_name = None;
        if let Some(data) = &options.ca_bundle {
            // The operation ID makes creation recoverable by exact name/ownership on the next repair.
            let name = format!(
                "citadel-node-agent-{}-ca-{}",
                claim.platform_id.simple(),
                claim.operation_id.simple()
            );
            ca_id = Some(
                self.runtime
                    .create_material(claim, NodeAgentResource::Config, &name, data, cancel)
                    .await?,
            );
            ca_name = Some(name);
        }
        let spec = system_spec(
            claim,
            options,
            pinned,
            secret,
            bootstrap.secret_name,
            ca_id,
            ca_name,
            required,
        );
        let service_id = self
            .runtime
            .apply_system(claim, &spec, current.as_ref(), cancel)
            .await?;
        self.store
            .service_applied(
                claim,
                &spec,
                &service_id,
                &options.image,
                &distribution.digest,
            )
            .await?;
        let _ = sender.try_send(item(
            claim.platform_id,
            claim.operation_id,
            "service",
            "Docker accepted the node-agent Service. Waiting for satellite coverage.",
            false,
            None,
        ));
        loop {
            let snapshot = self.runtime.snapshot(claim, cancel).await?;
            if let Some(service) = snapshot
                .swarm
                .as_ref()
                .and_then(|s| s.services.iter().find(|s| s.id == service_id))
                && matches!(
                    service.update_state.to_ascii_lowercase().as_str(),
                    "paused" | "rollback_paused" | "rollback_completed"
                )
            {
                return Err(fail(
                    "The node-agent Service rollout failed. Check its Tasks and repair coverage.",
                ));
            }
            let bindings = self.store.task_bindings(claim, &service_id).await?;
            let rollout_ready = snapshot.swarm.as_ref().is_some_and(|swarm| {
                swarm.services.iter().any(|s| {
                    s.id == service_id
                        && (s.update_state.is_empty()
                            || s.update_state.eq_ignore_ascii_case("none")
                            || s.update_state.eq_ignore_ascii_case("completed"))
                }) && eligible.iter().all(|(node, _)| {
                    swarm.tasks.iter().any(|t| {
                        t.node_id == *node
                            && t.service_id == service_id
                            && t.desired_state.eq_ignore_ascii_case("running")
                            && t.state.eq_ignore_ascii_case("running")
                            && t.image == spec.image
                            && bindings
                                .iter()
                                .any(|(bound_node, task)| bound_node == node && task == &t.id)
                    })
                })
            });
            self.store.persist_inventory(snapshot).await?;
            if rollout_ready
                && eligible
                    .iter()
                    .all(|(node, _)| self.runtime.connected(claim.platform_id, node))
            {
                break;
            }
            tokio::select! {()=cancel.cancelled()=>return Err(fail("Node-agent setup canceled.")),()=tokio::time::sleep(Duration::from_secs(2))=>{}}
        }
        // Revocation is committed by finish_setup; cleanup never deletes active material.
        for secret in &secrets {
            if secret.id != spec.secret_id && owned(&secret.labels, claim) {
                self.cleanup(claim, NodeAgentResource::Secret, &secret.id, sender, cancel)
                    .await;
            }
        }
        for config in configs
            .iter()
            .filter(|c| Some(&c.id) != spec.ca_config_id.as_ref() && owned(&c.labels, claim))
        {
            self.cleanup(claim, NodeAgentResource::Config, &config.id, sender, cancel)
                .await;
        }
        if cancel.is_cancelled() {
            return Err(fail("Node-agent setup canceled."));
        }
        Ok(())
    }
    async fn cleanup(
        &self,
        claim: &NodeAgentRemovalClaim,
        kind: NodeAgentResource,
        id: &str,
        sender: &mpsc::Sender<NodeAgentProgress>,
        cancel: &CancellationToken,
    ) {
        if self
            .runtime
            .delete_owned(claim, kind, id, cancel)
            .await
            .is_err()
        {
            let mut warning = item(
                claim.platform_id,
                claim.operation_id,
                "cleanup",
                "Prior node-agent material could not be removed; review it after the rollout.",
                false,
                None,
            );
            warning.is_warning = true;
            let _ = sender.try_send(warning);
        }
    }
}
pub fn ownership(claim: &NodeAgentRemovalClaim) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("com.citadel.system".into(), "true".into()),
        ("com.citadel.system-role".into(), "swarm-node-agent".into()),
        (
            "com.citadel.platform-id".into(),
            claim.platform_id.to_string(),
        ),
        (
            "com.citadel.swarm-cluster-id".into(),
            claim.cluster_id.clone(),
        ),
    ])
}
fn owned(labels: &BTreeMap<String, String>, claim: &NodeAgentRemovalClaim) -> bool {
    ownership(claim)
        .iter()
        .all(|(k, v)| labels.get(k) == Some(v))
}
pub fn architecture(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "x86_64" | "x86-64" => "amd64".into(),
        "aarch64" => "arm64".into(),
        v => v.into(),
    }
}
pub fn pin_image(
    reference: &str,
    distribution: &AgentDistribution,
    required: &BTreeSet<String>,
) -> Result<String, RuntimeCapabilityError> {
    if !required.is_subset(&distribution.linux_architectures) {
        return Err(fail(
            "The Agent image does not support all eligible node architectures.",
        ));
    }
    pin_image_reference(reference, &distribution.digest)
}

/// Use the installed digest for helpers too: Swarm pulls digest references,
/// which need not leave the user's mutable tag in every worker's image store.
pub fn pin_image_reference(
    reference: &str,
    digest: &str,
) -> Result<String, RuntimeCapabilityError> {
    let hash = digest
        .strip_prefix("sha256:")
        .filter(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| fail("Agent registry returned an invalid immutable digest."))?;
    if reference
        .split_once('@')
        .is_some_and(|(_, pinned)| pinned != digest)
    {
        return Err(fail(
            "Agent registry digest does not match the configured pinned image.",
        ));
    }
    let repository = reference.split('@').next().unwrap();
    let repository = match repository.rsplit_once(':') {
        Some((repo, tag)) if !tag.contains('/') => repo,
        _ => repository,
    };
    if repository.is_empty() {
        return Err(fail("Agent image repository is empty."));
    }
    Ok(format!("{repository}@sha256:{hash}"))
}
#[allow(clippy::too_many_arguments)]
fn system_spec(
    claim: &NodeAgentRemovalClaim,
    options: &SetupOptions,
    image: String,
    secret_id: String,
    secret_name: String,
    ca_config_id: Option<String>,
    ca_config_name: Option<String>,
    architectures: BTreeSet<String>,
) -> SystemAgentSpec {
    let mut environment = vec![
        "CITADEL_AGENT_MODE=edge".into(),
        "CITADEL_EDGE_AGENT_PROFILE=swarm-node".into(),
        format!(
            "CITADEL_CORE_URL={}",
            options.core_url.trim_end_matches('/')
        ),
        "CITADEL_EDGE_BOOTSTRAP_FILE=/run/secrets/citadel-edge-bootstrap".into(),
        "CITADEL_EDGE_AGENT_KEY_PATH=/app/data/edge-agent.key".into(),
        "CITADEL_EDGE_IDENTITY_PATH=/app/data/edge-agent.identity.json".into(),
        format!("CITADEL_PLATFORM_ID={}", claim.platform_id),
        format!("CITADEL_SWARM_CLUSTER_ID={}", claim.cluster_id),
        "CITADEL_SWARM_SERVICE_ID={{.Service.ID}}".into(),
        "CITADEL_SWARM_TASK_ID={{.Task.ID}}".into(),
        "CITADEL_SWARM_NODE_ID={{.Node.ID}}".into(),
        "CITADEL_SWARM_NODE_HOSTNAME={{.Node.Hostname}}".into(),
    ];
    if ca_config_id.is_some() {
        environment.push("CITADEL_EDGE_CORE_CA_CERTIFICATE_PATH=/citadel-core-ca.crt".into());
    }
    SystemAgentSpec {
        name: format!("citadel-node-agent-{}", claim.platform_id.simple()),
        image,
        environment,
        manager_node_id: claim.manager_node_id.clone(),
        volume_name: format!("citadel_swarm_node_agent_{}", claim.platform_id.simple()),
        secret_id,
        secret_name,
        ca_config_id,
        ca_config_name,
        architectures: architectures.into_iter().collect(),
        labels: ownership(claim),
    }
}
fn item(
    platform_id: Uuid,
    operation_id: Uuid,
    stage: &'static str,
    message: &str,
    is_completed: bool,
    error_message: Option<String>,
) -> NodeAgentProgress {
    NodeAgentProgress {
        platform_id,
        operation_id,
        stage,
        message: message.into(),
        is_completed,
        is_warning: false,
        error_message,
    }
}
fn fail(message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(crate::RuntimeErrorKind::Conflict, message, false)
}

impl SetupOptions {
    pub fn validate(&self) -> Result<(), RuntimeCapabilityError> {
        let url = url::Url::parse(&self.core_url)
            .map_err(|_| fail("Core URL must be an absolute HTTP or HTTPS URL."))?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || match url.host() {
                Some(url::Host::Domain(host)) => {
                    host.trim_end_matches('.').eq_ignore_ascii_case("localhost")
                }
                Some(url::Host::Ipv4(ip)) => ip.is_loopback() || ip.is_unspecified(),
                Some(url::Host::Ipv6(ip)) => ip.is_loopback() || ip.is_unspecified(),
                None => true,
            }
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path() != "/"
            || self.core_url.len() > 2048
        {
            return Err(fail(
                "Core URL must not contain credentials, a path, query or fragment.",
            ));
        }
        if self.image.is_empty()
            || self.image.len() > 1024
            || self.image.chars().any(char::is_whitespace)
        {
            return Err(fail("The configured Agent image is invalid."));
        }
        if self
            .ca_bundle
            .as_ref()
            .is_some_and(|b| b.is_empty() || b.len() > 1024 * 1024)
        {
            return Err(fail("Core CA bundle must be between 1 byte and 1 MiB."));
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "setup_tests.rs"]
mod tests;
