//! Deployment resource state, specification, claims, and runtime observations.
mod operations;
mod resource;
mod spec;
#[cfg(test)]
mod tests;
pub use operations::{
    ApplyClaim, DeletionClaim, DeploymentApplyError, DeploymentBindingSnapshot, DeploymentProgress,
    ImagePullProgress, PreparedDeploymentImage, ResolvedDeploymentBinding,
    ResolvedDeploymentBindings, ResolvedDeploymentBuild, RuntimeContainerState,
    RuntimeDeploymentCommand, RuntimeDeploymentResult,
};
pub use resource::{Deployment, DeploymentError};
pub use spec::{
    ContainerRestartPolicy, DeploymentImageInfo, DeploymentSpec, LifeCycleSpec, ResourceSpec,
    StopSignal, UpdateBehavior,
};
