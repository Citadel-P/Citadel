use super::super::*;
use crate::persistence::postgres::platforms::history_admission::Admission;

pub(super) async fn read(
    pool: &PgPool,
    admission: &Admission,
    container: Uuid,
) -> Result<Vec<citadel_platforms::container_health::ContainerHealthTransition>, AuthorizedReadError>
{
    let mut read = admission
        .begin(pool)
        .await
        .map_err(AuthorizedReadError::Runtime)?;
    let rows = sqlx::query(include_str!("container_health_history.sql"))
        .bind(container)
        .fetch_all(read.connection())
        .await
        .map_err(crate::persistence::postgres::platforms::history_admission::storage)
        .map_err(AuthorizedReadError::Runtime)?;
    read.finish().await.map_err(AuthorizedReadError::Runtime)?;
    rows.into_iter()
        .map(|row| {
            Ok(
                citadel_platforms::container_health::ContainerHealthTransition {
                    observed_at: row.try_get("observedat").map_err(storage)?,
                    first_observed_at: row.try_get("firstobservedat").map_err(storage)?,
                    sample_count: row.try_get("samplecount").map_err(storage)?,
                    state: row.try_get("state").map_err(storage)?,
                    origin: row.try_get("origin").map_err(storage)?,
                    health: row
                        .try_get::<Option<
                            sqlx::types::Json<
                                citadel_platforms::container_health::ContainerHealthObservation,
                            >,
                        >, _>("health")
                        .map_err(storage)?
                        .map(|v| v.0),
                },
            )
        })
        .collect()
}
