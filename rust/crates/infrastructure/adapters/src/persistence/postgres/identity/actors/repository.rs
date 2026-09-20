use citadel_identity::ActorType;
use citadel_identity::{ActorDetails, ActorRepository, IdentityError};
use futures_util::future::BoxFuture;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct PostgresActorRepository {
    pool: PgPool,
}

impl PostgresActorRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl ActorRepository for PostgresActorRepository {
    fn get(&self, id: Uuid) -> BoxFuture<'_, Result<ActorDetails, IdentityError>> {
        Box::pin(async move { read(&self.pool, id).await })
    }

    fn set_enabled(
        &self,
        id: Uuid,
        enabled: bool,
    ) -> BoxFuture<'_, Result<ActorDetails, IdentityError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            // Use the same serialization lock as User/Team/Role administration;
            // two simultaneous disables must not remove the last administrator.
            crate::persistence::postgres::identity::users::repository::lock_identity_mutations(
                &mut tx,
            )
            .await?;
            let mut actor = read(&mut *tx, id).await?;
            actor.set_enabled(enabled)?;
            sqlx::query("UPDATE actors SET isenabled=$2 WHERE id=$1")
                .bind(id)
                .bind(enabled)
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            if !enabled {
                crate::persistence::postgres::identity::users::repository::ensure_enabled_administrator_remains(&mut tx).await?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(actor)
        })
    }
}

async fn read<'e>(
    executor: impl sqlx::Executor<'e, Database = sqlx::Postgres>,
    id: Uuid,
) -> Result<ActorDetails, IdentityError> {
    if id.is_nil() {
        return Err(IdentityError::Validation(
            "Actor ID must not be empty.".into(),
        ));
    }
    let row = sqlx::query("SELECT a.id,a.type,a.isenabled,COALESCE(u.name,t.name,s.name,CASE WHEN a.type='System' THEN 'System' ELSE 'Unknown' END) AS name FROM actors a LEFT JOIN users u ON u.actorid=a.id LEFT JOIN teams t ON t.actorid=a.id LEFT JOIN serviceaccounts s ON s.actorid=a.id WHERE a.id=$1")
        .bind(id).fetch_optional(executor).await.map_err(storage)?.ok_or(IdentityError::NotFound)?;
    let kind: String = row.try_get("type").map_err(storage)?;
    Ok(ActorDetails {
        id,
        name: row.try_get("name").map_err(storage)?,
        is_enabled: row.try_get("isenabled").map_err(storage)?,
        actor_type: ActorType::from_database_str(&kind)
            .ok_or_else(|| IdentityError::Storage("Invalid persisted Actor type.".into()))?,
    })
}

fn storage(error: sqlx::Error) -> IdentityError {
    IdentityError::Storage(error.to_string())
}
