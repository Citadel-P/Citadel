#![forbid(unsafe_code)]
pub use citadel_primitives::PatchField;

pub mod adoption;
mod tasks;
pub use tasks::DeploymentTaskSpawner;
mod commands;
mod model;
mod read_models;
pub use commands::{CreateDeployment, UpdateDeploymentMetadata};
pub use read_models::{
    DeploymentConfig, DeploymentDraft, DeploymentDuplicateDraft, DeploymentFilter, DuplicateSource,
    DuplicateWarning,
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
    ApplyClaim, ContainerRestartPolicy, DeletionClaim, Deployment, DeploymentApplyError,
    DeploymentBindingSnapshot, DeploymentError, DeploymentImageInfo, DeploymentProgress,
    DeploymentSpec, ImagePullProgress, LifeCycleSpec, PreparedDeploymentImage,
    ResolvedDeploymentBinding, ResolvedDeploymentBindings, ResolvedDeploymentBuild, ResourceSpec,
    RuntimeContainerState, RuntimeDeploymentCommand, RuntimeDeploymentResult, StopSignal,
    UpdateBehavior,
};
pub use service::{
    DeploymentChangeNotifier, DeploymentService, DeploymentUpdateCheck,
    NoopDeploymentChangeNotifier, checkable_deployment_image,
};

mod status;
pub use status::DeploymentStatus;
