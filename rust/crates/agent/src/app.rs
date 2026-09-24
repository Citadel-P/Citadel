//! Agent hosting: validated composition, health listener and bounded shutdown.
use std::future::Future;
use std::io;
use std::net::{SocketAddr, TcpListener};
use std::pin::Pin;
use std::time::Duration;

use axum::{Json, Router, routing::get};
use axum_server::Handle;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

use crate::composition::{self, Components};
use crate::config::{AgentConfig, AgentMode, EdgeProfile};

pub type AgentError = Box<dyn std::error::Error + Send + Sync>;
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

pub struct Agent {
    config: AgentConfig,
    components: Components,
    listener: TcpListener,
}

impl Agent {
    pub async fn bind(config: AgentConfig) -> Result<Self, AgentError> {
        let components = composition::build(&config).await?;
        let listener = TcpListener::bind(config.listen_address())?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            config,
            components,
            listener,
        })
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    pub async fn serve(self, cancellation: CancellationToken) -> io::Result<()> {
        let cancellation = cancellation.child_token();
        let _guard = cancellation.clone().drop_guard();
        let edge_stop = cancellation.clone();
        let mut edge = Box::pin(async move {
            if let Some(edge) = self.components.edge {
                edge.run(edge_stop).await
            } else {
                edge_stop.cancelled().await;
                Ok(())
            }
        });
        let mode = match &self.config.mode {
            AgentMode::Direct(_) => "direct",
            AgentMode::Edge(edge) => match &edge.profile {
                EdgeProfile::Ordinary => "edge-agent",
                EdgeProfile::BuildPool => "edge-build-agent",
                EdgeProfile::SwarmNode(_) => "swarm-node",
            },
        };
        tracing::info!(address = %self.listener.local_addr()?, mode, tls = self.components.tls.is_some(), "Agent listener started");
        let router = match self.config.mode {
            AgentMode::Direct(direct) => health_router().merge(crate::direct::router(
                self.components.docker,
                direct.hub_public_key,
                cancellation.clone(),
                self.config.runtime_container,
            )),
            AgentMode::Edge(_) => health_router(),
        };
        let handle = Handle::new();
        let service = router.into_make_service();
        let server: Pin<Box<dyn Future<Output = io::Result<()>> + Send>> = match self.components.tls
        {
            Some(tls) => Box::pin(
                axum_server::from_tcp_rustls(self.listener, tls)?
                    .handle(handle.clone())
                    .serve(service),
            ),
            None => Box::pin(
                axum_server::from_tcp(self.listener)?
                    .handle(handle.clone())
                    .serve(service),
            ),
        };
        tokio::pin!(server);
        let edge_result = tokio::select! {
            result = &mut server => {
                cancellation.cancel();
                edge.await?;
                return result;
            },
            result = &mut edge => result,
            () = cancellation.cancelled() => Ok(()),
        };
        cancellation.cancel();
        drop(edge);
        tracing::info!("Agent shutting down");
        handle.graceful_shutdown(Some(SHUTDOWN_TIMEOUT));
        let server_result = tokio::time::timeout(SHUTDOWN_TIMEOUT + Duration::from_secs(1), server)
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Agent shutdown timed out"))?;
        edge_result?;
        server_result
    }
}

fn health_router() -> Router {
    Router::new().route(
        "/health",
        get(|| async {
            // Process liveness; Docker reachability belongs to Platform.CheckHealth.
            Json(serde_json::json!({"Status":"Healthy", "Duration":"00:00:00", "Entries":[]}))
        }),
    )
}

pub async fn run() -> Result<(), AgentError> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("citadel_agent=info")),
        )
        .with_ansi(false)
        .try_init()?;
    let config = AgentConfig::from_environment()?;
    let agent = Agent::bind(config).await?;
    let cancellation = CancellationToken::new();
    let serving = agent.serve(cancellation.clone());
    tokio::pin!(serving);
    tokio::select! {
        result = &mut serving => result?,
        signal = shutdown_signal() => {
            cancellation.cancel();
            serving.await?;
            signal?;
        }
    }
    Ok(())
}

async fn shutdown_signal() -> io::Result<()> {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        tokio::select! {
            result = tokio::signal::ctrl_c() => result,
            _ = terminate.recv() => Ok(()),
        }
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c().await
}
