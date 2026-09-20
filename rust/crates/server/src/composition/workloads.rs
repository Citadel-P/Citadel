//! Workloads service construction.
use super::RuntimeContext;
use citadel_adapters::{
    connectors::routing::{
        deployments::DeploymentRuntimeRouter, stacks::StackRuntimeRouter,
        swarm_services::SwarmServiceRuntimeRouter,
    },
    filesystem::stacks::materializer::GitStackSourceMaterializer,
    persistence::postgres::{
        alerts::PostgresAlertRepository,
        deployments::{PostgresDeploymentRepository, bindings::PostgresDeploymentBindingResolver},
        licensing::store::PostgresLicenseEntitlementService,
        stacks::{
            PostgresStackRepository, bindings::PostgresStackBindingResolver,
            build_images::PostgresStackBuildImageResolver,
        },
        swarm_services::{
            PostgresSwarmServiceRepository, bindings::PostgresSwarmServiceBindingResolver,
        },
    },
    security::identity::crypto::AesGcmSecretProtector,
};
use citadel_deployments::DeploymentService;
use citadel_git::GitRepositoryExecutionService;
use citadel_server::config::Config;
use citadel_stacks::StackService;
use citadel_swarm_services::SwarmServiceService;
use std::sync::Arc;
pub(super) struct WorkloadComponents {
    pub deployments: Arc<DeploymentService>,
    pub swarm_services: Arc<SwarmServiceService>,
    pub stacks: Arc<StackService>,
    pub image_scanner: Arc<citadel_adapters::connectors::routing::images::scanner::ImageScanner>,
}

pub(super) fn build(
    config: &Config,
    runtime: &RuntimeContext,
    secret_protector: &Arc<AesGcmSecretProtector>,
    git_execution: &Arc<GitRepositoryExecutionService>,
    entitlements: &Arc<PostgresLicenseEntitlementService>,
    alert_store: &Arc<PostgresAlertRepository>,
) -> Result<WorkloadComponents, Box<dyn std::error::Error>> {
    let RuntimeContext {
        pool,
        docker,
        agent,
        edge_registry,
        cancellation,
        dynamic_tasks,
        realtime_hub,
        container_runtime,
        ..
    } = runtime;
    let image_cache = Arc::new(
        citadel_adapters::connectors::registries::digest_cache::ImageDigestCache::default(),
    );
    let swarm_runtime = SwarmServiceRuntimeRouter::new(pool.clone(), docker.clone(), agent.clone())
        .with_image_cache(image_cache.clone())
        .with_edge(edge_registry.clone());
    let stack_runtime = StackRuntimeRouter::new(pool.clone(), docker.clone(), agent.clone())
        .with_image_cache(image_cache.clone())
        .with_edge(edge_registry.clone());
    let image_scanner = Arc::new(
        citadel_adapters::connectors::routing::images::scanner::ImageScanner::new(
            pool.clone(),
            Arc::new(
                DeploymentRuntimeRouter::new(pool.clone(), docker.clone(), agent.clone())
                    .with_edge(edge_registry.clone()),
            ),
            image_cache.clone(),
        ),
    );
    let deployments = Arc::new(
        DeploymentService::new(
            Arc::new(
                citadel_server::tasks::deployments::TrackedDeploymentTasks::new(
                    dynamic_tasks.clone(),
                ),
            ),
            Arc::new(PostgresDeploymentRepository::new(pool.clone())),
            Arc::new(
                DeploymentRuntimeRouter::new(pool.clone(), docker.clone(), agent.clone())
                    .with_image_cache(image_cache.clone())
                    .with_edge(edge_registry.clone()),
            ),
            entitlements.clone(),
            cancellation.clone(),
        )
        .with_notifier(Arc::new(
            citadel_server::realtime::notifiers::DeploymentsRealtimeNotifier::new(
                realtime_hub.clone(),
            ),
        ))
        .with_binding_resolver(Arc::new(PostgresDeploymentBindingResolver::new(
            pool.clone(),
            secret_protector.clone(),
        )?))
        .with_adoption(Arc::new(
            citadel_adapters::persistence::postgres::deployments::PostgresContainerAdoption::new(
                pool.clone(),
                container_runtime.clone(),
                secret_protector.clone(),
                config.identity.secret_encryption_key.expose(),
            ),
        ))
        .with_alerts(alert_store.clone()),
    );
    let swarm_services = Arc::new(
        SwarmServiceService::new(
            Arc::new(
                citadel_server::tasks::swarm_services::TrackedSwarmServiceTasks::new(
                    dynamic_tasks.clone(),
                ),
            ),
            Arc::new(PostgresSwarmServiceRepository::new(pool.clone())),
            Arc::new(swarm_runtime.clone()),
            cancellation.clone(),
        )
        .with_image_digests(Arc::new(swarm_runtime.clone()))
        .with_adoption(Arc::new(
            citadel_adapters::persistence::postgres::swarm_services::PostgresSwarmServiceAdoption::new(
                pool.clone(),
                swarm_runtime.clone(),
                config.identity.secret_encryption_key.expose(),
            ),
        ))
        .with_entitlements(entitlements.clone())
        .with_notifier(Arc::new(
            citadel_server::realtime::notifiers::SwarmServicesRealtimeNotifier::new(realtime_hub.clone()),
        ))
        .with_binding_resolver(Arc::new(PostgresSwarmServiceBindingResolver::new(
            pool.clone(),
            secret_protector.clone(),
        )?))
        .with_alerts(alert_store.clone()),
    );
    let stacks = Arc::new(
        StackService::new(
            Arc::new(citadel_server::tasks::stacks::TrackedStackTasks::new(
                dynamic_tasks.clone(),
            )),
            Arc::new(PostgresStackRepository::new(pool.clone())),
            Arc::new(stack_runtime.clone()),
            Arc::new(PostgresStackBindingResolver::new(
                pool.clone(),
                secret_protector.clone(),
            )?),
            Arc::new(
                citadel_server::realtime::notifiers::StacksRealtimeNotifier::new(
                    realtime_hub.clone(),
                ),
            ),
            cancellation.clone(),
        )
        .with_source_materializer(Arc::new(GitStackSourceMaterializer::new(Arc::clone(
            git_execution,
        ))))
        .with_build_image_resolver(Arc::new(PostgresStackBuildImageResolver::new(pool.clone())))
        .with_update_scanner(Arc::new(
            citadel_adapters::connectors::routing::stacks::StackUpdateRuntime::new(
                stack_runtime.clone(),
                git_execution.clone(),
            ),
        ))
        .with_entitlements(entitlements.clone())
        .with_alerts(alert_store.clone()),
    );
    Ok(WorkloadComponents {
        deployments,
        swarm_services,
        stacks,
        image_scanner,
    })
}
