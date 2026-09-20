//! Platforms service construction.
use super::RuntimeContext;
use citadel_adapters::{
    connectors::routing::{
        platforms::registration::PlatformRegistrationRuntimeRouter,
        volumes::content::VolumeContentAdapter,
    },
    persistence::postgres::platforms::{
        PostgresPlatformReader, registration::PostgresPlatformRegistrationRepository,
    },
};
use citadel_identity::IdentityService;
use citadel_platforms::{PlatformReadService, PlatformRegistrationService};
use citadel_server::{api::routes::platforms as platforms_http, config::Config};
use std::sync::Arc;
pub(super) struct PlatformComponents {
    pub platform_state: platforms_http::PlatformsHttpState,
    pub platform_reads: Arc<PlatformReadService>,
}

pub(super) fn build(
    config: &Config,
    runtime: &RuntimeContext,
    identity: &Arc<IdentityService>,
    registries: &Arc<dyn citadel_registries::RegistryRepository>,
) -> PlatformComponents {
    let RuntimeContext {
        pool,
        docker,
        agent,
        edge_registry,
        realtime_hub,
        container_runtime,
        ..
    } = runtime;
    let platform_reads = Arc::new(PlatformReadService::new(Arc::new(
        PostgresPlatformReader::new(pool.clone()),
    )));
    let platform_registrations = Arc::new(PlatformRegistrationService::new(
        Arc::new(PostgresPlatformRegistrationRepository::new(pool.clone())),
        Arc::new(PlatformRegistrationRuntimeRouter::new(
            docker.clone(),
            agent.clone(),
        )),
    ));
    let container_hub = realtime_hub.clone();
    let container_mutations = Arc::new(
        container_runtime
            .clone()
            .into_service(Arc::new(
                citadel_server::tasks::platforms::TrackedContainerTasks::new(
                    runtime.dynamic_tasks.clone(),
                ),
            ))
            .with_notifier(move |claim| {
                if let Some(hub) = &container_hub {
                    for target in &claim.targets {
                        hub.publish_runtime_change(
                            target.platform_id,
                            "container",
                            "update",
                            &target.docker_id,
                        );
                    }
                    for id in &claim.deployment_ids {
                        hub.publish_resource_change("Deployment", *id, "updated");
                    }
                    for id in &claim.stack_ids {
                        hub.publish_resource_change("Stack", *id, "updated");
                    }
                }
            }),
    );
    let platform_state = platforms_http::PlatformsHttpState {
        tasks: runtime.dynamic_tasks.clone(),
        volume_content: Arc::new(VolumeContentAdapter::new(
            pool.clone(),
            docker.clone(),
            agent.clone(),
            edge_registry.clone(),
            config.execution.volume_helper_image.clone(),
            runtime.dynamic_tasks.clone(),
        )),
        containers: container_mutations.clone(),
        identity: Arc::clone(identity),
        platforms: Arc::clone(&platform_reads),
        registrations: platform_registrations,
        pool: pool.clone(),
        registries: Arc::clone(registries),
        platform_metadata: Arc::new(
            citadel_adapters::persistence::postgres::platforms::PostgresPlatformMetadataRepository::new(
                pool.clone(),
            ),
        ),
        docker: docker.clone(),
        agent: agent.clone(),
        edge: edge_registry.clone(),
        realtime: realtime_hub.clone(),
        stats_sample_max_age: config
            .probe_interval
            .saturating_mul(3)
            .max(std::time::Duration::from_secs(30)),
    };
    PlatformComponents {
        platform_state,
        platform_reads,
    }
}
