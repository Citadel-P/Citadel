#![forbid(unsafe_code)]

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::middleware;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use citadel_adapters::PostgresAuthorizedPlatformReader;
use citadel_adapters::agent::{AgentClient, AgentRequestSigner};
use citadel_adapters::crypto::{
    Argon2PasswordHasher, JwtSessionTokenCodec, OpaqueServiceAccountTokenCodec,
};
use citadel_adapters::docker::DockerClient;
use citadel_adapters::identity_store::PostgresIdentityStore;
use citadel_adapters::license::PostgresLicenseEntitlementService;
use citadel_adapters::postgres_runtime;
use citadel_adapters::profile_store::PostgresProfileStore;
use citadel_adapters::service_account_store::PostgresServiceAccountStore;
use citadel_application::{
    AuthorizedPlatformReader, IdentityService, PlatformRuntimePort, ProfileService,
    ServiceAccountService, SystemClock, TaskSupervisor, service_account_last_used_channel,
};
use citadel_database::MigrationRunner;
use citadel_domain::ActorId;
use citadel_server::config::{Config, DatabaseConfig};
use citadel_server::metrics::Metrics;
use citadel_server::realtime::RealtimeService;
use citadel_server::{
    Readiness, identity_http, profile_http, service_accounts_http, transport, workers,
};
use clap::{Parser, Subcommand};
use futures_util::StreamExt;
use serde::Serialize;
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "citadel-server", about = "Citadel Rust migration server")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the Rust foundation server and supervised workers.
    Serve,
    /// Apply the embedded Citadel schema migrations and exit.
    Migrate,
    /// Print bounded process and cgroup diagnostics as JSON.
    Diagnostics,
    /// Print the effective non-secret configuration as JSON.
    PrintEffectiveConfig,
    /// Exercise the generated Docker subset and optional Actor-authorized read.
    Phase0Smoke {
        #[arg(long)]
        actor_id: Option<Uuid>,
    },
    /// Print the Base64 public key corresponding to a raw 32-byte Agent private key.
    AgentPublicKey {
        #[arg(long)]
        private_key_path: std::path::PathBuf,
    },
    /// Prove signed .NET Agent handshake, read, stream, cancellation, and Local equivalence.
    Phase0AgentSmoke,
    /// Probe an already-running Phase 0A server without curl in the image.
    Healthcheck {
        #[arg(long, default_value = "http://127.0.0.1:8000/health")]
        url: String,
    },
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Serve) {
        Command::Serve => serve(Config::from_env()?).await,
        Command::Migrate => migrate(DatabaseConfig::from_env()?).await,
        Command::Diagnostics => {
            println!(
                "{}",
                serde_json::to_string_pretty(&citadel_server::diagnostics::snapshot())?
            );
            Ok(())
        }
        Command::PrintEffectiveConfig => {
            println!(
                "{}",
                serde_json::to_string_pretty(&Config::from_env()?.effective()?)?
            );
            Ok(())
        }
        Command::Phase0Smoke { actor_id } => phase0_smoke(Config::from_env()?, actor_id).await,
        Command::AgentPublicKey { private_key_path } => {
            println!(
                "{}",
                AgentRequestSigner::from_file(&private_key_path)?.public_key_base64()
            );
            Ok(())
        }
        Command::Phase0AgentSmoke => phase0_agent_smoke(Config::from_env()?).await,
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
        "starting Rust foundation server"
    );
    if config.transport.mode == citadel_server::config::TransportMode::Disabled {
        tracing::warn!(
            "Citadel transport security is disabled; API and Agent traffic is not encrypted"
        );
    }
    let migration = MigrationRunner::migrate(&config.database_url).await?;
    tracing::info!(
        applied = migration.applied,
        already_applied = migration.already_applied,
        "database migrations completed"
    );
    let pool = connect_database(&config).await?;
    let docker = DockerClient::new(&config.docker_socket, config.docker_request_timeout)?;
    let cancellation = CancellationToken::new();
    let readiness = Arc::new(Readiness::default());
    let metrics = Arc::new(Metrics::default());
    let token_codec = Arc::new(JwtSessionTokenCodec::new(
        config.identity.jwt_key.expose(),
        config.identity.issuer.clone(),
        config.identity.audience.clone(),
    )?);
    let service_account_tokens = Arc::new(OpaqueServiceAccountTokenCodec);
    let entitlements = Arc::new(PostgresLicenseEntitlementService::new(pool.clone()));
    let clock = Arc::new(SystemClock);
    let identity_store = Arc::new(PostgresIdentityStore::new(pool.clone()));
    let (last_used_tracker, last_used_worker) = service_account_last_used_channel(
        identity_store.clone(),
        config.identity.service_account_last_used_capacity,
        Duration::from_secs(30),
        Duration::from_secs(5 * 60),
    );
    let identity = Arc::new(IdentityService::new(
        identity_store,
        Arc::new(Argon2PasswordHasher::default()),
        token_codec,
        service_account_tokens.clone(),
        entitlements.clone(),
        clock.clone(),
        Arc::new(last_used_tracker),
        chrono::Duration::from_std(config.identity.access_token_lifetime)?,
        chrono::Duration::from_std(config.identity.refresh_token_lifetime)?,
    ));
    let service_accounts = Arc::new(ServiceAccountService::new(
        Arc::new(PostgresServiceAccountStore::new(pool.clone())),
        service_account_tokens,
        entitlements,
        clock.clone(),
    ));
    let profiles = Arc::new(ProfileService::new(
        Arc::new(PostgresProfileStore::new(pool.clone())),
        Arc::clone(&identity),
        clock,
    ));
    let agent = if let Some(agent) = &config.agent {
        let signer = AgentRequestSigner::from_file(&agent.private_key_path)?;
        Some(
            AgentClient::connect(
                &agent.address,
                signer,
                agent.operation_timeout,
                agent.allow_insecure,
            )
            .await?,
        )
    } else {
        None
    };
    let realtime = config.realtime.as_ref().map(|realtime_config| {
        RealtimeService::new(
            realtime_config,
            Arc::new(PostgresAuthorizedPlatformReader::new(pool.clone())),
            Arc::new(docker.clone()),
            Arc::clone(&metrics),
            cancellation.clone(),
        )
    });
    let realtime_hub = realtime.as_ref().map(RealtimeService::hub);
    let mut supervisor = TaskSupervisor::new(cancellation.clone());
    supervisor.spawn(
        "service-account-last-used",
        last_used_worker.run(cancellation.child_token()),
    );
    workers::register(
        &mut supervisor,
        &cancellation,
        workers::WorkerDependencies {
            docker,
            pool: pool.clone(),
            readiness: Arc::clone(&readiness),
            metrics: Arc::clone(&metrics),
            agent,
            realtime: realtime_hub,
        },
        workers::WorkerSettings {
            queue_capacity: config.event_queue_capacity,
            probe_interval: config.probe_interval,
            agent_reconnect_delay: config
                .agent
                .as_ref()
                .map_or(Duration::from_secs(10), |agent| agent.reconnect_delay),
        },
    );

    let state = AppState {
        readiness: Arc::clone(&readiness),
        metrics,
        pool: pool.clone(),
    };
    let app = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/metrics", get(open_metrics))
        .with_state(state)
        .merge(identity_http::router(identity_http::IdentityHttpState {
            identity: Arc::clone(&identity),
            readiness: Arc::clone(&readiness),
            secure_cookies: config.transport.mode
                != citadel_server::config::TransportMode::Disabled,
        }))
        .merge(service_accounts_http::router(
            service_accounts_http::ServiceAccountHttpState {
                identity: Arc::clone(&identity),
                service_accounts,
            },
        ))
        .merge(profile_http::router(profile_http::ProfileHttpState {
            profiles,
        }));
    let mut app = if let Some(realtime) = realtime {
        app.merge(realtime.router())
    } else {
        app
    };
    if config.transport.openapi_enabled {
        app = app
            .route("/openapi/v1.json", get(openapi_full))
            .route("/openapi/public/v1.json", get(openapi_public));
    }
    let app = transport::secure_router(
        app.layer(middleware::from_fn_with_state(
            identity,
            identity_http::authentication_middleware,
        ))
        .layer(CatchPanicLayer::custom(transport::panic_response))
        .layer(TraceLayer::new_for_http()),
        &config.transport,
        Arc::clone(&readiness),
    )?;
    tracing::info!(
        address = %config.listen_address,
        mode = ?config.transport.mode,
        "Rust foundation server listening"
    );

    let server = run_server(
        config.listen_address,
        config.transport.clone(),
        app,
        cancellation.clone(),
        config.shutdown_timeout,
    );
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

