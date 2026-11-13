use citadel_adapters::{
    external::updates::{DockerReleaseInspection, DockerUpdateInstallation},
    persistence::postgres::updates::PostgresUpdateRepository,
};
use citadel_server::{
    config::Config,
    realtime::{RealtimeHub, notifiers::UpdatesRealtimeNotifier},
    tasks::updates::TrackedUpdateDrain,
};
use citadel_updates::{
    ReleaseInspectionService, UpdateDrainService, UpdateInstallCommand, UpdateInstallService,
};
use std::sync::Arc;
use tokio::sync::mpsc;

pub(crate) fn build_installation(
    config: &Config,
    pool: &sqlx::PgPool,
    realtime: Option<RealtimeHub>,
    admission: citadel_runtime::admission::AdmissionGate,
    tasks: citadel_runtime::DynamicTasks,
) -> Result<
    Option<(UpdateInstallService, mpsc::Receiver<UpdateInstallCommand>)>,
    citadel_updates::UpdateError,
> {
    let Some(directory) = config.update_state_directory.clone() else {
        return Ok(None);
    };
    let process = Arc::new(citadel_processes::SystemProcess);
    let repository = Arc::new(PostgresUpdateRepository::new(pool.clone()));
    Ok(Some(UpdateInstallService::new(
        repository.clone(),
        Arc::new(
            DockerUpdateInstallation::new(
                process.clone(),
                directory,
                env!("CITADEL_BUILD_VERSION").into(),
                pool.clone(),
            )
            .with_registration(config.update_registration_ready.clone()),
        ),
        Arc::new(ReleaseInspectionService::new(
            super::releases::artifacts(process.clone())?,
            Arc::new(DockerReleaseInspection::new(process)),
        )),
        UpdateDrainService::new(
            repository,
            Arc::new(TrackedUpdateDrain::new(admission, tasks)),
        ),
        Arc::new(UpdatesRealtimeNotifier::new(realtime)),
    )))
}
