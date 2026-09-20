use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use citadel_alerts::{AlertEventSink, AlertObservation};
use citadel_primitives::ActorId;
use tokio::sync::{Semaphore, mpsc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::CreateSwarmService;
use crate::RenameSwarmService;
use crate::ServiceOperationClaim;
use crate::ServiceOperationKind;
use crate::SwarmServiceDetails;
use crate::SwarmServiceError;
use crate::SwarmServiceFilter;
use crate::SwarmServiceProgressItem;
use crate::UpdateSwarmService;

mod updates;
pub use updates::{
    ServiceAutomationEntitlements, ServiceImageDigestPort, ServiceUpdateCheck,
    ServiceUpdateOutcome, checkable_image, evaluate_digest,
};
mod image_updates;

pub trait SwarmServiceChangeNotifier: Send + Sync {
    fn changed(&self, service_id: Uuid, event: &'static str);
}

#[derive(Default)]
pub struct NoopSwarmServiceChangeNotifier;

#[derive(Clone)]
pub struct SwarmServiceService {
    tasks: Arc<dyn crate::SwarmServiceTaskSpawner>,
    adoption: Option<Arc<dyn crate::adoption::SwarmServiceAdoptionPort>>,
    store: Arc<dyn SwarmServiceRepository>,
    runtime: Arc<dyn SwarmServiceRuntime>,
    notifier: Arc<dyn SwarmServiceChangeNotifier>,
    bindings: Arc<dyn SwarmServiceBindingResolverPort>,
    shutdown: CancellationToken,
    operation_timeout: Duration,
    slots: Arc<Semaphore>,
    alerts: Option<Arc<dyn AlertEventSink>>,
    image_digests: Option<Arc<dyn ServiceImageDigestPort>>,
    entitlements: Option<Arc<dyn ServiceAutomationEntitlements>>,
}
impl SwarmServiceService {
    #[must_use]
    pub fn new(
        tasks: Arc<dyn crate::SwarmServiceTaskSpawner>,
        store: Arc<dyn SwarmServiceRepository>,
        runtime: Arc<dyn SwarmServiceRuntime>,
        shutdown: CancellationToken,
    ) -> Self {
        Self {
            tasks,
            adoption: None,
            store,
            runtime,
            notifier: Arc::new(NoopSwarmServiceChangeNotifier),
            bindings: Arc::new(EmptySwarmServiceBindingResolver),
            shutdown,
            operation_timeout: Duration::from_secs(10 * 60),
            slots: Arc::new(Semaphore::new(4)),
            alerts: None,
            image_digests: None,
            entitlements: None,
        }
    }

    #[must_use]
    pub fn with_alerts(mut self, alerts: Arc<dyn AlertEventSink>) -> Self {
        self.alerts = Some(alerts);
        self
    }

    #[must_use]
    pub fn with_notifier(mut self, notifier: Arc<dyn SwarmServiceChangeNotifier>) -> Self {
        self.notifier = notifier;
        self
    }

    #[must_use]
    pub fn with_binding_resolver(
        mut self,
        bindings: Arc<dyn SwarmServiceBindingResolverPort>,
    ) -> Self {
        self.bindings = bindings;
        self
    }

    #[must_use]
    pub fn with_operation_timeout(mut self, timeout: Duration) -> Self {
        self.operation_timeout = timeout;
        self
    }

    pub fn with_adoption(
        mut self,
        adoption: Arc<dyn crate::adoption::SwarmServiceAdoptionPort>,
    ) -> Self {
        self.adoption = Some(adoption);
        self
    }
}

use crate::*;
mod bindings;
use bindings::validation;
use bindings::*;

mod apply;

mod read;

mod adoption;

mod mutations;

mod delete;

#[cfg(test)]
mod tests;

impl SwarmServiceService {
    fn spawn_result<T: Send + 'static>(
        &self,
        name: &'static str,
        operation: impl std::future::Future<Output = Result<T, SwarmServiceError>> + Send + 'static,
    ) -> Option<tokio::sync::oneshot::Receiver<Result<T, SwarmServiceError>>> {
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

impl SwarmServiceChangeNotifier for NoopSwarmServiceChangeNotifier {
    fn changed(&self, _service_id: Uuid, _event: &'static str) {}
}
