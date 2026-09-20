use crate::*;
use chrono::{DateTime, Utc};
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
pub trait LicenseStore: Send + Sync {
    fn load(&self, now: DateTime<Utc>) -> BoxFuture<'_, Result<LicenseSource, LicenseError>>;

    fn install<'a>(
        &'a self,
        expected_fingerprint: Option<&'a str>,
        installed: &'a InstalledLicense,
        actor_id: ActorId,
        info: &'a LicenseChange,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<bool, LicenseError>>;

    fn remove<'a>(
        &'a self,
        expected_fingerprint: &'a str,
        actor_id: ActorId,
        info: &'a LicenseChange,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<bool, LicenseError>>;

    fn persist_validation<'a>(
        &'a self,
        expected_fingerprint: &'a str,
        status: LicenseStatus,
        error_code: Option<&'a str>,
        transition_activity: Option<&'a LicenseChange>,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<LicenseValidationPersistence, LicenseError>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LicenseValidationPersistence {
    SourceChanged,
    Unchanged,
    Transitioned,
}
