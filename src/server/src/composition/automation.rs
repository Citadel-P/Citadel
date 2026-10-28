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
                citadel_adapters::filesystem::automation_workspace::LocalAutomationWorkspace::new(
                    dynamic_tasks.clone(),
                ),
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
        .with_remote(
            Arc::new(
                citadel_adapters::external::automation::AgentAutomationExecutor::new(
                    pool.clone(),
                    runtime.agent.clone(),
                    runtime.edge_registry.clone(),
                    identity.clone(),
                    config.transport.edge_agent_public_url.scheme() == "https",
                ),
            ),
            config.transport.public_url.to_string(),
        )
        .with_options(config.automation)?
        .with_sandbox(
            config.execution.automation.cache_root.clone(),
            config.execution.automation.allow_net.clone(),
        )
        .with_entitlements(entitlements.clone())
        .with_alerts(alert_store.clone())
        .with_change_notifier({
            let hub = realtime_hub.clone();
            Arc::new(move |action_id| {
                if let Some(hub) = &hub {
                    hub.publish_resource_change(
                        "AutomationAction",
                        action_id.unwrap_or_default(),
                        "resourceChanged",
                    );
                }
            })
        }),
    );
    Ok(automation)
}
