use crate::*;
use chrono::{DateTime, Utc};
use uuid::Uuid;
pub trait LicenseVerifier: Send + Sync {
    fn verify(
        &self,
        raw_license: &str,
        identity: &CitadelInstanceIdentity,
        now: DateTime<Utc>,
    ) -> LicenseVerificationResult;
}

pub trait LicenseStateNotifier: Send + Sync {
    fn license_state_changed(&self, instance_id: Uuid);
}

#[derive(Debug, Default)]
pub(crate) struct NoopLicenseStateNotifier;

impl LicenseStateNotifier for NoopLicenseStateNotifier {
    fn license_state_changed(&self, _instance_id: Uuid) {}
}
pub trait LicenseClock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}
impl<F: Fn() -> DateTime<Utc> + Send + Sync> LicenseClock for F {
    fn now(&self) -> DateTime<Utc> {
        self()
    }
}
