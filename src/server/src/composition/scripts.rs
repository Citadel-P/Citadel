//! Script service construction.
use super::RuntimeContext;
use citadel_adapters::{
    persistence::postgres::{
        alerts::PostgresAlertRepository, licensing::store::PostgresLicenseEntitlementService,
        scripts::PostgresScriptRepository,
    },
    security::identity::script_token::IdentityScriptRunTokenIssuer,
};
use citadel_identity::IdentityService;
use citadel_scripts::{ScriptRuntimeConfig, ScriptService};
use citadel_server::config::Config;
use std::{sync::Arc, time::Duration};

pub(super) fn build(
    config: &Config,
    runtime: &RuntimeContext,
    identity: &Arc<IdentityService>,
    entitlements: &Arc<PostgresLicenseEntitlementService>,
    alert_store: &Arc<PostgresAlertRepository>,
) -> Result<Arc<ScriptService>, Box<dyn std::error::Error>> {
    let RuntimeContext {
        pool,
        cancellation,
        dynamic_tasks,
        realtime_hub,
        ..
    } = runtime;
    let scripts = Arc::new(
        ScriptService::new(
            std::sync::Arc::new(citadel_processes::SystemProcess),
            std::sync::Arc::new(
                citadel_adapters::filesystem::script_workspace::LocalScriptWorkspace::new(
                    dynamic_tasks.clone(),
                ),
            ),
            Arc::new(citadel_server::tasks::scripts::TrackedScriptTasks::new(
                dynamic_tasks.clone(),
            )),
            cancellation.clone(),
            Arc::new(PostgresScriptRepository::new(pool.clone())),
            Arc::new(IdentityScriptRunTokenIssuer::new(Arc::clone(identity))),
            ScriptRuntimeConfig {
                deno_path: config.execution.scripts.deno.clone(),
                work_root: config.execution.scripts.work_root.clone(),
                internal_base_url: config.execution.scripts.internal_base_url.clone(),
                endpoint_catalog_json: citadel_server::script_endpoint_catalog_json(),
                maximum_log_bytes: config.execution.scripts.maximum_log_bytes,
                stale_after: Duration::from_secs(10 * 60),
            },
        )
        .with_remote(
            Arc::new(
                citadel_adapters::external::scripts::AgentScriptExecutor::new(
                    pool.clone(),
                    runtime.agent.clone(),
                    runtime.edge_registry.clone(),
                    identity.clone(),
                    config.transport.edge_agent_public_url.scheme() == "https",
                ),
            ),
            config.transport.public_url.to_string(),
        )
        .with_options(config.scripts)?
        .with_sandbox(
            config.execution.scripts.cache_root.clone(),
            config.execution.scripts.allow_net.clone(),
        )
        .with_entitlements(entitlements.clone())
        .with_alerts(alert_store.clone())
        .with_change_notifier({
            let hub = realtime_hub.clone();
            Arc::new(move |script_id| {
                if let Some(hub) = &hub {
                    hub.publish_resource_change(
                        "Script",
                        script_id.unwrap_or_default(),
                        "resourceChanged",
                    );
                }
            })
        }),
    );
    Ok(scripts)
}
