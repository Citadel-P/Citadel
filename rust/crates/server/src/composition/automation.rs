//! Automation service construction.
use super::RuntimeContext;
use citadel_adapters::{
    persistence::postgres::{
        alerts::PostgresAlertRepository, automation::PostgresAutomationRepository,
        licensing::store::PostgresLicenseEntitlementService,
    },
    security::identity::automation_token::IdentityAutomationRunTokenIssuer,
};
use citadel_automation::{AutomationRuntimeConfig, AutomationService};
use citadel_identity::IdentityService;
use citadel_server::config::Config;
use std::{sync::Arc, time::Duration};

pub(super) fn build(
    config: &Config,
    runtime: &RuntimeContext,
    identity: &Arc<IdentityService>,
    entitlements: &Arc<PostgresLicenseEntitlementService>,
    alert_store: &Arc<PostgresAlertRepository>,
) -> Result<Arc<AutomationService>, Box<dyn std::error::Error>> {
    let RuntimeContext {
        pool,
        cancellation,
        dynamic_tasks,
        realtime_hub,
        ..
    } = runtime;
    let automation = Arc::new(
        AutomationService::new(
            std::sync::Arc::new(citadel_processes::SystemProcess),
            std::sync::Arc::new(
                citadel_adapters::filesystem::automation_workspace::LocalAutomationWorkspace,
            ),
            Arc::new(
                citadel_server::tasks::automation::TrackedAutomationTasks::new(
                    dynamic_tasks.clone(),
                ),
            ),
            cancellation.clone(),
            Arc::new(PostgresAutomationRepository::new(pool.clone())),
            Arc::new(IdentityAutomationRunTokenIssuer::new(Arc::clone(identity))),
            AutomationRuntimeConfig {
                deno_path: config.execution.automation.deno.clone(),
                work_root: config.execution.automation.work_root.clone(),
                internal_base_url: config.execution.automation.internal_base_url.clone(),
                endpoint_catalog_json: citadel_server::automation_endpoint_catalog_json(),
                maximum_log_bytes: config.execution.automation.maximum_log_bytes,
                stale_after: Duration::from_secs(10 * 60),
            },
        )
        .with_options(config.automation)?
        .with_sandbox(
            config.execution.automation.cache_root.clone(),
            config.execution.automation.allow_net.clone(),
        )
        .with_entitlements(entitlements.clone())
        .with_alerts(alert_store.clone())
        .with_change_notifier(citadel_server::realtime::change_callback(
            realtime_hub.clone(),
            "AutomationAction",
        )),
    );
    Ok(automation)
}
