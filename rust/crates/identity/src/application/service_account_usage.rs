use chrono::{DateTime, Utc};
use futures_util::future::BoxFuture;
use uuid::Uuid;

use crate::IdentityError;

pub trait ServiceAccountLastUsedTracker: Send + Sync {
    fn track(&self, credential_id: Uuid, used_at: DateTime<Utc>);
}

pub trait ServiceAccountLastUsedStore: Send + Sync {
    fn update_service_account_last_used(
        &self,
        credential_id: Uuid,
        used_at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), IdentityError>>;
}

#[derive(Debug, Default)]
pub struct NoopServiceAccountLastUsedTracker;

impl ServiceAccountLastUsedTracker for NoopServiceAccountLastUsedTracker {
    fn track(&self, _credential_id: Uuid, _used_at: DateTime<Utc>) {}
}
