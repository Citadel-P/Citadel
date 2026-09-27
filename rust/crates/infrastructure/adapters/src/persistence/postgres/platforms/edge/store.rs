use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use chrono::{DateTime, Utc};
use citadel_contracts::citadel::edge::v1::{AgentHello, EnrollmentRequest};
use citadel_platforms::jobs::{ProjectionKind, ProjectionWrite, SnapshotGeneration};
use ed25519_dalek::VerifyingKey;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::connectors::edge::EdgeTarget;

#[derive(serde::Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct EdgeStatus {
    pub connection_status: String,
    pub last_connected_at_utc: Option<DateTime<Utc>>,
    pub last_disconnected_at_utc: Option<DateTime<Utc>>,
    pub last_heartbeat_at_utc: Option<DateTime<Utc>>,
    pub last_seen_version: Option<String>,
    pub last_seen_hostname: Option<String>,
    pub agent_fingerprint: Option<String>,
    pub protocol_version: Option<i32>,
    pub revoked_at_utc: Option<DateTime<Utc>>,
    pub enrollment_expires_at_utc: Option<DateTime<Utc>>,
}

#[derive(Debug, thiserror::Error)]
pub enum EdgeStoreError {
    #[error("Edge Agent resource was not found.")]
    NotFound,
    #[error("Edge Agent identity or enrollment is invalid, expired, revoked, or already in use.")]
    Unauthorized,
    #[error("{0}")]
    Invalid(&'static str),
    #[error("Edge Agent persistence failed.")]
    Storage(#[from] sqlx::Error),
}

#[derive(Clone)]
pub struct EdgeBinding {
    pub target: EdgeTarget,
    pub agent_id: Uuid,
    pub public_key: VerifyingKey,
    pub daemon_id: String,
    pub cluster_id: Option<String>,
    node_reconnect: Option<NodeReconnect>,
}

#[derive(Clone)]
struct NodeReconnect {
    previous_node_id: String,
    hostname: String,
    role: String,
    service_id: String,
    task_id: String,
}

#[derive(Clone)]
pub struct PostgresEdgeStore {
    pool: PgPool,
    node_policy: crate::persistence::postgres::platforms::node_agents::reconciliation::NodeAgentReconciliationPolicy,
}
impl PostgresEdgeStore {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            node_policy: Default::default(),
        }
    }
    pub fn with_node_policy(
        mut self,
        policy: crate::persistence::postgres::platforms::node_agents::reconciliation::NodeAgentReconciliationPolicy,
    ) -> Self {
        self.node_policy = policy;
        self
    }

    pub async fn status(&self, target: &EdgeTarget) -> Result<EdgeStatus, EdgeStoreError> {
        let mut tx = self.pool.begin().await?;
        validate_target(&mut tx, target).await?;
        let result = sqlx::query_as("SELECT COALESCE(CASE WHEN b.revokedatutc IS NOT NULL THEN 'Revoked' WHEN b.lastheartbeatatutc < now()-interval '90 seconds' THEN 'Offline' ELSE b.connectionstatus END,'PendingEnrollment') AS connection_status, b.lastconnectedatutc AS last_connected_at_utc,b.lastdisconnectedatutc AS last_disconnected_at_utc,b.lastheartbeatatutc AS last_heartbeat_at_utc,b.lastseenversion AS last_seen_version,b.lastseenhostname AS last_seen_hostname,left(b.agentfingerprint,20) AS agent_fingerprint,b.protocolversion AS protocol_version,b.revokedatutc AS revoked_at_utc,(SELECT max(expiresatutc) FROM edgeagentenrollments WHERE resourceid=$1 AND resourcetype=$2 AND usedatutc IS NULL AND revokedatutc IS NULL AND expiresatutc>now()) AS enrollment_expires_at_utc FROM (SELECT 1) seed LEFT JOIN LATERAL (SELECT * FROM edgeagentbindings WHERE resourceid=$1 AND resourcetype=$2 AND dockernodeid IS NULL ORDER BY createdatutc DESC LIMIT 1) b ON true")
            .bind(target.resource_id).bind(resource_type(target)).fetch_one(&mut *tx).await?;
        tx.commit().await?;
        Ok(result)
    }

