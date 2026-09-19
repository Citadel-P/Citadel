#![forbid(unsafe_code)]

pub mod adoption;
mod tasks;
pub use tasks::DeploymentTaskSpawner;
mod commands;
mod model;
mod queries;
pub use commands::{CreateDeployment, FieldPatch, UpdateDeploymentMetadata};
pub use queries::{
    DeploymentConfig, DeploymentDetails, DeploymentDraft, DeploymentDuplicateDraft,
    DeploymentFilter, DuplicateSource, DuplicateWarning,
};
pub mod permissions;
mod repository;
mod runtime;
mod service;
pub use repository::DeploymentRepository;
pub use runtime::{
    DeploymentBindingResolverPort, DeploymentEntitlementPort, DeploymentRuntime,
    EmptyDeploymentBindingResolver,
};

pub use model::{
    ApplyClaim, AutoUpdateState, ContainerRestartPolicy, DeletionClaim, Deployment,
    DeploymentApplyError, DeploymentBindingSnapshot, DeploymentError, DeploymentImageInfo,
    DeploymentProgress, DeploymentSpec, ImagePullProgress, LifeCycleSpec, PreparedDeploymentImage,
    ResolvedDeploymentBinding, ResolvedDeploymentBindings, ResolvedDeploymentBuild, ResourceSpec,
    RuntimeContainerState, RuntimeDeploymentCommand, RuntimeDeploymentResult, StopSignal,
    TagSummary, UpdateBehavior,
};
pub use service::{
    DeploymentChangeNotifier, DeploymentService, DeploymentUpdateCheck,
    NoopDeploymentChangeNotifier, checkable_deployment_image, evaluate_deployment_digest,
};
