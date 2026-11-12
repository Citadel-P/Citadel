//! Progress notifications are delivered after commit; payloads contain only IDs.
use super::*;
const CHANNEL: &str = "citadel_resource_copies";

pub(super) async fn publish(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT pg_notify($1,$2)")
        .bind(CHANNEL)
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(crate) async fn restore_changed(
    tx: &mut Transaction<'_, Postgres>,
    restore: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT pg_notify($1,copyid::text) FROM resourcecopyrestores WHERE restoreid=$2")
        .bind(CHANNEL)
        .bind(restore)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

use citadel_runtime::notifications::{NotificationSource, RuntimeNotification};

pub struct PostgresCopyChanges(Box<dyn NotificationSource>);
impl PostgresCopyChanges {
    pub fn from_notifications(source: Box<dyn NotificationSource>) -> Self {
        Self(source)
    }
    pub async fn connect(pool: &PgPool) -> Result<Self, sqlx::Error> {
        let source =
            super::super::notifications::PostgresNotifications::connect(pool, CHANNEL).await?;
        Ok(Self::from_notifications(Box::new(source)))
    }
}
impl CopyChangeSource for PostgresCopyChanges {
    fn next(&mut self) -> BoxFuture<'_, Result<Option<Uuid>, CopyError>> {
        Box::pin(async move {
            loop {
                match self.0.recv().await.map_err(storage_error)? {
                    RuntimeNotification::ResourceCopy(id) => return Ok(Some(id)),
                    RuntimeNotification::Resync => return Ok(None),
                    _ => {}
                }
            }
        })
    }
}
