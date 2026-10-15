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

pub struct PostgresCopyChanges(sqlx::postgres::PgListener);
impl PostgresCopyChanges {
    pub async fn connect(pool: &PgPool) -> Result<Self, sqlx::Error> {
        let dedicated = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy_with((*pool.connect_options()).clone().application_name(CHANNEL));
        let mut listener = sqlx::postgres::PgListener::connect_with(&dedicated).await?;
        listener.listen(CHANNEL).await?;
        Ok(Self(listener))
    }
}
impl CopyChangeSource for PostgresCopyChanges {
    fn next(&mut self) -> BoxFuture<'_, Result<Option<Uuid>, CopyError>> {
        Box::pin(async move {
            let Some(event) = self.0.try_recv().await.map_err(storage_error)? else {
                return Ok(None);
            };
            event.payload().parse().map(Some).map_err(storage_error)
        })
    }
}
