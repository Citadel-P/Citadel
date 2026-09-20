//! Read-mostly connector snapshot and reusable channels, keyed only by persisted Platforms.
//! No authorization data is cached. PostgreSQL and the periodic refresh are authoritative.
use citadel_adapters::agent::AgentClient;
use sqlx::{PgPool, Row};
use std::{sync::Arc, time::Duration};
use tokio::sync::{RwLock, watch};
use tokio_util::sync::CancellationToken;

// Workload classes have independent capacity. Inventory is sequential inside
// each target, so the target limit is also its maximum external-operation count.
pub const HEALTH_CONCURRENCY: usize = 4;
pub const INVENTORY_CONCURRENCY: usize = 2;
pub const STATS_CONCURRENCY: usize = 8;

#[derive(Clone)]
pub(crate) struct PlatformTarget {
    pub id: uuid::Uuid,
    pub name: String,
    pub address: String,
    pub connector_type: citadel_platforms::ConnectorKind,
    pub platform_type: citadel_platforms::PlatformKind,
    pub agent: Option<Arc<AgentClient>>,
}

pub struct PlatformRuntimeRegistry {
    /// Shared by Direct/Local and Edge inventory: at most two external operations.
    pub inventory_budget: citadel_application::IoBudget,
    pool: PgPool,
    base: Option<AgentClient>,
    targets: RwLock<Arc<Vec<PlatformTarget>>>,
    changed: watch::Sender<u64>,
}

impl PlatformRuntimeRegistry {
    pub fn new(pool: PgPool, base: Option<AgentClient>) -> Arc<Self> {
        Arc::new(Self {
            inventory_budget: citadel_application::IoBudget::new(
                INVENTORY_CONCURRENCY.try_into().unwrap(),
                citadel_application::runtime_metrics::RuntimeWork::Inventory,
            ),
            pool,
            base,
            targets: RwLock::new(Arc::new(Vec::new())),
            changed: watch::channel(0).0,
        })
    }

    pub(crate) async fn snapshot(&self) -> Arc<Vec<PlatformTarget>> {
        self.targets.read().await.clone()
    }
    pub(crate) fn changes(&self) -> watch::Receiver<u64> {
        self.changed.subscribe()
    }

    pub(crate) async fn refresh(&self) -> Result<(), sqlx::Error> {
        let _iteration = citadel_application::runtime_metrics::RuntimeWork::Targets.start();
        let rows = sqlx::query("SELECT id,name,address,connectortype,COALESCE(platformdescriptor->>'$type','Docker') AS platformtype FROM platforms WHERE connectortype IN ('Local','Agent','EdgeAgent') ORDER BY id").fetch_all(&self.pool).await?;
        let previous = self.snapshot().await;
        let mut next = Vec::with_capacity(rows.len());
        for row in rows {
            let id = row.try_get("id")?;
            let address: String = row.try_get("address")?;
            let connector_type =
                citadel_adapters::postgres::platform_classification::connector_kind(
                    row.try_get("connectortype")?,
                )?;
            let agent = if connector_type == citadel_platforms::ConnectorKind::Agent {
                previous
                    .iter()
                    .find(|target| {
                        target.id == id
                            && target.address == address
                            && target.connector_type == connector_type
                    })
                    .and_then(|target| target.agent.clone())
                    .or_else(|| {
                        self.base
                            .as_ref()
                            .and_then(|base| match base.at_address(&address) {
                                Ok(client) => Some(Arc::new(client)),
                                Err(error) => {
                                    tracing::warn!(%error, %id, "Platform Agent address rejected");
                                    None
                                }
                            })
                    })
            } else {
                None
            };
            next.push(PlatformTarget {
                id,
                address,
                connector_type,
                agent,
                name: row.try_get("name")?,
                platform_type: citadel_adapters::postgres::platform_classification::platform_kind(
                    row.try_get("platformtype")?,
                )?,
            });
        }
        let changed = previous.len() != next.len()
            || previous.iter().zip(&next).any(|(old, new)| {
                old.id != new.id
                    || old.name != new.name
                    || old.address != new.address
                    || old.connector_type != new.connector_type
                    || old.platform_type != new.platform_type
            });
        *self.targets.write().await = Arc::new(next);
        if changed {
            self.changed
                .send_modify(|generation| *generation = generation.wrapping_add(1));
        }
        Ok(())
    }

