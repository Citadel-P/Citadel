use crate::ActivityAccess;
use crate::ActivityError;
use crate::ActivityResourceType;
use crate::read_models::*;
use futures_util::future::BoxFuture;
use uuid::Uuid;

pub trait WebhookActivitySink: Send + Sync {
    fn record_webhook(
        &self,
        resource_type: ActivityResourceType,
        id: Uuid,
        details: crate::WebhookActivityDetails,
    ) -> BoxFuture<'_, Result<(), ActivityError>>;
}

pub trait ActivityQueryStore: Send + Sync {
    fn get_authorized<'a>(
        &'a self,
        principal: &'a ActivityAccess,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Option<ActivityRecord>, ActivityError>>;

    fn list_authorized<'a>(
        &'a self,
        principal: &'a ActivityAccess,
        filter: ValidatedActivityFilter,
    ) -> BoxFuture<'a, Result<PagedActivityRecords, ActivityError>>;
}

pub trait VolumeDownloadActivitySink: Send + Sync {
    fn record_volume_download(
        &self,
        actor: citadel_primitives::ActorId,
        platform: Uuid,
        details: crate::VolumeContentDownloaded,
    ) -> BoxFuture<'_, Result<(), ActivityError>>;
}
