#![forbid(unsafe_code)]

pub mod adoption;
mod model;
mod service;

mod repository;
mod runtime;

pub mod permissions;

mod tasks;
pub use tasks::SwarmServiceTaskSpawner;

mod commands;

mod queries;

pub use model::{
    AutoUpdateState, MountKind, PortPublishMode, RestartCondition, RuntimeServiceResult,
    SchedulingMode, ServiceDeletionClaim, ServiceOperationClaim, ServiceOperationKind,
    SwarmService, SwarmServiceConfigReference, SwarmServiceError, SwarmServiceHealthCheck,
    SwarmServiceImageInfo, SwarmServiceMount, SwarmServiceOperation, SwarmServicePort,
    SwarmServiceProgressItem, SwarmServiceResources, SwarmServiceRestartPolicy,
    SwarmServiceSecretReference, SwarmServiceSpec, SwarmServiceUpdatePolicy,
    SwarmServiceWebhookConfig, UpdateBehavior, UpdateFailureAction, UpdateOrder,
};

pub use commands::{
    CreateSwarmService, RenameSwarmService, ScaleSwarmService, SwarmServiceDuplicateSource,
    UpdateSwarmService,
};

pub use queries::{
    SwarmServiceDetails, SwarmServiceDuplicateDraft, SwarmServiceFilter, TagSummary,
};

pub use service::{
    NoopSwarmServiceChangeNotifier, ServiceAutomationEntitlements, ServiceImageDigestPort,
    ServiceUpdateCheck, ServiceUpdateOutcome, SwarmServiceChangeNotifier, SwarmServiceService,
    checkable_image, evaluate_digest,
};

pub use repository::{ServiceOperationRequest, SwarmServiceRepository};

pub use runtime::{
    EmptySwarmServiceBindingResolver, ResolvedSwarmServiceBinding, ResolvedSwarmServiceBindings,
    SwarmServiceBindingResolverPort, SwarmServiceRuntime,
};
