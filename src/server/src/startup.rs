//! One-time initialization, completed before jobs and listeners are started.
use crate::composition::ServerComponents;
use citadel_database::MigrationRunner;
use citadel_server::config::Config;

/// This check runs before migrations, setup, recovery and worker construction.
/// This check never releases ownership. A retained claim can only enter the
/// separate target-acceptance path; ordinary migration/startup remains blocked.
pub async fn check_updates(config: &Config) -> Result<(), citadel_updates::UpdateError> {
    use citadel_updates::UpdateError;
    let registered_instance = match config.update_state_directory.as_ref() {
        None => None,
        Some(_)
            if !config
                .update_registration_ready
                .load(std::sync::atomic::Ordering::Acquire) =>
        {
            None
        }
        Some(path) => {
            #[cfg(target_os = "linux")]
            {
                use citadel_adapters::external::updates::{
                    HostUpdateClient, HostUpdateRequest, HostUpdateResponse,
                    check_startup_registration,
                };
                if std::path::Path::new("/.dockerenv").exists() {
                    let client = HostUpdateClient::new(
                        std::sync::Arc::new(citadel_processes::SystemProcess),
                        path.clone(),
                    );
                    let (response, _, _) = client
                        .exchange(
                            HostUpdateRequest::Startup {
                                state_directory: path.clone(),
                            },
                            &tokio_util::sync::CancellationToken::new(),
                        )
                        .await?;
                    let HostUpdateResponse::Registered(registration) = response else {
                        return Err(UpdateError::Unavailable(
                            "Cannot verify update registration.".into(),
                        ));
                    };
                    Some(registration.instance_id)
                } else {
                    Some(check_startup_registration(path)?.instance_id)
                }
            }
            #[cfg(not(target_os = "linux"))]
            {
                let _ = path;
                return Err(UpdateError::Unavailable(
                    "Update registration requires Linux.".into(),
                ));
            }
        }
    };
    let pool = citadel_adapters::persistence::postgres::connection::connect_pool(
        &config.database_url,
        1,
        std::time::Duration::from_secs(5),
    )
    .await
    .map_err(|_| UpdateError::Unavailable("Cannot inspect database update state.".into()))?;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        citadel_adapters::persistence::postgres::updates::check_startup_updates(
            &pool,
            registered_instance,
        ),
    )
    .await;
    if tokio::time::timeout(std::time::Duration::from_secs(5), pool.close())
        .await
        .is_err()
    {
        return Err(UpdateError::Unavailable(
            "Database update-state inspection did not close within its startup budget.".into(),
        ));
    }
    match result {
        Ok(Err(UpdateError::Storage(_))) | Err(_) => Err(UpdateError::Unavailable(
            "Cannot verify database update state before startup; check database availability and schema.".into(),
        )),
        Ok(result) => result,
    }
}

/// Called only after update admission. Resolve saved settings before constructing
/// startup-bound consumers, and install the logger before reporting startup evidence.
pub async fn configure(
    config: &mut Config,
    lease: &mut CoreJobLease,
) -> Result<(), Box<dyn std::error::Error>> {
    let migration = while_owned(lease, MigrationRunner::migrate(&config.database_url))
        .await
        .map_err(|error| phase_error("database migration lease", error))?
        .map_err(|error| phase_error("database migrations", error))?;
    let saved = while_owned(
        lease,
        crate::composition::load_instance_configuration(config),
    )
    .await
    .map_err(|error| phase_error("Instance settings load lease", error))?
    .map_err(|error| phase_error("loading saved Instance settings", error))?;
    config
        .resolve_instance_configuration(&saved)
        .map_err(|error| phase_error("effective Instance configuration validation", error))?;
    crate::logging::init(
        &config.instance_configuration.log_verbosity,
        config.instance_configuration.log_format.into(),
    )
    .map_err(|_| {
        phase_error(
            "logging configuration",
            "check the log verbosity, format and color settings",
        )
    })?;
    let effective = config.effective()?;
    tracing::info!("Starting Citadel Core");
    tracing::info!(
        applied = migration.applied,
        already_applied = migration.already_applied,
        "database migrations completed"
    );
    tracing::debug!(
        effective_configuration = %serde_json::to_string(&effective)?,
        "Effective configuration"
    );
    if config.transport.mode == citadel_server::config::TransportMode::Disabled {
        tracing::warn!(
            "Citadel transport security is disabled; API and Agent traffic is not encrypted"
        );
    }
    Ok(())
}

pub async fn run(
    state: &ServerComponents,
    lease: &mut CoreJobLease,
) -> Result<(), Box<dyn std::error::Error>> {
    while_owned(
        lease,
        citadel_server::bootstrap::initialize_from_environment(&state.identity),
    )
    .await??;
    // Login and /ready must see persisted setup state before listeners open.
    state.readiness.set_setup(
        !while_owned(lease, state.identity.setup_status())
            .await??
            .requires_setup,
    );
    while_owned(
        lease,
        citadel_adapters::persistence::postgres::maintenance::recover_on_startup(&state.pool),
    )
    .await??;
    state.readiness.set(true, false);
    Ok(())
}

pub use citadel_adapters::persistence::postgres::job_lease::CoreJobLease;

/// Startup work has no detached monitor: the caller owns both futures. Verify
/// before polling work, and monitor until it finishes. Each subsequent phase
/// verifies again rather than trusting a successful probe from an earlier phase.
pub async fn while_owned<F: std::future::Future>(
    lease: &mut CoreJobLease,
    work: F,
) -> Result<F::Output, sqlx::Error> {
    lease.verify().await?;
    tokio::select! {
        biased;
        error = lease.wait_for_loss() => Err(error),
        result = work => Ok(result),
    }
}

pub async fn acquire_job_lease(config: &Config) -> Result<CoreJobLease, sqlx::Error> {
    CoreJobLease::acquire(&config.database_url).await
}

pub async fn watch_job_lease(
    lease: CoreJobLease,
    token: tokio_util::sync::CancellationToken,
) -> Result<CoreJobLease, sqlx::Error> {
    lease.watch(token).await
}

/// Early startup has no subscriber yet. Preserve bounded phase context in the
/// returned error, without dumping Config, environment values or recovery state.
pub(crate) fn phase_error(
    phase: &'static str,
    error: impl std::fmt::Display,
) -> Box<dyn std::error::Error> {
    let detail: String = error.to_string().chars().take(2048).collect();
    std::io::Error::other(format!("Core startup failed during {phase}: {detail}")).into()
}
