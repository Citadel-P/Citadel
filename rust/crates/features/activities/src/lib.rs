#![forbid(unsafe_code)]
pub mod model;
pub use model::{
    ActivityChangedField, ActivityChangedFieldName, ActivityEvent, ActivityEventInfo,
    ActivityEventType, ActivityInvariantError, ActivityResourceType, ActivitySourceResource,
    ActivityStatus, AlertRuleActivitySnapshot, AutomationActionActivitySnapshot,
    BackupPolicyActivitySnapshot, BuildAgentPoolActivitySnapshot, BuildProjectActivitySnapshot,
    BuildSecretActivitySnapshot, DeploymentActivitySnapshot, DeploymentResultActivitySnapshot,
    GitRepositoryActivitySnapshot, GitRepositorySyncActivitySnapshot,
    IdentityResourceAccessSnapshot, LicenseActivitySnapshot, OidcProviderActivitySnapshot,
    PlatformActivitySnapshot, RegistryActivitySnapshot, RoleActivitySnapshot,
    RolePermissionActivitySnapshot, ServiceAccountActivitySnapshot,
    ServiceAccountResourceAccessSnapshot, StackActivitySnapshot, StackResultActivitySnapshot,
    SwarmServiceActivitySnapshot, TeamActivitySnapshot, UserActivitySnapshot,
    VolumeContentDownloaded, WebhookActivityDetails, WebhookActivitySource,
};

mod error;
mod read_models;
mod repository;
mod service;
pub use error::ActivityError;
pub use read_models::{
    ActivityAccess, ActivityFilter, ActivityRecord, ActivitySummary, DEFAULT_ACTIVITY_PAGE_SIZE,
    MAXIMUM_ACTIVITY_PAGE_SIZE, PagedActivityRecords, ValidatedActivityFilter,
};
pub use repository::{ActivityQueryStore, WebhookActivitySink};
pub use service::ActivityService;
