//! First-install registration through the same restricted helper as update execution.
use super::super::{AutomaticComposeSetup, docker::failed};
use super::{HostUpdateClient, HostUpdateRequest, HostUpdateResponse};
use citadel_execution::ProcessRunner;
use citadel_updates::{UpdateError, UpdateRegistrationRuntime};
use futures_util::future::BoxFuture;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio_util::sync::CancellationToken;

pub struct AutomaticComposeRegistration {
    setup: AutomaticComposeSetup,
    ready: Arc<AtomicBool>,
    client: HostUpdateClient,
}

impl AutomaticComposeRegistration {
    pub fn new(
        setup: AutomaticComposeSetup,
        ready: Arc<AtomicBool>,
        process: Arc<dyn ProcessRunner>,
    ) -> Self {
        let client = HostUpdateClient::new(process, setup.state_directory.clone());
        Self {
            setup,
            ready,
            client,
        }
    }
}

impl UpdateRegistrationRuntime for AutomaticComposeRegistration {
    fn register<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(async move {
            let (response, _, _) = self
                .client
                .exchange(
                    HostUpdateRequest::RegisterAutomatic(self.setup.clone()),
                    cancellation,
                )
                .await?;
            let HostUpdateResponse::Registered(registration) = response else {
                return Err(failed());
            };
            if registration.directory != self.setup.inputs.directory
                || registration.project != self.setup.inputs.project
                || registration.compose_files != self.setup.inputs.compose_files
                || registration.state_directory != self.setup.state_directory
            {
                return Err(failed());
            }
            registration.validate()?;
            self.ready.store(true, Ordering::Release);
            Ok(())
        })
    }
}
