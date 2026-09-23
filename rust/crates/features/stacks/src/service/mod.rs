use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;

use citadel_activities::ActivityEventInfo;
use citadel_alerts::{AlertEventSink, AlertObservation};
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use tokio::sync::{Semaphore, mpsc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
mod update_checks;
pub use update_checks::StackUpdateScanner;
mod resolution_message;

use crate::ApplyStack;
use crate::ComposeModel;
use crate::ComposeProjectImportDraft;
use crate::ComposeProjectImportSource;
use crate::ComposeProjectImportValidation;
use crate::ComposeProjectServiceComparison;
use crate::ComposeProjectStackDraft;
use crate::CreateStack;
use crate::ImportComposeProject;
use crate::ResolvedStackBindings;
use crate::ResolvedStackBuildImageBinding;
use crate::RollbackStack;
use crate::StackAction;
use crate::StackAdoptionIssue;
use crate::StackConfig;
use crate::StackDetails;
use crate::StackDrift;
use crate::StackDriftMonitorFailure;
use crate::StackDriftMonitorResult;
use crate::StackDriftReport;
use crate::StackError;
use crate::StackFilter;
use crate::StackImportClaim;
use crate::StackOperationClaim;
use crate::StackOrchestrationMode;
use crate::StackProgressItem;
use crate::StackReleaseDetails;
use crate::StackReleaseStatus;
use crate::StackRuntimeResult;
use crate::StackRuntimeSnapshot;
use crate::StackSourceFile;
use crate::StackSpec;
use crate::UpdateStack;
use crate::analyze_swarm_compatibility;
use crate::compose_digest;
use crate::create_ownership_labels_override;
use crate::merge_json;
use crate::normalize_description;
use crate::normalize_name;
use crate::normalize_tags;
use crate::parse_compose;
use crate::validation;

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
