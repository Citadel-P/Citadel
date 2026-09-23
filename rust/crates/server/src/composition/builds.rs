//! Builds service construction.
use super::RuntimeContext;
use citadel_adapters::{
    external::builds::executor::{
        AgentDockerBuildExecutor, LocalDockerBuildExecutor, PlatformBuildExecutor,
    },
    persistence::postgres::{
        alerts::PostgresAlertRepository,
        builds::{
            PostgresBuildRepository,
            credentials::{PostgresBuildRegistryCredentialResolver, PostgresBuildSecretResolver},
        },
        licensing::store::PostgresLicenseEntitlementService,
    },
    security::identity::crypto::AesGcmSecretProtector,
};
use citadel_builds::BuildService;
use citadel_git::GitRepositoryExecutionService;
use citadel_server::config::Config;
use std::sync::Arc;

pub(super) fn build(
    config: &Config,
    runtime: &RuntimeContext,
    secret_protector: &Arc<AesGcmSecretProtector>,
    git_execution: &Arc<GitRepositoryExecutionService>,
    entitlements: &Arc<PostgresLicenseEntitlementService>,
    alert_store: &Arc<PostgresAlertRepository>,
) -> Result<Arc<BuildService>, Box<dyn std::error::Error>> {
    let RuntimeContext {
        pool,
        agent,
        edge_registry,
        cancellation,
        dynamic_tasks,
        realtime_hub,
        ..
    } = runtime;
    let data_root = &config.execution.paths.data_root;
    let build_secrets = Arc::new(PostgresBuildSecretResolver::new(
        pool.clone(),
        secret_protector.clone(),
    )?);
    let build_registries = Arc::new(PostgresBuildRegistryCredentialResolver::new(pool.clone()));
    let local_builds = Arc::new(LocalDockerBuildExecutor::new(
        git_execution.clone(),
        config.execution.tools.docker.clone(),
        build_secrets.clone(),
        build_registries.clone(),
        4 * 1024 * 1024,
        pool.clone(),
    ));
    let agent_builds = agent.clone().map(|client| {
        AgentDockerBuildExecutor::new(
            data_root.join("git-repositories"),
            client,
            build_secrets.clone(),
            build_registries.clone(),
            4 * 1024 * 1024,
        )
    });
    let builds = Arc::new(
        BuildService::new(
            Arc::new(citadel_server::tasks::builds::TrackedBuildTasks::new(
                dynamic_tasks.clone(),
            )),
            cancellation.clone(),
            Arc::new(PostgresBuildRepository::new(pool.clone())),
            Arc::new(
                PlatformBuildExecutor::new(pool.clone(), local_builds, agent_builds)
                    .with_git_source(git_execution.clone())
                    .with_edge(AgentDockerBuildExecutor::new_edge(
                        data_root.join("git-repositories"),
                        edge_registry.clone(),
                        build_secrets,
                        build_registries,
                        4 * 1024 * 1024,
                    )),
            ),
            chrono::Duration::minutes(10),
        )
        .with_alerts(alert_store.clone())
        .with_change_notifier(citadel_server::realtime::change_callback(
            realtime_hub.clone(),
            "Build",
        ))
        .with_entitlements(entitlements.clone())
        .with_pool_change_notifier(citadel_server::realtime::change_callback(
            realtime_hub.clone(),
            "BuildAgentPool",
        ))
        .with_pool_checker(Arc::new(
            citadel_adapters::connectors::agent::build_pool_checker::AgentBuildPoolChecker {
                agent: agent.clone(),
                edge: edge_registry.clone(),
            },
        ))
        .with_log_notifier(citadel_server::realtime::build_log_callback(
            realtime_hub.clone(),
        )),
    );
    Ok(builds)
}
