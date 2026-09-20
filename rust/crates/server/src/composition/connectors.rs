//! Connectors service construction.
use citadel_adapters::connectors::{
    agent::client::{AgentClient, AgentRequestSigner},
    docker::DockerClient,
};
use citadel_server::{api::routes::platforms as platforms_http, config::Config};
use sqlx::PgPool;
use std::sync::Arc;
pub(super) struct ConnectorComponents {
    pub agent: Option<AgentClient>,
    pub agent_setup: Arc<citadel_server::api::resources::platforms::views::AgentSetupView>,
    pub agent_setup_context: platforms_http::AgentSetupContext,
    pub edge_context: platforms_http::EdgeHttpContext,
    pub edge_registry: citadel_adapters::connectors::edge::EdgeRegistry,
    pub runtime_targets: Arc<citadel_server::runtime_targets::PlatformRuntimeRegistry>,
    pub container_runtime:
        citadel_adapters::connectors::routing::containers::ContainerRuntimeRouter,
}

pub(super) fn build(
    config: &Config,
    pool: &PgPool,
    docker: &DockerClient,
) -> Result<ConnectorComponents, Box<dyn std::error::Error>> {
    let data_root = &config.execution.paths.data_root;
    let agent_signer = if let Some(agent) = &config.agent {
        AgentRequestSigner::from_file(&agent.private_key_path)?
    } else {
        AgentRequestSigner::load_or_create(&data_root.join("agent/signing-key"))?
    };
    let agent_image = config.execution.edge_agent.image.clone();
    let agent_setup = Arc::new(
        citadel_server::api::resources::platforms::views::AgentSetupView::new(
            agent_signer.public_key_base64(),
            agent_image.clone(),
            !config.execution.edge_agent.allow_insecure,
        ),
    );
    let agent = if let Some(agent) = &config.agent {
        Some(AgentClient::lazy(
            &agent.address,
            agent_signer.clone(),
            agent.operation_timeout,
            agent.allow_insecure,
        )?)
    } else if config.execution.edge_agent.allow_insecure {
        // Persisted Platforms supply the actual endpoint. No job probes
        // this signing context's placeholder address. Direct TLS is not
        // supported yet, so keep Direct transport disabled when h2c is off.
        Some(AgentClient::lazy(
            "http://localhost",
            agent_signer.clone(),
            config.docker_request_timeout,
            true,
        )?)
    } else {
        None
    };
    let runtime_targets =
        citadel_server::runtime_targets::PlatformRuntimeRegistry::new(pool.clone(), agent.clone());
    let edge_registry = citadel_adapters::connectors::edge::EdgeRegistry::default();
    let container_runtime =
        citadel_adapters::connectors::routing::containers::ContainerRuntimeRouter::new(
            pool.clone(),
            docker.clone(),
            agent.clone(),
            edge_registry.clone(),
        );
    let agent_setup_context = platforms_http::AgentSetupContext {
        signer: agent_signer,
        image: agent_image.clone(),
        requires_tls: !config.execution.edge_agent.allow_insecure,
    };
    let edge_context = platforms_http::EdgeHttpContext {
        node_agent_ca_bundle: config.execution.edge_agent.node_agent_ca_bundle.clone(),
        store:
            citadel_adapters::persistence::postgres::platforms::edge::store::PostgresEdgeStore::new(
                pool.clone(),
            ),
        registry: edge_registry.clone(),
        core_url: config
            .transport
            .edge_agent_public_url
            .to_string()
            .trim_end_matches('/')
            .to_owned(),
        agent_image,
    };
    Ok(ConnectorComponents {
        agent,
        agent_setup,
        agent_setup_context,
        edge_context,
        edge_registry,
        runtime_targets,
        container_runtime,
    })
}
