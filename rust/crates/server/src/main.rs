#![forbid(unsafe_code)]

mod config;
mod metrics;
mod workers;

use std::future::IntoFuture;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use citadel_adapters::PostgresAuthorizedPlatformReader;
use citadel_adapters::docker::DockerClient;
use citadel_application::{AuthorizedPlatformReader, TaskSupervisor};
use citadel_domain::ActorId;
use clap::{Parser, Subcommand};
use futures_util::StreamExt;
use serde::Serialize;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tokio_util::sync::CancellationToken;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

use crate::config::Config;
use crate::metrics::Metrics;

#[derive(Parser)]
#[command(
    name = "citadel-server",
    about = "Citadel Rust Phase 0A viability prototype"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the Phase 0A health/readiness server and bounded Docker event worker.
    Serve,
    /// Print the effective non-secret Phase 0A configuration as JSON.
    PrintEffectiveConfig,
    /// Exercise the generated Docker subset and optional Actor-authorized read.
    Phase0Smoke {
        #[arg(long)]
        actor_id: Option<Uuid>,
    },
    /// Probe an already-running Phase 0A server without curl in the image.
    Healthcheck {
        #[arg(long, default_value = "http://127.0.0.1:8000/health")]
        url: String,
    },
}

#[derive(Default)]
struct Readiness {
    database: AtomicBool,
    docker: AtomicBool,
}

impl Readiness {
    fn set(&self, database: bool, docker: bool) {
        self.database.store(database, Ordering::Release);
        self.docker.store(docker, Ordering::Release);
    }

    fn snapshot(&self) -> ReadinessResponse {
        let database = self.database.load(Ordering::Acquire);
        let docker = self.docker.load(Ordering::Acquire);
        ReadinessResponse {
            status: if database && docker {
                "ready"
            } else {
                "not-ready"
            },
            database,
            docker,
        }
    }
}

