//! Registry credentials and image mutation claims/projections.
use citadel_platforms::{
    RuntimeCapabilityError, RuntimeErrorKind, RuntimeImageSummary, image_mutations::*,
};
use futures_util::future::BoxFuture;
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;
pub struct PostgresImageMutationStore(pub PgPool);
impl ImageMutationStore for PostgresImageMutationStore {
    fn prepare_pull<'a>(
        &'a self,
        registry: Uuid,
        reference: &'a str,
    ) -> BoxFuture<'a, Result<PreparedImagePull, RuntimeCapabilityError>> {
        Box::pin(prepare(&self.0, registry, reference))
    }
    fn persist_pull<'a>(
        &'a self,
        platform: Uuid,
        registry: Uuid,
        image: &'a RuntimeImageSummary,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(persist(&self.0, platform, registry, image))
    }
    fn claim_deletion<'a>(
        &'a self,
        platform: Uuid,
        ids: &'a [String],
    ) -> BoxFuture<'a, Result<ImageDeletionClaim, RuntimeCapabilityError>> {
        Box::pin(claim_deletion(&self.0, platform, ids))
    }
    fn complete_deletion<'a>(
        &'a self,
        platform: Uuid,
        claim: &'a ImageDeletionClaim,
        observed: Option<&'a ImageDeletionObservation>,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(complete_deletion(&self.0, platform, claim, observed))
    }
}
async fn claim_deletion(
    pool: &PgPool,
    platform_id: Uuid,
    ids: &[String],
) -> Result<ImageDeletionClaim, RuntimeCapabilityError> {
    let mut tx = pool.begin().await.map_err(storage)?;
    sqlx::query("SET LOCAL lock_timeout = '5s'")
        .execute(&mut *tx)
        .await
        .map_err(storage)?;
    let rows = sqlx::query("SELECT id,dockerimageid,controlstate,controlstartedat,rowversion FROM images WHERE platformid=$1 AND dockerimageid=ANY($2) ORDER BY id FOR UPDATE")
        .bind(platform_id).bind(ids).fetch_all(&mut *tx).await.map_err(storage)?;
    if rows.len() != ids.len() {
        return Err(error(
            RuntimeErrorKind::NotFound,
            "One or more images were not found on this Platform.",
        ));
    }
    let started = chrono::Utc::now().timestamp_millis();
    // A prior process can die mid-operation. Expired claims are released only
    // for an explicit new request, never by replaying a destructive command.
    if rows.iter().any(|row| {
        row.get::<Option<String>, _>("controlstate").as_deref() == Some("Processing")
            && row
                .get::<Option<i64>, _>("controlstartedat")
                .is_none_or(|at| at > started - 300_000)
    }) {
        return Err(error(
            RuntimeErrorKind::Conflict,
            "An Image is already processing another operation.",
        ));
    }
    let claimed: Vec<Uuid> = rows.iter().map(|row| row.get("id")).collect();
    sqlx::query("UPDATE images SET controlstate='Processing',controlstartedat=$2,rowversion=rowversion+1 WHERE id=ANY($1)")
        .bind(&claimed).bind(started).execute(&mut *tx).await.map_err(storage)?;
    tx.commit().await.map_err(storage)?;

    Ok(ImageDeletionClaim {
        ids: claimed,
        started,
        versions: rows
            .iter()
            .map(|row| row.get::<i64, _>("rowversion") + 1)
            .collect(),
    })
}
async fn complete_deletion(
    pool: &PgPool,
    platform_id: Uuid,
    claim: &ImageDeletionClaim,
    observed: Option<&ImageDeletionObservation>,
) -> Result<(), RuntimeCapabilityError> {
    let write = citadel_platforms::jobs::ProjectionWrite::begin(
        platform_id,
        None,
        citadel_platforms::jobs::ProjectionKind::Images,
    )
    .await;
    let mut tx = pool.begin().await.map_err(storage)?;
    sqlx::query("SET LOCAL lock_timeout = '5s'")
        .execute(&mut *tx)
        .await
        .map_err(storage)?;
    sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR NO KEY UPDATE")
        .bind(platform_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(storage)?;
    if let Some(observed) = observed.filter(|observed| observed.generation.matches(&write)) {
        let remaining: Vec<_> = observed
            .images
            .iter()
            .map(|image| image.id.as_str())
            .collect();
        sqlx::query("DELETE FROM images USING unnest($1::uuid[], $4::bigint[]) AS claimed(id, version) WHERE images.id=claimed.id AND images.rowversion=claimed.version AND controlstartedat=$2 AND NOT (dockerimageid=ANY($3))")
            .bind(&claim.ids).bind(claim.started).bind(&remaining).bind(&claim.versions).execute(&mut *tx).await.map_err(storage)?;
    }
    sqlx::query("UPDATE images SET controlstate='Idle',controlstartedat=NULL,rowversion=rowversion+1 WHERE id=ANY($1) AND controlstartedat=$2")
        .bind(&claim.ids).bind(claim.started).execute(&mut *tx).await.map_err(storage)?;
    sqlx::query("UPDATE platforms SET imagecount=(SELECT count(*) FROM images WHERE platformid=$1) WHERE id=$1")
        .bind(platform_id).execute(&mut *tx).await.map_err(storage)?;
    tx.commit().await.map_err(storage)?;
    write.committed();
    Ok(())
}
pub async fn prepare(
    pool: &PgPool,
    id: Uuid,
    reference: &str,
) -> Result<PreparedImagePull, RuntimeCapabilityError> {
    let mut reference = normalize_reference(reference)?;
    let row = sqlx::query("SELECT registryhost,configuration,status FROM registries WHERE id=$1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(db)?
        .ok_or_else(|| error(RuntimeErrorKind::NotFound, "Registry not found."))?;
    if row.get::<String, _>("status") == "Disabled" {
        return Err(error(RuntimeErrorKind::Conflict, "Registry is disabled."));
    }
    let host: String = row.get("registryhost");
    let cfg: Value = row.get("configuration");
    let kind = cfg.get("$type").and_then(Value::as_str).unwrap_or_default();
    let get = |fields: &[&str]| {
        fields
            .iter()
            .find_map(|f| cfg.get(*f).and_then(Value::as_str))
    };
    let host = host
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_ascii_lowercase();
    let default_registry = id == Uuid::from_u128(0x100);
    if !default_registry && !reference.starts_with(&format!("{host}/")) {
        let namespace = match kind {
            "DockerHub" => get(&["UserName", "userName", "username"]),
            "GitHub" => get(&["NameSpace", "nameSpace"]),
            _ => None,
        };
        reference = match namespace.filter(|v| !v.is_empty()) {
            Some(ns) => format!("{host}/{ns}/{reference}"),
            None => format!("{host}/{reference}"),
        };
    }
    let auth = crate::connectors::routing::deployments::registry_auth(id, &host, &cfg)
        .map_err(|_| failure("Registry configuration is invalid."))?;
    Ok((reference, auth))
}

fn normalize_reference(reference: &str) -> Result<String, RuntimeCapabilityError> {
    // Tags are case-sensitive. Preserve the requested reference; Docker validates
    // repository syntax instead of silently pulling a different tag.
    let mut reference = reference.trim().to_owned();
    if reference.is_empty()
        || reference.len() > 2048
        || reference.chars().any(char::is_whitespace)
        || reference.chars().any(char::is_control)
    {
        return Err(error(
            RuntimeErrorKind::InvalidRequest,
            "A valid image reference is required.",
        ));
    }
    if !reference.contains('@')
        && !reference
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .contains(':')
    {
        reference.push_str(":latest");
    }
    Ok(reference)
}

pub async fn persist(
    pool: &PgPool,
    platform: Uuid,
    registry: Uuid,
    image: &RuntimeImageSummary,
) -> Result<(), RuntimeCapabilityError> {
    let write = citadel_platforms::jobs::ProjectionWrite::begin(
        platform,
        None,
        citadel_platforms::jobs::ProjectionKind::Images,
    )
    .await;
    let mut tx = pool.begin().await.map_err(db)?;
    sqlx::query("SET LOCAL lock_timeout = '5s'")
        .execute(&mut *tx)
        .await
        .map_err(db)?;
    sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR NO KEY UPDATE")
        .bind(platform)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(|| {
            error(
                RuntimeErrorKind::NotFound,
                "Platform was removed during image pull.",
            )
        })?;
    sqlx::query("INSERT INTO images(id,containers,createdat,dockerimageid,name,platformid,registryid,size,tags,controlstate) VALUES($1,$2,to_timestamp($3),$4,$5,$6,$7,$8,$9,'Idle') ON CONFLICT(dockerimageid,platformid) DO UPDATE SET containers=EXCLUDED.containers,name=EXCLUDED.name,registryid=EXCLUDED.registryid,size=EXCLUDED.size,tags=EXCLUDED.tags,updatedat=now(),rowversion=images.rowversion+1")
        .bind(Uuid::now_v7()).bind(i32::try_from(image.containers).unwrap_or(i32::MAX)).bind(image.created as f64).bind(&image.id).bind(image.repo_tags.first().unwrap_or(&image.id)).bind(platform).bind(registry).bind(image.size as f64).bind(serde_json::json!(image.repo_tags)).execute(&mut *tx).await.map_err(db)?;
    sqlx::query("UPDATE platforms SET imagecount=(SELECT count(*) FROM images WHERE platformid=$1) WHERE id=$1").bind(platform).execute(&mut *tx).await.map_err(db)?;
    tx.commit().await.map_err(db)?;
    write.committed();
    Ok(())
}
fn error(kind: RuntimeErrorKind, message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(kind, message, false)
}
fn storage(error: sqlx::Error) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Remote, error.to_string(), false)
}

