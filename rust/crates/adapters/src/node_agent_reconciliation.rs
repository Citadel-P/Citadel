//! Swarm node-agent credentials follow an authoritative cluster observation.
use citadel_platforms::{RuntimeInventorySnapshot, RuntimeSwarmInventory};
use sqlx::{Postgres, Row, Transaction};

#[derive(Clone, Debug)]
pub struct NodeAgentReconciliationPolicy {
    pub removal_grace: std::time::Duration,
    pub supported_architectures: Vec<String>,
}
impl Default for NodeAgentReconciliationPolicy {
    fn default() -> Self {
        Self {
            removal_grace: std::time::Duration::from_secs(600),
            supported_architectures: vec!["amd64".into(), "arm64".into()],
        }
    }
}
fn architecture(value: &str) -> &str {
    match value {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => other,
    }
}

pub(crate) async fn reconcile(
    tx: &mut Transaction<'_, Postgres>,
    snapshot: &RuntimeInventorySnapshot,
    swarm: &RuntimeSwarmInventory,
    policy: &NodeAgentReconciliationPolicy,
) -> Result<(), sqlx::Error> {
    let row=sqlx::query("SELECT * FROM swarmnodeagentinstallations WHERE platformid=$1 AND desiredstate='Installed' FOR UPDATE")
        .bind(snapshot.platform_id).fetch_optional(&mut **tx).await?;
    let Some(row) = row else { return Ok(()) };
    let Some(info) = snapshot.info.swarm.as_ref() else {
        return Ok(());
    };
    let cluster: String = row.try_get("clusterid")?;
    if info.cluster_id.as_deref() != Some(&cluster) {
        return Ok(());
    }
    let needs_satellites = swarm.nodes.iter().any(|node| {
        node.id != info.node_id
            && node.status.eq_ignore_ascii_case("ready")
            && node.availability.eq_ignore_ascii_case("active")
            && node.operating_system.eq_ignore_ascii_case("linux")
            && policy.supported_architectures.iter().any(|supported| {
                architecture(&supported.to_ascii_lowercase())
                    == architecture(&node.architecture.to_ascii_lowercase())
            })
    });
    if needs_satellites
        && row
            .try_get::<Option<String>, _>("operationstate")?
            .as_deref()
            != Some("Running")
    {
        let service_id: Option<String> = row.try_get("dockerserviceid")?;
        let digest: String = row.try_get("agentimagedigest")?;
        let valid = swarm
            .services
            .iter()
            .find(|service| Some(service.id.as_str()) == service_id.as_deref())
            .is_some_and(|service| {
                service.mode.eq_ignore_ascii_case("Global")
                    && (digest.trim().is_empty()
                        || service
                            .image
                            .rsplit_once('@')
                            .is_some_and(|(_, live)| live.eq_ignore_ascii_case(&digest)))
                    && service
                        .labels
                        .get("com.citadel.system")
                        .is_some_and(|value| value.eq_ignore_ascii_case("true"))
                    && service
                        .labels
                        .get("com.citadel.system-role")
                        .is_some_and(|value| value == "swarm-node-agent")
                    && service
                        .labels
                        .get("com.citadel.platform-id")
                        .and_then(|value| uuid::Uuid::parse_str(value).ok())
                        == Some(snapshot.platform_id)
                    && service.labels.get("com.citadel.swarm-cluster-id") == Some(&cluster)
            });
        if !valid {
            sqlx::query("UPDATE swarmnodeagentbootstraps SET revokedatutc=$2,updatedatutc=$2 WHERE platformid=$1 AND revokedatutc IS NULL")
                .bind(snapshot.platform_id).bind(snapshot.observed_at).execute(&mut **tx).await?;
        }
    }
    let current: Vec<_> = swarm.nodes.iter().map(|node| node.id.clone()).collect();
    // Include connection/disconnection and heartbeat time; a reconnect during
    // enumeration is not evidence that an absent node has exceeded grace.
    sqlx::query("UPDATE edgeagentbindings SET revokedatutc=$3,updatedatutc=$3,connectionstatus='Revoked',revocationreason='The Swarm node left the cluster beyond the configured removal grace period.' WHERE platformid=$1 AND profile='SwarmNode' AND revokedatutc IS NULL AND dockernodeid IS NOT NULL AND dockernodeid<>'' AND NOT(dockernodeid=ANY($2)) AND GREATEST(updatedatutc,lastconnectedatutc,lastdisconnectedatutc,lastheartbeatatutc)<=$3 - make_interval(secs=>$4)")
        .bind(snapshot.platform_id).bind(current).bind(snapshot.observed_at).bind(policy.removal_grace.as_secs_f64()).execute(&mut **tx).await?;
    Ok(())
}
