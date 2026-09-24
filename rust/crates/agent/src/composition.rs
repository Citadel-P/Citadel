//! Construct shared runtime dependencies without starting background work.
use std::io;
use std::time::Duration;

use axum_server::tls_rustls::RustlsConfig;
use citadel_adapters::connectors::docker::DockerClient;

use crate::config::{AgentConfig, AgentMode};

pub struct Components {
    pub docker: DockerClient,
    pub tls: Option<RustlsConfig>,
    pub edge: Option<crate::edge::EdgeAgent>,
}

pub async fn build(config: &AgentConfig) -> Result<Components, super::app::AgentError> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let tls = match &config.mode {
        AgentMode::Direct(direct) => match &direct.tls {
            Some(tls) => Some(
                RustlsConfig::from_pem_file(&tls.certificate_path, &tls.private_key_path)
                    .await
                    .map_err(|_| {
                        io::Error::other("Could not load Direct TLS certificate and private key")
                    })?,
            ),
            None => None,
        },
        AgentMode::Edge(_) => None,
    };
    let docker = DockerClient::with_endpoint(
        config.docker_endpoint.clone(),
        Duration::from_secs(30),
        &config.host_root,
    )?;
    let edge = match &config.mode {
        AgentMode::Edge(edge) => Some(
            crate::edge::EdgeAgent::new(edge.clone(), docker.clone())
                .await?
                .with_runtime_container(config.runtime_container.clone()),
        ),
        AgentMode::Direct(_) => None,
    };
    Ok(Components { docker, tls, edge })
}