    pub async fn create_enrollment(
        &self,
        target: &EdgeTarget,
        actor_id: Uuid,
    ) -> Result<(Uuid, String, DateTime<Utc>), EdgeStoreError> {
        let mut transaction = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext('citadel-platform-registration'))")
            .execute(&mut *transaction)
            .await?;
        validate_target(&mut transaction, target).await?;
        let revoked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM edgeagentbindings WHERE resourceid=$1 AND resourcetype=$2 AND dockernodeid IS NULL AND revokedatutc IS NOT NULL)")
            .bind(target.resource_id).bind(resource_type(target)).fetch_one(&mut *transaction).await?;
        if revoked {
            return Err(EdgeStoreError::Unauthorized);
        }
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes)
            .map_err(|_| EdgeStoreError::Invalid("Could not generate enrollment credential."))?;
        let token = URL_SAFE_NO_PAD.encode(bytes);
        let id = Uuid::now_v7();
        let expires = Utc::now() + chrono::Duration::hours(24);
        sqlx::query("INSERT INTO edgeagentenrollments(id,platformid,resourceid,resourcetype,tokenhash,expiresatutc,createdbyactorid) VALUES($1,$2,$3,$4,$5,$6,$7)")
            .bind(id).bind(target.platform_id).bind(target.resource_id).bind(resource_type(target))
            .bind(token_hash(&token)).bind(expires).bind(actor_id).execute(&mut *transaction).await?;
        transaction.commit().await?;
        Ok((id, token, expires))
    }

    pub async fn enroll(&self, request: &EnrollmentRequest) -> Result<EdgeBinding, EdgeStoreError> {
        validate_capabilities(
            request.protocol_version,
            &request.capabilities_json,
            request.profile,
            false,
        )?;
        let key: [u8; 32] = request
            .public_key
            .as_slice()
            .try_into()
            .map_err(|_| EdgeStoreError::Unauthorized)?;
        let public_key =
            VerifyingKey::from_bytes(&key).map_err(|_| EdgeStoreError::Unauthorized)?;
        if public_key.is_weak()
            || request.enrollment_token.len() > 512
            || request.daemon_id.trim().is_empty()
        {
            return Err(EdgeStoreError::Unauthorized);
        }
        let mut transaction = self.pool.begin().await?;
        // Same lock as Platform registration: engine ownership must be atomic
        // across Local, Agent and Edge registration, not just Edge bindings.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext('citadel-platform-registration'))")
            .execute(&mut *transaction)
            .await?;
        let target = if request.profile == 1 {
            let bootstrap = sqlx::query("SELECT platformid,clusterid FROM swarmnodeagentbootstraps WHERE tokenhash=$1 AND revokedatutc IS NULL AND expiresatutc>now() FOR UPDATE")
                .bind(token_hash(&request.enrollment_token)).fetch_optional(&mut *transaction).await?.ok_or(EdgeStoreError::Unauthorized)?;
            if bootstrap.try_get::<String, _>("clusterid")? != request.cluster_id {
                return Err(EdgeStoreError::Unauthorized);
            }
            let target =
                EdgeTarget::node(bootstrap.try_get("platformid")?, request.node_id.clone());
            validate_node(
                &mut transaction,
                &target,
                &request.cluster_id,
                &request.docker_hostname,
                &request.swarm_role,
                &request.service_id,
                &request.task_id,
            )
            .await?;
            target
        } else {
            let enrollment = sqlx::query("UPDATE edgeagentenrollments SET usedatutc=now() WHERE tokenhash=$1 AND usedatutc IS NULL AND revokedatutc IS NULL AND expiresatutc>now() RETURNING platformid,resourceid,resourcetype")
                .bind(token_hash(&request.enrollment_token)).fetch_optional(&mut *transaction).await?.ok_or(EdgeStoreError::Unauthorized)?;
            let resource: String = enrollment.try_get("resourcetype")?;
            let target = match resource.as_str() {
                "Platform" => EdgeTarget::platform(enrollment.try_get("resourceid")?),
                "BuildAgentPool" => EdgeTarget::build_pool(enrollment.try_get("resourceid")?),
                _ => return Err(EdgeStoreError::Unauthorized),
            };
            validate_target(&mut transaction, &target).await?;
            validate_capabilities(
                request.protocol_version,
                &request.capabilities_json,
                request.profile,
                target.resource_type == 1,
            )?;
            if target.resource_type == 0 {
                validate_daemon(&mut transaction, &target, &request.daemon_id).await?;
            }
            target
        };
        let agent_id = Uuid::now_v7();
        let inserted = sqlx::query("INSERT INTO edgeagentbindings(id,platformid,resourceid,resourcetype,agentid,agentpublickey,agentfingerprint,connectionstatus,protocolversion,profile,dockernodeid,dockerdaemonid,clusterid,lastseenhostname,lastseenversion,capabilitiesjson,firstenrolledatutc,dockerhostname,swarmrole,lastobservedserviceid,lastobservedtaskid) VALUES($1,$2,$3,$4,$5,$6,$7,'Offline',$19,$8,$9,$10,$11,$12,$13,$14,now(),$15,$16,$17,$18) ON CONFLICT DO NOTHING")
            .bind(Uuid::now_v7()).bind(target.platform_id).bind(target.resource_id).bind(resource_type(&target))
            .bind(agent_id).bind(STANDARD.encode(key)).bind(fingerprint(&key))
            .bind(if target.node_id.is_some() { "SwarmNode" } else { "Ordinary" })
            .bind(&target.node_id).bind(&request.daemon_id).bind(nonempty(&request.cluster_id))
            .bind(&request.hostname).bind(&request.agent_version).bind(serde_json::from_str::<Value>(&request.capabilities_json).map_err(|_| EdgeStoreError::Unauthorized)?)
            .bind(nonempty(&request.docker_hostname)).bind(nonempty(&request.swarm_role))
            .bind(nonempty(&request.service_id)).bind(nonempty(&request.task_id))
            .bind(request.protocol_version)
            .execute(&mut *transaction).await?.rows_affected();
        if inserted != 1 {
            return Err(EdgeStoreError::Unauthorized);
        }
        transaction.commit().await?;
        Ok(EdgeBinding {
            target,
            agent_id,
            public_key,
            daemon_id: request.daemon_id.clone(),
            cluster_id: nonempty(&request.cluster_id).map(str::to_owned),
            node_reconnect: None,
        })
    }

    pub async fn reconnect(&self, hello: &AgentHello) -> Result<EdgeBinding, EdgeStoreError> {
        validate_capabilities(
            hello.protocol_version,
            &hello.capabilities_json,
            hello.profile,
            hello.resource_type == 1,
        )?;
        let resource_id = Uuid::parse_str(if hello.resource_id.is_empty() {
            &hello.platform_id
        } else {
            &hello.resource_id
        })
        .map_err(|_| EdgeStoreError::Unauthorized)?;
        let platform_id =
            Uuid::parse_str(&hello.platform_id).map_err(|_| EdgeStoreError::Unauthorized)?;
        let agent_id =
            Uuid::parse_str(&hello.agent_id).map_err(|_| EdgeStoreError::Unauthorized)?;
        let target = match (hello.resource_type, hello.profile) {
            (0, 0) if platform_id == resource_id => EdgeTarget::platform(resource_id),
            (0, 1) if platform_id == resource_id && !hello.node_id.is_empty() => {
                EdgeTarget::node(resource_id, hello.node_id.clone())
            }
            (1, 0) if platform_id.is_nil() => EdgeTarget::build_pool(resource_id),
            _ => return Err(EdgeStoreError::Unauthorized),
        };
        let mut transaction = self.pool.begin().await?;
        let row = sqlx::query("SELECT agentpublickey,agentfingerprint,dockerdaemonid,clusterid,dockernodeid,profile FROM edgeagentbindings WHERE resourceid=$1 AND resourcetype=$2 AND agentid=$3 AND revokedatutc IS NULL")
            .bind(resource_id).bind(resource_type(&target)).bind(agent_id)
            .fetch_optional(&mut *transaction).await?.ok_or(EdgeStoreError::Unauthorized)?;
        let previous_node: Option<String> = row.try_get("dockernodeid")?;
        if previous_node.is_some() != target.node_id.is_some()
            || (target.node_id.is_some() && row.try_get::<String, _>("profile")? != "SwarmNode")
        {
            return Err(EdgeStoreError::Unauthorized);
        }
        let node_reconnect = previous_node.map(|previous_node_id| NodeReconnect {
            previous_node_id,
            hostname: hello.docker_hostname.clone(),
            role: hello.swarm_role.clone(),
            service_id: hello.service_id.clone(),
            task_id: hello.task_id.clone(),
        });
        if row.try_get::<String, _>("agentfingerprint")? != hello.agent_fingerprint
            || row
                .try_get::<Option<String>, _>("dockerdaemonid")?
                .as_deref()
                != Some(&hello.daemon_id)
        {
            return Err(EdgeStoreError::Unauthorized);
        }
        if target.node_id.is_some() {
            if row.try_get::<Option<String>, _>("clusterid")?.as_deref() != Some(&hello.cluster_id)
            {
                return Err(EdgeStoreError::Unauthorized);
            }
            validate_node(
                &mut transaction,
                &target,
                &hello.cluster_id,
                &hello.docker_hostname,
                &hello.swarm_role,
                &hello.service_id,
                &hello.task_id,
            )
            .await?;
            if let Some(rebind) = &node_reconnect
                && target.node_id.as_ref() != Some(&rebind.previous_node_id)
            {
                validate_rebind(
                    &mut transaction,
                    &target,
                    agent_id,
                    &rebind.previous_node_id,
                )
                .await?;
            }
        } else {
            validate_target(&mut transaction, &target).await?;
            if target.resource_type == 0 {
                validate_daemon(&mut transaction, &target, &hello.daemon_id).await?;
            }
        }
        let bytes = STANDARD
            .decode(row.try_get::<String, _>("agentpublickey")?)
            .map_err(|_| EdgeStoreError::Unauthorized)?;
        let key = bytes.try_into().map_err(|_| EdgeStoreError::Unauthorized)?;
        let public_key =
            VerifyingKey::from_bytes(&key).map_err(|_| EdgeStoreError::Unauthorized)?;
        transaction.commit().await?;
        Ok(EdgeBinding {
            target,
            agent_id,
            public_key,
            daemon_id: hello.daemon_id.clone(),
            cluster_id: nonempty(&hello.cluster_id).map(str::to_owned),
            node_reconnect,
        })
    }

    pub async fn connected(
        &self,
        binding: &EdgeBinding,
        at: DateTime<Utc>,
    ) -> Result<(), EdgeStoreError> {
        let mut tx = self.pool.begin().await?;
        // Record the current Task only after signature proof, including a rollout
        // on the same Node. Recheck membership under the binding row lock.
        if let Some(rebind) = &binding.node_reconnect {
            sqlx::query("SELECT id FROM edgeagentbindings WHERE agentid=$1 AND revokedatutc IS NULL FOR UPDATE")
                .bind(binding.agent_id).fetch_optional(&mut *tx).await?.ok_or(EdgeStoreError::Unauthorized)?;
            let changed_node = binding.target.node_id.as_ref() != Some(&rebind.previous_node_id);
            if changed_node {
                validate_rebind(
                    &mut tx,
                    &binding.target,
                    binding.agent_id,
                    &rebind.previous_node_id,
                )
                .await?;
            }
            validate_node(
                &mut tx,
                &binding.target,
                binding
                    .cluster_id
                    .as_deref()
                    .ok_or(EdgeStoreError::Unauthorized)?,
                &rebind.hostname,
                &rebind.role,
                &rebind.service_id,
                &rebind.task_id,
            )
            .await?;
            let changed = sqlx::query("UPDATE edgeagentbindings SET dockernodeid=$3,dockerhostname=$4,swarmrole=$5,lastobservedserviceid=$6,lastobservedtaskid=$7 WHERE agentid=$1 AND dockernodeid=$2 AND revokedatutc IS NULL")
                .bind(binding.agent_id).bind(&rebind.previous_node_id).bind(&binding.target.node_id)
                .bind(&rebind.hostname).bind(&rebind.role).bind(&rebind.service_id).bind(&rebind.task_id)
                .execute(&mut *tx).await?.rows_affected();
            if changed != 1 {
                return Err(EdgeStoreError::Unauthorized);
            }
            if changed_node {
                mark_node_stale(
                    &mut tx,
                    binding.target.platform_id,
                    &rebind.previous_node_id,
                )
                .await?;
            }
        }
        let updated = sqlx::query("UPDATE edgeagentbindings SET connectionstatus='Connected',lastconnectedatutc=$2,lastauthenticatedatutc=$2,lastheartbeatatutc=$2,updatedatutc=now() WHERE agentid=$1 AND dockernodeid IS NOT DISTINCT FROM $3 AND revokedatutc IS NULL AND (lastconnectedatutc IS NULL OR lastconnectedatutc<=$2)")
            .bind(binding.agent_id).bind(at).bind(&binding.target.node_id).execute(&mut *tx).await?.rows_affected();
        if updated != 1 {
            return Err(EdgeStoreError::Unauthorized);
        }
        tx.commit().await?;
        Ok(())
    }
    pub async fn disconnected(
        &self,
        agent_id: Uuid,
        connected_at: DateTime<Utc>,
    ) -> Result<(), EdgeStoreError> {
        let mut tx = self.pool.begin().await?;
        let changed: Option<(Uuid, Option<String>, String)> = sqlx::query_as("UPDATE edgeagentbindings SET connectionstatus='Offline',lastdisconnectedatutc=now(),updatedatutc=now() WHERE agentid=$1 AND lastconnectedatutc=$2 AND revokedatutc IS NULL RETURNING platformid,dockernodeid,resourcetype")
            .bind(agent_id).bind(connected_at).fetch_optional(&mut *tx).await?;
        let offline_platform = changed.as_ref().and_then(|(platform, node, kind)| {
            (kind == "Platform" && node.is_none()).then_some(*platform)
        });
        if let Some((platform, node, resource_type)) = changed
            && resource_type == "Platform"
        {
            if let Some(node) = node {
                mark_node_stale(&mut tx, platform, &node).await?;
            } else {
                crate::persistence::postgres::platforms::status::platform_offline_in(
                    &mut tx, platform,
                )
                .await?;
            }
        }
        tx.commit().await?;
        if let Some(platform) = offline_platform {
            crate::persistence::postgres::platforms::status::reconcile_deployments(
                &self.pool, platform, None, false,
            )
            .await?;
        }
        Ok(())
    }
    pub async fn persist_inventory(
        &self,
        session: &crate::connectors::edge::EdgeSession,
        snapshot: &citadel_platforms::RuntimeInventorySnapshot,
    ) -> Result<(), EdgeStoreError> {
        let containers = ProjectionWrite::begin(
            snapshot.platform_id,
            session.target.node_id.as_deref(),
            ProjectionKind::Containers,
        )
        .await;
        let images = ProjectionWrite::begin(
            snapshot.platform_id,
            session.target.node_id.as_deref(),
            ProjectionKind::Images,
        )
        .await;
        let mut tx = self.pool.begin().await?;
        if snapshot.platform_id != session.target.platform_id {
            return Err(EdgeStoreError::Unauthorized);
        }
        let daemon: Option<String>=sqlx::query_scalar("SELECT dockerdaemonid FROM edgeagentbindings WHERE agentid=$1 AND platformid=$2 AND lastconnectedatutc=$3 AND revokedatutc IS NULL AND connectionstatus='Connected' AND dockernodeid IS NOT DISTINCT FROM $4 AND resourcetype='Platform' FOR SHARE")
            .bind(session.agent_id).bind(snapshot.platform_id).bind(session.connected_at).bind(&session.target.node_id).fetch_optional(&mut *tx).await?;
        if session.is_closed() || daemon.as_deref() != Some(snapshot.info.daemon_id.as_str()) {
            return Err(EdgeStoreError::Unauthorized);
        }
        if let Some(node_id) = &session.target.node_id {
            if snapshot
                .info
                .swarm
                .as_ref()
                .is_none_or(|swarm| swarm.node_id != *node_id)
            {
                return Err(EdgeStoreError::Unauthorized);
            }
            crate::persistence::postgres::platforms::nodes::store::persist(
                &mut tx, snapshot, node_id,
            )
            .await
            .map_err(|_| EdgeStoreError::Invalid("Node inventory persistence failed."))?;
        } else {
            crate::persistence::postgres::platforms::inventory::store::validate_snapshot_identity(
                &mut tx, snapshot,
            )
            .await
            .map_err(|_| EdgeStoreError::Invalid("Edge inventory identity validation failed."))?;
            crate::persistence::postgres::platforms::inventory::store::persist_snapshot(
                &mut tx,
                snapshot,
                Some(&self.node_policy),
            )
            .await
            .map_err(|_| EdgeStoreError::Invalid("Edge inventory persistence failed."))?;
        }
        let identities = crate::persistence::postgres::platforms::runtime_index::stage_scope(
            &self.pool,
            &mut tx,
            snapshot.platform_id,
            session.target.node_id.as_deref(),
        )
        .await?;
        tx.commit().await?;
        if let Some(identities) = identities {
            identities.committed();
        }
        containers.committed();
        images.committed();
        crate::persistence::postgres::platforms::status::reconcile_deployments(
            &self.pool,
            snapshot.platform_id,
            None,
            false,
        )
        .await?;
        Ok(())
    }
    /// Mark node recovery complete only after every requested resource committed.
    pub async fn complete_resource_recovery(
        &self,
        session: &crate::connectors::edge::EdgeSession,
        started: DateTime<Utc>,
    ) -> Result<citadel_platforms::jobs::ProjectionChange, EdgeStoreError> {
        let mut tx = self.pool.begin().await?;
        let current: Option<Uuid> = sqlx::query_scalar("SELECT agentid FROM edgeagentbindings WHERE agentid=$1 AND platformid=$2 AND lastconnectedatutc=$3 AND revokedatutc IS NULL AND connectionstatus='Connected' AND dockernodeid IS NOT DISTINCT FROM $4 FOR SHARE")
            .bind(session.agent_id).bind(session.target.platform_id).bind(session.connected_at)
            .bind(&session.target.node_id).fetch_optional(&mut *tx).await?;
        if session.is_closed() || current.is_none() {
            return Err(EdgeStoreError::Unauthorized);
        }
        let changed = sqlx::query("UPDATE swarmnoderuntimeprojectionstates SET reconciliationcompletedat=now(),lastsuccessfulreconciliationat=now(),isstale=false,stalesince=NULL,stalereason=NULL WHERE platformid=$1 AND dockernodeid=$2 AND reconciliationstartedat=$3 AND isstale")
            .bind(session.target.platform_id).bind(&session.target.node_id).bind(started)
            .execute(&mut *tx).await?.rows_affected() > 0;
        tx.commit().await?;
        Ok(citadel_platforms::jobs::ProjectionChange::committed(
            changed,
        ))
    }

    /// A scoped event refresh uses the same revocation/session fence as deltas.
    pub async fn persist_resource(
        &self,
        session: &crate::connectors::edge::EdgeSession,
        snapshot: &citadel_platforms::jobs::ResourceSnapshot,
    ) -> Result<(), EdgeStoreError> {
        self.persist_resource_checked(session, snapshot, None)
            .await
            .map(|_| ())
    }

    pub async fn persist_resource_checked(
        &self,
        session: &crate::connectors::edge::EdgeSession,
        snapshot: &citadel_platforms::jobs::ResourceSnapshot,
        generation: Option<&SnapshotGeneration>,
    ) -> Result<bool, EdgeStoreError> {
        Ok(self
            .persist_resource_committed(session, snapshot, generation)
            .await?
            .accepted())
    }
    pub async fn persist_resource_committed(
        &self,
        session: &crate::connectors::edge::EdgeSession,
        snapshot: &citadel_platforms::jobs::ResourceSnapshot,
        generation: Option<&SnapshotGeneration>,
    ) -> Result<citadel_platforms::jobs::ProjectionChange, EdgeStoreError> {
        if snapshot.platform_id != session.target.platform_id {
            return Err(EdgeStoreError::Unauthorized);
        }
        let write = match snapshot.inventory.projection_kind() {
            Some(kind) => Some(
                ProjectionWrite::begin(
                    snapshot.platform_id,
                    session.target.node_id.as_deref(),
                    kind,
                )
                .await,
            ),
            None => None,
        };
        if generation.is_some_and(|stamp| write.as_ref().is_none_or(|write| !stamp.matches(write)))
        {
            return Ok(citadel_platforms::jobs::ProjectionChange::Unavailable);
        }
        let mut tx = self.pool.begin().await?;
        let current: Option<Uuid> = sqlx::query_scalar("SELECT agentid FROM edgeagentbindings WHERE agentid=$1 AND platformid=$2 AND lastconnectedatutc=$3 AND revokedatutc IS NULL AND connectionstatus='Connected' AND resourcetype='Platform' AND dockernodeid IS NOT DISTINCT FROM $4 FOR SHARE")
            .bind(session.agent_id).bind(snapshot.platform_id).bind(session.connected_at).bind(&session.target.node_id).fetch_optional(&mut *tx).await?;
        if session.is_closed() || current.is_none() {
            return Err(EdgeStoreError::Unauthorized);
        }
        if let citadel_platforms::jobs::ResourceInventory::Platform(info) = &snapshot.inventory {
            let daemon: String =
                sqlx::query_scalar("SELECT dockerdaemonid FROM edgeagentbindings WHERE agentid=$1")
                    .bind(session.agent_id)
                    .fetch_one(&mut *tx)
                    .await?;
            if daemon.is_empty()
                || daemon != info.daemon_id
                || session
                    .target
                    .node_id
                    .as_ref()
                    .is_some_and(|node| info.swarm.as_ref().is_none_or(|s| s.node_id != *node))
            {
                return Err(EdgeStoreError::Unauthorized);
            }
        }
        let mut changed =
            crate::persistence::postgres::platforms::inventory::resource::persist_resource(
                &mut tx,
                snapshot,
                session.target.node_id.as_deref(),
                &self.node_policy,
            )
            .await
            .map_err(|_| EdgeStoreError::Invalid("Scoped Edge projection failed."))?;
        if session.target.node_id.is_none()
            && matches!(
                snapshot.inventory,
                citadel_platforms::jobs::ResourceInventory::Platform(_)
            )
        {
            changed |= crate::persistence::postgres::platforms::status::platform_status(
                &mut tx,
                snapshot.platform_id,
                "Online",
            )
            .await?;
        }
        let identities = if matches!(
            snapshot.inventory,
            citadel_platforms::jobs::ResourceInventory::Containers(_)
        ) {
            crate::persistence::postgres::platforms::runtime_index::stage_scope(
                &self.pool,
                &mut tx,
                snapshot.platform_id,
                session.target.node_id.as_deref(),
            )
            .await?
        } else {
            None
        };
        tx.commit().await?;
        if let Some(identities) = identities {
            identities.committed();
        }
        if let Some(write) = write {
            write.committed();
        }
        if matches!(
            snapshot.inventory,
            citadel_platforms::jobs::ResourceInventory::Containers(_)
        ) {
            changed |= crate::persistence::postgres::platforms::status::reconcile_deployments(
                &self.pool,
                snapshot.platform_id,
                None,
                false,
            )
            .await?
                > 0;
        }
        Ok(citadel_platforms::jobs::ProjectionChange::committed(
            changed,
        ))
    }

    /// Use the same session fence as inventory and metrics for targeted daemon updates.
    pub async fn persist_resource_event(
        &self,
        session: &crate::connectors::edge::EdgeSession,
        event: &crate::connectors::agent::client::AgentDaemonEvent,
    ) -> Result<citadel_platforms::jobs::ProjectionChange, EdgeStoreError> {
        let Some(delta) = event.resource.as_ref().filter(|d| d.valid_for(event.kind)) else {
            return Ok(citadel_platforms::jobs::ProjectionChange::Unavailable);
        };
        let write = ProjectionWrite::begin(
            session.target.platform_id,
            session.target.node_id.as_deref(),
            delta.projection_kind(),
        )
        .await;
        let mut tx = self.pool.begin().await?;
        let current: Option<Uuid> = sqlx::query_scalar("SELECT agentid FROM edgeagentbindings WHERE agentid=$1 AND platformid=$2 AND lastconnectedatutc=$3 AND revokedatutc IS NULL AND connectionstatus='Connected' AND resourcetype='Platform' AND dockernodeid IS NOT DISTINCT FROM $4 FOR SHARE")
            .bind(session.agent_id).bind(session.target.platform_id).bind(session.connected_at).bind(&session.target.node_id).fetch_optional(&mut *tx).await?;
        if session.is_closed() || current.is_none() {
            return Err(EdgeStoreError::Unauthorized);
        }
        let changed = super::super::inventory::delta::persist_delta(
            &mut tx,
            session.target.platform_id,
            session.target.node_id.as_deref(),
            delta,
        )
        .await
        .map_err(|e| EdgeStoreError::Storage(sqlx::Error::Protocol(e.to_string())))?;
        tx.commit().await?;
        write.committed();
        Ok(citadel_platforms::jobs::ProjectionChange::committed(
            changed,
        ))
    }

    /// One session-fenced projection transaction for an already coalesced batch.
    pub async fn persist_container_state_deltas(
        &self,
        session: &crate::connectors::edge::EdgeSession,
        deltas: &[citadel_platforms::jobs::ContainerStateDelta],
    ) -> Result<Vec<citadel_platforms::jobs::ContainerDeltaResult>, EdgeStoreError> {
        let write = ProjectionWrite::begin(
            session.target.platform_id,
            session.target.node_id.as_deref(),
            ProjectionKind::Containers,
        )
        .await;
        let mut tx = self.pool.begin().await?;
        let current: Option<Uuid> = sqlx::query_scalar("SELECT agentid FROM edgeagentbindings WHERE agentid=$1 AND platformid=$2 AND lastconnectedatutc=$3 AND revokedatutc IS NULL AND connectionstatus='Connected' AND resourcetype='Platform' AND dockernodeid IS NOT DISTINCT FROM $4 FOR SHARE")
            .bind(session.agent_id).bind(session.target.platform_id).bind(session.connected_at).bind(&session.target.node_id).fetch_optional(&mut *tx).await?;
        if session.is_closed() || current.is_none() {
            return Err(EdgeStoreError::Unauthorized);
        }
        let results = super::super::status::container_state_deltas_in(
            &self.pool,
            &mut tx,
            session.target.platform_id,
            session.target.node_id.as_deref(),
            deltas,
        )
        .await?;
        tx.commit().await?;
        for result in &results {
            super::super::runtime_index::committed_identity(
                &self.pool,
                session.target.platform_id,
                session.target.node_id.as_deref(),
                &result.docker_id,
                result.container_id,
            );
        }
        write.committed();
        super::super::containers::coordination::state_batch(
            &self.pool,
            session.target.platform_id,
            session.target.node_id.as_deref(),
            deltas,
            &results,
        );
        super::super::status::reconcile_state_deployments(
            &self.pool,
            session.target.platform_id,
            &results,
        )
        .await?;
        Ok(results)
    }

    pub async fn persist_container_event(
        &self,
        session: &crate::connectors::edge::EdgeSession,
        event: &crate::connectors::agent::client::AgentDaemonEvent,
    ) -> Result<citadel_platforms::jobs::ProjectionChange, EdgeStoreError> {
        self.persist_container_event_at(session, event, chrono::Utc::now().timestamp_millis())
            .await
    }

    pub async fn persist_container_event_at(
        &self,
        session: &crate::connectors::edge::EdgeSession,
        event: &crate::connectors::agent::client::AgentDaemonEvent,
        observed_millis: i64,
    ) -> Result<citadel_platforms::jobs::ProjectionChange, EdgeStoreError> {
        let Some(id) = event.container_id.as_deref() else {
            return Ok(citadel_platforms::jobs::ProjectionChange::Unavailable);
        };
        let citadel_platforms::jobs::RuntimeEventKind::Container(change) = event.kind else {
            return Ok(citadel_platforms::jobs::ProjectionChange::Unavailable);
        };
        let observed = observed_millis / 1000;
        if let Some(state) = change.state_delta() {
            let result = self
                .persist_container_state_deltas(
                    session,
                    &[citadel_platforms::jobs::ContainerStateDelta {
                        docker_id: id.to_owned(),
                        state,
                        observed_at: observed,
                        observed_at_millis: observed_millis,
                    }],
                )
                .await?
                .remove(0);
            return Ok(if result.accepted {
                citadel_platforms::jobs::ProjectionChange::committed(result.changed)
            } else {
                citadel_platforms::jobs::ProjectionChange::Unavailable
            });
        }
        let destroyed = change == citadel_platforms::jobs::ContainerChange::Tombstone;
        if !destroyed
            && (event.container_state.is_none()
                || event
                    .container
                    .as_ref()
                    .is_none_or(|container| container.id != id || container.image_id.is_empty()))
        {
            return Ok(citadel_platforms::jobs::ProjectionChange::Unavailable);
        }
        // Stamp receipt before any projection/session-lock wait.
        let write = ProjectionWrite::begin(
            session.target.platform_id,
            session.target.node_id.as_deref(),
            ProjectionKind::Containers,
        )
        .await;
        let mut tx = self.pool.begin().await?;
        let current: Option<Uuid> = sqlx::query_scalar("SELECT agentid FROM edgeagentbindings WHERE agentid=$1 AND platformid=$2 AND lastconnectedatutc=$3 AND revokedatutc IS NULL AND connectionstatus='Connected' AND resourcetype='Platform' AND dockernodeid IS NOT DISTINCT FROM $4 FOR SHARE")
            .bind(session.agent_id).bind(session.target.platform_id).bind(session.connected_at).bind(&session.target.node_id).fetch_optional(&mut *tx).await?;
        if session.is_closed() || current.is_none() {
            return Err(EdgeStoreError::Unauthorized);
        }
        let changed = if !destroyed && let Some(container) = &event.container {
            crate::persistence::postgres::platforms::status::container_observation_in(
                &mut tx,
                session.target.platform_id,
                session.target.node_id.as_deref(),
                container,
                observed,
            )
            .await?
        } else {
            crate::persistence::postgres::platforms::status::container_event_in(
                &mut tx,
                session.target.platform_id,
                session.target.node_id.as_deref(),
                id,
                if destroyed {
                    None
                } else {
                    event.container_state.as_deref()
                },
                event.container_name.as_deref(),
                observed,
                super::super::runtime_index::hint(
                    &self.pool,
                    session.target.platform_id,
                    session.target.node_id.as_deref(),
                    id,
                ),
            )
            .await?
        };
        tx.commit().await?;
        if changed.identity.is_some() || !changed.accepted {
            super::super::runtime_index::committed_identity(
                &self.pool,
                session.target.platform_id,
                session.target.node_id.as_deref(),
                id,
                if destroyed && changed.changed {
                    None
                } else {
                    changed.identity
                },
            );
        }
        write.committed();
        if destroyed
            && changed.changed
            && let (Some(operation), Some(id)) = (changed.operation_id, changed.identity)
        {
            super::super::containers::coordination::committed(
                &self.pool,
                operation,
                &citadel_platforms::containers::ContainerTarget {
                    id,
                    platform_id: session.target.platform_id,
                    node_id: session.target.node_id.clone(),
                    docker_id: event.container_id.clone().unwrap(),
                },
                None,
                observed_millis,
            );
        }
        if changed.changed {
            crate::persistence::postgres::platforms::status::reconcile_deployments(
                &self.pool,
                session.target.platform_id,
                Some(&changed.deployments),
                true,
            )
            .await?;
        }
        Ok(if changed.accepted {
            citadel_platforms::jobs::ProjectionChange::committed(changed.changed)
        } else {
            citadel_platforms::jobs::ProjectionChange::Unavailable
        })
    }
    pub async fn persist_stats(
        &self,
        session: &crate::connectors::edge::EdgeSession,
        stats: &[citadel_platforms::RuntimeContainerStat],
    ) -> Result<usize, EdgeStoreError> {
        self.persist_stats_with_disk(session, stats, None).await
    }

    pub async fn persist_stats_with_disk(
        &self,
        session: &crate::connectors::edge::EdgeSession,
        stats: &[citadel_platforms::RuntimeContainerStat],
        disk: Option<citadel_platforms::HostDiskUsage>,
    ) -> Result<usize, EdgeStoreError> {
        self.persist_stats_sample(session, stats, disk, None).await
    }

    pub async fn persist_stats_with_platform_stats(
        &self,
        session: &crate::connectors::edge::EdgeSession,
        stats: &[citadel_platforms::RuntimeContainerStat],
        sample: Option<&citadel_platforms::RuntimePlatformStats>,
    ) -> Result<usize, EdgeStoreError> {
        self.persist_stats_sample(
            session,
            stats,
            sample.and_then(citadel_platforms::RuntimePlatformStats::disk),
            sample,
        )
        .await
    }

    async fn persist_stats_sample(
        &self,
        session: &crate::connectors::edge::EdgeSession,
        stats: &[citadel_platforms::RuntimeContainerStat],
        disk: Option<citadel_platforms::HostDiskUsage>,
        sample: Option<&citadel_platforms::RuntimePlatformStats>,
    ) -> Result<usize, EdgeStoreError> {
        let mut tx = self.pool.begin().await?;
        let current: Option<Uuid> = sqlx::query_scalar("SELECT agentid FROM edgeagentbindings WHERE agentid=$1 AND platformid=$2 AND lastconnectedatutc=$3 AND revokedatutc IS NULL AND connectionstatus='Connected' AND resourcetype='Platform' AND dockernodeid IS NOT DISTINCT FROM $4 FOR SHARE")
            .bind(session.agent_id).bind(session.target.platform_id).bind(session.connected_at).bind(&session.target.node_id).fetch_optional(&mut *tx).await?;
        if session.is_closed() || current.is_none() {
            return Err(EdgeStoreError::Unauthorized);
        }
        if session.target.node_id.is_none()
            && let Some(sample) = sample
        {
            crate::persistence::postgres::platforms::statistics::store::persist_platform_metadata(
                &mut tx,
                session.target.platform_id,
                sample,
            )
            .await
            .map_err(|error| EdgeStoreError::Storage(sqlx::Error::Protocol(error.to_string())))?;
        }
        let inserted =
            crate::persistence::postgres::platforms::statistics::store::persist_scoped_with_disk(
                &mut tx,
                session.target.platform_id,
                session.target.node_id.as_deref(),
                stats,
                disk,
            )
            .await
            .map_err(|error| EdgeStoreError::Storage(sqlx::Error::Protocol(error.to_string())))?;
        tx.commit().await?;
        Ok(inserted)
    }

    pub async fn heartbeat(
        &self,
        agent_id: Uuid,
        connected_at: DateTime<Utc>,
    ) -> Result<(), EdgeStoreError> {
        let updated = sqlx::query("UPDATE edgeagentbindings SET lastheartbeatatutc=now(),updatedatutc=now() WHERE agentid=$1 AND lastconnectedatutc=$2 AND revokedatutc IS NULL")
            .bind(agent_id).bind(connected_at).execute(&self.pool).await?.rows_affected();
        if updated != 1 {
            return Err(EdgeStoreError::Unauthorized);
        }
        Ok(())
    }
    pub async fn revoke(&self, target: &EdgeTarget) -> Result<(), EdgeStoreError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext('citadel-platform-registration'))")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE edgeagentenrollments SET revokedatutc=now() WHERE resourceid=$1 AND resourcetype=$2 AND revokedatutc IS NULL")
            .bind(target.resource_id).bind(resource_type(target)).execute(&mut *tx).await?;
        sqlx::query("UPDATE edgeagentbindings SET revokedatutc=now(),connectionstatus='Revoked',updatedatutc=now() WHERE resourceid=$1 AND resourcetype=$2 AND dockernodeid IS NOT DISTINCT FROM $3 AND revokedatutc IS NULL")
            .bind(target.resource_id).bind(resource_type(target)).bind(&target.node_id).execute(&mut *tx).await?;
        if target.resource_type == 0 && target.node_id.is_none() {
            sqlx::query(
                "UPDATE platforms SET status='Offline' WHERE id=$1 AND connectortype='EdgeAgent'",
            )
            .bind(target.platform_id)
            .execute(&mut *tx)
            .await?;
        }
        if let Some(node) = &target.node_id {
            mark_node_stale(&mut tx, target.platform_id, node).await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

async fn mark_node_stale(
    tx: &mut Transaction<'_, Postgres>,
    platform: Uuid,
    node: &str,
) -> Result<(), sqlx::Error> {
    for query in [
        "UPDATE containers SET projectionstalesince=COALESCE(projectionstalesince,EXTRACT(EPOCH FROM now())::bigint),projectionstalereason='Node Agent is disconnected or unavailable.' WHERE platformid=$1 AND dockernodeid=$2",
        "UPDATE swarmnodeimageprojections SET isstale=true WHERE platformid=$1 AND dockernodeid=$2",
        "UPDATE swarmnodevolumeprojections SET isstale=true WHERE platformid=$1 AND dockernodeid=$2",
        "UPDATE swarmnodenetworkprojections SET isstale=true WHERE platformid=$1 AND dockernodeid=$2",
        "UPDATE swarmnoderuntimeprojectionstates SET isstale=true,stalesince=COALESCE(stalesince,now()),stalereason='Node Agent is disconnected or unavailable.' WHERE platformid=$1 AND dockernodeid=$2",
    ] {
        sqlx::query(query)
            .bind(platform)
            .bind(node)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

fn nonempty(value: &str) -> Option<&str> {
    if value.is_empty() { None } else { Some(value) }
}
fn resource_type(target: &EdgeTarget) -> &'static str {
    if target.resource_type == 1 {
        "BuildAgentPool"
    } else {
        "Platform"
    }
}
pub(crate) fn token_hash(token: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(token.as_bytes()))
}
pub(crate) fn fingerprint(key: &[u8]) -> String {
    let hex: String = Sha256::digest(key)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!("SHA256:{hex}")
}

pub(crate) fn validate_capabilities(
    version: i32,
    capabilities: &str,
    profile: i32,
    build: bool,
) -> Result<(), EdgeStoreError> {
    if version != citadel_contracts::EDGE_AGENT_PROTOCOL_VERSION
        || !(0..=1).contains(&profile)
        || capabilities.len() > 16 * 1024
    {
        return Err(EdgeStoreError::Unauthorized);
    }
    let value: Value =
        serde_json::from_str(capabilities).map_err(|_| EdgeStoreError::Unauthorized)?;
    let commands = value
        .get("commands")
        .and_then(Value::as_array)
        .ok_or(EdgeStoreError::Unauthorized)?;
    let required = ["platform.checkHealth", "containers.list", "containers.logs"];
    let extra: &[&str] = if build {
        &["images.build", "images.push", "images.checkBuildHost"]
    } else if profile == 1 {
        &[
            "platform.getInfo",
            "platform.events",
            "containers.inspect",
            "containers.patch",
            "containers.delete",
            "containers.stats",
            "containers.exec",
        ]
    } else {
        &[]
    };
    if required.iter().chain(extra).any(|required| {
        !commands
            .iter()
            .any(|command| command.as_str() == Some(*required))
    }) {
        return Err(EdgeStoreError::Unauthorized);
    }
    Ok(())
}

async fn validate_target(
    tx: &mut Transaction<'_, Postgres>,
    target: &EdgeTarget,
) -> Result<(), EdgeStoreError> {
    let mode: Option<String> = if target.resource_type == 0 {
        sqlx::query_scalar("SELECT connectortype FROM platforms WHERE id=$1")
            .bind(target.resource_id)
            .fetch_optional(&mut **tx)
            .await?
    } else {
        sqlx::query_scalar("SELECT COALESCE(providerspec->>'ConnectionMode',providerspec->>'connectionMode','') FROM buildagentpools WHERE id=$1 AND provider='SelfManagedVm' AND archivedat IS NULL AND enabled").bind(target.resource_id).fetch_optional(&mut **tx).await?
    };
    match mode.as_deref() {
        None => return Err(EdgeStoreError::NotFound),
        Some("EdgeAgent") => {}
        Some(_) => {
            return Err(EdgeStoreError::Invalid(
                "The selected resource does not use an Edge Agent.",
            ));
        }
    }
    Ok(())
}

async fn validate_daemon(
    tx: &mut Transaction<'_, Postgres>,
    target: &EdgeTarget,
    daemon: &str,
) -> Result<(), EdgeStoreError> {
    let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM platforms WHERE id=$1 AND COALESCE(NULLIF(platformdescriptor::jsonb->>'daemonId',''),$2)=$2) AND NOT EXISTS(SELECT 1 FROM platforms WHERE id<>$1 AND platformdescriptor::jsonb->>'daemonId'=$2)")
        .bind(target.platform_id).bind(daemon).fetch_one(&mut **tx).await?;
    if !valid || daemon.is_empty() {
        return Err(EdgeStoreError::Unauthorized);
    }
    sqlx::query("UPDATE platforms SET platformdescriptor=jsonb_set(platformdescriptor::jsonb,'{daemonId}',to_jsonb($2::text)) WHERE id=$1")
        .bind(target.platform_id).bind(daemon).execute(&mut **tx).await?;
    Ok(())
}

