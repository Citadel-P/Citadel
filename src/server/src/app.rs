//! Executable lifecycle: initialization, supervised serving, and bounded shutdown.
use crate::{
    composition::{ServerComponents, background},
    logging, router as api, startup,
};
use axum::Router;
use citadel_server::{config::Config, transport};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub async fn serve(mut config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let mut job_lease = startup::acquire_job_lease(&config)
        .await
        .map_err(|error| startup::phase_error("background-job admission", error))?;
    #[cfg(target_os = "linux")]
    let mut automatic_setup = None;
    let mut recovered_update = false;
    if let Err(error) = startup::while_owned(&mut job_lease, async {
        #[cfg(target_os = "linux")]
        {
            automatic_setup = crate::composition::updates::prepare(&mut config).await?;
        }
        startup::check_updates(&config).await
    })
    .await
    .map_err(|error| startup::phase_error("update admission lease", error))?
    {
        #[cfg(not(target_os = "linux"))]
        let (recovered, acceptance_failure) = (false, None::<String>);
        #[cfg(target_os = "linux")]
        let (recovered, acceptance_failure) = {
            let cancellation = CancellationToken::new();
            let acceptance = startup::while_owned(
                &mut job_lease,
                crate::composition::updates::accept_target(&config, &cancellation),
            );
            tokio::pin!(acceptance);
            let result = tokio::select! {
                result = &mut acceptance => result.map_err(|error| startup::phase_error("update recovery lease", error))?,
                () = shutdown_signal(cancellation.clone()) => {
                    acceptance.await.map_err(|error| startup::phase_error("update recovery lease", error))?
                        .map_err(|error| startup::phase_error("update recovery cancellation", error))?;
                    return Ok(());
                }
            };
            match result {
                Ok(()) => (true, None),
                Err(error) => (false, Some(error.to_string())),
            }
        };
        if !recovered {
            logging::init_from_environment()?;
            if let Some(error) = acceptance_failure {
                tracing::warn!(%error, "Update target acceptance did not authorize admission");
            }
            tracing::error!(%error, "Core is in update maintenance; migrations, normal APIs and workers are stopped. Preserve update evidence and resolve recovery before restarting.");
            return serve_maintenance(&config, job_lease).await;
        }
        startup::while_owned(&mut job_lease, startup::check_updates(&config))
            .await
            .map_err(|error| startup::phase_error("recovered update admission lease", error))?
            .map_err(|error| startup::phase_error("recovered update admission", error))?;
        config
            .update_registration_ready
            .store(true, std::sync::atomic::Ordering::Release);
        #[cfg(target_os = "linux")]
        {
            automatic_setup = None;
        }
        recovered_update = true;
    }
    startup::configure(&mut config, &mut job_lease).await?;
    if recovered_update {
        tracing::info!("Update target verified, host reconciled and maintenance released.");
    }

    let (state, background_services) =
        startup::while_owned(&mut job_lease, ServerComponents::build(&config)).await??;
    let cancellation = state.cancellation.clone();
    let restart = state.restart.clone();
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
    #[cfg(target_os = "linux")]
    if let Some(setup) = automatic_setup {
        supervisor.spawn(
            "automatic-update-setup",
            citadel_server::workers::updates::run_automatic_setup(
                crate::composition::updates::registration(
                    setup,
                    config.update_registration_ready.clone(),
                ),
                state.identity.clone(),
                state.realtime_hub.clone(),
                cancellation.child_token(),
            ),
        );
    }
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
        restart.clone(),
    );
    tokio::pin!(server);
    let exit = tokio::select! {
        server_result = &mut server => ServerExit::Server(server_result),
        task_result = supervisor.wait_for_exit() => ServerExit::Task(task_result),
        result = restart.wait_for_request() => ServerExit::Restart(result),
    };
    let shutdown_deadline = tokio::time::Instant::now() + config.shutdown_timeout;
    cancellation.cancel();
    let server_shutdown = if matches!(exit, ServerExit::Task(_) | ServerExit::Restart(_)) {
        Some(tokio::time::timeout_at(shutdown_deadline, &mut server).await)
    } else {
        None
    };
    let supervisor_result = supervisor
        .shutdown(shutdown_deadline.saturating_duration_since(tokio::time::Instant::now()))
        .await;
    let dynamic_result = dynamic_tasks
        .drain(shutdown_deadline.saturating_duration_since(tokio::time::Instant::now()))
        .await;
    let _retained_job_lease = lease_retained.await.ok();
    let pool_close_result = tokio::time::timeout(config.shutdown_timeout, pool.close()).await;

    let restarting = matches!(exit, ServerExit::Restart(Ok(())));
    match exit {
        ServerExit::Server(result) => result?,
        ServerExit::Task(result) => result?,
        ServerExit::Restart(ref result) => {
            if let Err(error) = result {
                return Err(error.to_string().into());
            }
        }
    }
    supervisor_result?;
    if restarting {
        server_shutdown
            .expect("restart drains the listeners")
            .map_err(|_| "Core listener shutdown exceeded the restart budget")??;
    }
    if dynamic_result.is_err() {
        return Err(
            "Dynamic task cleanup exceeded the shutdown budget; durable claims remain recoverable"
                .into(),
        );
    }
    if pool_close_result.is_err() {
        return Err("PostgreSQL pool close exceeded the shutdown timeout".into());
    }
    drop(_retained_job_lease);
    if restarting {
        crate::restart::relaunch()?;
    }
    Ok(())
}

enum ServerExit {
    Restart(std::io::Result<()>),
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
        crate::restart::RestartControl::default(),
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
    restart: crate::restart::RestartControl,
) -> std::io::Result<()> {
    let http_handle = axum_server::Handle::new();
    let edge_handle = axum_server::Handle::new();
    let http = async {
        tokio::try_join!(
            transport::serve_with_handle(
                address,
                &transport_config,
                app,
                cancellation.clone(),
                shutdown_timeout,
                http_handle.clone()
            ),
            transport::serve_with_handle(
                std::net::SocketAddr::new(address.ip(), transport_config.edge_grpc_port),
                &transport_config,
                edge_app,
                cancellation.clone(),
                shutdown_timeout,
                edge_handle.clone()
            ),
            restart.acknowledge(http_handle.clone(), edge_handle.clone()),
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
