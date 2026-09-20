//! Backups service construction.
use super::RuntimeContext;
use citadel_adapters::external::backups::restic::DockerResticBackupExecutor;
use citadel_adapters::external::backups::system_recovery::PgDumpSystemBackupBuilder;
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
    let data_root = &config.execution.paths.data_root;
    let backups = Arc::new(
        BackupService::new(
            Arc::new(PostgresBackupPersistence::new(pool.clone())),
            Arc::new(
                DockerResticBackupExecutor::new(
                    config.execution.tools.docker.clone(),
                    config.execution.restic_image.clone(),
                    Arc::new(PostgresBackupSecretResolver::new(
                        pool.clone(),
                        secret_protector.clone(),
                    )?),
                    4 * 1024 * 1024,
                    pool.clone(),
                )
                .with_restic(config.execution.tools.restic.clone())
                .with_agent(agent.clone())
                .with_edge(edge_registry.clone()),
            ),
            Arc::new(
                PostgresBackupSourcePlanner::new(pool.clone())
                    .with_runtime(container_runtime.clone())
                    .with_git_execution(Arc::clone(git_execution))
                    .with_system_builder(Arc::new(PgDumpSystemBackupBuilder::new(
                        pool.clone(),
                        config.database_url.clone(),
                        config.execution.tools.pg_dump.clone(),
                        data_root.join("backups/citadel-system"),
                        1024 * 1024,
                    ))),
            ),
            chrono::Duration::minutes(10),
            Arc::new(IdentityBackupRunAuthorizer::new(Arc::clone(identity))),
        )
        .with_entitlements(entitlements.clone())
        .with_change_notifier(citadel_server::realtime::change_callback(
            realtime_hub.clone(),
            "BackupPolicy",
        )),
    );
    Ok(backups)
}