async fn validate_rebind(
    tx: &mut Transaction<'_, Postgres>,
    target: &EdgeTarget,
    agent_id: Uuid,
    previous: &str,
) -> Result<(), EdgeStoreError> {
    let valid: bool = sqlx::query_scalar(
        r#"
SELECT EXISTS (
    SELECT 1 FROM edgeagentbindings b
    WHERE agentid=$2 AND platformid=$1 AND dockernodeid=$3
      AND profile='SwarmNode' AND revokedatutc IS NULL
      AND GREATEST(lastheartbeatatutc,lastdisconnectedatutc,lastauthenticatedatutc,
                   firstenrolledatutc,createdatutc) < now()-interval '10 minutes'
      AND NOT EXISTS (SELECT 1 FROM swarmnodeprojections n
                      WHERE n.platformid=$1 AND (n.dockernodeid=$3 OR n.isstale))
      AND NOT EXISTS (SELECT 1 FROM edgeagentbindings other
                      WHERE other.revokedatutc IS NULL AND other.agentid<>b.agentid
                        AND ((other.platformid=$1 AND other.dockernodeid=$4)
                             OR other.dockerdaemonid=b.dockerdaemonid))
)"#,
    )
    .bind(target.platform_id)
    .bind(agent_id)
    .bind(previous)
    .bind(&target.node_id)
    .fetch_one(&mut **tx)
    .await?;
    if valid {
        Ok(())
    } else {
        Err(EdgeStoreError::Unauthorized)
    }
}

