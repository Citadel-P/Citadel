use super::store::*;
use crate::persistence::postgres::connection_events::{self, ConnectionResource, ConnectionState};
use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub(super) async fn activity(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    agent_id: Uuid,
    state: ConnectionState,
    previous_status: String,
    reason: &str,
) -> Result<(), sqlx::Error> {
    let name: Option<String> = sqlx::query_scalar("SELECT name FROM buildagentpools WHERE id=$1")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?;
    let Some(name) = name else { return Ok(()) };
    let reason = reason.to_owned();
    let info = if state == ConnectionState::Online {
        citadel_activities::ActivityEventInfo::BuildAgentPoolConnected {
            agent_id,
            previous_status,
            reason,
        }
    } else {
        citadel_activities::ActivityEventInfo::BuildAgentPoolDisconnected {
            agent_id,
            previous_status,
            reason,
        }
    };
    let event = citadel_activities::ActivityEvent::new_build_pool_event(
        id,
        name,
        citadel_primitives::ActorId::new(citadel_identity::SYSTEM_ACTOR_ID),
        info,
        Utc::now(),
    )
    .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
    crate::persistence::postgres::activities::store::insert_activity(tx, &event)
        .await
        .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
    connection_events::publish(tx, ConnectionResource::BuildAgentPool, id, state).await
}

impl PostgresEdgeStore {
    /// Recover lost disconnects after a Core crash. The update rechecks heartbeat
    /// age under the row lock so a concurrent heartbeat or reconnect wins safely.
    pub async fn expire_build_pool_connections(&self) -> Result<(), EdgeStoreError> {
        let expired: Vec<(Uuid, DateTime<Utc>)> = sqlx::query_as("SELECT agentid,lastconnectedatutc FROM edgeagentbindings WHERE resourcetype='BuildAgentPool' AND connectionstatus='Connected' AND revokedatutc IS NULL AND lastheartbeatatutc < now()-interval '90 seconds' ORDER BY lastheartbeatatutc LIMIT 256")
            .fetch_all(&self.pool).await?;
        for (agent, at) in expired {
            self.disconnect(agent, at, true).await?;
        }
        Ok(())
    }
}
