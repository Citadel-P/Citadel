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
                    let mut platforms = std::collections::BTreeMap::<_, Vec<_>>::new();
                    for target in &claim.targets {
                        platforms
                            .entry(target.platform_id)
                            .or_default()
                            .push(target.docker_id.clone());
                    }
                    for (platform, ids) in platforms {
                        hub.publish_container_changes(platform, "update", &ids);
                    }
                    hub.publish_resource_changes("Deployment", &claim.deployment_ids);
                    hub.publish_resource_changes("Stack", &claim.stack_ids);
                }
            }),
    );
    let mut volume_content = VolumeContentAdapter::new(
        pool.clone(),
        docker.clone(),
        agent.clone(),
        edge_registry.clone(),
        config
            .execution
            .volume_helper_image
            .clone()
            .unwrap_or_else(|| config.execution.edge_agent.image.clone()),
        runtime.dynamic_tasks.clone(),
    );
    if config.execution.volume_helper_image.is_none() {
        volume_content =
            volume_content.with_core_container(config.execution.core_container_hostname.clone());
    }
    let platform_state = platforms_http::PlatformsHttpState {
        tasks: runtime.dynamic_tasks.clone(),
        volume_content: Arc::new(volume_content),
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
