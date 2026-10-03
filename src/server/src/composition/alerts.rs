//! Alerts service construction.
use super::RuntimeContext;
use citadel_adapters::external::alerts::shoutrrr::ShoutrrrAlertDelivery;
use citadel_adapters::persistence::postgres::alerts::PostgresAlertRepository;
use citadel_adapters::persistence::postgres::licensing::store::PostgresLicenseEntitlementService;
use citadel_alerts::AlertDeliveryService;
use citadel_server::config::Config;
use std::sync::Arc;
use std::time::Duration;
pub(super) struct AlertComponents {
    pub alert_store: Arc<PostgresAlertRepository>,
    pub alert_delivery: Arc<ShoutrrrAlertDelivery>,
    pub alert_deliveries: Arc<AlertDeliveryService>,
}

pub(super) fn build(
    config: &Config,
    runtime: &RuntimeContext,
    entitlements: &Arc<PostgresLicenseEntitlementService>,
) -> AlertComponents {
    let RuntimeContext {
        pool, realtime_hub, ..
    } = runtime;
    let alert_store = Arc::new(
        PostgresAlertRepository::new(pool.clone())
            .with_entitlements(entitlements.clone())
            .with_change_notifier(citadel_server::realtime::change_callback(
                realtime_hub.clone(),
                "Alert",
            )),
    );
    let alert_delivery = Arc::new(ShoutrrrAlertDelivery::new(
        config.execution.tools.shoutrrr.clone(),
        Duration::from_secs(15),
    ));
    let alert_deliveries = Arc::new(AlertDeliveryService::new(
        alert_store.clone(),
        alert_delivery.clone(),
    ));
    AlertComponents {
        alert_store,
        alert_delivery,
        alert_deliveries,
    }
}
