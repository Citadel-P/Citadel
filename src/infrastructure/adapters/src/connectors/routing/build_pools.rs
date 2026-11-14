//! Direct transports owned only by registered Build Pools. Authorization, health
//! and attempt recovery remain the caller's responsibility.
use crate::{
    connectors::agent::client::AgentClient, persistence::postgres::builds::execution_target,
};
use citadel_builds::{BuildError, BuildPoolConnections, BuildPoolTarget, pool_target};
use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind};
use futures_util::future::BoxFuture;
use sqlx::PgPool;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub struct BuildPoolRuntimeRegistry {
    pool: PgPool,
    // A registry's transport/trust policy is immutable. Signer clones observe key rotation.
    base: Option<AgentClient>,
    clients: Mutex<HashMap<Uuid, Arc<AgentClient>>>,
}
impl BuildPoolRuntimeRegistry {
    pub fn new(pool: PgPool, base: Option<AgentClient>) -> Arc<Self> {
        Arc::new(Self {
            pool,
            base,
            clients: Mutex::new(HashMap::new()),
        })
    }

    pub async fn resolve(
        &self,
        id: Uuid,
        expected_address: &str,
        cancel: &CancellationToken,
    ) -> Result<Arc<AgentClient>, RuntimeCapabilityError> {
        let operation = async {
            let base = self.base.as_ref().ok_or_else(unavailable)?;
            // Always consult registration; a retained channel is never permission
            // to use a deleted/reconfigured target or an arbitrary supplied URL.
            let spec = execution_target::registered_pool(&self.pool, id)
                .await
                .map_err(|_| unavailable())?;
            let current = spec
                .as_ref()
                .and_then(|s| pool_target(s).ok())
                .and_then(|target| {
                    if let BuildPoolTarget::Inbound(address) = target {
                        base.normalize_address(&address).ok()
                    } else {
                        None
                    }
                });
            let expected = base.normalize_address(expected_address)?;
            let mut clients = self.clients.lock().await;
            let Some(address) = current else {
                clients.remove(&id);
                return Err(unavailable());
            };
            if clients
                .get(&id)
                .is_some_and(|client| client.address() != address)
            {
                clients.remove(&id);
            }
            if address != expected {
                return Err(unavailable());
            }
            if let Some(client) = clients.get(&id) {
                return Ok(client.clone());
            }
            // Lazy construction is synchronous: no connect or RPC under this lock.
            // Concurrent first use shares the same channel's connection establishment.
            let client = Arc::new(base.at_address(&address, id)?);
            clients.insert(id, client.clone());
            Ok(client)
        };
        tokio::select! {
            biased;
            () = cancel.cancelled() => Err(RuntimeCapabilityError::new(RuntimeErrorKind::Cancelled, "Agent selection cancelled", false)),
            result = operation => result,
        }
    }
}

impl BuildPoolConnections for BuildPoolRuntimeRegistry {
    fn expire_connections(&self) -> BoxFuture<'_, Result<(), BuildError>> {
        Box::pin(async move {
            if self.clients.lock().await.is_empty() {
                return Ok(());
            }
            let rows = execution_target::registered_pools(&self.pool)
                .await
                .map_err(|e| BuildError::Storage(e.to_string()))?;
            let current: HashMap<_, _> = rows
                .into_iter()
                .filter_map(|(id, spec)| {
                    let BuildPoolTarget::Inbound(address) = pool_target(&spec).ok()? else {
                        return None;
                    };
                    Some((id, self.base.as_ref()?.normalize_address(&address).ok()?))
                })
                .collect();
            self.clients.lock().await.retain(|id, client| {
                current
                    .get(id)
                    .is_some_and(|address| address == client.address())
            });
            // Dropping the registry's reference does not cancel work holding a clone.
            Ok(())
        })
    }
}
fn unavailable() -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(
        RuntimeErrorKind::Unavailable,
        "Build Pool connection changed or is unavailable",
        true,
    )
}