    pub(crate) async fn agent(&self, platform: uuid::Uuid) -> Option<AgentClient> {
        self.snapshot()
            .await
            .iter()
            .find(|target| target.id == platform)
            .and_then(|target| target.agent.as_ref().map(|agent| agent.as_ref().clone()))
    }

    pub(crate) async fn reconfigured(&self, id: uuid::Uuid, address: &str) {
        let mut changes = self.changes();
        loop {
            let same = self.snapshot().await.iter().any(|target| {
                target.id == id
                    && target.connector_type == citadel_platforms::ConnectorKind::Agent
                    && target.address.trim_end_matches('/') == address.trim_end_matches('/')
            });
            if !same || changes.changed().await.is_err() {
                return;
            }
        }
    }

    pub async fn run(
        self: Arc<Self>,
        cancellation: CancellationToken,
        mut updates: watch::Receiver<()>,
    ) -> Result<(), std::convert::Infallible> {
        let mut tick = tokio::time::interval(Duration::from_secs(60));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                () = cancellation.cancelled() => return Ok(()),
                _ = tick.tick() => {},
                result = updates.changed() => if result.is_err() {
                    tokio::select! { ()=cancellation.cancelled()=>return Ok(()), _=tokio::time::sleep(Duration::from_secs(60))=>{} }
                }
            }
            // Watch coalesces arbitrarily many mutations into one refresh.
            tokio::select! { ()=cancellation.cancelled()=>return Ok(()), _=tokio::time::sleep(Duration::from_millis(100))=>{} }
            updates.borrow_and_update();
            let result = tokio::select! { ()=cancellation.cancelled()=>return Ok(()), result=self.refresh()=>result };
            if let Err(error) = result {
                tracing::warn!(%error, "Platform target refresh failed; retaining last snapshot");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires CITADEL_TEST_DATABASE_URL"]
    async fn persisted_generations_reuse_channels_and_remove_deleted_targets() {
        let url = std::env::var("CITADEL_TEST_DATABASE_URL").unwrap();
        citadel_database::MigrationRunner::migrate(&url)
            .await
            .unwrap();
        let pool = PgPool::connect(&url).await.unwrap();
        let base = AgentClient::lazy(
            "http://localhost",
            citadel_adapters::agent::AgentRequestSigner::from_bytes(&[42; 32]),
            Duration::from_secs(1),
            true,
        )
        .unwrap();
        let registry = PlatformRuntimeRegistry::new(pool.clone(), Some(base));
        let id = uuid::Uuid::now_v7();
        sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status) VALUES($1,$1::text,'http://127.0.0.1:49151','Agent',0,0,0,0,0,'{\"$type\":\"Docker\"}','Offline')").bind(id).execute(&pool).await.unwrap();
        registry.refresh().await.unwrap();
        let first = registry
            .snapshot()
            .await
            .iter()
            .find(|target| target.id == id)
            .unwrap()
            .agent
            .clone()
            .unwrap();
        registry.refresh().await.unwrap();
        let unchanged = registry
            .snapshot()
            .await
            .iter()
            .find(|target| target.id == id)
            .unwrap()
            .agent
            .clone()
            .unwrap();
        assert!(Arc::ptr_eq(&first, &unchanged));
        sqlx::query("UPDATE platforms SET address='http://127.0.0.1:49152' WHERE id=$1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
        registry.refresh().await.unwrap();
        let changed = registry
            .snapshot()
            .await
            .iter()
            .find(|target| target.id == id)
            .unwrap()
            .agent
            .clone()
            .unwrap();
        assert!(!Arc::ptr_eq(&first, &changed));
        assert_eq!(changed.address(), "http://127.0.0.1:49152");
        sqlx::query("DELETE FROM platforms WHERE id=$1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
        // No notification: the exact same authoritative refresh used by the timer recovers it.
        registry.refresh().await.unwrap();
        assert!(registry.agent(id).await.is_none());
        assert!(
            !registry
                .snapshot()
                .await
                .iter()
                .any(|target| target.id == id)
        );
        pool.close().await;
    }
}