async fn migrate(config: DatabaseConfig) -> Result<(), Box<dyn std::error::Error>> {
    let outcome = MigrationRunner::migrate(&config.database_url).await?;
    println!(
        "database ready: {} migration(s) applied, {} already applied",
        outcome.applied, outcome.already_applied
    );
    Ok(())
}

enum ServerExit {
    Server(std::io::Result<()>),
    Task(Result<(), citadel_application::SupervisedTaskError>),
}

async fn run_server(
    address: std::net::SocketAddr,
    transport_config: citadel_server::config::TransportConfig,
    app: Router,
    cancellation: CancellationToken,
    shutdown_timeout: Duration,
) -> std::io::Result<()> {
    let http = transport::serve(
        address,
        &transport_config,
        app,
        cancellation.clone(),
        shutdown_timeout,
    );
    tokio::pin!(http);
    tokio::select! {
        result = &mut http => result,
        () = shutdown_signal(cancellation) => http.await,
    }
}

async fn connect_database(config: &Config) -> Result<PgPool, sqlx::Error> {
    postgres_runtime::connect_pool(
        &config.database_url,
        config.database_max_connections,
        config.docker_request_timeout,
    )
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

async fn phase0_agent_smoke(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let agent_config = config
        .agent
        .as_ref()
        .ok_or("CITADEL_RUST_AGENT_ADDRESS and CITADEL_RUST_AGENT_PRIVATE_KEY_PATH are required")?;
    let local = DockerClient::new(&config.docker_socket, config.docker_request_timeout)?;
    let agent = AgentClient::connect(
        &agent_config.address,
        AgentRequestSigner::from_file(&agent_config.private_key_path)?,
        agent_config.operation_timeout,
        agent_config.allow_insecure,
    )
    .await?;
    let cancellation = CancellationToken::new();
    let local_info = local.get_info(&cancellation).await?;
    let agent_info = agent.get_info(&cancellation).await?;
    let local_containers = PlatformRuntimePort::list_containers(&local, &cancellation).await?;
    let agent_containers = agent.list_containers(&cancellation).await?;
    let local_ids = local_containers
        .iter()
        .map(|container| container.id.as_str())
        .collect::<Vec<_>>();
    let agent_ids = agent_containers
        .iter()
        .map(|container| container.id.as_str())
        .collect::<Vec<_>>();
    let equivalent = local_info.daemon_id == agent_info.daemon_id
        && local_info.api_version == agent_info.api_version
        && local_info.minimum_api_version == agent_info.minimum_api_version
        && local_ids == agent_ids;
    if !equivalent {
        return Err("Local and Agent capability results differ".into());
    }

    let mut stream = agent
        .stream_stats(config.probe_interval, &cancellation)
        .await?;
    let stats = tokio::time::timeout(Duration::from_secs(15), stream.next())
        .await
        .map_err(|_| "Agent did not produce a stats sample within 15 seconds")?
        .ok_or("Agent ended the stats stream before producing a sample")??;

    let stream_cancellation = cancellation.child_token();
    let mut cancellation_stream = agent
        .stream_stats(config.probe_interval, &stream_cancellation)
        .await?;
    stream_cancellation.cancel();
    let cancellation_observed =
        tokio::time::timeout(Duration::from_secs(2), cancellation_stream.next())
            .await
            .is_ok_and(|item| item.is_none());
    if !cancellation_observed {
        return Err("Agent stats stream did not observe cancellation within two seconds".into());
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "agentVersion": agent_info.agent_version,
            "daemonId": agent_info.daemon_id,
            "apiVersion": agent_info.api_version,
            "minimumApiVersion": agent_info.minimum_api_version,
            "containerCount": agent_containers.len(),
            "localAndAgentEquivalent": equivalent,
            "streamSample": stats,
            "streamCancellationObserved": cancellation_observed,
        }))?
    );
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

async fn openapi_full() -> Response {
    (
        [(header::CONTENT_TYPE, "application/json")],
        include_str!("../../../generated/openapi/v1.json"),
    )
        .into_response()
}

async fn openapi_public() -> Response {
    (
        [(header::CONTENT_TYPE, "application/json")],
        include_str!("../../../generated/openapi/public-v1.json"),
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
