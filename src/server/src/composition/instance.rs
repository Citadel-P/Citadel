use citadel_adapters::persistence::postgres::instance::PostgresInstanceRepository;
use citadel_identity::SystemClock;
use citadel_instance::{InstallationPolicy, InstanceService};
use citadel_server::config::Config;
use sqlx::PgPool;
use std::sync::Arc;

pub(super) fn build(config: &Config, pool: &PgPool) -> Arc<InstanceService> {
    Arc::new(
        InstanceService::new(
            Arc::new(PostgresInstanceRepository::new(pool.clone())),
            Arc::new(SystemClock),
        )
        .with_installation_policy(InstallationPolicy {
            running: config.instance_configuration.clone(),
            overrides: config.instance_overrides.clone(),
            update_checks_allowed: config.updates_enabled,
            minimum_mfa_policy: config.identity.mfa.policy,
            deployment_core_agent_requires_http: config.deployment_core_agent_requires_http(),
        }),
    )
}

/// Read only after migration/recovery admission, before constructing startup-bound consumers.
pub(crate) async fn load_instance_configuration(
    config: &Config,
) -> Result<citadel_instance::InstanceConfiguration, Box<dyn std::error::Error>> {
    use citadel_instance::InstanceRepository;
    let pool = super::connect_database(config).await?;
    let result = PostgresInstanceRepository::new(pool.clone())
        .installation_settings()
        .await;
    pool.close().await;
    Ok(result?.configuration)
}