async fn validate_node(
    tx: &mut Transaction<'_, Postgres>,
    target: &EdgeTarget,
    cluster: &str,
    hostname: &str,
    role: &str,
    service: &str,
    task: &str,
) -> Result<(), EdgeStoreError> {
    let valid: bool = sqlx::query_scalar(
        r#"
SELECT EXISTS (
    SELECT 1 FROM platforms p
    JOIN swarmnodeagentinstallations i ON i.platformid=p.id
    JOIN swarmnodeprojections n ON n.platformid=p.id AND n.dockernodeid=$2
    JOIN swarmserviceprojections s ON s.platformid=p.id AND s.dockerserviceid=$6
    JOIN swarmtaskprojections t ON t.platformid=p.id AND t.dockertaskid=$7
    WHERE p.id=$1 AND p.clusterid=$3 AND p.platformdescriptor::jsonb->>'$type'='DockerSwarm'
      AND p.platformdescriptor::jsonb->>'nodeID'<>$2
      AND i.clusterid=$3 AND i.desiredstate='Installed' AND i.dockerserviceid=$6
      AND n.isstale=false AND lower(n.status)='ready' AND lower(n.availability)='active'
      AND lower(n.operatingsystem)='linux' AND n.hostname=$4 AND lower(n.role)=lower($5)
      AND s.isstale=false AND lower(s.labels->>'com.citadel.system')='true'
      AND s.labels->>'com.citadel.system-role'='swarm-node-agent'
      AND lower(s.labels->>'com.citadel.platform-id')=$1::text
      AND s.labels->>'com.citadel.swarm-cluster-id'=$3
      AND t.isstale=false AND t.dockerserviceid=$6 AND t.dockernodeid=$2
      AND lower(t.desiredstate)='running' AND lower(t.state)='running'
)"#,
    )
    .bind(target.platform_id)
    .bind(&target.node_id)
    .bind(cluster)
    .bind(hostname)
    .bind(role)
    .bind(service)
    .bind(task)
    .fetch_one(&mut **tx)
    .await?;
    if !valid {
        return Err(EdgeStoreError::Unauthorized);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ports EdgeAgentCapabilitiesTests and extends it to profile negotiation.
    #[test]
    fn capabilities_require_the_protocol_and_profile_specific_commands() {
        let ordinary =
            r#"{"commands":["platform.checkHealth","containers.list","containers.logs"]}"#;
        assert!(validate_capabilities(2, ordinary, 0, false).is_ok());
        assert!(validate_capabilities(1, ordinary, 0, false).is_err());
        assert!(validate_capabilities(3, ordinary, 0, false).is_err());
        assert!(validate_capabilities(2, ordinary, 2, false).is_err());
        assert!(validate_capabilities(2, ordinary, 1, false).is_err());
        assert!(validate_capabilities(2, ordinary, 0, true).is_err());
        assert!(
            validate_capabilities(
                2,
                r#"{"commands":["containers.list","containers.logs"]}"#,
                0,
                false
            )
            .is_err()
        );
        assert!(validate_capabilities(2, "not-json", 0, false).is_err());
        assert!(validate_capabilities(2, &" ".repeat(16 * 1024 + 1), 0, false).is_err());
    }
}
