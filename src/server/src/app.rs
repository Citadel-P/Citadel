//! Executable lifecycle: initialization, supervised serving, and bounded shutdown.
use crate::{
    cli,
    composition::{ServerComponents, background},
    router as api, startup,
};
use axum::Router;
use citadel_server::{
    config::{Config, LogFormat},
    transport,
};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing()?;
    cli::run().await
}

fn init_tracing() -> Result<(), Box<dyn std::error::Error>> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let format = citadel_server::config::log_format_from_env()?;
    let color = citadel_server::config::log_color_from_env()?;
    if format == LogFormat::Text {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(color)
            .with_target(false)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .json()
            .with_ansi(false)
            .with_current_span(false)
            .init();
    }
    Ok(())
}

pub async fn serve(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let effective = config.effective()?;
    tracing::info!("Starting Citadel Core");
    tracing::debug!(
        effective_configuration = %serde_json::to_string(&effective)?,
        "Effective configuration"
    );
    if config.transport.mode == citadel_server::config::TransportMode::Disabled {
        tracing::warn!(
            "Citadel transport security is disabled; API and Agent traffic is not encrypted"
        );
    }
    let mut job_lease = startup::acquire_job_lease(&config).await?;
    if let Err(error) =
        startup::while_owned(&mut job_lease, startup::check_updates(&config)).await?
    {
        #[cfg(target_os = "linux")]
        {
            let cancellation = CancellationToken::new();
            let acceptance = startup::while_owned(
                &mut job_lease,
                crate::composition::updates::accept_target(&config, &cancellation),
            );
            tokio::pin!(acceptance);
            let result = tokio::select! {
                result = &mut acceptance => result?,
                () = shutdown_signal(cancellation.clone()) => {
                    acceptance.await??;
                    return Ok(());
                }
            };
            match result {
                Ok(()) => tracing::info!(
                    "Update target verified and helper exited cleanly; maintenance is retained pending host reconciliation."
                ),
                Err(error) => {
                    tracing::warn!(%error, "Update target acceptance did not authorize admission")
                }
            }
        }
        tracing::error!(%error, "Core is in update maintenance; migrations, normal APIs and workers are stopped. Preserve update evidence and resolve recovery before restarting.");
        return serve_maintenance(&config, job_lease).await;
    }
    startup::while_owned(&mut job_lease, startup::migrate(&config)).await??;
    let (state, background_services) =
        startup::while_owned(&mut job_lease, ServerComponents::build(&config)).await??;
    let cancellation = state.cancellation.clone();
    // Also cancel owned work if initialization fails before normal shutdown is
    // installed (for example, lease loss while registering background workers).
    let _cancel_on_exit = cancellation.clone().drop_guard();
    startup::run(&state, &mut job_lease).await?;
    let pool = state.pool.clone();
    let dynamic_tasks = state.dynamic_tasks.clone();
    let mut supervisor = startup::while_owned(
        &mut job_lease,
        background::spawn_all(&state, background_services, &config),
    )
    .await??;
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
        "Citadel Core listening"
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

async fn serve_maintenance(
    config: &Config,
    mut job_lease: startup::CoreJobLease,
) -> Result<(), Box<dyn std::error::Error>> {
    let cancellation = CancellationToken::new();
    job_lease.verify().await?;
    let api::Routers { http, edge } = api::maintenance_router(config)?;
    let server = run_server(
        config.listen_address,
        config.transport.clone(),
        http,
        edge,
        cancellation.clone(),
        config.shutdown_timeout,
    );
    // Both futures are owned here. Maintenance starts no normal background jobs,
    // but losing the single-Core lease still terminates the listeners.
    let lease = startup::watch_job_lease(job_lease, cancellation.clone());
    tokio::pin!(server, lease);
    tokio::select! {
        result = &mut server => {
            cancellation.cancel();
            let _retained_lease = tokio::time::timeout(config.shutdown_timeout, &mut lease).await??;
            result?;
        }
        result = &mut lease => {
            cancellation.cancel();
            let _ = tokio::time::timeout(config.shutdown_timeout, &mut server).await;
            result?;
        }
    }
    Ok(())
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

pub(crate) async fn shutdown_signal(cancellation: CancellationToken) {
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
