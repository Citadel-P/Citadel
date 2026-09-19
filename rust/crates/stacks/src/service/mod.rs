use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;

use citadel_alerts::{AlertEventSink, AlertObservation};
use citadel_domain::{ActivityEventInfo, ActorId};
use futures_util::future::BoxFuture;
use tokio::sync::{Semaphore, mpsc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
mod update_checks;
pub use update_checks::StackUpdateScanner;
mod resolution_message;

use crate::{
    ApplyStack, ComposeModel, ComposeProjectImportDraft, ComposeProjectImportSource,
    ComposeProjectImportValidation, ComposeProjectServiceComparison, ComposeProjectStackDraft,
    CreateStack, ImportComposeProject, ResolvedStackBindings, ResolvedStackBuildImageBinding,
    RollbackStack, StackAction, StackAdoptionIssue, StackConfig, StackDetails, StackDrift,
    StackDriftMonitorFailure, StackDriftMonitorResult, StackDriftReport, StackError, StackFilter,
    StackImportClaim, StackOperationClaim, StackOrchestrationMode, StackProgressItem,
    StackReleaseDetails, StackReleaseStatus, StackRuntimeResult, StackRuntimeSnapshot,
    StackSourceFile, StackSpec, UpdateStack, analyze_swarm_compatibility, compose_digest,
    create_ownership_labels_override, merge_json, normalize_description, normalize_name,
    normalize_tags, parse_compose, validation,
};

/// Bounded, redacted progress shared with the HTTP stream during execution.
pub struct StackProgress {
    sender: mpsc::Sender<StackProgressItem>,
    secrets: Vec<String>,
    cancellation: CancellationToken,
}

pub trait StackChangeNotifier: Send + Sync {
    fn changed(&self, stack_id: Uuid, event: &'static str);
}

#[derive(Default)]
pub struct NoopStackChangeNotifier;

pub struct StackService {
    tasks: Arc<dyn crate::StackTaskSpawner>,
    update_scanner: Option<Arc<dyn StackUpdateScanner>>,
    pub(crate) store: Arc<dyn StackRepository>,
    runtime: Arc<dyn StackRuntime>,
    source_materializer: Option<Arc<dyn StackSourceMaterializerPort>>,
    bindings: Arc<dyn StackBindingResolverPort>,
    build_images: Option<Arc<dyn StackBuildImageResolverPort>>,
    notifier: Arc<dyn StackChangeNotifier>,
    operations: Arc<Semaphore>,
    pub(crate) shutdown: CancellationToken,
    pub(crate) entitlements: Option<Arc<dyn crate::StackEntitlements>>,
    timeout: Duration,
    alerts: Option<Arc<dyn AlertEventSink>>,
}
impl StackService {
    #[must_use]
    pub fn new(
        tasks: Arc<dyn crate::StackTaskSpawner>,
        store: Arc<dyn StackRepository>,
        runtime: Arc<dyn StackRuntime>,
        bindings: Arc<dyn StackBindingResolverPort>,
        notifier: Arc<dyn StackChangeNotifier>,
        shutdown: CancellationToken,
    ) -> Self {
        Self {
            tasks,
            update_scanner: None,
            store,
            runtime,
            source_materializer: None,
            bindings,
            build_images: None,
            notifier,
            operations: Arc::new(Semaphore::new(4)),
            shutdown,
            timeout: Duration::from_secs(15 * 60),
            alerts: None,
            entitlements: None,
        }
    }

    #[must_use]
    pub fn with_source_materializer(
        mut self,
        source_materializer: Arc<dyn StackSourceMaterializerPort>,
    ) -> Self {
        self.source_materializer = Some(source_materializer);
        self
    }

    #[must_use]
    pub fn with_build_image_resolver(
        mut self,
        resolver: Arc<dyn StackBuildImageResolverPort>,
    ) -> Self {
        self.build_images = Some(resolver);
        self
    }

    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    #[must_use]
    pub fn with_alerts(mut self, alerts: Arc<dyn AlertEventSink>) -> Self {
        self.alerts = Some(alerts);
        self
    }
}

use crate::*;
mod bindings;
use bindings::*;

mod read;

mod drift;
use drift::*;

mod mutations;

mod apply;

mod delete;

mod adoption;

#[cfg(test)]
mod tests;

pub use bindings::import_runtime_fingerprint;
pub use drift::calculate_drift;

impl StackChangeNotifier for NoopStackChangeNotifier {
    fn changed(&self, _stack_id: Uuid, _event: &'static str) {}
}
