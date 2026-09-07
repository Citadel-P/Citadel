use super::*;
use crate::{RuntimeSwarmInventory, RuntimeSwarmNode, RuntimeSwarmTask};
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

fn options() -> SetupOptions {
    SetupOptions {
        core_url: "https://core.example.test".into(),
        image: "registry.example:5000/citadel/agent:latest".into(),
        ca_bundle: None,
    }
}
fn distribution() -> AgentDistribution {
    AgentDistribution {
        digest: format!("sha256:{}", "a".repeat(64)),
        linux_architectures: BTreeSet::from(["amd64".into(), "arm64".into()]),
    }
}
fn claim() -> NodeAgentRemovalClaim {
    NodeAgentRemovalClaim {
        platform_id: Uuid::now_v7(),
        platform_name: "fixture".into(),
        actor: ActorId::new(Uuid::now_v7()),
        operation_id: Uuid::now_v7(),
        cluster_id: "cluster".into(),
        manager_node_id: "manager".into(),
        manager_daemon_id: "daemon".into(),
        service_id: None,
        ca_config_id: None,
        secret_ids: vec![],
    }
}
#[test]
fn setup_rejects_unreachable_or_credential_bearing_core_urls() {
    for url in [
        "http://localhost",
        "http://localhost./",
        "http://127.0.0.2",
        "http://[::1]",
        "http://0.0.0.0",
        "https://user:password@core.test",
        "https://core.test/path",
        "https://core.test/?token=secret",
        "https://core.test/#fragment",
        "file:///tmp/core",
    ] {
        let mut options = options();
        options.core_url = url.into();
        assert!(options.validate().is_err(), "{url}");
    }
    assert!(options().validate().is_ok());
}
#[test]
fn image_pin_preserves_registry_port_and_requires_digest_and_architecture_coverage() {
    let mut d = distribution();
    let required = BTreeSet::from(["amd64".into(), "arm64".into()]);
    assert_eq!(
        pin_image(&options().image, &d, &required).unwrap(),
        format!("registry.example:5000/citadel/agent@{}", d.digest)
    );
    assert!(pin_image("agent@sha256:other", &d, &required).is_err());
    d.linux_architectures.remove("arm64");
    assert!(pin_image("agent", &d, &required).is_err());
    d.digest = "sha256:short".into();
    assert!(pin_image("agent", &d, &BTreeSet::new()).is_err());
    assert_eq!(architecture(" X86_64 "), "amd64");
    assert_eq!(architecture("AARCH64"), "arm64");
}
#[test]
fn system_identity_and_credentials_use_scoped_labels_and_file_mounts() {
    let c = claim();
    let spec = system_spec(
        &c,
        &options(),
        "agent@digest".into(),
        "secret-id".into(),
        "secret-name".into(),
        Some("ca-id".into()),
        Some("ca-name".into()),
        BTreeSet::from(["amd64".into()]),
    );
    assert_eq!(spec.labels.len(), 4);
    assert_eq!(
        spec.labels["com.citadel.platform-id"],
        c.platform_id.to_string()
    );
    assert_eq!(
        spec.volume_name,
        format!("citadel_swarm_node_agent_{}", c.platform_id.simple())
    );
    assert!(
        spec.environment
            .iter()
            .any(|e| e == "CITADEL_EDGE_BOOTSTRAP_FILE=/run/secrets/citadel-edge-bootstrap")
    );
    assert!(
        spec.environment
            .iter()
            .any(|e| e == "CITADEL_EDGE_CORE_CA_CERTIFICATE_PATH=/citadel-core-ca.crt")
    );
    assert!(!spec.environment.iter().any(|e| e.contains("TOKEN=")));
}

