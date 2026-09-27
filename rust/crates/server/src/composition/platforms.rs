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
            .with_notifier(move |claim, notice, completion_patches| {
                if let Some(hub) = &container_hub {
                    let mut platforms = std::collections::BTreeMap::<_, Vec<_>>::new();
                    match notice {
                        citadel_platforms::containers::ContainerMutationNotice::Claimed
                        | citadel_platforms::containers::ContainerMutationNotice::Released => {
                            let control_state = if notice
                                == citadel_platforms::containers::ContainerMutationNotice::Claimed
                            {
                                "Processing"
                            } else {
                                "Idle"
                            };
                            for target in &claim.targets {
                                platforms.entry(target.platform_id).or_default().push(
                                    citadel_platforms::containers::ContainerStatePatch {
                                        id: target.id,
                                        platform_id: target.platform_id,
                                        container_id: target.docker_id.clone(),
                                        state: None,
                                        control_state: Some(control_state.into()),
                                        updated: None,
                                        docker_node_id: target.node_id.clone(),
                                    },
                                );
                            }
                        }
                        citadel_platforms::containers::ContainerMutationNotice::Completed => {
                            for patch in completion_patches {
                                platforms
                                    .entry(patch.platform_id)
                                    .or_default()
                                    .push(patch.clone());
                            }
                        }
                    }
                    for (platform, patches) in platforms {
                        hub.publish_container_state_patches(platform, &patches);
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
