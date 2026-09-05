use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct LocalDockerTargetGuard {
    pool: PgPool,
}

impl LocalDockerTargetGuard {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub(crate) async fn require_local(&self, platform_id: Uuid) -> Result<(), String> {
        let row = sqlx::query("SELECT connectortype,status FROM platforms WHERE id=$1")
            .bind(platform_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| format!("Platform lookup failed: {error}"))?
            .ok_or_else(|| "The selected Platform was not found.".to_owned())?;
        let status: String = row
            .try_get("status")
            .map_err(|error| format!("Platform status could not be read: {error}"))?;
        if status != "Online" {
            return Err("The selected Platform is disconnected or unavailable.".to_owned());
        }
        let connector: String = row
            .try_get("connectortype")
            .map_err(|error| format!("Platform connector could not be read: {error}"))?;
        if connector != "Local" {
            return Err(format!(
                "Platform connector '{connector}' requires Agent execution; Core will not run this operation against its local Docker daemon."
            ));
        }
        Ok(())
    }

    pub(crate) const fn pool(&self) -> &PgPool {
        &self.pool
    }
}
