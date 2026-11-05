use citadel_adapters::{
    connectors::{
        agent::client::AgentClient,
        edge::EdgeRegistry,
        routing::{
            agent_updates::AgentUpdateRuntimeRouter, platforms::runtime::PlatformRuntimeRouter,
        },
    },
    external::updates::GithubReleaseArtifacts,
    persistence::postgres::{
        licensing::store::PostgresLicenseEntitlementService, updates::PostgresUpdateRepository,
    },
};
use citadel_server::{
    api::routes::platforms::PlatformsHttpState, config::Config,
    realtime::notifiers::UpdatesRealtimeNotifier,
};
use citadel_updates::AgentUpdateService;
use std::sync::Arc;

pub(crate) fn build_agents(
    config: &Config,
    pool: &sqlx::PgPool,
    docker: &citadel_docker::client::DockerClient,
    agent: Option<AgentClient>,
    edge: EdgeRegistry,
    platforms: &PlatformsHttpState,
    entitlements: Arc<PostgresLicenseEntitlementService>,
) -> Result<AgentUpdateService, citadel_updates::UpdateError> {
    let realtime = platforms.realtime.clone();
    let notifications = realtime.clone();
    let runtime = Arc::new(AgentUpdateRuntimeRouter {
        pool: pool.clone(),
        platforms: PlatformRuntimeRouter::new(
            pool.clone(),
            docker.clone(),
            agent.clone(),
            edge.clone(),
        ),
        agent,
        edge,
        artifacts: GithubReleaseArtifacts::new(Arc::new(citadel_processes::SystemProcess))?,
        swarm_store: platforms.node_agent_store.clone(),
        swarm_runtime: platforms.node_agent_runtime.clone(),
        swarm_options: citadel_platforms::node_agents::setup::SetupOptions {
            policy: config.execution.edge_agent.setup_policy.clone(),
            core_url: config.transport.edge_agent_public_url.to_string(),
            image: config.execution.edge_agent.image.clone(),
            ca_bundle: config.execution.edge_agent.node_agent_ca_bundle.clone(),
        },
        changed: Arc::new(move |id| {
            if let Some(hub) = &notifications {
                hub.publish_resource_change("Platform", id, "resourceChanged");
            }
        }),
    });
    #[cfg(test)]
    let runtime = Arc::new(super::agent_fixture::Runtime(runtime));
    Ok(AgentUpdateService::new(
        env!("CITADEL_BUILD_VERSION").into(),
        Arc::new(PostgresUpdateRepository::new(pool.clone())),
        runtime,
        entitlements,
        Arc::new(UpdatesRealtimeNotifier::new(realtime)),
    )
    .with_updates_allowed(config.updates_enabled))
}
