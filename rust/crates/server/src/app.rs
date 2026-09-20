//! Executable lifecycle: initialization, supervised serving, and bounded shutdown.
use crate::{cli, jobs, router as api, startup, state::AppState};
use axum::Router;
use citadel_server::{config::Config, transport};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    cli::run().await
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

pub async fn serve(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!(
        effective_configuration = %serde_json::to_string(&config.effective()?)?,
        "starting Rust foundation server"
    );
    if config.transport.mode == citadel_server::config::TransportMode::Disabled {
        tracing::warn!(
            "Citadel transport security is disabled; API and Agent traffic is not encrypted"
        );
    }
    startup::migrate(&config).await?;
    let job_lease = startup::acquire_job_lease(&config).await?;
    let (state, pending_jobs) = AppState::build(&config).await?;
    startup::run(&state).await?;
    let cancellation = state.cancellation.clone();
    let pool = state.pool.clone();
    let dynamic_tasks = state.dynamic_tasks.clone();
    let mut supervisor = jobs::spawn_all(&state, pending_jobs, &config).await?;
    // The watcher owns the connection by value. Return it on cancellation so
    // the advisory lease remains held through worker/dynamic-task cleanup.
    let (lease_return, lease_retained) = tokio::sync::oneshot::channel();
    let lease_cancel = cancellation.clone();
    supervisor.spawn("core-job-lease", async move {
        let connection = startup::watch_job_lease(job_lease, lease_cancel).await?;
        let _ = lease_return.send(connection);
        Ok::<_, sqlx::Error>(())
    });
    let api::Routers {
        http: app,
        edge: edge_app,
    } = api::router(state, &config)?;
    tracing::info!(
        address = %config.listen_address,
        mode = ?config.transport.mode,
        "Rust foundation server listening"
    );

    let server = run_server(
        config.listen_address,
        config.transport.clone(),
        app,
        edge_app,
        cancellation.clone(),
        config.shutdown_timeout,
    );
    tokio::pin!(server);
    let exit = tokio::select! {
        server_result = &mut server => ServerExit::Server(server_result),
        task_result = supervisor.wait_for_exit() => ServerExit::Task(task_result),
    };
    let shutdown_deadline = tokio::time::Instant::now() + config.shutdown_timeout;
    cancellation.cancel();
    if matches!(exit, ServerExit::Task(_)) {
        let _ = tokio::time::timeout_at(shutdown_deadline, &mut server).await;
    }
    let supervisor_result = supervisor
        .shutdown(shutdown_deadline.saturating_duration_since(tokio::time::Instant::now()))
        .await;
    let dynamic_result = dynamic_tasks
        .drain(shutdown_deadline.saturating_duration_since(tokio::time::Instant::now()))
        .await;
    let _retained_job_lease = lease_retained.await.ok();
    let pool_close_result = tokio::time::timeout(config.shutdown_timeout, pool.close()).await;

    match exit {
        ServerExit::Server(result) => result?,
        ServerExit::Task(result) => result?,
    }
    supervisor_result?;
    if dynamic_result.is_err() {
        return Err(
            "Dynamic task cleanup exceeded the shutdown budget; durable claims remain recoverable"
                .into(),
        );
    }
    if pool_close_result.is_err() {
        return Err("PostgreSQL pool close exceeded the shutdown timeout".into());
    }
    Ok(())
}

enum ServerExit {
    Server(std::io::Result<()>),
    Task(Result<(), citadel_runtime::SupervisedTaskError>),
}

async fn run_server(
    address: std::net::SocketAddr,
    transport_config: citadel_server::config::TransportConfig,
    app: Router,
    edge_app: Router,
    cancellation: CancellationToken,
    shutdown_timeout: Duration,
) -> std::io::Result<()> {
    let http = async {
        tokio::try_join!(
            transport::serve(
                address,
                &transport_config,
                app,
                cancellation.clone(),
                shutdown_timeout
            ),
            transport::serve(
                std::net::SocketAddr::new(address.ip(), transport_config.edge_grpc_port),
                &transport_config,
                edge_app,
                cancellation.clone(),
                shutdown_timeout
            ),
        )
        .map(|_| ())
    };
    tokio::pin!(http);
    tokio::select! {
        result = &mut http => result,
        () = shutdown_signal(cancellation.clone()) => http.await,
    }
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
