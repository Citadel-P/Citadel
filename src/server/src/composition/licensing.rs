//! Licensing service construction.
use citadel_adapters::{
    persistence::postgres::licensing::store::{
        PostgresLicenseEntitlementService, PostgresLicenseStore,
    },
    security::licensing::verifier::Ed25519LicenseVerifier,
};
use citadel_licensing::{LicenseService, LicenseTransitionMonitor};
use citadel_server::{application_info_http, license_realtime, realtime::RealtimeHub};
use sqlx::PgPool;
use std::sync::Arc;
pub(super) struct LicensingComponents {
    pub licenses: Arc<LicenseService>,
    pub entitlements: Arc<PostgresLicenseEntitlementService>,
    pub license_transition_monitor: LicenseTransitionMonitor,
    pub license_realtime_hub: license_realtime::LicenseRealtimeHub,
}

pub(super) fn build(pool: &PgPool, realtime_hub: &Option<RealtimeHub>) -> LicensingComponents {
    let license_store = Arc::new(PostgresLicenseStore::new(pool.clone()));
    let license_verifier = Arc::new(Ed25519LicenseVerifier::default());
    let entitlements = Arc::new(PostgresLicenseEntitlementService::with(
        license_store.clone(),
        license_verifier.clone(),
    ));
    let license_realtime_hub =
        license_realtime::LicenseRealtimeHub::default().with_realtime(realtime_hub.clone());
    let licenses = Arc::new(
        LicenseService::new(
            license_store.clone(),
            license_verifier.clone(),
            Arc::new(chrono::Utc::now),
            application_info_http::application_info().version.to_owned(),
        )
        .with_notifier(Arc::new(license_realtime_hub.clone())),
    );
    let license_transition_monitor =
        LicenseTransitionMonitor::new(license_store, license_verifier, Arc::new(chrono::Utc::now));
    LicensingComponents {
        licenses,
        entitlements,
        license_transition_monitor,
        license_realtime_hub,
    }
}
