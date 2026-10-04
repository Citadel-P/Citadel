//! Connection transitions are delivered by PostgreSQL only after commit. They
//! are notifications, not durable work: subscribers reconcile after a gap.
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub const CHANNEL: &str = "citadel_connection_events";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionResource {
    Platform,
    BuildAgentPool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionState {
    Online,
    Offline,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionEvent {
    pub resource: ConnectionResource,
    pub resource_id: Uuid,
    pub state: ConnectionState,
}

pub(crate) async fn publish(
    tx: &mut Transaction<'_, Postgres>,
    resource: ConnectionResource,
    resource_id: Uuid,
    state: ConnectionState,
) -> Result<(), sqlx::Error> {
    let payload = serde_json::to_string(&ConnectionEvent {
        resource,
        resource_id,
        state,
    })
    .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
    sqlx::query("SELECT pg_notify($1, $2)")
        .bind(CHANNEL)
        .bind(payload)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires disposable CITADEL_POOL_DATABASE_URL"]
    async fn notifications_are_commit_only_and_rollback_safe() {
        let pool = sqlx::PgPool::connect(&std::env::var("CITADEL_POOL_DATABASE_URL").unwrap())
            .await
            .unwrap();
        let mut listener = sqlx::postgres::PgListener::connect_with(&pool)
            .await
            .unwrap();
        listener.listen(CHANNEL).await.unwrap();
        let id = Uuid::now_v7();
        let mut tx = pool.begin().await.unwrap();
        publish(
            &mut tx,
            ConnectionResource::Platform,
            id,
            ConnectionState::Online,
        )
        .await
        .unwrap();
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(100), listener.recv())
                .await
                .is_err()
        );
        tx.rollback().await.unwrap();
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(100), listener.recv())
                .await
                .is_err()
        );
        let mut tx = pool.begin().await.unwrap();
        publish(
            &mut tx,
            ConnectionResource::Platform,
            id,
            ConnectionState::Online,
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        let notification = tokio::time::timeout(std::time::Duration::from_secs(2), listener.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            serde_json::from_str::<ConnectionEvent>(notification.payload()).unwrap(),
            ConnectionEvent {
                resource: ConnectionResource::Platform,
                resource_id: id,
                state: ConnectionState::Online,
            }
        );
    }
}
