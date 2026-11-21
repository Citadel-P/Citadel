//! Share the authoritative eligibility read for callers admitted before the read begins.
use super::{LicenseCapability, PostgresLicenseEntitlementService};
use std::sync::{Arc, Mutex};
use tokio::sync::OnceCell;

type Check = OnceCell<Result<bool, String>>;
#[derive(Default)]
pub(super) struct AdmissionChecks(Mutex<Option<Arc<Check>>>);

impl AdmissionChecks {
    pub(super) async fn check(
        &self,
        service: &PostgresLicenseEntitlementService,
    ) -> Result<bool, String> {
        let group = self
            .0
            .lock()
            .expect("entitlement checks")
            .get_or_insert_with(Arc::default)
            .clone();
        group
            .get_or_init(|| async {
                // Give concurrently ready admissions one turn to join. Seal the group
                // before SQL starts so every caller observes a read after its own arrival.
                tokio::task::yield_now().await;
                {
                    let mut pending = self.0.lock().expect("entitlement checks");
                    // A cancelled initializer may be retried after a new group has opened.
                    if pending
                        .as_ref()
                        .is_some_and(|current| Arc::ptr_eq(current, &group))
                    {
                        pending.take();
                    }
                }
                service
                    .enabled(LicenseCapability::AutomatedOperations)
                    .await
                    .map_err(|err| err.to_string())
            })
            .await
            .clone()
    }
}
