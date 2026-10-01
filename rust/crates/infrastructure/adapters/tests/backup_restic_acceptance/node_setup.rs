use super::*;
use citadel_adapters::connectors::docker::DockerClient;
use citadel_adapters::connectors::routing::node_agents::NodeAgentRuntimeRouter;
use citadel_adapters::persistence::postgres::platforms::node_agents::store::PostgresNodeAgentLifecycleStore;
use citadel_platforms::node_agents::setup::{NodeAgentSetupService, SetupKind, SetupOptions};

// Uses real Docker distribution, signed manager commands, PostgreSQL enrollment,
// and worker Edge sessions. No pre-created installation or bootstrap records.
pub(super) async fn verify(
    cluster: &Cluster,
    pool: &sqlx::PgPool,
    platform: Uuid,
    registry: &EdgeRegistry,
    agent: &AgentClient,
    core_url: &str,
    image_registry: &str,
) {
    let candidate = std::env::var("CITADEL_PHASE7_AGENT_IMAGE").unwrap();
    let first = format!("{image_registry}/citadel-agent:initial");
    cluster.node(0, &["tag", &candidate, &first], None).await;
    cluster.node(0, &["push", &first], None).await;
    let service = NodeAgentSetupService {
        store: Arc::new(PostgresNodeAgentLifecycleStore(pool.clone())),
        runtime: Arc::new(NodeAgentRuntimeRouter::new(
            pool.clone(),
            DockerClient::new("/no-core-daemon.sock", Duration::from_secs(2)).unwrap(),
            Some(agent.clone()),
            registry.clone(),
        )),
        changed: Arc::new(|_| {}),
    };
    let mut original_service = None;
    let mut original_digest = None;
    for kind in [SetupKind::Install, SetupKind::Repair, SetupKind::Upgrade] {
        let image = if matches!(kind, SetupKind::Upgrade) {
            let upgraded = format!("{image_registry}/citadel-agent:upgraded");
            let dockerfile = format!("FROM {first}\nLABEL citadel.acceptance.revision=upgraded\n");
            cluster
                .node(
                    0,
                    &["build", "--tag", &upgraded, "-"],
                    Some(dockerfile.as_bytes()),
                )
                .await;
            cluster.node(0, &["push", &upgraded], None).await;
            upgraded
        } else {
            first.clone()
        };
        let (sender, mut receiver) = tokio::sync::mpsc::channel(32);
        let run = service.run(
            ActorId::new(SYSTEM_ACTOR_ID),
            platform,
            kind,
            SetupOptions {
                policy: Default::default(),
                core_url: core_url.into(),
                image,
                ca_bundle: None,
            },
            sender,
            cluster.cancellation.child_token(),
        );
        let (_, progress) = tokio::join!(run, async {
            let mut progress = vec![];
            while let Some(item) = receiver.recv().await {
                progress.push(item);
            }
            progress
        });
        if progress.last().map(|p| p.stage) != Some("completed") {
            let logs = docker(&["logs", "--tail", "50", &cluster.manager_agent], None).await;
            eprintln!(
                "Manager Agent setup diagnostics: {}",
                String::from_utf8_lossy(&logs)
            );
        }
        assert_eq!(
            progress.last().map(|p| p.stage),
            Some("completed"),
            "{}: {:?}",
            kind.as_str(),
            progress
        );
        let (state, error, id, digest): (String, Option<String>, String, String) = sqlx::query_as(
            "SELECT operationstate,operationerror,dockerserviceid,agentimagedigest FROM swarmnodeagentinstallations WHERE platformid=$1")
            .bind(platform).fetch_one(pool).await.unwrap();
        assert_eq!(state, "Completed", "{error:?}");
        assert!(error.is_none());
        assert!(digest.starts_with("sha256:"));
        if let Some(original) = &original_service {
            assert_eq!(&id, original);
        }
        if matches!(kind, SetupKind::Upgrade) {
            assert_ne!(Some(&digest), original_digest.as_ref());
        } else if original_digest.is_none() {
            original_digest = Some(digest);
        }
        original_service = Some(id);
    }
    let bootstraps: i64 = sqlx::query_scalar("SELECT count(*) FROM swarmnodeagentbootstraps WHERE platformid=$1 AND revokedatutc IS NULL")
        .bind(platform).fetch_one(pool).await.unwrap();
    assert_eq!(
        bootstraps, 0,
        "Completed setup must revoke every bootstrap; enrolled Agents reconnect with their keys"
    );
}
