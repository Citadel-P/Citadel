//! Backups service construction.
use super::RuntimeContext;
use citadel_adapters::external::backups::restic::DockerResticBackupExecutor;
use citadel_adapters::external::backups::system_recovery::{
    PgDumpSystemBackupBuilder, RecoveryAssets,
};
use citadel_adapters::persistence::postgres::backups::PostgresBackupPersistence;
use citadel_adapters::persistence::postgres::backups::secrets::PostgresBackupSecretResolver;
use citadel_adapters::persistence::postgres::backups::source_planner::PostgresBackupSourcePlanner;
use citadel_adapters::persistence::postgres::licensing::store::PostgresLicenseEntitlementService;
use citadel_adapters::security::identity::backup_authorization::IdentityBackupRunAuthorizer;
use citadel_adapters::security::identity::crypto::AesGcmSecretProtector;
use citadel_backups::BackupService;
use citadel_git::GitRepositoryExecutionService;
use citadel_identity::IdentityService;
use citadel_server::config::Config;
use std::sync::Arc;

pub(super) fn build(
    config: &Config,
    runtime: &RuntimeContext,
    secret_protector: &Arc<AesGcmSecretProtector>,
    identity: &Arc<IdentityService>,
    git_execution: &Arc<GitRepositoryExecutionService>,
    entitlements: &Arc<PostgresLicenseEntitlementService>,
) -> Result<Arc<BackupService>, Box<dyn std::error::Error>> {
    let RuntimeContext {
        pool,
        agent,
        edge_registry,
        realtime_hub,
        container_runtime,
        ..
    } = runtime;
    let options = &config.execution.backups;
    let backups = Arc::new(
        BackupService::new(
            Arc::new(
                PostgresBackupPersistence::new(pool.clone()).with_lease_options(
                    options.repository_lease_seconds,
                    options.source_lease_seconds,
                    options.settings.default_timeout.as_secs() as i32,
                ),
            ),
            Arc::new(
                DockerResticBackupExecutor::new(
                    config.execution.tools.docker.clone(),
                    config.execution.restic_image.clone(),
                    Arc::new(PostgresBackupSecretResolver::new(
                        pool.clone(),
                        secret_protector.clone(),
                    )?),
                    options.maximum_log_bytes,
                    pool.clone(),
                )
                .with_restic(config.execution.tools.restic.clone())
                .with_settings(options.settings.clone())
                .with_agent(agent.clone())
                .with_edge(edge_registry.clone()),
            ),
            Arc::new(
                PostgresBackupSourcePlanner::new(pool.clone())
                    .with_runtime(container_runtime.clone())
                    .with_git_execution(Arc::clone(git_execution))
                    .with_system_builder(Arc::new(
                        PgDumpSystemBackupBuilder::new(
                            pool.clone(),
                            config.database_url.clone(),
                            config.execution.tools.pg_dump.clone(),
                            options.settings.working_directory.join("citadel-system"),
                            options.maximum_log_bytes,
                        )
                        .with_recovery_assets(RecoveryAssets {
                            core_data_path: options.core_data_path.clone(),
                            jwt_key_is_external: config.identity.jwt_key_is_external,
                            encryption_key_is_external: config.identity.encryption_key_is_external,
                        }),
                    )),
            ),
            chrono::Duration::minutes(10),
            Arc::new(IdentityBackupRunAuthorizer::new(Arc::clone(identity))),
        )
        .with_repository_operation_lease(chrono::Duration::seconds(
            options.settings.default_timeout.as_secs() as i64 * 2
                + i64::from(options.repository_lease_seconds),
        ))
        .with_entitlements(entitlements.clone())
        .with_change_notifier(citadel_server::realtime::change_callback(
            realtime_hub.clone(),
            "BackupPolicy",
        )),
    );
    Ok(backups)
}
