use super::*;
use citadel_activities::VolumeContentDownloaded;
use citadel_primitives::ActorId;

impl PostgresActivityStore {
    pub async fn record_volume_download(
        &self,
        actor: ActorId,
        platform: Uuid,
        details: VolumeContentDownloaded,
    ) -> Result<(), IdentityError> {
        let event = ActivityEvent::volume_downloaded(platform, actor, details, chrono::Utc::now())
            .map_err(storage)?;
        let mut tx = self.pool.begin().await.map_err(storage)?;
        insert_activity(&mut tx, &event).await?;
        tx.commit().await.map_err(storage)
    }
}
