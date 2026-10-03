use super::*;
use citadel_activities::WebhookActivityDetails;
use citadel_activities::WebhookActivitySink;

impl WebhookActivitySink for PostgresActivityStore {
    fn record_webhook(
        &self,
        resource_type: ActivityResourceType,
        id: Uuid,
        details: WebhookActivityDetails,
    ) -> BoxFuture<'_, Result<(), ActivityError>> {
        Box::pin(async move {
            let query = match resource_type {
                ActivityResourceType::GitRepository => {
                    "SELECT name,NULL::uuid platformid FROM gitrepositories WHERE id=$1"
                }
                ActivityResourceType::Build => {
                    "SELECT name,NULL::uuid platformid FROM buildprojects WHERE id=$1"
                }
                ActivityResourceType::Stack => {
                    "SELECT s.name,r.platformid FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE s.id=$1"
                }
                ActivityResourceType::SwarmService => {
                    "SELECT name,platformid FROM swarmservices WHERE id=$1"
                }
                _ => {
                    return Err(ActivityError::Validation(
                        "Unsupported webhook activity resource.".into(),
                    ));
                }
            };
            let mut tx = self.pool.begin().await.map_err(activity_storage)?;
            let Some(row) = sqlx::query(query)
                .bind(id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(activity_storage)?
            else {
                return Ok(()); // An unknown target must not create a phantom resource.
            };
            let event = ActivityEvent::new_webhook_event(
                id,
                row.try_get("name").map_err(activity_storage)?,
                row.try_get("platformid").map_err(activity_storage)?,
                resource_type,
                details,
                chrono::Utc::now(),
            )
            .map_err(activity_storage)?;
            insert_activity(&mut tx, &event)
                .await
                .map_err(activity_storage)?;
            tx.commit().await.map_err(activity_storage)
        })
    }
}