#[derive(Clone)]
struct AppState {
    readiness: Arc<Readiness>,
    metrics: Arc<Metrics>,
    pool: PgPool,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReadinessResponse {
    status: &'static str,
    database: bool,
    docker: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Serve) {
        Command::Serve => serve(Config::from_env()?).await,
        Command::PrintEffectiveConfig => {
            println!(
                "{}",
                serde_json::to_string_pretty(&Config::from_env()?.effective()?)?
            );
            Ok(())
        }
        Command::Phase0Smoke { actor_id } => phase0_smoke(Config::from_env()?, actor_id).await,
        Command::Healthcheck { url } => {
            reqwest::Client::builder()
                .timeout(Duration::from_secs(2))
                .build()?
                .get(url)
                .send()
                .await?
                .error_for_status()?;
            Ok(())
        }
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("citadel_server=info,citadel_adapters=info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .with_current_span(false)
        .init();
}

async fn serve(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!(
        effective_configuration = %serde_json::to_string(&config.effective()?)?,
        "starting Phase 0A server"
    );
    let pool = connect_database(&config).await?;
    let docker = DockerClient::new(&config.docker_socket, config.docker_request_timeout)?;
    let cancellation = CancellationToken::new();
    let readiness = Arc::new(Readiness::default());
    let metrics = Arc::new(Metrics::default());
    let mut supervisor = TaskSupervisor::new(cancellation.clone());
    workers::register(
        &mut supervisor,
        &cancellation,
        docker,
        pool.clone(),
        Arc::clone(&readiness),
        Arc::clone(&metrics),
        workers::WorkerSettings {
            queue_capacity: config.event_queue_capacity,
            probe_interval: config.probe_interval,
        },
    );

    let state = AppState {
        readiness,
        metrics,
        pool: pool.clone(),
    };
    let app = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/metrics", get(open_metrics))
        .layer(CatchPanicLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(config.listen_address).await?;
    tracing::info!(address = %config.listen_address, "Phase 0A server listening");

    let server = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(cancellation.clone()))
        .into_future();
    tokio::pin!(server);
    let exit = tokio::select! {
        server_result = &mut server => ServerExit::Server(server_result),
        task_result = supervisor.wait_for_exit() => ServerExit::Task(task_result),
    };
    cancellation.cancel();
    if matches!(exit, ServerExit::Task(_)) {
        let _ = tokio::time::timeout(config.shutdown_timeout, &mut server).await;
    }
    let supervisor_result = supervisor.shutdown(config.shutdown_timeout).await;
    let pool_close_result = tokio::time::timeout(config.shutdown_timeout, pool.close()).await;

    match exit {
        ServerExit::Server(result) => result?,
        ServerExit::Task(result) => result?,
    }
    supervisor_result?;
    if pool_close_result.is_err() {
        return Err("PostgreSQL pool close exceeded the shutdown timeout".into());
    }
    Ok(())
}

enum ServerExit {
    Server(std::io::Result<()>),
    Task(Result<(), citadel_application::SupervisedTaskError>),
}

async fn connect_database(config: &Config) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .min_connections(0)
        .max_connections(config.database_max_connections)
        .acquire_timeout(config.docker_request_timeout)
        .connect(&config.database_url)
        .await
}

async fn phase0_smoke(
    config: Config,
    actor_id: Option<Uuid>,
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = connect_database(&config).await?;
    let docker = DockerClient::new(&config.docker_socket, config.docker_request_timeout)?;
    docker.ping().await?;
    let version = docker.version().await?;
    let negotiated = docker.negotiated_version().await?;
    let info = docker.info().await?;
    let containers = docker.list_containers(true).await?;
    let inspected = if let Some(container) = containers.first() {
        Some(docker.inspect_container(&container.id).await?.id)
    } else {
        None
    };
    let running = containers
        .iter()
        .find(|container| container.state == "running");
    let stats_sample = if let Some(container) = running {
        let mut stream = docker.container_stats(&container.id).await?;
        match tokio::time::timeout(Duration::from_secs(15), stream.next()).await {
            Ok(Some(Ok(sample))) => Some(sample.id),
            Ok(Some(Err(error))) => return Err(error.into()),
            _ => None,
        }
    } else {
        None
    };
    let event_stream_opened = docker.events(None, None).await.is_ok();
    let authorized_platform_count = if let Some(actor_id) = actor_id {
        let reader = PostgresAuthorizedPlatformReader::new(pool.clone());
        Some(reader.list_authorized(ActorId::new(actor_id)).await?.len())
    } else {
        None
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "dockerVersion": version.version,
            "daemonApiVersion": version.api_version,
            "negotiatedApiVersion": negotiated.to_string(),
            "daemonId": info.id,
            "containerCount": containers.len(),
            "inspectedContainerId": inspected,
            "statsContainerId": stats_sample,
            "eventStreamOpened": event_stream_opened,
            "authorizedPlatformCount": authorized_platform_count,
        }))?
    );
    pool.close().await;
    Ok(())
}

async fn health() -> axum::Json<HealthResponse> {
    axum::Json(HealthResponse { status: "ok" })
}

async fn ready(State(state): State<AppState>) -> Response {
    let readiness = state.readiness.snapshot();
    let status = if readiness.database && readiness.docker {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status, axum::Json(readiness)).into_response()
}

async fn open_metrics(State(state): State<AppState>) -> Response {
    (
        [(
            header::CONTENT_TYPE,
            "application/openmetrics-text; version=1.0.0; charset=utf-8",
        )],
        state.metrics.render(&state.pool),
    )
        .into_response()
}

async fn shutdown_signal(cancellation: CancellationToken) {
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                tracing::error!(%error, "failed to install SIGTERM handler");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        result = tokio::signal::ctrl_c() => {
            if let Err(error) = result {
                tracing::error!(%error, "failed to install Ctrl+C handler");
            }
        }
        () = terminate => {}
        () = cancellation.cancelled() => return,
    }
    cancellation.cancel();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readiness_requires_database_and_docker() {
        let readiness = Readiness::default();
        readiness.set(true, false);
        assert_eq!(readiness.snapshot().status, "not-ready");
        readiness.set(true, true);
        assert_eq!(readiness.snapshot().status, "ready");
    }
}
