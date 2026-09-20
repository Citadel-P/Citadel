use citadel_activities::{ActivityEvent, ActivityStatus};
use citadel_platforms::{
    RuntimeCapabilityError, RuntimeErrorKind, RuntimePlatformInfo,
    node_agents::lifecycle::{NodeAgentLifecycleStore, NodeAgentRemovalClaim},
};
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub struct PostgresNodeAgentLifecycleStore(pub PgPool);
mod setup;
impl PostgresNodeAgentLifecycleStore {
    async fn claim_operation(
        &self,
        actor: ActorId,
        id: Uuid,
        info: RuntimePlatformInfo,
        kind: &'static str,
    ) -> Result<NodeAgentRemovalClaim, RuntimeCapabilityError> {
        let mut tx = self.0.begin().await.map_err(storage)?;
        let row = sqlx::query(
            "SELECT name,clusterid,platformdescriptor,status FROM platforms WHERE id=$1 FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(storage)?
        .ok_or_else(|| invalid("Platform not found."))?;
        let descriptor: Value = row.try_get("platformdescriptor").map_err(storage)?;
        let cluster: Option<String> = row.try_get("clusterid").map_err(storage)?;
        let field = |camel, pascal| {
            descriptor
                .get(camel)
                .or_else(|| descriptor.get(pascal))
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
        };
        let node = field("nodeID", "NodeID");
        let daemon = field("daemonId", "DaemonId");
        if row.try_get::<String, _>("status").map_err(storage)? != "Online"
            || descriptor.get("$type").and_then(Value::as_str) != Some("DockerSwarm")
            || !info.swarm.as_ref().is_some_and(|swarm| {
                swarm.control_available
                    && swarm.local_node_state.eq_ignore_ascii_case("active")
                    && cluster
                        .as_deref()
                        .is_some_and(|id| !id.is_empty() && Some(id) == swarm.cluster_id.as_deref())
                    && node == Some(swarm.node_id.as_str())
            })
            || daemon != Some(info.daemon_id.as_str())
        {
            return Err(invalid(
                "The online, pinned Swarm manager identity must be validated before managing node agents.",
            ));
        }
        if kind == "Install" {
            let changed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM swarmnodeagentinstallations WHERE platformid=$1 AND desiredstate='Installed' AND (managerdockernodeid<>$2 OR managerdockerdaemonid<>$3))")
                    .bind(id).bind(node).bind(daemon).fetch_one(&mut *tx).await.map_err(storage)?;
            if changed {
                return Err(invalid(
                    "The selected manager changed. Use Repair coverage to hand over the data plane.",
                ));
            }
        }
        let operation_id = Uuid::now_v7();
        let saved = sqlx::query("INSERT INTO swarmnodeagentinstallations(platformid,clusterid,managerdockernodeid,managerdockerdaemonid,dockerservicename,agentimagereference,agentimagedigest,desiredstate,operationid,operationkind,operationstate,operationstartedatutc,operationactorid) VALUES($1,$2,$3,$4,$5,'','',CASE WHEN $8='Remove' THEN 'Removed' ELSE 'Installed' END,$6,$8,'Running',now(),$7) ON CONFLICT(platformid) DO UPDATE SET operationid=$6,operationkind=$8,operationstate='Running',operationstartedatutc=now(),operationactorid=$7,operationerror=NULL,updatedatutc=now(),managerdockernodeid=$3,managerdockerdaemonid=$4,desiredstate=CASE WHEN $8='Remove' THEN swarmnodeagentinstallations.desiredstate ELSE 'Installed' END WHERE swarmnodeagentinstallations.clusterid=$2 AND (swarmnodeagentinstallations.operationstate IS DISTINCT FROM 'Running' OR swarmnodeagentinstallations.operationstartedatutc<now()-interval '30 minutes') RETURNING dockerserviceid,dockercaconfigid")
                .bind(id).bind(cluster.as_deref()).bind(node).bind(daemon).bind(format!("citadel-node-agent-{}", id.simple())).bind(operation_id).bind(actor.value()).bind(kind)
                .fetch_optional(&mut *tx).await.map_err(storage)?.ok_or_else(|| invalid("A node-agent operation is already running, or its cluster identity changed."))?;
        let secret_ids = sqlx::query_scalar("SELECT DISTINCT dockersecretid FROM swarmnodeagentbootstraps WHERE platformid=$1 AND dockersecretid IS NOT NULL LIMIT 1001")
                .bind(id).fetch_all(&mut *tx).await.map_err(storage)?;
        if secret_ids.len() > 1000 {
            return Err(invalid(
                "Too many node-agent bootstrap Secrets to remove in one operation.",
            ));
        }
        let claim = NodeAgentRemovalClaim {
            platform_id: id,
            platform_name: row.try_get("name").map_err(storage)?,
            actor,
            operation_id,
            cluster_id: cluster.unwrap(),
            manager_node_id: node.unwrap().into(),
            manager_daemon_id: daemon.unwrap().into(),
            service_id: saved.try_get("dockerserviceid").map_err(storage)?,
            ca_config_id: saved.try_get("dockercaconfigid").map_err(storage)?,
            secret_ids,
        };
        audit_kind(
            &mut tx,
            &claim,
            ActivityStatus::Information,
            "Node-agent lifecycle operation requested.",
            kind,
        )
        .await?;
        tx.commit().await.map_err(storage)?;
        Ok(claim)
    }
}
impl NodeAgentLifecycleStore for PostgresNodeAgentLifecycleStore {
    fn claim_remove(
        &self,
        actor: ActorId,
        id: Uuid,
        info: RuntimePlatformInfo,
    ) -> BoxFuture<'_, Result<NodeAgentRemovalClaim, RuntimeCapabilityError>> {
        Box::pin(self.claim_operation(actor, id, info, "Remove"))
    }
    fn revoke<'a>(
        &'a self,
        claim: &'a NodeAgentRemovalClaim,
    ) -> BoxFuture<'a, Result<Vec<String>, RuntimeCapabilityError>> {
        Box::pin(async move {
            let mut tx = self.0.begin().await.map_err(storage)?;
            fence(&mut tx, claim).await?;
            sqlx::query("UPDATE swarmnodeagentbootstraps SET revokedatutc=COALESCE(revokedatutc,now()),updatedatutc=now() WHERE platformid=$1")
                .bind(claim.platform_id).execute(&mut *tx).await.map_err(storage)?;
            let nodes: Vec<String> = sqlx::query_scalar("UPDATE edgeagentbindings SET revokedatutc=now(),connectionstatus='Revoked',updatedatutc=now() WHERE platformid=$1 AND profile='SwarmNode' AND dockernodeid IS NOT NULL AND revokedatutc IS NULL RETURNING dockernodeid")
                .bind(claim.platform_id).fetch_all(&mut *tx).await.map_err(storage)?;
            sqlx::query("UPDATE swarmnoderuntimeprojectionstates SET isstale=true,stalesince=COALESCE(stalesince,now()),stalereason='Node-agent data plane was removed.' WHERE platformid=$1 AND dockernodeid=ANY($2)")
                .bind(claim.platform_id).bind(&nodes).execute(&mut *tx).await.map_err(storage)?;
            for statement in [
                "UPDATE swarmnodeimageprojections SET isstale=true WHERE platformid=$1 AND dockernodeid=ANY($2)",
                "UPDATE swarmnodevolumeprojections SET isstale=true WHERE platformid=$1 AND dockernodeid=ANY($2)",
                "UPDATE swarmnodenetworkprojections SET isstale=true WHERE platformid=$1 AND dockernodeid=ANY($2)",
            ] {
                sqlx::query(statement)
                    .bind(claim.platform_id)
                    .bind(&nodes)
                    .execute(&mut *tx)
                    .await
                    .map_err(storage)?;
            }
            sqlx::query("UPDATE containers SET projectionstalesince=COALESCE(projectionstalesince,EXTRACT(EPOCH FROM now())::bigint),projectionstalereason='Node-agent data plane was removed.' WHERE platformid=$1 AND dockernodeid=ANY($2)")
                .bind(claim.platform_id).bind(&nodes).execute(&mut *tx).await.map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            Ok(nodes)
        })
    }
    fn finish<'a>(
        &'a self,
        claim: &'a NodeAgentRemovalClaim,
        error: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            let mut tx = self.0.begin().await.map_err(storage)?;
            fence(&mut tx, claim).await?;
            sqlx::query("UPDATE swarmnodeagentinstallations SET operationstate=$3,operationerror=$4,updatedatutc=now(),desiredstate=CASE WHEN $4::text IS NULL THEN 'Removed' ELSE desiredstate END,dockerserviceid=CASE WHEN $4::text IS NULL THEN NULL ELSE dockerserviceid END,dockercaconfigid=CASE WHEN $4::text IS NULL THEN NULL ELSE dockercaconfigid END,dockercaconfigname=CASE WHEN $4::text IS NULL THEN NULL ELSE dockercaconfigname END WHERE platformid=$1 AND operationid=$2")
                .bind(claim.platform_id).bind(claim.operation_id).bind(if error.is_some(){"Failed"}else{"Completed"}).bind(error)
                .execute(&mut *tx).await.map_err(storage)?;
            audit(
                &mut tx,
                claim,
                if error.is_some() {
                    ActivityStatus::Failure
                } else {
                    ActivityStatus::Success
                },
                error.unwrap_or(
                    "Node agents removed; credentials revoked and state volumes preserved.",
                ),
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }
}
async fn fence(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    claim: &NodeAgentRemovalClaim,
) -> Result<(), RuntimeCapabilityError> {
    let current:Option<Uuid>=sqlx::query_scalar("SELECT operationid FROM swarmnodeagentinstallations WHERE platformid=$1 AND operationid=$2 AND operationstate='Running' FOR UPDATE")
        .bind(claim.platform_id).bind(claim.operation_id).fetch_optional(&mut **tx).await.map_err(storage)?;
    if current.is_none() {
        return Err(invalid("Node-agent operation no longer owns its claim."));
    }
    Ok(())
}
async fn audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    claim: &NodeAgentRemovalClaim,
    status: ActivityStatus,
    message: &str,
) -> Result<(), RuntimeCapabilityError> {
    audit_kind(tx, claim, status, message, "Remove").await
}
async fn audit_kind(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    claim: &NodeAgentRemovalClaim,
    status: ActivityStatus,
    message: &str,
    kind: &'static str,
) -> Result<(), RuntimeCapabilityError> {
    let event = ActivityEvent::node_agent_lifecycle(
        claim.platform_id,
        claim.platform_name.clone(),
        claim.actor,
        claim.operation_id,
        status,
        message.into(),
        chrono::Utc::now(),
        kind,
    )
    .map_err(storage)?;
    crate::persistence::postgres::activities::store::insert_activity(tx, &event)
        .await
        .map_err(storage)
}
fn invalid(message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Conflict, message, false)
}
fn storage(error: impl std::fmt::Display) -> RuntimeCapabilityError {
    tracing::error!(%error, "Node-agent lifecycle persistence failed");
    RuntimeCapabilityError::new(
        RuntimeErrorKind::Remote,
        "Could not persist node-agent lifecycle state.",
        false,
    )
}