fn db(error: sqlx::Error) -> RuntimeCapabilityError {
    storage(error)
}
fn failure(message: &str) -> RuntimeCapabilityError {
    error(RuntimeErrorKind::Remote, message)
}
#[cfg(test)]
mod reference_tests {
    use super::*;

    #[tokio::test]
    async fn input_validation_precedes_storage_and_storage_errors_remain_internal() {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://unused@localhost/unused")
            .unwrap();
        pool.close().await;
        assert_eq!(
            prepare(&pool, Uuid::now_v7(), "nginx bad")
                .await
                .unwrap_err()
                .kind,
            RuntimeErrorKind::InvalidRequest
        );
        assert_eq!(
            prepare(&pool, Uuid::now_v7(), "nginx")
                .await
                .unwrap_err()
                .kind,
            RuntimeErrorKind::Remote
        );
    }

    #[test]
    fn pull_reference_preserves_case_sensitive_tags_and_defaults_only_untagged_images() {
        assert_eq!(
            normalize_reference(" nginx:ReleaseA ").unwrap(),
            "nginx:ReleaseA"
        );
        assert_eq!(
            normalize_reference("localhost:5000/nginx").unwrap(),
            "localhost:5000/nginx:latest"
        );
        assert_eq!(
            normalize_reference("nginx@sha256:abcdef").unwrap(),
            "nginx@sha256:abcdef"
        );
        assert_eq!(
            normalize_reference("nginx bad").unwrap_err().kind,
            RuntimeErrorKind::InvalidRequest
        );
    }
}
