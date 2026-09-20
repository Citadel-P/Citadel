use super::*;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use citadel_platforms::{
    InventoryProjectionStore, RuntimeInventorySnapshot, node_agents::setup::*,
};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

impl NodeAgentSetupStore for PostgresNodeAgentLifecycleStore {
    fn claim_setup(
        &self,
        actor: ActorId,
        id: Uuid,
        info: RuntimePlatformInfo,
        kind: SetupKind,
    ) -> BoxFuture<'_, Result<NodeAgentRemovalClaim, RuntimeCapabilityError>> {
        Box::pin(self.claim_operation(actor, id, info, kind.as_str()))
    }
    fn bootstrap<'a>(
        &'a self,
        claim: &'a NodeAgentRemovalClaim,
    ) -> BoxFuture<'a, Result<Bootstrap, RuntimeCapabilityError>> {
        Box::pin(async move {
            let mut tx = self.0.begin().await.map_err(storage)?;
            fence(&mut tx, claim).await?;
            let mut bytes = Zeroizing::new([0u8; 32]);
            getrandom::fill(bytes.as_mut()).map_err(storage)?;
            let token = Zeroizing::new(URL_SAFE_NO_PAD.encode(bytes.as_ref()).into_bytes());
            let hash = URL_SAFE_NO_PAD.encode(Sha256::digest(token.as_slice()));
            let version:i64=sqlx::query_scalar("SELECT COALESCE(max(version),0)::bigint+1 FROM swarmnodeagentbootstraps WHERE platformid=$1").bind(claim.platform_id).fetch_one(&mut *tx).await.map_err(storage)?;
            let version = i32::try_from(version).map_err(storage)?;
            let id = Uuid::now_v7();
            let name = format!(
                "citadel-node-agent-{}-bootstrap-{version}",
                claim.platform_id.simple()
            );
            sqlx::query("UPDATE swarmnodeagentbootstraps SET revokedatutc=COALESCE(revokedatutc,now()),updatedatutc=now() WHERE platformid=$1").bind(claim.platform_id).execute(&mut *tx).await.map_err(storage)?;
            sqlx::query("INSERT INTO swarmnodeagentbootstraps(id,platformid,clusterid,version,tokenhash,dockersecretname,expiresatutc,createdbyactorid) VALUES($1,$2,$3,$4,$5,$6,now()+interval '10 minutes',$7)")
                .bind(id).bind(claim.platform_id).bind(&claim.cluster_id).bind(version).bind(hash).bind(&name).bind(claim.actor.value()).execute(&mut *tx).await.map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            Ok(Bootstrap {
                id,
                secret_name: name,
                token,
            })
        })
    }
    fn secret_created<'a>(
        &'a self,
        claim: &'a NodeAgentRemovalClaim,
        bootstrap: Uuid,
        secret: &'a str,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            let mut tx = self.0.begin().await.map_err(storage)?;
            fence(&mut tx, claim).await?;
            let count=sqlx::query("UPDATE swarmnodeagentbootstraps SET dockersecretid=$3,updatedatutc=now() WHERE id=$1 AND platformid=$2 AND revokedatutc IS NULL").bind(bootstrap).bind(claim.platform_id).bind(secret).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if count != 1 {
                return Err(invalid("Bootstrap enrollment window is no longer active."));
            }
            tx.commit().await.map_err(storage)
        })
    }
    fn service_applied<'a>(
        &'a self,
        claim: &'a NodeAgentRemovalClaim,
        spec: &'a SystemAgentSpec,
        id: &'a str,
        reference: &'a str,
        digest: &'a str,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            let mut tx = self.0.begin().await.map_err(storage)?;
            fence(&mut tx, claim).await?;
            sqlx::query("UPDATE swarmnodeagentinstallations SET dockerserviceid=$3,dockerservicename=$4,agentimagereference=$5,agentimagedigest=$6,dockercaconfigid=$7,dockercaconfigname=$8,updatedatutc=now() WHERE platformid=$1 AND operationid=$2")
                .bind(claim.platform_id).bind(claim.operation_id).bind(id).bind(&spec.name).bind(reference).bind(digest).bind(&spec.ca_config_id).bind(&spec.ca_config_name).execute(&mut *tx).await.map_err(storage)?;
            tx.commit().await.map_err(storage)
        })
    }
    fn finish_setup<'a>(
        &'a self,
        claim: &'a NodeAgentRemovalClaim,
        kind: SetupKind,
        error: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            let mut tx = self.0.begin().await.map_err(storage)?;
            fence(&mut tx, claim).await?;
            sqlx::query("UPDATE swarmnodeagentbootstraps SET revokedatutc=COALESCE(revokedatutc,now()),updatedatutc=now() WHERE platformid=$1").bind(claim.platform_id).execute(&mut *tx).await.map_err(storage)?;
            sqlx::query("UPDATE swarmnodeagentinstallations SET operationstate=$3,operationerror=$4,updatedatutc=now() WHERE platformid=$1 AND operationid=$2").bind(claim.platform_id).bind(claim.operation_id).bind(if error.is_some(){"Failed"}else{"Completed"}).bind(error).execute(&mut *tx).await.map_err(storage)?;
            audit_kind(
                &mut tx,
                claim,
                if error.is_some() {
                    ActivityStatus::Failure
                } else {
                    ActivityStatus::Success
                },
                error.unwrap_or("Node-agent coverage is ready."),
                kind.as_str(),
            )
            .await?;
            tx.commit().await.map_err(storage)
        })
    }
    fn persist_inventory(
        &self,
        snapshot: RuntimeInventorySnapshot,
    ) -> BoxFuture<'_, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            crate::persistence::postgres::platforms::inventory::store::PostgresInventoryProjectionStore::new(self.0.clone())
                .persist(&snapshot)
                .await
                .map(|_| ())
                .map_err(storage)
        })
    }
    fn promote_manager<'a>(
        &'a self,
        claim: &'a NodeAgentRemovalClaim,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            let mut tx = self.0.begin().await.map_err(storage)?;
            fence(&mut tx, claim).await?;
            sqlx::query("UPDATE edgeagentbindings SET revokedatutc=now(),connectionstatus='Revoked',updatedatutc=now() WHERE platformid=$1 AND dockernodeid=$2 AND profile='SwarmNode' AND revokedatutc IS NULL").bind(claim.platform_id).bind(&claim.manager_node_id).execute(&mut *tx).await.map_err(storage)?;
            tx.commit().await.map_err(storage)
        })
    }
    fn task_bindings<'a>(
        &'a self,
        claim: &'a NodeAgentRemovalClaim,
        service: &'a str,
    ) -> BoxFuture<'a, Result<Vec<(String, String)>, RuntimeCapabilityError>> {
        Box::pin(async move {
            sqlx::query_as("SELECT dockernodeid,lastobservedtaskid FROM edgeagentbindings WHERE platformid=$1 AND clusterid=$2 AND lastobservedserviceid=$3 AND profile='SwarmNode' AND revokedatutc IS NULL AND dockernodeid IS NOT NULL AND lastobservedtaskid IS NOT NULL")
                .bind(claim.platform_id).bind(&claim.cluster_id).bind(service)
                .fetch_all(&self.0).await.map_err(storage)
        })
    }
}