#[derive(Default)]
struct Fixture {
    calls: Mutex<Vec<String>>,
    image: Mutex<String>,
    snapshots: AtomicUsize,
    manager_only: bool,
    paused: bool,
    update_state: Option<&'static str>,
    stale_binding: bool,
    cancel_apply: bool,
    cancel_cleanup: bool,
}
impl Fixture {
    fn record(&self, call: impl Into<String>) {
        self.calls.lock().unwrap().push(call.into());
    }
}
impl NodeAgentLifecycleStore for Fixture {
    fn claim_remove(
        &self,
        _: ActorId,
        _: Uuid,
        _: RuntimePlatformInfo,
    ) -> BoxFuture<'_, Result<NodeAgentRemovalClaim, RuntimeCapabilityError>> {
        Box::pin(async { panic!("unexpected removal") })
    }
    fn revoke<'a>(
        &'a self,
        _: &'a NodeAgentRemovalClaim,
    ) -> BoxFuture<'a, Result<Vec<String>, RuntimeCapabilityError>> {
        Box::pin(async { panic!("unexpected revoke") })
    }
    fn finish<'a>(
        &'a self,
        _: &'a NodeAgentRemovalClaim,
        _: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async { panic!("unexpected removal completion") })
    }
}
impl NodeAgentSetupStore for Fixture {
    fn claim_setup(
        &self,
        _: ActorId,
        _: Uuid,
        _: RuntimePlatformInfo,
        _: SetupKind,
    ) -> BoxFuture<'_, Result<NodeAgentRemovalClaim, RuntimeCapabilityError>> {
        Box::pin(async {
            self.record("claim");
            Ok(claim())
        })
    }
    fn bootstrap<'a>(
        &'a self,
        _: &'a NodeAgentRemovalClaim,
    ) -> BoxFuture<'a, Result<Bootstrap, RuntimeCapabilityError>> {
        Box::pin(async {
            self.record("bootstrap");
            Ok(Bootstrap {
                id: Uuid::now_v7(),
                secret_name: "bootstrap".into(),
                token: zeroize::Zeroizing::new(b"sensitive-bootstrap".to_vec()),
            })
        })
    }
    fn secret_created<'a>(
        &'a self,
        _: &'a NodeAgentRemovalClaim,
        _: Uuid,
        _: &'a str,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async {
            self.record("secret-saved");
            Ok(())
        })
    }
    fn service_applied<'a>(
        &'a self,
        _: &'a NodeAgentRemovalClaim,
        _: &'a SystemAgentSpec,
        _: &'a str,
        _: &'a str,
        _: &'a str,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async {
            self.record("service-saved");
            Ok(())
        })
    }
    fn finish_setup<'a>(
        &'a self,
        _: &'a NodeAgentRemovalClaim,
        _: SetupKind,
        error: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            self.record(if error.is_some() {
                "finish:failed"
            } else {
                "finish:completed"
            });
            Ok(())
        })
    }
    fn persist_inventory(
        &self,
        _: RuntimeInventorySnapshot,
    ) -> BoxFuture<'_, Result<(), RuntimeCapabilityError>> {
        Box::pin(async {
            self.record("inventory");
            Ok(())
        })
    }
    fn promote_manager<'a>(
        &'a self,
        _: &'a NodeAgentRemovalClaim,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async {
            self.record("promote-manager");
            Ok(())
        })
    }
    fn task_bindings<'a>(
        &'a self,
        _: &'a NodeAgentRemovalClaim,
        _: &'a str,
    ) -> BoxFuture<'a, Result<Vec<(String, String)>, RuntimeCapabilityError>> {
        Box::pin(async {
            Ok(vec![(
                "worker".into(),
                if self.stale_binding {
                    "old-task"
                } else {
                    "task"
                }
                .into(),
            )])
        })
    }
}
impl NodeAgentLifecycleRuntime for Fixture {
    fn info<'a>(
        &'a self,
        _: Uuid,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimePlatformInfo, RuntimeCapabilityError>> {
        Box::pin(async { Ok(RuntimePlatformInfo::default()) })
    }
    fn delete_owned<'a>(
        &'a self,
        _: &'a NodeAgentRemovalClaim,
        _: NodeAgentResource,
        _: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            self.record("cleanup");
            if self.cancel_cleanup {
                cancel.cancel();
            }
            Ok(())
        })
    }
    fn disconnect(&self, _: Uuid, nodes: &[String]) {
        assert_eq!(nodes, ["manager"]);
        self.record("disconnect-manager-satellite");
    }
}
impl NodeAgentSetupRuntime for Fixture {
    fn snapshot<'a>(
        &'a self,
        c: &'a NodeAgentRemovalClaim,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeInventorySnapshot, RuntimeCapabilityError>> {
        Box::pin(async move {
            let iteration = self.snapshots.fetch_add(1, Ordering::SeqCst);
            let image = self.image.lock().unwrap().clone();
            let mut swarm = RuntimeSwarmInventory::default();
            if !self.manager_only {
                swarm.nodes.push(RuntimeSwarmNode {
                    id: "worker".into(),
                    status: "ready".into(),
                    availability: "active".into(),
                    operating_system: "linux".into(),
                    architecture: "amd64".into(),
                    ..Default::default()
                });
            }
            if self.cancel_cleanup {
                swarm.secrets.push(crate::RuntimeSwarmSecret {
                    id: "old-secret".into(),
                    labels: ownership(c),
                    ..Default::default()
                });
            }
            if iteration > 0 {
                swarm.services.push(RuntimeSwarmService {
                    id: "service".into(),
                    update_state: if self.paused {
                        "paused"
                    } else {
                        self.update_state.unwrap_or("completed")
                    }
                    .into(),
                    ..Default::default()
                });
                swarm.tasks.push(RuntimeSwarmTask {
                    id: "task".into(),
                    service_id: "service".into(),
                    node_id: "worker".into(),
                    state: "running".into(),
                    desired_state: "running".into(),
                    image,
                    ..Default::default()
                });
            }
            Ok(RuntimeInventorySnapshot {
                platform_id: c.platform_id,
                info: RuntimePlatformInfo::default(),
                containers: vec![],
                images: vec![],
                networks: vec![],
                volumes: vec![],
                swarm: Some(swarm),
                observed_at: chrono::Utc::now(),
            })
        })
    }
    fn distribution<'a>(
        &'a self,
        _: &'a NodeAgentRemovalClaim,
        _: &'a str,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<AgentDistribution, RuntimeCapabilityError>> {
        Box::pin(async {
            self.record("distribution");
            Ok(distribution())
        })
    }
    fn create_material<'a>(
        &'a self,
        _: &'a NodeAgentRemovalClaim,
        _: NodeAgentResource,
        _: &'a str,
        data: &'a [u8],
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<String, RuntimeCapabilityError>> {
        Box::pin(async move {
            assert_eq!(data, b"sensitive-bootstrap");
            self.record("secret-created");
            Ok("secret".into())
        })
    }
    fn apply_system<'a>(
        &'a self,
        _: &'a NodeAgentRemovalClaim,
        spec: &'a SystemAgentSpec,
        _: Option<&'a RuntimeSwarmService>,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<String, RuntimeCapabilityError>> {
        Box::pin(async move {
            self.record("apply");
            *self.image.lock().unwrap() = spec.image.clone();
            if self.cancel_apply {
                cancel.cancel();
                std::future::pending::<()>().await;
            }
            Ok("service".into())
        })
    }
    fn connected(&self, _: Uuid, _: &str) -> bool {
        true
    }
}
async fn run(f: Arc<Fixture>) -> Vec<NodeAgentProgress> {
    let (tx, mut rx) = mpsc::channel(32);
    NodeAgentSetupService {
        store: f.clone(),
        runtime: f,
        changed: Arc::new(|_| {}),
    }
    .run(
        ActorId::new(Uuid::now_v7()),
        Uuid::now_v7(),
        SetupKind::Install,
        options(),
        tx,
        CancellationToken::new(),
    )
    .await;
    let mut result = vec![];
    while let Some(p) = rx.recv().await {
        result.push(p);
    }
    result
}
#[tokio::test]
async fn manager_only_install_needs_no_bootstrap_or_system_service() {
    let f = Arc::new(Fixture {
        manager_only: true,
        ..Default::default()
    });
    let p = run(f.clone()).await;
    assert!(p.last().unwrap().error_message.is_none());
    assert!(
        !f.calls
            .lock()
            .unwrap()
            .iter()
            .any(|c| c == "bootstrap" || c == "apply")
    );
}
#[tokio::test]
async fn setup_persists_identity_before_waiting_for_current_task_coverage() {
    let f = Arc::new(Fixture::default());
    let p = run(f.clone()).await;
    assert!(p.last().unwrap().error_message.is_none());
    let calls = f.calls.lock().unwrap();
    assert!(
        calls.iter().position(|c| c == "service-saved").unwrap()
            < calls.iter().rposition(|c| c == "inventory").unwrap()
    );
    assert_eq!(calls.last().unwrap(), "finish:completed");
    assert!(p.iter().all(|p| !p.message.contains("sensitive-bootstrap")));
}
#[tokio::test(start_paused = true)]
async fn initial_install_accepts_both_docker_empty_and_agent_none_update_states() {
    for state in ["", "None", "none", "Completed"] {
        let fixture = Arc::new(Fixture {
            update_state: Some(state),
            ..Default::default()
        });
        let progress = run(fixture).await;
        assert_eq!(progress.last().unwrap().stage, "completed", "{state}");
    }
}

#[tokio::test]
async fn paused_rollout_and_cancellation_persist_failure() {
    for f in [
        Fixture {
            paused: true,
            ..Default::default()
        },
        Fixture {
            cancel_apply: true,
            ..Default::default()
        },
        Fixture {
            cancel_cleanup: true,
            ..Default::default()
        },
    ] {
        let f = Arc::new(f);
        let p = run(f.clone()).await;
        assert!(p.last().unwrap().error_message.is_some());
        assert_eq!(f.calls.lock().unwrap().last().unwrap(), "finish:failed");
    }
}
#[tokio::test(start_paused = true)]
async fn old_connected_task_cannot_complete_upgrade_and_times_out() {
    let f = Arc::new(Fixture {
        stale_binding: true,
        ..Default::default()
    });
    let p = run(f.clone()).await;
    assert!(
        p.last()
            .unwrap()
            .error_message
            .as_ref()
            .unwrap()
            .contains("timed out")
    );
    assert_eq!(f.calls.lock().unwrap().last().unwrap(), "finish:failed");
}
