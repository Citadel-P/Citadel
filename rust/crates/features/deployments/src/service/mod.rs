use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use citadel_alerts::{AlertEventSink, AlertObservation};
use citadel_licensing::LicenseCapability;
use citadel_primitives::ActorId;
use serde_json::Value;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::ApplyClaim;
use crate::CreateDeployment;
use crate::DeletionClaim;
use crate::DeploymentBindingSnapshot;
use crate::DeploymentConfig;
use crate::DeploymentDetails;
use crate::DeploymentDuplicateDraft;
use crate::DeploymentError;
use crate::DeploymentFilter;
use crate::DeploymentImageInfo;
use crate::DeploymentProgress;
use crate::DeploymentSpec;
use crate::FieldPatch;
use crate::ResolvedDeploymentBindings;
use crate::RuntimeContainerState;
use crate::RuntimeDeploymentCommand;
use crate::UpdateDeploymentMetadata;

mod adoption;
mod apply;
mod bindings;
mod delete;
mod mutations;
mod read;
#[cfg(test)]
mod tests;
mod updates;
use crate::DeploymentBindingResolverPort;
use crate::DeploymentEntitlementPort;
use crate::DeploymentRepository;
use crate::DeploymentRuntime;
use crate::EmptyDeploymentBindingResolver;
use bindings::*;
use mutations::{normalize_description, normalize_name, unique_ids};
pub use updates::{DeploymentUpdateCheck, checkable_deployment_image, evaluate_deployment_digest};
pub trait DeploymentChangeNotifier: Send + Sync {
    fn changed(&self, deployment_id: Uuid, event: &'static str);
    fn adopted(&self, deployment: &DeploymentDetails) {
        self.changed(deployment.id, "created");
    }
}

#[derive(Default)]
pub struct NoopDeploymentChangeNotifier;

impl DeploymentChangeNotifier for NoopDeploymentChangeNotifier {
    fn changed(&self, _deployment_id: Uuid, _event: &'static str) {}
}

#[derive(Clone)]
pub struct DeploymentService {
    tasks: Arc<dyn crate::DeploymentTaskSpawner>,
    store: Arc<dyn DeploymentRepository>,
    runtime: Arc<dyn DeploymentRuntime>,
    entitlements: Arc<dyn DeploymentEntitlementPort>,
    notifier: Arc<dyn DeploymentChangeNotifier>,
    bindings: Arc<dyn DeploymentBindingResolverPort>,
    shutdown: CancellationToken,
    delete_timeout: Duration,
    apply_timeout: Duration,
    apply_slots: Arc<Semaphore>,
    alerts: Option<Arc<dyn AlertEventSink>>,
    adoption: Option<Arc<dyn crate::adoption::ContainerAdoptionPort>>,
}

impl DeploymentService {
    #[must_use]
    pub fn new(
        tasks: Arc<dyn crate::DeploymentTaskSpawner>,
        store: Arc<dyn DeploymentRepository>,
        runtime: Arc<dyn DeploymentRuntime>,
        entitlements: Arc<dyn DeploymentEntitlementPort>,
        shutdown: CancellationToken,
    ) -> Self {
        Self {
            tasks,
            store,
            runtime,
            entitlements,
            notifier: Arc::new(NoopDeploymentChangeNotifier),
            bindings: Arc::new(EmptyDeploymentBindingResolver),
            shutdown,
            delete_timeout: Duration::from_secs(30),
            apply_timeout: Duration::from_secs(10 * 60),
            apply_slots: Arc::new(Semaphore::new(4)),
            alerts: None,
            adoption: None,
        }
    }

    #[must_use]
    pub fn with_notifier(mut self, notifier: Arc<dyn DeploymentChangeNotifier>) -> Self {
        self.notifier = notifier;
        self
    }

    pub fn with_adoption(
        mut self,
        adoption: Arc<dyn crate::adoption::ContainerAdoptionPort>,
    ) -> Self {
        self.adoption = Some(adoption);
        self
    }

    #[must_use]
    pub fn with_binding_resolver(
        mut self,
        bindings: Arc<dyn DeploymentBindingResolverPort>,
    ) -> Self {
        self.bindings = bindings;
        self
    }

    #[must_use]
    pub fn with_alerts(mut self, alerts: Arc<dyn AlertEventSink>) -> Self {
        self.alerts = Some(alerts);
        self
    }

    #[must_use]
    pub fn with_delete_timeout(mut self, timeout: Duration) -> Self {
        self.delete_timeout = timeout;
        self
    }

    #[must_use]
    pub fn with_apply_timeout(mut self, timeout: Duration) -> Self {
        self.apply_timeout = timeout;
        self
    }
}

impl DeploymentService {
    fn spawn_result<T: Send + 'static>(
        &self,
        name: &'static str,
        operation: impl std::future::Future<Output = Result<T, DeploymentError>> + Send + 'static,
    ) -> Option<tokio::sync::oneshot::Receiver<Result<T, DeploymentError>>> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        self.tasks
            .spawn(
                name,
                Box::pin(async move {
                    let result = operation.await;
                    let failure = result.as_ref().err().cloned();
                    let _ = sender.send(result);
                    failure.map_or(Ok(()), Err)
                }),
            )
            .then_some(receiver)
    }
}
