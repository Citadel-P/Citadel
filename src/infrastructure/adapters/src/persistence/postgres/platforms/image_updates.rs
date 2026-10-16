use citadel_platforms::{
    RuntimeCapabilityError, RuntimeErrorKind, image_update_store::*, image_updates::*,
};
use futures_util::future::BoxFuture;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub struct PostgresImageUpdateStore(pub PgPool);
impl ImageUpdateStore for PostgresImageUpdateStore {
    fn candidates(
        &self,
        after: Uuid,
    ) -> BoxFuture<'_, Result<Vec<ImageUpdateCandidate>, RuntimeCapabilityError>> {
        Box::pin(async move {
            let rows = sqlx::query("SELECT i.id,i.platformid,i.dockerimageid,i.tags,i.registryid FROM images i JOIN platforms p ON p.id=i.platformid WHERE i.id>$1 AND p.status='Online' AND p.platformdescriptor->>'$type' IN ('Docker','DockerStandalone') AND i.controlstate='Idle' AND json_array_length(i.tags)>0 ORDER BY i.id LIMIT 100")
                .bind(after).fetch_all(&self.0).await.map_err(storage)?;
            rows.into_iter()
                .map(|r| {
                    Ok(ImageUpdateCandidate {
                        id: r.get("id"),
                        platform: r.get("platformid"),
                        image: r.get("dockerimageid"),
                        registry_id: r.get("registryid"),
                        tags: serde_json::from_value(r.get("tags"))
                            .map_err(|_| failure("Invalid image inventory tags."))?,
                    })
                })
                .collect()
        })
    }
    fn load<'a>(
        &'a self,
        platform: Uuid,
        images: &'a [String],
        registries: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<StoredImageUpdate>, RuntimeCapabilityError>> {
        Box::pin(async move {
            let rows = sqlx::query("SELECT i.dockerimageid,o.registrykey,o.registryrevision,o.result FROM imageupdateobservations o JOIN images i ON i.id=o.imageid WHERE i.platformid=$1 AND i.dockerimageid=ANY($2) AND o.registrykey=ANY($3) AND i.tags::jsonb ? o.reference")
                .bind(platform).bind(images).bind(registries).fetch_all(&self.0).await.map_err(storage)?;
            rows.into_iter()
                .map(|r| {
                    Ok(StoredImageUpdate {
                        image: r.get("dockerimageid"),
                        registry: r.get("registrykey"),
                        revision: r.get("registryrevision"),
                        update: serde_json::from_value(r.get("result"))
                            .map_err(|_| failure("Invalid image update observation."))?,
                    })
                })
                .collect()
        })
    }
    fn save<'a>(
        &'a self,
        platform: Uuid,
        image: &'a str,
        registry: &'a ImageUpdateRegistry,
        result: &'a ImageTagUpdate,
    ) -> BoxFuture<'a, Result<bool, RuntimeCapabilityError>> {
        Box::pin(async move {
            let payload = serde_json::to_value(result)
                .map_err(|_| failure("Invalid image update observation."))?;
            // Lock inventory identity while recording the observation. An inventory
            // writer can then remove the tag/image; readers always join current tags.
            let changed = sqlx::query("WITH current_image AS (SELECT id FROM images WHERE platformid=$1 AND dockerimageid=$2 AND tags::jsonb ? $3 FOR SHARE) INSERT INTO imageupdateobservations(imageid,reference,registrykey,registryrevision,attemptedat,result) SELECT id,$3,$4,$5,$6,$7 FROM current_image ON CONFLICT(imageid,reference,registrykey) DO UPDATE SET registryrevision=EXCLUDED.registryrevision,attemptedat=EXCLUDED.attemptedat,result=EXCLUDED.result WHERE imageupdateobservations.attemptedat<=EXCLUDED.attemptedat")
                .bind(platform).bind(image).bind(&result.reference).bind(registry.id).bind(&registry.revision).bind(result.last_attempt_at).bind(payload).execute(&self.0).await.map_err(storage)?.rows_affected()>0;
            Ok(changed)
        })
    }
}
fn failure(message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Remote, message, false)
}
fn storage(error: sqlx::Error) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Remote, error.to_string(), true)
}
