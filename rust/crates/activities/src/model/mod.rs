use chrono::{DateTime, Utc};
use citadel_licensing::{LicenseCapability, LicenseStatus};
use citadel_primitives::{ActorId, PermissionLevel, ResourceType};
use serde::Serialize;
use serde_json::Value;
use std::fmt;
use uuid::Uuid;
mod alert;
pub use alert::AlertRuleActivitySnapshot;
mod automation;
pub use automation::AutomationActionActivitySnapshot;
mod backup;
pub use backup::BackupPolicyActivitySnapshot;
mod build;
pub use build::{
    BuildAgentPoolActivitySnapshot, BuildProjectActivitySnapshot, BuildSecretActivitySnapshot,
};
mod changes;
pub use changes::{ActivityChangedField, ActivityChangedFieldName};
mod deployment;
pub use deployment::{DeploymentActivitySnapshot, DeploymentResultActivitySnapshot};
mod event;
pub use event::{ActivityEvent, ActivityInvariantError};
mod git;
pub use git::{GitRepositoryActivitySnapshot, GitRepositorySyncActivitySnapshot};
mod identity;
pub use identity::{
    IdentityResourceAccessSnapshot, OidcProviderActivitySnapshot, RoleActivitySnapshot,
    RolePermissionActivitySnapshot, ServiceAccountActivitySnapshot,
    ServiceAccountResourceAccessSnapshot, TeamActivitySnapshot, UserActivitySnapshot,
};
mod info;
pub use info::ActivityEventInfo;
mod license;
pub use license::LicenseActivitySnapshot;
mod platform;
pub use platform::PlatformActivitySnapshot;
mod registry;
pub use registry::RegistryActivitySnapshot;
mod sources;
pub use sources::{ActivitySourceResource, VolumeContentDownloaded};
mod stack;
pub use stack::{StackActivitySnapshot, StackResultActivitySnapshot};
mod swarm;
pub use swarm::SwarmServiceActivitySnapshot;
mod vocabulary;
pub use vocabulary::{ActivityEventType, ActivityResourceType, ActivityStatus};
mod webhooks;
pub use webhooks::{WebhookActivityDetails, WebhookActivitySource};
#[cfg(test)]
mod tests;
