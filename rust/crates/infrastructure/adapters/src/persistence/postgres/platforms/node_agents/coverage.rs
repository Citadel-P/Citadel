use crate::connectors::edge::EdgeRegistry;
use crate::connectors::edge::EdgeTarget;
use chrono::{DateTime, Utc};
use citadel_platforms::{
    PlatformDetails,
    node_agents::{NodeAgentOperation, NodeCoverageInput, SwarmNodeAgentCoverage},
};
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Called after Platform Read authorization. Loads one bounded membership set;
/// bindings and latest task states are joined, never fetched per node.
pub async fn read(
    pool: &PgPool,
    sessions: &EdgeRegistry,
    platform: &PlatformDetails,
) -> Result<SwarmNodeAgentCoverage, sqlx::Error> {
    let id = platform.id;
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;
    let installation = sqlx::query("SELECT * FROM swarmnodeagentinstallations WHERE platformid=$1")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
    let service_id: Option<String> = installation
        .as_ref()
        .map(|row| row.try_get("dockerserviceid"))
        .transpose()?
        .flatten();
    let rows = sqlx::query(r#"
SELECT n.*, b.id AS bindingid, b.protocolversion, b.lastheartbeatatutc,
       COALESCE(r.isstale,true) AS runtimestale,r.lastsuccessfulreconciliationat,r.stalesince,r.stalereason,
       t.state AS taskstate
FROM swarmnodeprojections n
LEFT JOIN LATERAL (
    SELECT id,protocolversion,lastheartbeatatutc FROM edgeagentbindings
    WHERE platformid=n.platformid AND dockernodeid=n.dockernodeid AND revokedatutc IS NULL
    ORDER BY createdatutc DESC LIMIT 1
) b ON true
LEFT JOIN swarmnoderuntimeprojectionstates r ON r.platformid=n.platformid AND r.dockernodeid=n.dockernodeid
LEFT JOIN LATERAL (
    SELECT state FROM swarmtaskprojections
    WHERE platformid=n.platformid AND dockernodeid=n.dockernodeid AND dockerserviceid=$2 AND NOT isstale
    ORDER BY COALESCE(dockerupdatedat,observedat) DESC,dockertaskid LIMIT 1
) t ON true
WHERE n.platformid=$1 ORDER BY n.dockernodeid LIMIT 10001
"#).bind(id).bind(&service_id).fetch_all(&mut *tx).await?;
    if rows.len() > 10_000 {
        return Err(sqlx::Error::Protocol(
            "Node-agent coverage exceeds the 10000-node limit.".into(),
        ));
    }
    let manager = platform
        .platform_descriptor
        .get("nodeID")
        .or_else(|| platform.platform_descriptor.get("NodeID"))
        .and_then(Value::as_str);
    let now = Utc::now();
    let mut observed = None;
    let mut nodes = Vec::with_capacity(rows.len());
    for row in rows {
        let node_id: String = row.try_get("dockernodeid")?;
        let manager_source = manager == Some(node_id.as_str());
        let binding: Option<Uuid> = row.try_get("bindingid")?;
        let connected = if manager_source {
            platform.status == citadel_primitives::PlatformStatus::Online
        } else {
            binding.is_some()
                && sessions
                    .get(&EdgeTarget::node(id, node_id.clone()))
                    .is_ok_and(|session| !session.is_closed())
        };
        let at: DateTime<Utc> = row.try_get("observedat")?;
        observed = Some(observed.map_or(at, |previous: DateTime<Utc>| previous.max(at)));
        nodes.push(
            NodeCoverageInput {
                docker_node_id: node_id,
                hostname: row.try_get("hostname")?,
                role: row.try_get("role")?,
                availability: row.try_get("availability")?,
                status: row.try_get("status")?,
                architecture: row.try_get("architecture")?,
                operating_system: row.try_get("operatingsystem")?,
                manager_source,
                membership_stale: row.try_get("isstale")?,
                runtime_stale: row.try_get("runtimestale")?,
                binding_present: binding.is_some(),
                connected,
                protocol_compatible: row
                    .try_get::<Option<i32>, _>("protocolversion")?
                    .is_none_or(|version| {
                        version == citadel_contracts::EDGE_AGENT_PROTOCOL_VERSION
                    }),
                task_state: row.try_get("taskstate")?,
                heartbeat: row.try_get("lastheartbeatatutc")?,
                reconciled_at: row.try_get("lastsuccessfulreconciliationat")?,
                stale_since: row.try_get("stalesince")?,
                stale_reason: row.try_get("stalereason")?,
            }
            .evaluate(now),
        );
    }
    let installed = installation
        .as_ref()
        .map(|row| row.try_get::<String, _>("desiredstate"))
        .transpose()?
        .as_deref()
        == Some("Installed");
    let mut drifted = false;
    let mut operation = None;
    if let Some(row) = &installation {
        let operation_id: Option<Uuid> = row.try_get("operationid")?;
        let kind: Option<String> = row.try_get("operationkind")?;
        let state: Option<String> = row.try_get("operationstate")?;
        let started: Option<DateTime<Utc>> = row.try_get("operationstartedatutc")?;
        if let (Some(operation_id), Some(kind), Some(state), Some(started_at_utc)) =
            (operation_id, kind, state, started)
        {
            // An expired claim is retryable. Do not leave the existing UI's
            // lifecycle actions disabled forever after a Core crash.
            let expired =
                state == "Running" && started_at_utc < now - chrono::Duration::minutes(30);
            operation = Some(NodeAgentOperation {
                operation_id,
                kind,
                state: if expired { "Failed".into() } else { state },
                started_at_utc,
                error: if expired {
                    Some(
                        "The node-agent operation expired. Retry to reconcile partial state."
                            .into(),
                    )
                } else {
                    row.try_get("operationerror")?
                },
            });
        }
        if installed
            && (service_id.is_some()
                || nodes
                    .iter()
                    .any(|node| node.eligible && node.data_source == "Satellite"))
        {
            // Include system Services here; the user-facing inventory excludes them.
            let service = sqlx::query("SELECT mode,image,labels,isstale FROM swarmserviceprojections WHERE platformid=$1 AND dockerserviceid=$2").bind(id).bind(&service_id).fetch_optional(&mut *tx).await?;
            drifted = match service {
                None => true,
                Some(service) => {
                    let labels: Value = service.try_get("labels")?;
                    let digest: String = row.try_get("agentimagedigest")?;
                    let image: String = service.try_get("image")?;
                    let mode: String = service.try_get("mode")?;
                    service.try_get::<bool, _>("isstale")?
                        || !mode.eq_ignore_ascii_case("global")
                        || labels["com.citadel.system"]
                            .as_str()
                            .is_none_or(|value| !value.eq_ignore_ascii_case("true"))
                        || labels["com.citadel.system-role"] != "swarm-node-agent"
                        || labels["com.citadel.platform-id"]
                            .as_str()
                            .and_then(|value| Uuid::parse_str(value).ok())
                            != Some(id)
                        || labels["com.citadel.swarm-cluster-id"].as_str()
                            != platform.cluster_id.as_deref()
                        || (!digest.is_empty()
                            && image
                                .rsplit_once('@')
                                .is_none_or(|(_, value)| !value.eq_ignore_ascii_case(&digest)))
                }
            };
        }
    }
    let mut result = SwarmNodeAgentCoverage::aggregate(nodes, installed, drifted, operation);
    result.last_membership_reconciliation_at_utc = observed;
    if let Some(row) = installation {
        result.agent_image_reference = Some(row.try_get("agentimagereference")?);
        result.agent_image_digest = Some(row.try_get("agentimagedigest")?);
    }
    result.enrollment_expires_at_utc = sqlx::query_scalar("SELECT max(expiresatutc) FROM swarmnodeagentbootstraps WHERE platformid=$1 AND revokedatutc IS NULL AND expiresatutc>now()").bind(id).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok(result)
}
