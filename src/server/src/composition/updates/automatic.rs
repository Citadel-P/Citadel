//! Resolve Compose's fixed internal mount before normal startup admission.
use citadel_adapters::{
    external::updates::{
        AUTOMATIC_STATE_MOUNT, AutomaticComposeSetup, HostUpdateClient, HostUpdateRequest,
        HostUpdateResponse, discover_automatic_setup,
    },
    persistence::postgres::{connection, updates::automatic_setup_is_fresh},
};
use citadel_server::config::Config;
use citadel_updates::UpdateError;
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use tokio_util::sync::CancellationToken;

pub(crate) async fn prepare(
    config: &mut Config,
) -> Result<Option<AutomaticComposeSetup>, UpdateError> {
    if config.update_state_directory.as_deref() != Some(std::path::Path::new(AUTOMATIC_STATE_MOUNT))
    {
        return Ok(None);
    }
    let cancellation = CancellationToken::new();
    let process = Arc::new(citadel_processes::SystemProcess);
    let setup = discover_automatic_setup(process.as_ref(), &cancellation).await?;
    config.update_state_directory = Some(setup.state_directory.clone());
    let pool = connection::connect_pool(&config.database_url, 1, Duration::from_secs(5))
        .await
        .map_err(|_| {
            UpdateError::Unavailable("Cannot inspect automatic setup database state.".into())
        })?;
    let fresh = tokio::time::timeout(Duration::from_secs(5), async {
        citadel_adapters::persistence::postgres::updates::check_startup_updates(&pool, None)
            .await?;
        automatic_setup_is_fresh(&pool).await
    })
    .await;
    tokio::time::timeout(Duration::from_secs(5), pool.close())
        .await
        .map_err(|_| {
            UpdateError::Unavailable("Automatic setup database inspection did not close.".into())
        })?;
    let fresh = fresh.map_err(|_| {
        UpdateError::Unavailable("Automatic setup database check timed out.".into())
    })??;
    let (response, _, _) = HostUpdateClient::new(process, setup.state_directory.clone())
        .exchange(
            HostUpdateRequest::AutomaticStartup {
                setup: setup.clone(),
                allow_initialize: fresh,
            },
            &cancellation,
        )
        .await?;
    match response {
        HostUpdateResponse::Registered(_) => Ok(None),
        HostUpdateResponse::RegistrationPending => {
            config
                .update_registration_ready
                .store(false, Ordering::Release);
            Ok(Some(setup))
        }
        _ => Err(UpdateError::Unavailable(
            "Cannot verify automatic update setup.".into(),
        )),
    }
}

pub(crate) fn registration(
    setup: AutomaticComposeSetup,
    ready: Arc<std::sync::atomic::AtomicBool>,
) -> Arc<dyn citadel_updates::UpdateRegistrationRuntime> {
    Arc::new(
        citadel_adapters::external::updates::AutomaticComposeRegistration::new(
            setup,
            ready,
            Arc::new(citadel_processes::SystemProcess),
        ),
    )
}
