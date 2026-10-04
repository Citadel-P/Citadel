//! Server-owned OpenAPI descriptions of activities values.
//! Field types and exhaustive conversions keep these descriptions tied to the domain.

use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

schema_model! {
    citadel_activities::RegistryActivitySnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[schema(as = RegistryActivitySnapshot)]
    pub struct RegistryActivitySnapshotSchema {
        #[serde(rename = "Id")]
        pub id: Uuid,
        #[serde(rename = "Name")]
        pub name: String,
        #[serde(rename = "Description")]
        pub description: String,
        #[serde(rename = "RegistryHost")]
        pub registry_host: String,
        #[serde(rename = "Status")]
        pub status: String,
        #[serde(rename = "Configuration")]
        pub configuration: Value,
    }
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(tag = "$type")]
#[schema(as = ActivityEventInfo)]
pub enum ActivityEventInfoSchema {
    InitialAdministratorCreated {
        #[serde(rename = "UserId")]
        user_id: Uuid,
        #[serde(rename = "UserName")]
        user_name: String,
        #[serde(rename = "Mode")]
        mode: String,
    },

    AlertRuleCreated {
        #[serde(rename = "AlertRule")]
        #[schema(value_type = crate::api::resources::schema_models::activities::AlertRuleActivitySnapshotSchema)]
        alert_rule: citadel_activities::AlertRuleActivitySnapshot,
    },

    AlertRuleUpdated {
        #[serde(rename = "OldRule")]
        #[schema(value_type = crate::api::resources::schema_models::activities::AlertRuleActivitySnapshotSchema)]
        old_rule: citadel_activities::AlertRuleActivitySnapshot,
        #[serde(rename = "NewRule")]
        #[schema(value_type = crate::api::resources::schema_models::activities::AlertRuleActivitySnapshotSchema)]
        new_rule: citadel_activities::AlertRuleActivitySnapshot,
    },

    BackupPolicyCreated {
        #[serde(rename = "Policy")]
        #[schema(value_type = crate::api::resources::schema_models::activities::BackupPolicyActivitySnapshotSchema)]
        policy: citadel_activities::BackupPolicyActivitySnapshot,
    },

    BackupRunQueued {
        #[serde(rename = "RunId")]
        run_id: Uuid,
        #[serde(rename = "Trigger")]
        trigger: String,
    },

    BackupRunStarted {
        #[serde(rename = "RunId")]
        run_id: Uuid,
        #[serde(rename = "Trigger")]
        trigger: String,
    },

    BackupRunCompleted {
        #[serde(rename = "RunId")]
        run_id: Uuid,
        #[serde(rename = "Trigger")]
        trigger: String,
        #[serde(rename = "Status")]
        status: String,
        #[serde(rename = "DurationMs")]
        duration_ms: Option<i64>,
        #[serde(rename = "ErrorMessage")]
        error_message: Option<String>,
    },

    BackupPolicyRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },

    BackupPolicyUpdated {
        #[serde(rename = "OldPolicy")]
        #[schema(value_type = crate::api::resources::schema_models::activities::BackupPolicyActivitySnapshotSchema)]
        old_policy: citadel_activities::BackupPolicyActivitySnapshot,
        #[serde(rename = "NewPolicy")]
        #[schema(value_type = crate::api::resources::schema_models::activities::BackupPolicyActivitySnapshotSchema)]
        new_policy: citadel_activities::BackupPolicyActivitySnapshot,
    },

    AlertRuleRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },

    VolumeContentDownloaded(
        crate::api::resources::schema_models::activities::VolumeContentDownloadedSchema,
    ),

    GitRepoWebhookReceived(
        crate::api::resources::schema_models::activities::WebhookActivityDetailsSchema,
    ),

    StackWebhookReceived(
        crate::api::resources::schema_models::activities::WebhookActivityDetailsSchema,
    ),

    BuildWebhookReceived(
        crate::api::resources::schema_models::activities::WebhookActivityDetailsSchema,
    ),

    ActionWebhookReceived(
        crate::api::resources::schema_models::activities::WebhookActivityDetailsSchema,
    ),

    BackupPolicyWebhookReceived(
        crate::api::resources::schema_models::activities::WebhookActivityDetailsSchema,
    ),

    SwarmServiceWebhookReceived(
        crate::api::resources::schema_models::activities::WebhookActivityDetailsSchema,
    ),

    UserProfileUpdated {
        #[serde(rename = "Changes")]
        #[schema(value_type = Vec < crate::api::resources::schema_models::activities::ActivityChangedFieldSchema >)]
        changes: Vec<citadel_activities::ActivityChangedField>,
    },

    UserPreferencesUpdated {
        #[serde(rename = "Changes")]
        #[schema(value_type = Vec < crate::api::resources::schema_models::activities::ActivityChangedFieldSchema >)]
        changes: Vec<citadel_activities::ActivityChangedField>,
    },

    UserPasswordChanged,

    UserSessionRevoked {
        #[serde(rename = "SessionId")]
        session_id: Uuid,
    },

    UserOtherSessionsRevoked {
        #[serde(rename = "Count")]
        count: i32,
    },

    UserMfaEnabled,

    UserMfaDisabled,

    UserMfaVerificationFailed,

    UserMfaRecoveryCodeUsed,

    UserMfaRecoveryCodesRegenerated,

    UserMfaResetByAdministrator {
        #[serde(rename = "TargetUserId")]
        target_user_id: Uuid,
    },

    UserCreated {
        #[serde(rename = "User")]
        #[schema(value_type = crate::api::resources::schema_models::activities::UserActivitySnapshotSchema)]
        user: citadel_activities::UserActivitySnapshot,
    },

    UserUpdated {
        #[serde(rename = "OldUser")]
        #[schema(value_type = crate::api::resources::schema_models::activities::UserActivitySnapshotSchema)]
        old_user: citadel_activities::UserActivitySnapshot,
        #[serde(rename = "NewUser")]
        #[schema(value_type = crate::api::resources::schema_models::activities::UserActivitySnapshotSchema)]
        new_user: citadel_activities::UserActivitySnapshot,
        #[serde(rename = "PasswordChanged")]
        password_changed: bool,
    },

    UserRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },

    UserDeleted {
        #[serde(rename = "User")]
        #[schema(value_type = crate::api::resources::schema_models::activities::UserActivitySnapshotSchema)]
        user: citadel_activities::UserActivitySnapshot,
    },

    TeamCreated {
        #[serde(rename = "Team")]
        #[schema(value_type = crate::api::resources::schema_models::activities::TeamActivitySnapshotSchema)]
        team: citadel_activities::TeamActivitySnapshot,
    },

    TeamUpdated {
        #[serde(rename = "OldTeam")]
        #[schema(value_type = crate::api::resources::schema_models::activities::TeamActivitySnapshotSchema)]
        old_team: citadel_activities::TeamActivitySnapshot,
        #[serde(rename = "NewTeam")]
        #[schema(value_type = crate::api::resources::schema_models::activities::TeamActivitySnapshotSchema)]
        new_team: citadel_activities::TeamActivitySnapshot,
    },

    TeamRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },

    TeamDeleted {
        #[serde(rename = "Team")]
        #[schema(value_type = crate::api::resources::schema_models::activities::TeamActivitySnapshotSchema)]
        team: citadel_activities::TeamActivitySnapshot,
    },

    RoleCreated {
        #[serde(rename = "Role")]
        #[schema(value_type = crate::api::resources::schema_models::activities::RoleActivitySnapshotSchema)]
        role: citadel_activities::RoleActivitySnapshot,
    },

    RoleUpdated {
        #[serde(rename = "OldRole")]
        #[schema(value_type = crate::api::resources::schema_models::activities::RoleActivitySnapshotSchema)]
        old_role: citadel_activities::RoleActivitySnapshot,
        #[serde(rename = "NewRole")]
        #[schema(value_type = crate::api::resources::schema_models::activities::RoleActivitySnapshotSchema)]
        new_role: citadel_activities::RoleActivitySnapshot,
    },

    RoleRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },

    RoleDeleted {
        #[serde(rename = "Role")]
        #[schema(value_type = crate::api::resources::schema_models::activities::RoleActivitySnapshotSchema)]
        role: citadel_activities::RoleActivitySnapshot,
    },

    ServiceAccountCreated {
        #[serde(rename = "Account")]
        #[schema(value_type = crate::api::resources::schema_models::activities::ServiceAccountActivitySnapshotSchema)]
        account: citadel_activities::ServiceAccountActivitySnapshot,
    },

    ServiceAccountUpdated {
        #[serde(rename = "OldAccount")]
        #[schema(value_type = crate::api::resources::schema_models::activities::ServiceAccountActivitySnapshotSchema)]
        old_account: citadel_activities::ServiceAccountActivitySnapshot,
        #[serde(rename = "NewAccount")]
        #[schema(value_type = crate::api::resources::schema_models::activities::ServiceAccountActivitySnapshotSchema)]
        new_account: citadel_activities::ServiceAccountActivitySnapshot,
    },

    ServiceAccountRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },

    ServiceAccountEnabled {
        #[serde(rename = "AccountId")]
        account_id: Uuid,
    },

    ServiceAccountDisabled {
        #[serde(rename = "AccountId")]
        account_id: Uuid,
    },

    ServiceAccountArchived {
        #[serde(rename = "AccountId")]
        account_id: Uuid,
    },

    ServiceAccountTokenCreated {
        #[serde(rename = "AccountId")]
        account_id: Uuid,
        #[serde(rename = "TokenId")]
        token_id: Uuid,
        #[serde(rename = "TokenName")]
        token_name: String,
        #[serde(rename = "PublicHint")]
        public_hint: String,
        #[serde(rename = "ExpiresAtUtc")]
        expires_at_utc: Option<DateTime<Utc>>,
    },

    ServiceAccountTokenRevoked {
        #[serde(rename = "AccountId")]
        account_id: Uuid,
        #[serde(rename = "TokenId")]
        token_id: Uuid,
        #[serde(rename = "PublicHint")]
        public_hint: String,
    },

    LicenseInstalled {
        #[serde(rename = "License")]
        #[schema(value_type = Box < crate::api::resources::schema_models::activities::LicenseActivitySnapshotSchema >)]
        license: Box<citadel_activities::LicenseActivitySnapshot>,
    },

    LicenseReplaced {
        #[serde(rename = "OldLicense")]
        #[schema(value_type = Box < crate::api::resources::schema_models::activities::LicenseActivitySnapshotSchema >)]
        old_license: Box<citadel_activities::LicenseActivitySnapshot>,
        #[serde(rename = "NewLicense")]
        #[schema(value_type = Box < crate::api::resources::schema_models::activities::LicenseActivitySnapshotSchema >)]
        new_license: Box<citadel_activities::LicenseActivitySnapshot>,
    },

    LicenseRemoved {
        #[serde(rename = "License")]
        #[schema(value_type = Box < crate::api::resources::schema_models::activities::LicenseActivitySnapshotSchema >)]
        license: Box<citadel_activities::LicenseActivitySnapshot>,
    },

    LicenseEnteredGracePeriod {
        #[serde(rename = "License")]
        #[schema(value_type = Box < crate::api::resources::schema_models::activities::LicenseActivitySnapshotSchema >)]
        license: Box<citadel_activities::LicenseActivitySnapshot>,
    },

    LicenseExpired {
        #[serde(rename = "License")]
        #[schema(value_type = Box < crate::api::resources::schema_models::activities::LicenseActivitySnapshotSchema >)]
        license: Box<citadel_activities::LicenseActivitySnapshot>,
    },

    LicenseValidationFailed {
        #[serde(rename = "Fingerprint")]
        fingerprint: Option<String>,
        #[serde(rename = "Status")]
        #[schema(value_type = crate::api::resources::schema_models::licensing::LicenseStatusSchema)]
        status: citadel_licensing::LicenseStatus,
        #[serde(rename = "ErrorCode")]
        error_code: Option<String>,
    },

    OidcProviderCreated {
        #[serde(rename = "Provider")]
        #[schema(value_type = Box < crate::api::resources::schema_models::activities::OidcProviderActivitySnapshotSchema >)]
        provider: Box<citadel_activities::OidcProviderActivitySnapshot>,
    },

    OidcProviderUpdated {
        #[serde(rename = "OldProvider")]
        #[schema(value_type = Box < crate::api::resources::schema_models::activities::OidcProviderActivitySnapshotSchema >)]
        old_provider: Box<citadel_activities::OidcProviderActivitySnapshot>,
        #[serde(rename = "NewProvider")]
        #[schema(value_type = Box < crate::api::resources::schema_models::activities::OidcProviderActivitySnapshotSchema >)]
        new_provider: Box<citadel_activities::OidcProviderActivitySnapshot>,
    },

    OidcProviderRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },

    OidcProviderDeleted {
        #[serde(rename = "Provider")]
        #[schema(value_type = Box < crate::api::resources::schema_models::activities::OidcProviderActivitySnapshotSchema >)]
        provider: Box<citadel_activities::OidcProviderActivitySnapshot>,
    },

    RegistryCreated {
        #[serde(rename = "Registry")]
        #[schema(value_type = crate::api::resources::schema_models::activities::RegistryActivitySnapshotSchema)]
        registry: citadel_activities::RegistryActivitySnapshot,
    },

    RegistryUpdated {
        #[serde(rename = "OldRegistry")]
        #[schema(value_type = crate::api::resources::schema_models::activities::RegistryActivitySnapshotSchema)]
        old_registry: citadel_activities::RegistryActivitySnapshot,
        #[serde(rename = "NewRegistry")]
        #[schema(value_type = crate::api::resources::schema_models::activities::RegistryActivitySnapshotSchema)]
        new_registry: citadel_activities::RegistryActivitySnapshot,
    },

    RegistryRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },

    RegistryDeleted {
        #[serde(rename = "Registry")]
        #[schema(value_type = crate::api::resources::schema_models::activities::RegistryActivitySnapshotSchema)]
        registry: citadel_activities::RegistryActivitySnapshot,
    },

    DeploymentCreated {
        #[serde(rename = "Deployment")]
        #[schema(value_type = crate::api::resources::schema_models::activities::DeploymentActivitySnapshotSchema)]
        deployment: citadel_activities::DeploymentActivitySnapshot,
    },

    DeploymentAdopted {
        #[serde(rename = "Deployment")]
        #[schema(value_type = crate::api::resources::schema_models::activities::DeploymentActivitySnapshotSchema)]
        deployment: citadel_activities::DeploymentActivitySnapshot,
        #[serde(rename = "ContainerId")]
        container_id: String,
        #[serde(rename = "ContainerName")]
        container_name: String,
    },

    DeploymentDuplicated {
        #[serde(rename = "Deployment")]
        #[schema(value_type = crate::api::resources::schema_models::activities::DeploymentActivitySnapshotSchema)]
        deployment: citadel_activities::DeploymentActivitySnapshot,
        #[serde(rename = "Source")]
        #[schema(value_type = crate::api::resources::schema_models::activities::ActivitySourceResourceSchema)]
        source: citadel_activities::ActivitySourceResource,
    },

    DeploymentUpdated {
        #[serde(rename = "OldDeployment")]
        #[schema(value_type = crate::api::resources::schema_models::activities::DeploymentActivitySnapshotSchema)]
        old_deployment: citadel_activities::DeploymentActivitySnapshot,
        #[serde(rename = "NewDeployment")]
        #[schema(value_type = crate::api::resources::schema_models::activities::DeploymentActivitySnapshotSchema)]
        new_deployment: citadel_activities::DeploymentActivitySnapshot,
    },

    DeploymentRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },

    DeploymentDeleted {
        #[serde(rename = "Deployment")]
        #[schema(value_type = crate::api::resources::schema_models::activities::DeploymentActivitySnapshotSchema)]
        deployment: citadel_activities::DeploymentActivitySnapshot,
    },

    DeploymentStarted {
        #[serde(rename = "ContainerIds")]
        container_ids: Vec<String>,
    },

    DeploymentStopped {
        #[serde(rename = "ContainerIds")]
        container_ids: Vec<String>,
    },

    DeploymentPaused {
        #[serde(rename = "ContainerIds")]
        container_ids: Vec<String>,
    },

    DeploymentDegraded {
        #[serde(rename = "Reason")]
        reason: String,
    },

    StackDegraded {
        #[serde(rename = "Reason")]
        reason: String,
    },

    DeploymentApplied {
        #[serde(rename = "Deployment")]
        #[schema(value_type = Option < crate::api::resources::schema_models::activities::DeploymentActivitySnapshotSchema >)]
        deployment: Option<citadel_activities::DeploymentActivitySnapshot>,
        #[serde(rename = "Result")]
        #[schema(value_type = crate::api::resources::schema_models::activities::DeploymentResultActivitySnapshotSchema)]
        result: citadel_activities::DeploymentResultActivitySnapshot,
    },

    StackCreated {
        #[serde(rename = "Stack")]
        #[schema(value_type = crate::api::resources::schema_models::activities::StackActivitySnapshotSchema)]
        stack: citadel_activities::StackActivitySnapshot,
    },

    StackDuplicated {
        #[serde(rename = "Stack")]
        #[schema(value_type = crate::api::resources::schema_models::activities::StackActivitySnapshotSchema)]
        stack: citadel_activities::StackActivitySnapshot,
        #[serde(rename = "Source")]
        #[schema(value_type = crate::api::resources::schema_models::activities::ActivitySourceResourceSchema)]
        source: citadel_activities::ActivitySourceResource,
    },

    StackUpdated {
        #[serde(rename = "OldStack")]
        #[schema(value_type = crate::api::resources::schema_models::activities::StackActivitySnapshotSchema)]
        old_stack: citadel_activities::StackActivitySnapshot,
        #[serde(rename = "NewStack")]
        #[schema(value_type = crate::api::resources::schema_models::activities::StackActivitySnapshotSchema)]
        new_stack: citadel_activities::StackActivitySnapshot,
    },

    StackRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },

    StackDeleted {
        #[serde(rename = "Stack")]
        #[schema(value_type = crate::api::resources::schema_models::activities::StackActivitySnapshotSchema)]
        stack: citadel_activities::StackActivitySnapshot,
    },

    StackStarted {
        #[serde(rename = "ContainerIds")]
        container_ids: Vec<String>,
    },

    StackStopped {
        #[serde(rename = "ContainerIds")]
        container_ids: Vec<String>,
    },

    StackPaused {
        #[serde(rename = "ContainerIds")]
        container_ids: Vec<String>,
    },

    StackApplied {
        #[serde(rename = "Stack")]
        #[schema(value_type = Option < crate::api::resources::schema_models::activities::StackActivitySnapshotSchema >)]
        stack: Option<citadel_activities::StackActivitySnapshot>,
        #[serde(rename = "Result")]
        #[schema(value_type = crate::api::resources::schema_models::activities::StackResultActivitySnapshotSchema)]
        result: citadel_activities::StackResultActivitySnapshot,
    },

    StackRollback {
        #[serde(rename = "OldStack")]
        #[schema(value_type = Option < crate::api::resources::schema_models::activities::StackActivitySnapshotSchema >)]
        old_stack: Option<citadel_activities::StackActivitySnapshot>,
        #[serde(rename = "NewStack")]
        #[schema(value_type = Option < crate::api::resources::schema_models::activities::StackActivitySnapshotSchema >)]
        new_stack: Option<citadel_activities::StackActivitySnapshot>,
        #[serde(rename = "Result")]
        #[schema(value_type = crate::api::resources::schema_models::activities::StackResultActivitySnapshotSchema)]
        result: citadel_activities::StackResultActivitySnapshot,
    },

    StackDriftDetected {
        #[serde(rename = "Reason")]
        reason: String,
        #[serde(rename = "Fingerprint")]
        fingerprint: String,
    },

    StackDriftResolved {
        #[serde(rename = "PreviousFingerprint")]
        previous_fingerprint: String,
    },

    StackImported {
        #[serde(rename = "Stack")]
        #[schema(value_type = crate::api::resources::schema_models::activities::StackActivitySnapshotSchema)]
        stack: citadel_activities::StackActivitySnapshot,
        #[serde(rename = "ProjectName")]
        project_name: String,
        #[serde(rename = "ServiceNames", default)]
        service_names: Vec<String>,
    },

    SwarmServiceCreated {
        #[serde(rename = "Service")]
        #[schema(value_type = crate::api::resources::schema_models::activities::SwarmServiceActivitySnapshotSchema)]
        service: citadel_activities::SwarmServiceActivitySnapshot,
    },

    SwarmServiceDuplicated {
        #[serde(rename = "Service")]
        #[schema(value_type = crate::api::resources::schema_models::activities::SwarmServiceActivitySnapshotSchema)]
        service: citadel_activities::SwarmServiceActivitySnapshot,
        #[serde(rename = "Source")]
        #[schema(value_type = crate::api::resources::schema_models::activities::ActivitySourceResourceSchema)]
        source: citadel_activities::ActivitySourceResource,
    },

    SwarmServiceAdopted {
        #[serde(rename = "Service")]
        #[schema(value_type = crate::api::resources::schema_models::activities::SwarmServiceActivitySnapshotSchema)]
        service: citadel_activities::SwarmServiceActivitySnapshot,
        #[serde(rename = "DockerServiceId")]
        docker_service_id: String,
    },

    SwarmServiceUpdated {
        #[serde(rename = "OldService")]
        #[schema(value_type = crate::api::resources::schema_models::activities::SwarmServiceActivitySnapshotSchema)]
        old_service: citadel_activities::SwarmServiceActivitySnapshot,
        #[serde(rename = "NewService")]
        #[schema(value_type = crate::api::resources::schema_models::activities::SwarmServiceActivitySnapshotSchema)]
        new_service: citadel_activities::SwarmServiceActivitySnapshot,
    },

    SwarmServiceRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },

    SwarmServiceDeleted {
        #[serde(rename = "Service")]
        #[schema(value_type = crate::api::resources::schema_models::activities::SwarmServiceActivitySnapshotSchema)]
        service: citadel_activities::SwarmServiceActivitySnapshot,
    },

    SwarmServiceApplied {
        #[serde(rename = "OperationId")]
        operation_id: Uuid,
        #[serde(rename = "Warnings")]
        warnings: Vec<String>,
    },

    SwarmServiceScaled {
        #[serde(rename = "OperationId")]
        operation_id: Uuid,
        #[serde(rename = "Replicas")]
        replicas: i32,
        #[serde(rename = "Warnings")]
        warnings: Vec<String>,
    },

    SwarmServiceForceUpdated {
        #[serde(rename = "OperationId")]
        operation_id: Uuid,
        #[serde(rename = "Warnings")]
        warnings: Vec<String>,
    },

    SwarmServiceOperationFailed {
        #[serde(rename = "OperationId")]
        operation_id: Uuid,
        #[serde(rename = "Kind")]
        kind: String,
        #[serde(rename = "Reason")]
        reason: String,
    },

    PlatformConnected {
        #[serde(rename = "Platform")]
        #[schema(value_type = crate::api::resources::schema_models::activities::PlatformActivitySnapshotSchema)]
        platform: citadel_activities::PlatformActivitySnapshot,
        #[serde(rename = "PreviousStatus")]
        previous_status: String,
    },

    PlatformDisconnected {
        #[serde(rename = "Platform")]
        #[schema(value_type = crate::api::resources::schema_models::activities::PlatformActivitySnapshotSchema)]
        platform: citadel_activities::PlatformActivitySnapshot,
        #[serde(rename = "PreviousStatus")]
        previous_status: String,
    },

    PlatformCreated {
        #[serde(rename = "Platform")]
        #[schema(value_type = crate::api::resources::schema_models::activities::PlatformActivitySnapshotSchema)]
        platform: citadel_activities::PlatformActivitySnapshot,
    },

    PlatformRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },

    PlatformDeleted {
        #[serde(rename = "Platform")]
        #[schema(value_type = crate::api::resources::schema_models::activities::PlatformActivitySnapshotSchema)]
        platform: citadel_activities::PlatformActivitySnapshot,
    },

    PlatformNodeAgentLifecycle {
        #[serde(rename = "OperationId")]
        operation_id: Uuid,
        #[serde(rename = "Kind")]
        kind: &'static str,
        #[serde(rename = "State")]
        state: &'static str,
        #[serde(rename = "Message")]
        message: String,
    },

    GitRepoCreated {
        #[serde(rename = "GitRepo")]
        #[schema(value_type = crate::api::resources::schema_models::activities::GitRepositoryActivitySnapshotSchema)]
        git_repo: citadel_activities::GitRepositoryActivitySnapshot,
    },

    GitRepoUpdated {
        #[serde(rename = "OldGitRepo")]
        #[schema(value_type = crate::api::resources::schema_models::activities::GitRepositoryActivitySnapshotSchema)]
        old_git_repo: citadel_activities::GitRepositoryActivitySnapshot,
        #[serde(rename = "NewGitRepo")]
        #[schema(value_type = crate::api::resources::schema_models::activities::GitRepositoryActivitySnapshotSchema)]
        new_git_repo: citadel_activities::GitRepositoryActivitySnapshot,
    },

    GitRepoRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },

    GitRepoDeleted {
        #[serde(rename = "GitRepo")]
        #[schema(value_type = crate::api::resources::schema_models::activities::GitRepositoryActivitySnapshotSchema)]
        git_repo: citadel_activities::GitRepositoryActivitySnapshot,
    },

    GitRepoPulled {
        #[serde(rename = "GitRepo")]
        #[schema(value_type = crate::api::resources::schema_models::activities::GitRepositoryActivitySnapshotSchema)]
        git_repo: citadel_activities::GitRepositoryActivitySnapshot,
        #[serde(rename = "Result")]
        #[schema(value_type = crate::api::resources::schema_models::activities::GitRepositorySyncActivitySnapshotSchema)]
        result: citadel_activities::GitRepositorySyncActivitySnapshot,
    },

    GitRepoCloned {
        #[serde(rename = "GitRepo")]
        #[schema(value_type = crate::api::resources::schema_models::activities::GitRepositoryActivitySnapshotSchema)]
        git_repo: citadel_activities::GitRepositoryActivitySnapshot,
        #[serde(rename = "Result")]
        #[schema(value_type = crate::api::resources::schema_models::activities::GitRepositorySyncActivitySnapshotSchema)]
        result: citadel_activities::GitRepositorySyncActivitySnapshot,
    },

    ActionCreated {
        #[serde(rename = "Action")]
        #[schema(value_type = crate::api::resources::schema_models::activities::AutomationActionActivitySnapshotSchema)]
        action: citadel_activities::AutomationActionActivitySnapshot,
    },

    ActionUpdated {
        #[serde(rename = "OldAction")]
        #[schema(value_type = crate::api::resources::schema_models::activities::AutomationActionActivitySnapshotSchema)]
        old_action: citadel_activities::AutomationActionActivitySnapshot,
        #[serde(rename = "NewAction")]
        #[schema(value_type = crate::api::resources::schema_models::activities::AutomationActionActivitySnapshotSchema)]
        new_action: citadel_activities::AutomationActionActivitySnapshot,
    },

    ActionRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },

    ActionDeleted {
        #[serde(rename = "Action")]
        #[schema(value_type = crate::api::resources::schema_models::activities::AutomationActionActivitySnapshotSchema)]
        action: citadel_activities::AutomationActionActivitySnapshot,
    },

    ActionRunQueued {
        #[serde(rename = "RunId")]
        run_id: Uuid,
        #[serde(rename = "Trigger")]
        trigger: String,
    },

    ActionRunStarted {
        #[serde(rename = "RunId")]
        run_id: Uuid,
        #[serde(rename = "Trigger")]
        trigger: String,
    },

    ActionRunSucceeded {
        #[serde(rename = "RunId")]
        run_id: Uuid,
        #[serde(rename = "Trigger")]
        trigger: String,
        #[serde(rename = "ExitCode")]
        exit_code: Option<i32>,
        #[serde(rename = "DurationMs")]
        duration_ms: Option<i64>,
    },

    ActionRunFailed {
        #[serde(rename = "RunId")]
        run_id: Uuid,
        #[serde(rename = "Trigger")]
        trigger: String,
        #[serde(rename = "ExitCode")]
        exit_code: Option<i32>,
        #[serde(rename = "DurationMs")]
        duration_ms: Option<i64>,
        #[serde(rename = "ErrorMessage")]
        error_message: Option<String>,
    },

    ActionRunTimedOut {
        #[serde(rename = "RunId")]
        run_id: Uuid,
        #[serde(rename = "Trigger")]
        trigger: String,
        #[serde(rename = "DurationMs")]
        duration_ms: Option<i64>,
        #[serde(rename = "ErrorMessage")]
        error_message: Option<String>,
    },

    ActionRunCancelled {
        #[serde(rename = "RunId")]
        run_id: Uuid,
        #[serde(rename = "Trigger")]
        trigger: String,
    },

    ActionRunRejected {
        #[serde(rename = "RunId")]
        run_id: Uuid,
        #[serde(rename = "Trigger")]
        trigger: String,
        #[serde(rename = "Reason")]
        reason: String,
    },

    BuildRunQueued {
        #[serde(rename = "RunId")]
        run_id: Uuid,
        #[serde(rename = "Trigger")]
        trigger: String,
    },

    BuildRunStarted {
        #[serde(rename = "RunId")]
        run_id: Uuid,
        #[serde(rename = "Trigger")]
        trigger: String,
    },

    BuildRunSucceeded {
        #[serde(rename = "RunId")]
        run_id: Uuid,
        #[serde(rename = "Trigger")]
        trigger: String,
        #[serde(rename = "ExitCode")]
        exit_code: Option<i32>,
        #[serde(rename = "DurationMs")]
        duration_ms: Option<i64>,
        #[serde(rename = "ImageDigest")]
        image_digest: Option<String>,
    },

    BuildRunFailed {
        #[serde(rename = "RunId")]
        run_id: Uuid,
        #[serde(rename = "Trigger")]
        trigger: String,
        #[serde(rename = "Status")]
        status: String,
        #[serde(rename = "ExitCode")]
        exit_code: Option<i32>,
        #[serde(rename = "DurationMs")]
        duration_ms: Option<i64>,
        #[serde(rename = "ErrorMessage")]
        error_message: Option<String>,
    },

    BuildRunTimedOut {
        #[serde(rename = "RunId")]
        run_id: Uuid,
        #[serde(rename = "Trigger")]
        trigger: String,
        #[serde(rename = "DurationMs")]
        duration_ms: Option<i64>,
        #[serde(rename = "ErrorMessage")]
        error_message: Option<String>,
    },

    BuildRunCancelled {
        #[serde(rename = "RunId")]
        run_id: Uuid,
        #[serde(rename = "Trigger")]
        trigger: String,
    },

    BuildCreated {
        #[serde(rename = "Build")]
        #[schema(value_type = crate::api::resources::schema_models::activities::BuildProjectActivitySnapshotSchema)]
        build: citadel_activities::BuildProjectActivitySnapshot,
    },

    BuildUpdated {
        #[serde(rename = "OldBuild")]
        #[schema(value_type = crate::api::resources::schema_models::activities::BuildProjectActivitySnapshotSchema)]
        old_build: citadel_activities::BuildProjectActivitySnapshot,
        #[serde(rename = "NewBuild")]
        #[schema(value_type = crate::api::resources::schema_models::activities::BuildProjectActivitySnapshotSchema)]
        new_build: citadel_activities::BuildProjectActivitySnapshot,
    },

    BuildRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },

    BuildDeleted {
        #[serde(rename = "Build")]
        #[schema(value_type = crate::api::resources::schema_models::activities::BuildProjectActivitySnapshotSchema)]
        build: citadel_activities::BuildProjectActivitySnapshot,
    },

    BuildAgentPoolConnected {
        #[serde(rename = "AgentId")]
        agent_id: Uuid,
        #[serde(rename = "PreviousStatus")]
        previous_status: String,
        #[serde(rename = "Reason")]
        reason: String,
    },
    BuildAgentPoolDisconnected {
        #[serde(rename = "AgentId")]
        agent_id: Uuid,
        #[serde(rename = "PreviousStatus")]
        previous_status: String,
        #[serde(rename = "Reason")]
        reason: String,
    },
    BuildAgentPoolCreated {
        #[serde(rename = "Pool")]
        #[schema(value_type = crate::api::resources::schema_models::activities::BuildAgentPoolActivitySnapshotSchema)]
        pool: citadel_activities::BuildAgentPoolActivitySnapshot,
    },

    BuildAgentPoolUpdated {
        #[serde(rename = "OldPool")]
        #[schema(value_type = crate::api::resources::schema_models::activities::BuildAgentPoolActivitySnapshotSchema)]
        old_pool: citadel_activities::BuildAgentPoolActivitySnapshot,
        #[serde(rename = "NewPool")]
        #[schema(value_type = crate::api::resources::schema_models::activities::BuildAgentPoolActivitySnapshotSchema)]
        new_pool: citadel_activities::BuildAgentPoolActivitySnapshot,
    },

    BuildAgentPoolRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },

    BuildAgentPoolDeleted {
        #[serde(rename = "Pool")]
        #[schema(value_type = crate::api::resources::schema_models::activities::BuildAgentPoolActivitySnapshotSchema)]
        pool: citadel_activities::BuildAgentPoolActivitySnapshot,
    },

    BuildAgentPoolTested {
        #[serde(rename = "Pool")]
        #[schema(value_type = crate::api::resources::schema_models::activities::BuildAgentPoolActivitySnapshotSchema)]
        pool: citadel_activities::BuildAgentPoolActivitySnapshot,
        #[serde(rename = "Status")]
        status: String,
        #[serde(rename = "Message")]
        message: String,
    },
}

impl From<citadel_activities::ActivityEventInfo> for ActivityEventInfoSchema {
    fn from(value: citadel_activities::ActivityEventInfo) -> Self {
        match value {
            citadel_activities::ActivityEventInfo::InitialAdministratorCreated {
                user_id,
                user_name,
                mode,
            } => Self::InitialAdministratorCreated {
                user_id,
                user_name,
                mode,
            },
            citadel_activities::ActivityEventInfo::AlertRuleCreated { alert_rule } => {
                Self::AlertRuleCreated { alert_rule }
            }
            citadel_activities::ActivityEventInfo::AlertRuleUpdated { old_rule, new_rule } => {
                Self::AlertRuleUpdated { old_rule, new_rule }
            }
            citadel_activities::ActivityEventInfo::BackupPolicyCreated { policy } => {
                Self::BackupPolicyCreated { policy }
            }
            citadel_activities::ActivityEventInfo::BackupRunQueued { run_id, trigger } => {
                Self::BackupRunQueued { run_id, trigger }
            }
            citadel_activities::ActivityEventInfo::BackupRunStarted { run_id, trigger } => {
                Self::BackupRunStarted { run_id, trigger }
            }
            citadel_activities::ActivityEventInfo::BackupRunCompleted {
                run_id,
                trigger,
                status,
                duration_ms,
                error_message,
            } => Self::BackupRunCompleted {
                run_id,
                trigger,
                status,
                duration_ms,
                error_message,
            },
            citadel_activities::ActivityEventInfo::BackupPolicyRenamed { old_name, new_name } => {
                Self::BackupPolicyRenamed { old_name, new_name }
            }
            citadel_activities::ActivityEventInfo::BackupPolicyUpdated {
                old_policy,
                new_policy,
            } => Self::BackupPolicyUpdated {
                old_policy,
                new_policy,
            },
            citadel_activities::ActivityEventInfo::AlertRuleRenamed { old_name, new_name } => {
                Self::AlertRuleRenamed { old_name, new_name }
            }
            citadel_activities::ActivityEventInfo::VolumeContentDownloaded(field_0) => {
                Self::VolumeContentDownloaded(field_0.into())
            }
            citadel_activities::ActivityEventInfo::GitRepoWebhookReceived(field_0) => {
                Self::GitRepoWebhookReceived(field_0.into())
            }
            citadel_activities::ActivityEventInfo::StackWebhookReceived(field_0) => {
                Self::StackWebhookReceived(field_0.into())
            }
            citadel_activities::ActivityEventInfo::BuildWebhookReceived(field_0) => {
                Self::BuildWebhookReceived(field_0.into())
            }
            citadel_activities::ActivityEventInfo::ActionWebhookReceived(field_0) => {
                Self::ActionWebhookReceived(field_0.into())
            }
            citadel_activities::ActivityEventInfo::BackupPolicyWebhookReceived(field_0) => {
                Self::BackupPolicyWebhookReceived(field_0.into())
            }
            citadel_activities::ActivityEventInfo::SwarmServiceWebhookReceived(field_0) => {
                Self::SwarmServiceWebhookReceived(field_0.into())
            }
            citadel_activities::ActivityEventInfo::UserProfileUpdated { changes } => {
                Self::UserProfileUpdated { changes }
            }
            citadel_activities::ActivityEventInfo::UserPreferencesUpdated { changes } => {
                Self::UserPreferencesUpdated { changes }
            }
            citadel_activities::ActivityEventInfo::UserPasswordChanged => Self::UserPasswordChanged,
            citadel_activities::ActivityEventInfo::UserSessionRevoked { session_id } => {
                Self::UserSessionRevoked { session_id }
            }
            citadel_activities::ActivityEventInfo::UserOtherSessionsRevoked { count } => {
                Self::UserOtherSessionsRevoked { count }
            }
            citadel_activities::ActivityEventInfo::UserMfaEnabled => Self::UserMfaEnabled,
            citadel_activities::ActivityEventInfo::UserMfaDisabled => Self::UserMfaDisabled,
            citadel_activities::ActivityEventInfo::UserMfaVerificationFailed => {
                Self::UserMfaVerificationFailed
            }
            citadel_activities::ActivityEventInfo::UserMfaRecoveryCodeUsed => {
                Self::UserMfaRecoveryCodeUsed
            }
            citadel_activities::ActivityEventInfo::UserMfaRecoveryCodesRegenerated => {
                Self::UserMfaRecoveryCodesRegenerated
            }
            citadel_activities::ActivityEventInfo::UserMfaResetByAdministrator {
                target_user_id,
            } => Self::UserMfaResetByAdministrator { target_user_id },
            citadel_activities::ActivityEventInfo::UserCreated { user } => {
                Self::UserCreated { user }
            }
            citadel_activities::ActivityEventInfo::UserUpdated {
                old_user,
                new_user,
                password_changed,
            } => Self::UserUpdated {
                old_user,
                new_user,
                password_changed,
            },
            citadel_activities::ActivityEventInfo::UserRenamed { old_name, new_name } => {
                Self::UserRenamed { old_name, new_name }
            }
            citadel_activities::ActivityEventInfo::UserDeleted { user } => {
                Self::UserDeleted { user }
            }
            citadel_activities::ActivityEventInfo::TeamCreated { team } => {
                Self::TeamCreated { team }
            }
            citadel_activities::ActivityEventInfo::TeamUpdated { old_team, new_team } => {
                Self::TeamUpdated { old_team, new_team }
            }
            citadel_activities::ActivityEventInfo::TeamRenamed { old_name, new_name } => {
                Self::TeamRenamed { old_name, new_name }
            }
            citadel_activities::ActivityEventInfo::TeamDeleted { team } => {
                Self::TeamDeleted { team }
            }
            citadel_activities::ActivityEventInfo::RoleCreated { role } => {
                Self::RoleCreated { role }
            }
            citadel_activities::ActivityEventInfo::RoleUpdated { old_role, new_role } => {
                Self::RoleUpdated { old_role, new_role }
            }
            citadel_activities::ActivityEventInfo::RoleRenamed { old_name, new_name } => {
                Self::RoleRenamed { old_name, new_name }
            }
            citadel_activities::ActivityEventInfo::RoleDeleted { role } => {
                Self::RoleDeleted { role }
            }
            citadel_activities::ActivityEventInfo::ServiceAccountCreated { account } => {
                Self::ServiceAccountCreated { account }
            }
            citadel_activities::ActivityEventInfo::ServiceAccountUpdated {
                old_account,
                new_account,
            } => Self::ServiceAccountUpdated {
                old_account,
                new_account,
            },
            citadel_activities::ActivityEventInfo::ServiceAccountRenamed { old_name, new_name } => {
                Self::ServiceAccountRenamed { old_name, new_name }
            }
            citadel_activities::ActivityEventInfo::ServiceAccountEnabled { account_id } => {
                Self::ServiceAccountEnabled { account_id }
            }
            citadel_activities::ActivityEventInfo::ServiceAccountDisabled { account_id } => {
                Self::ServiceAccountDisabled { account_id }
            }
            citadel_activities::ActivityEventInfo::ServiceAccountArchived { account_id } => {
                Self::ServiceAccountArchived { account_id }
            }
            citadel_activities::ActivityEventInfo::ServiceAccountTokenCreated {
                account_id,
                token_id,
                token_name,
                public_hint,
                expires_at_utc,
            } => Self::ServiceAccountTokenCreated {
                account_id,
                token_id,
                token_name,
                public_hint,
                expires_at_utc,
            },
            citadel_activities::ActivityEventInfo::ServiceAccountTokenRevoked {
                account_id,
                token_id,
                public_hint,
            } => Self::ServiceAccountTokenRevoked {
                account_id,
                token_id,
                public_hint,
            },
            citadel_activities::ActivityEventInfo::LicenseInstalled { license } => {
                Self::LicenseInstalled { license }
            }
            citadel_activities::ActivityEventInfo::LicenseReplaced {
                old_license,
                new_license,
            } => Self::LicenseReplaced {
                old_license,
                new_license,
            },
            citadel_activities::ActivityEventInfo::LicenseRemoved { license } => {
                Self::LicenseRemoved { license }
            }
            citadel_activities::ActivityEventInfo::LicenseEnteredGracePeriod { license } => {
                Self::LicenseEnteredGracePeriod { license }
            }
            citadel_activities::ActivityEventInfo::LicenseExpired { license } => {
                Self::LicenseExpired { license }
            }
            citadel_activities::ActivityEventInfo::LicenseValidationFailed {
                fingerprint,
                status,
                error_code,
            } => Self::LicenseValidationFailed {
                fingerprint,
                status,
                error_code,
            },
            citadel_activities::ActivityEventInfo::OidcProviderCreated { provider } => {
                Self::OidcProviderCreated { provider }
            }
            citadel_activities::ActivityEventInfo::OidcProviderUpdated {
                old_provider,
                new_provider,
            } => Self::OidcProviderUpdated {
                old_provider,
                new_provider,
            },
            citadel_activities::ActivityEventInfo::OidcProviderRenamed { old_name, new_name } => {
                Self::OidcProviderRenamed { old_name, new_name }
            }
            citadel_activities::ActivityEventInfo::OidcProviderDeleted { provider } => {
                Self::OidcProviderDeleted { provider }
            }
            citadel_activities::ActivityEventInfo::RegistryCreated { registry } => {
                Self::RegistryCreated { registry }
            }
            citadel_activities::ActivityEventInfo::RegistryUpdated {
                old_registry,
                new_registry,
            } => Self::RegistryUpdated {
                old_registry,
                new_registry,
            },
            citadel_activities::ActivityEventInfo::RegistryRenamed { old_name, new_name } => {
                Self::RegistryRenamed { old_name, new_name }
            }
            citadel_activities::ActivityEventInfo::RegistryDeleted { registry } => {
                Self::RegistryDeleted { registry }
            }
            citadel_activities::ActivityEventInfo::DeploymentCreated { deployment } => {
                Self::DeploymentCreated { deployment }
            }
            citadel_activities::ActivityEventInfo::DeploymentAdopted {
                deployment,
                container_id,
                container_name,
            } => Self::DeploymentAdopted {
                deployment,
                container_id,
                container_name,
            },
            citadel_activities::ActivityEventInfo::DeploymentDuplicated { deployment, source } => {
                Self::DeploymentDuplicated { deployment, source }
            }
            citadel_activities::ActivityEventInfo::DeploymentUpdated {
                old_deployment,
                new_deployment,
            } => Self::DeploymentUpdated {
                old_deployment,
                new_deployment,
            },
            citadel_activities::ActivityEventInfo::DeploymentRenamed { old_name, new_name } => {
                Self::DeploymentRenamed { old_name, new_name }
            }
            citadel_activities::ActivityEventInfo::DeploymentDeleted { deployment } => {
                Self::DeploymentDeleted { deployment }
            }
            citadel_activities::ActivityEventInfo::DeploymentStarted { container_ids } => {
                Self::DeploymentStarted { container_ids }
            }
            citadel_activities::ActivityEventInfo::DeploymentStopped { container_ids } => {
                Self::DeploymentStopped { container_ids }
            }
            citadel_activities::ActivityEventInfo::DeploymentPaused { container_ids } => {
                Self::DeploymentPaused { container_ids }
            }
            citadel_activities::ActivityEventInfo::DeploymentDegraded { reason } => {
                Self::DeploymentDegraded { reason }
            }
            citadel_activities::ActivityEventInfo::StackDegraded { reason } => {
                Self::StackDegraded { reason }
            }
            citadel_activities::ActivityEventInfo::DeploymentApplied { deployment, result } => {
                Self::DeploymentApplied { deployment, result }
            }
            citadel_activities::ActivityEventInfo::StackCreated { stack } => {
                Self::StackCreated { stack }
            }
            citadel_activities::ActivityEventInfo::StackDuplicated { stack, source } => {
                Self::StackDuplicated { stack, source }
            }
            citadel_activities::ActivityEventInfo::StackUpdated {
                old_stack,
                new_stack,
            } => Self::StackUpdated {
                old_stack,
                new_stack,
            },
            citadel_activities::ActivityEventInfo::StackRenamed { old_name, new_name } => {
                Self::StackRenamed { old_name, new_name }
            }
            citadel_activities::ActivityEventInfo::StackDeleted { stack } => {
                Self::StackDeleted { stack }
            }
            citadel_activities::ActivityEventInfo::StackStarted { container_ids } => {
                Self::StackStarted { container_ids }
            }
            citadel_activities::ActivityEventInfo::StackStopped { container_ids } => {
                Self::StackStopped { container_ids }
            }
            citadel_activities::ActivityEventInfo::StackPaused { container_ids } => {
                Self::StackPaused { container_ids }
            }
            citadel_activities::ActivityEventInfo::StackApplied { stack, result } => {
                Self::StackApplied { stack, result }
            }
            citadel_activities::ActivityEventInfo::StackRollback {
                old_stack,
                new_stack,
                result,
            } => Self::StackRollback {
                old_stack,
                new_stack,
                result,
            },
            citadel_activities::ActivityEventInfo::StackDriftDetected {
                reason,
                fingerprint,
            } => Self::StackDriftDetected {
                reason,
                fingerprint,
            },
            citadel_activities::ActivityEventInfo::StackDriftResolved {
                previous_fingerprint,
            } => Self::StackDriftResolved {
                previous_fingerprint,
            },
            citadel_activities::ActivityEventInfo::StackImported {
                stack,
                project_name,
                service_names,
            } => Self::StackImported {
                stack,
                project_name,
                service_names,
            },
            citadel_activities::ActivityEventInfo::SwarmServiceCreated { service } => {
                Self::SwarmServiceCreated { service }
            }
            citadel_activities::ActivityEventInfo::SwarmServiceDuplicated { service, source } => {
                Self::SwarmServiceDuplicated { service, source }
            }
            citadel_activities::ActivityEventInfo::SwarmServiceAdopted {
                service,
                docker_service_id,
            } => Self::SwarmServiceAdopted {
                service,
                docker_service_id,
            },
            citadel_activities::ActivityEventInfo::SwarmServiceUpdated {
                old_service,
                new_service,
            } => Self::SwarmServiceUpdated {
                old_service,
                new_service,
            },
            citadel_activities::ActivityEventInfo::SwarmServiceRenamed { old_name, new_name } => {
                Self::SwarmServiceRenamed { old_name, new_name }
            }
            citadel_activities::ActivityEventInfo::SwarmServiceDeleted { service } => {
                Self::SwarmServiceDeleted { service }
            }
            citadel_activities::ActivityEventInfo::SwarmServiceApplied {
                operation_id,
                warnings,
            } => Self::SwarmServiceApplied {
                operation_id,
                warnings,
            },
            citadel_activities::ActivityEventInfo::SwarmServiceScaled {
                operation_id,
                replicas,
                warnings,
            } => Self::SwarmServiceScaled {
                operation_id,
                replicas,
                warnings,
            },
            citadel_activities::ActivityEventInfo::SwarmServiceForceUpdated {
                operation_id,
                warnings,
            } => Self::SwarmServiceForceUpdated {
                operation_id,
                warnings,
            },
            citadel_activities::ActivityEventInfo::SwarmServiceOperationFailed {
                operation_id,
                kind,
                reason,
            } => Self::SwarmServiceOperationFailed {
                operation_id,
                kind,
                reason,
            },
            citadel_activities::ActivityEventInfo::PlatformConnected {
                platform,
                previous_status,
            } => Self::PlatformConnected {
                platform,
                previous_status,
            },
            citadel_activities::ActivityEventInfo::PlatformDisconnected {
                platform,
                previous_status,
            } => Self::PlatformDisconnected {
                platform,
                previous_status,
            },
            citadel_activities::ActivityEventInfo::PlatformCreated { platform } => {
                Self::PlatformCreated { platform }
            }
            citadel_activities::ActivityEventInfo::PlatformRenamed { old_name, new_name } => {
                Self::PlatformRenamed { old_name, new_name }
            }
            citadel_activities::ActivityEventInfo::PlatformDeleted { platform } => {
                Self::PlatformDeleted { platform }
            }
            citadel_activities::ActivityEventInfo::PlatformNodeAgentLifecycle {
                operation_id,
                kind,
                state,
                message,
            } => Self::PlatformNodeAgentLifecycle {
                operation_id,
                kind,
                state,
                message,
            },
            citadel_activities::ActivityEventInfo::GitRepoCreated { git_repo } => {
                Self::GitRepoCreated { git_repo }
            }
            citadel_activities::ActivityEventInfo::GitRepoUpdated {
                old_git_repo,
                new_git_repo,
            } => Self::GitRepoUpdated {
                old_git_repo,
                new_git_repo,
            },
            citadel_activities::ActivityEventInfo::GitRepoRenamed { old_name, new_name } => {
                Self::GitRepoRenamed { old_name, new_name }
            }
            citadel_activities::ActivityEventInfo::GitRepoDeleted { git_repo } => {
                Self::GitRepoDeleted { git_repo }
            }
            citadel_activities::ActivityEventInfo::GitRepoPulled { git_repo, result } => {
                Self::GitRepoPulled { git_repo, result }
            }
            citadel_activities::ActivityEventInfo::GitRepoCloned { git_repo, result } => {
                Self::GitRepoCloned { git_repo, result }
            }
            citadel_activities::ActivityEventInfo::ActionCreated { action } => {
                Self::ActionCreated { action }
            }
            citadel_activities::ActivityEventInfo::ActionUpdated {
                old_action,
                new_action,
            } => Self::ActionUpdated {
                old_action,
                new_action,
            },
            citadel_activities::ActivityEventInfo::ActionRenamed { old_name, new_name } => {
                Self::ActionRenamed { old_name, new_name }
            }
            citadel_activities::ActivityEventInfo::ActionDeleted { action } => {
                Self::ActionDeleted { action }
            }
            citadel_activities::ActivityEventInfo::ActionRunQueued { run_id, trigger } => {
                Self::ActionRunQueued { run_id, trigger }
            }
            citadel_activities::ActivityEventInfo::ActionRunStarted { run_id, trigger } => {
                Self::ActionRunStarted { run_id, trigger }
            }
            citadel_activities::ActivityEventInfo::ActionRunSucceeded {
                run_id,
                trigger,
                exit_code,
                duration_ms,
            } => Self::ActionRunSucceeded {
                run_id,
                trigger,
                exit_code,
                duration_ms,
            },
            citadel_activities::ActivityEventInfo::ActionRunFailed {
                run_id,
                trigger,
                exit_code,
                duration_ms,
                error_message,
            } => Self::ActionRunFailed {
                run_id,
                trigger,
                exit_code,
                duration_ms,
                error_message,
            },
            citadel_activities::ActivityEventInfo::ActionRunTimedOut {
                run_id,
                trigger,
                duration_ms,
                error_message,
            } => Self::ActionRunTimedOut {
                run_id,
                trigger,
                duration_ms,
                error_message,
            },
            citadel_activities::ActivityEventInfo::ActionRunCancelled { run_id, trigger } => {
                Self::ActionRunCancelled { run_id, trigger }
            }
            citadel_activities::ActivityEventInfo::ActionRunRejected {
                run_id,
                trigger,
                reason,
            } => Self::ActionRunRejected {
                run_id,
                trigger,
                reason,
            },
            citadel_activities::ActivityEventInfo::BuildRunQueued { run_id, trigger } => {
                Self::BuildRunQueued { run_id, trigger }
            }
            citadel_activities::ActivityEventInfo::BuildRunStarted { run_id, trigger } => {
                Self::BuildRunStarted { run_id, trigger }
            }
            citadel_activities::ActivityEventInfo::BuildRunSucceeded {
                run_id,
                trigger,
                exit_code,
                duration_ms,
                image_digest,
            } => Self::BuildRunSucceeded {
                run_id,
                trigger,
                exit_code,
                duration_ms,
                image_digest,
            },
            citadel_activities::ActivityEventInfo::BuildRunFailed {
                run_id,
                trigger,
                status,
                exit_code,
                duration_ms,
                error_message,
            } => Self::BuildRunFailed {
                run_id,
                trigger,
                status,
                exit_code,
                duration_ms,
                error_message,
            },
            citadel_activities::ActivityEventInfo::BuildRunTimedOut {
                run_id,
                trigger,
                duration_ms,
                error_message,
            } => Self::BuildRunTimedOut {
                run_id,
                trigger,
                duration_ms,
                error_message,
            },
            citadel_activities::ActivityEventInfo::BuildRunCancelled { run_id, trigger } => {
                Self::BuildRunCancelled { run_id, trigger }
            }
            citadel_activities::ActivityEventInfo::BuildCreated { build } => {
                Self::BuildCreated { build }
            }
            citadel_activities::ActivityEventInfo::BuildUpdated {
                old_build,
                new_build,
            } => Self::BuildUpdated {
                old_build,
                new_build,
            },
            citadel_activities::ActivityEventInfo::BuildRenamed { old_name, new_name } => {
                Self::BuildRenamed { old_name, new_name }
            }
            citadel_activities::ActivityEventInfo::BuildDeleted { build } => {
                Self::BuildDeleted { build }
            }
            citadel_activities::ActivityEventInfo::BuildAgentPoolCreated { pool } => {
                Self::BuildAgentPoolCreated { pool }
            }
            citadel_activities::ActivityEventInfo::BuildAgentPoolUpdated { old_pool, new_pool } => {
                Self::BuildAgentPoolUpdated { old_pool, new_pool }
            }
            citadel_activities::ActivityEventInfo::BuildAgentPoolRenamed { old_name, new_name } => {
                Self::BuildAgentPoolRenamed { old_name, new_name }
            }
            citadel_activities::ActivityEventInfo::BuildAgentPoolDeleted { pool } => {
                Self::BuildAgentPoolDeleted { pool }
            }
            citadel_activities::ActivityEventInfo::BuildAgentPoolConnected {
                agent_id,
                previous_status,
                reason,
            } => Self::BuildAgentPoolConnected {
                agent_id,
                previous_status,
                reason,
            },
            citadel_activities::ActivityEventInfo::BuildAgentPoolDisconnected {
                agent_id,
                previous_status,
                reason,
            } => Self::BuildAgentPoolDisconnected {
                agent_id,
                previous_status,
                reason,
            },
            citadel_activities::ActivityEventInfo::BuildAgentPoolTested {
                pool,
                status,
                message,
            } => Self::BuildAgentPoolTested {
                pool,
                status,
                message,
            },
        }
    }
}

schema_model! {
    citadel_activities::WebhookActivityDetails =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[doc = " Safe webhook audit metadata. Authentication headers, credentials and request"]
    #[doc = " bodies are deliberately not representable in the persisted event."]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = WebhookActivityDetails)]
    pub struct WebhookActivityDetailsSchema {
        pub request_id: Uuid,
        pub auth_type: String,
        pub execution: String,
        pub status: &'static str,
        pub reason: Option<&'static str>,
        #[serde(flatten)]
        #[schema(value_type = crate::api::resources::schema_models::activities::WebhookActivitySourceSchema)]
        pub source: citadel_activities::WebhookActivitySource,
        pub dispatched_branch: Option<String>,
        pub dispatched_commit_sha: Option<String>,
    }
}

schema_model! {
    citadel_activities::WebhookActivitySource =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = WebhookActivitySource)]
    pub struct WebhookActivitySourceSchema {
        pub delivery_id: Option<String>,
        pub event_type: Option<String>,
        pub branch: Option<String>,
        pub commit_sha: Option<String>,
        pub repository_full_name: Option<String>,
    }
}

enum_schema!(
    ActivityStatusSchema,
    "ActivityStatus",
    citadel_activities::ActivityStatus,
    [Success, Failure, Warning, Information]
);

enum_schema!(
    ActivityResourceTypeSchema,
    "ActivityResourceType",
    citadel_activities::ActivityResourceType,
    [
        Platform,
        Registry,
        Deployment,
        Stack,
        AlertRule,
        GitRepository,
        OidcProvider,
        AutomationAction,
        User,
        Team,
        Role,
        License,
        Build,
        BuildAgentPool,
        Volume,
        BackupPolicy,
        SwarmService,
        ServiceAccount
    ]
);

enum_schema!(
    ActivityEventTypeSchema,
    "ActivityEventType",
    citadel_activities::ActivityEventType,
    [
        DeploymentCreated,
        DeploymentDuplicated,
        DeploymentUpdated,
        DeploymentRenamed,
        DeploymentDeleted,
        DeploymentStarted,
        DeploymentStopped,
        DeploymentPaused,
        DeploymentApplied,
        DeploymentDegraded,
        DeploymentAdopted,
        PlatformCreated,
        PlatformDeleted,
        PlatformConnected,
        PlatformDisconnected,
        PlatformRenamed,
        PlatformNodeAgentLifecycle,
        RegistryCreated,
        RegistryRenamed,
        RegistryUpdated,
        RegistryDeleted,
        AlertRuleCreated,
        AlertRuleUpdated,
        AlertRuleDeleted,
        AlertRuleRenamed,
        GitRepoCreated,
        GitRepoUpdated,
        GitRepoDeleted,
        GitRepoRenamed,
        GitRepoPulled,
        GitRepoCloned,
        GitRepoWebhookReceived,
        OidcProviderCreated,
        OidcProviderUpdated,
        OidcProviderRenamed,
        OidcProviderDeleted,
        ActionCreated,
        ActionUpdated,
        ActionRenamed,
        ActionDeleted,
        ActionRunQueued,
        ActionRunStarted,
        ActionRunSucceeded,
        ActionRunFailed,
        ActionRunTimedOut,
        ActionRunCancelled,
        ActionRunRejected,
        ActionWebhookReceived,
        StackCreated,
        StackDuplicated,
        StackUpdated,
        StackRenamed,
        StackDeleted,
        StackStarted,
        StackStopped,
        StackPaused,
        StackApplied,
        StackRollback,
        StackDegraded,
        StackDriftDetected,
        StackDriftResolved,
        StackReconciliationAttempted,
        StackGitUpdateAvailable,
        StackGitAutoUpdated,
        StackGitAutoDeployFailed,
        StackImported,
        StackWebhookReceived,
        InitialAdministratorCreated,
        UserProfileUpdated,
        UserPreferencesUpdated,
        UserPasswordChanged,
        UserSessionRevoked,
        UserOtherSessionsRevoked,
        UserMfaEnabled,
        UserMfaDisabled,
        UserMfaVerificationFailed,
        UserMfaRecoveryCodeUsed,
        UserMfaRecoveryCodesRegenerated,
        UserMfaResetByAdministrator,
        UserCreated,
        UserUpdated,
        UserRenamed,
        UserDeleted,
        TeamCreated,
        TeamUpdated,
        TeamRenamed,
        TeamDeleted,
        RoleCreated,
        RoleUpdated,
        RoleRenamed,
        RoleDeleted,
        LicenseInstalled,
        LicenseReplaced,
        LicenseRemoved,
        LicenseEnteredGracePeriod,
        LicenseExpired,
        LicenseValidationFailed,
        VolumeContentDownloaded,
        BuildCreated,
        BuildUpdated,
        BuildRenamed,
        BuildDeleted,
        BuildRunQueued,
        BuildRunStarted,
        BuildRunSucceeded,
        BuildRunFailed,
        BuildRunTimedOut,
        BuildRunCancelled,
        BuildWebhookReceived,
        BuildAgentPoolCreated,
        BuildAgentPoolUpdated,
        BuildAgentPoolRenamed,
        BuildAgentPoolDeleted,
        BuildAgentPoolTested,
        BuildAgentPoolConnected,
        BuildAgentPoolDisconnected,
        BackupPolicyCreated,
        BackupPolicyUpdated,
        BackupPolicyRenamed,
        BackupPolicyArchived,
        BackupRunQueued,
        BackupRunStarted,
        BackupRunCompleted,
        BackupPolicyWebhookReceived,
        SwarmServiceCreated,
        SwarmServiceAdopted,
        SwarmServiceUpdated,
        SwarmServiceRenamed,
        SwarmServiceDeleted,
        SwarmServiceApplied,
        SwarmServiceScaled,
        SwarmServiceForceUpdated,
        SwarmServiceOperationFailed,
        SwarmServiceDuplicated,
        SwarmServiceWebhookReceived,
        ServiceAccountCreated,
        ServiceAccountUpdated,
        ServiceAccountRenamed,
        ServiceAccountEnabled,
        ServiceAccountDisabled,
        ServiceAccountArchived,
        ServiceAccountTokenCreated,
        ServiceAccountTokenRevoked
    ]
);

schema_model! {
    citadel_activities::ActivitySourceResource =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[schema(as = ActivitySourceResource)]
    pub struct ActivitySourceResourceSchema {
        #[serde(rename = "ResourceType")]
        #[schema(value_type = crate::api::resources::schema_models::activities::ActivityResourceTypeSchema)]
        pub resource_type: citadel_activities::ActivityResourceType,
        #[serde(rename = "ResourceId")]
        pub resource_id: Uuid,
        #[serde(rename = "ResourceName")]
        pub resource_name: String,
    }
}

schema_model! {
    citadel_activities::VolumeContentDownloaded =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = VolumeContentDownloaded)]
    pub struct VolumeContentDownloadedSchema {
        pub volume_name: String,
        pub path: String,
        pub is_directory: bool,
        pub file_name: String,
    }
}

schema_model! {
    citadel_activities::IdentityResourceAccessSnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[schema(as = IdentityResourceAccessSnapshot)]
    pub struct IdentityResourceAccessSnapshotSchema {
        #[serde(rename = "ResourceType")]
        #[schema(value_type = crate::api::resources::schema_models::primitives::ResourceTypeSchema)]
        pub resource_type: citadel_primitives::ResourceType,
        #[serde(rename = "ResourceId")]
        pub resource_id: Uuid,
        #[serde(rename = "PermissionLevel")]
        #[schema(value_type = crate::api::resources::schema_models::primitives::PermissionLevelSchema)]
        pub permission_level: citadel_primitives::PermissionLevel,
        #[serde(rename = "SpecificPermissions")]
        pub specific_permissions: i32,
    }
}

schema_model! {
    citadel_activities::UserActivitySnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[schema(as = UserActivitySnapshot)]
    pub struct UserActivitySnapshotSchema {
        #[serde(rename = "Email")]
        pub email: String,
        #[serde(rename = "IsEnabled")]
        pub is_enabled: bool,
        #[serde(rename = "TeamIds")]
        pub team_ids: Vec<Uuid>,
        #[serde(rename = "RoleIds")]
        pub role_ids: Vec<Uuid>,
        #[serde(rename = "ResourceAccesses")]
        #[schema(value_type = Vec < crate::api::resources::schema_models::activities::IdentityResourceAccessSnapshotSchema >)]
        pub resource_accesses: Vec<citadel_activities::IdentityResourceAccessSnapshot>,
    }
}

schema_model! {
    citadel_activities::TeamActivitySnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[schema(as = TeamActivitySnapshot)]
    pub struct TeamActivitySnapshotSchema {
        #[serde(rename = "IsEnabled")]
        pub is_enabled: bool,
        #[serde(rename = "MemberActorIds")]
        pub member_actor_ids: Vec<Uuid>,
        #[serde(rename = "RoleIds")]
        pub role_ids: Vec<Uuid>,
        #[serde(rename = "ResourceAccesses")]
        #[schema(value_type = Vec < crate::api::resources::schema_models::activities::IdentityResourceAccessSnapshotSchema >)]
        pub resource_accesses: Vec<citadel_activities::IdentityResourceAccessSnapshot>,
    }
}

schema_model! {
    citadel_activities::RolePermissionActivitySnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[schema(as = RolePermissionActivitySnapshot)]
    pub struct RolePermissionActivitySnapshotSchema {
        #[serde(rename = "ResourceType")]
        #[schema(value_type = crate::api::resources::schema_models::primitives::ResourceTypeSchema)]
        pub resource_type: citadel_primitives::ResourceType,
        #[serde(rename = "PermissionLevel")]
        #[schema(value_type = crate::api::resources::schema_models::primitives::PermissionLevelSchema)]
        pub permission_level: citadel_primitives::PermissionLevel,
        #[serde(rename = "SpecificPermissions")]
        pub specific_permissions: i32,
    }
}

schema_model! {
    citadel_activities::RoleActivitySnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[schema(as = RoleActivitySnapshot)]
    pub struct RoleActivitySnapshotSchema {
        #[serde(rename = "RoleType")]
        pub role_type: String,
        #[serde(rename = "Permissions")]
        #[schema(value_type = Vec < crate::api::resources::schema_models::activities::RolePermissionActivitySnapshotSchema >)]
        pub permissions: Vec<citadel_activities::RolePermissionActivitySnapshot>,
    }
}

schema_model! {
    citadel_activities::ServiceAccountResourceAccessSnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[schema(as = ServiceAccountResourceAccessSnapshot)]
    pub struct ServiceAccountResourceAccessSnapshotSchema {
        #[serde(rename = "ResourceType")]
        #[schema(value_type = crate::api::resources::schema_models::primitives::ResourceTypeSchema)]
        pub resource_type: citadel_primitives::ResourceType,
        #[serde(rename = "ResourceId")]
        pub resource_id: Uuid,
        #[serde(rename = "PermissionLevel")]
        #[schema(value_type = crate::api::resources::schema_models::primitives::PermissionLevelSchema)]
        pub permission_level: citadel_primitives::PermissionLevel,
        #[serde(rename = "SpecificPermissions")]
        pub specific_permissions: i32,
    }
}

schema_model! {
    citadel_activities::ServiceAccountActivitySnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[schema(as = ServiceAccountActivitySnapshot)]
    pub struct ServiceAccountActivitySnapshotSchema {
        #[serde(rename = "Id")]
        pub id: Uuid,
        #[serde(rename = "Name")]
        pub name: String,
        #[serde(rename = "Description")]
        pub description: Option<String>,
        #[serde(rename = "IsEnabled")]
        pub is_enabled: bool,
        #[serde(rename = "TeamIds")]
        pub team_ids: Vec<Uuid>,
        #[serde(rename = "RoleIds")]
        pub role_ids: Vec<Uuid>,
        #[serde(rename = "ResourceAccesses")]
        #[schema(value_type = Vec < crate::api::resources::schema_models::activities::ServiceAccountResourceAccessSnapshotSchema >)]
        pub resource_accesses: Vec<citadel_activities::ServiceAccountResourceAccessSnapshot>,
    }
}

schema_model! {
    citadel_activities::OidcProviderActivitySnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[schema(as = OidcProviderActivitySnapshot)]
    pub struct OidcProviderActivitySnapshotSchema {
        #[serde(rename = "Id")]
        pub id: Uuid,
        #[serde(rename = "Name")]
        pub name: String,
        #[serde(rename = "Description")]
        pub description: Option<String>,
        #[serde(rename = "DisplayName")]
        pub display_name: String,
        #[serde(rename = "Issuer")]
        pub issuer: String,
        #[serde(rename = "ClientId")]
        pub client_id: String,
        #[serde(rename = "Scopes")]
        pub scopes: String,
        #[serde(rename = "Enabled")]
        pub enabled: bool,
        #[serde(rename = "AutoProvisionUsers")]
        pub auto_provision_users: bool,
        #[serde(rename = "AllowEmailAutoLink")]
        pub allow_email_auto_link: bool,
        #[serde(rename = "RequireEmailVerified")]
        pub require_email_verified: bool,
        #[serde(rename = "AllowedEmailDomains")]
        pub allowed_email_domains: Option<String>,
        #[serde(rename = "RequiredClaimName")]
        pub required_claim_name: Option<String>,
        #[serde(rename = "RequiredClaimValues")]
        pub required_claim_values: Option<String>,
        #[serde(rename = "DefaultRoleId")]
        pub default_role_id: Option<Uuid>,
    }
}

schema_model! {
    citadel_activities::BuildProjectActivitySnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = BuildProjectActivitySnapshot)]
    pub struct BuildProjectActivitySnapshotSchema {
        pub id: Uuid,
        pub name: String,
        pub description: Option<String>,
        pub enabled: bool,
        pub git_repository_id: Uuid,
        pub branch: String,
        pub context_path: String,
        pub dockerfile_path: String,
        pub target: Option<String>,
        pub builder_kind: String,
        pub platform_id: Option<Uuid>,
        pub build_agent_pool_id: Option<Uuid>,
        pub push_to_registry: bool,
        pub registry_id: Option<Uuid>,
        pub image_repository: String,
        pub tag_templates: Vec<String>,
        #[schema(value_type = Option < crate::api::resources::schema_models::primitives::WebhookConfigSchema >)]
        pub webhook: Option<citadel_primitives::WebhookConfig>,
        pub timeout_seconds: i32,
        pub retention_run_count: i32,
        #[schema(value_type = Vec < crate::api::resources::schema_models::activities::BuildSecretActivitySnapshotSchema >)]
        pub build_secrets: Vec<citadel_activities::BuildSecretActivitySnapshot>,
    }
}

schema_model! {
    citadel_activities::BuildSecretActivitySnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = BuildSecretActivitySnapshot)]
    pub struct BuildSecretActivitySnapshotSchema {
        pub id: String,
        pub secret_id: Uuid,
    }
}

schema_model! {
    citadel_activities::BuildAgentPoolActivitySnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = BuildAgentPoolActivitySnapshot)]
    pub struct BuildAgentPoolActivitySnapshotSchema {
        pub id: Uuid,
        pub name: String,
        pub description: Option<String>,
        pub enabled: bool,
        pub provider: String,
        pub provider_spec: Value,
        pub architecture: String,
        pub region: String,
        pub instance_type: String,
        pub max_active_builders: i32,
        pub queue_timeout_seconds: i32,
        pub provisioning_timeout_seconds: i32,
        pub registration_timeout_seconds: i32,
        pub heartbeat_timeout_seconds: i32,
        pub cleanup_timeout_seconds: i32,
        pub maximum_instance_lifetime_seconds: i32,
        pub failure_retention_minutes: i32,
        pub last_validation_status: String,
        pub last_validation_message: Option<String>,
        pub last_validated_at: Option<DateTime<Utc>>,
    }
}

schema_model! {
    citadel_activities::SwarmServiceActivitySnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[schema(as = SwarmServiceActivitySnapshot)]
    pub struct SwarmServiceActivitySnapshotSchema {
        #[serde(rename = "Id")]
        pub id: Uuid,
        #[serde(rename = "PlatformId")]
        pub platform_id: Uuid,
        #[serde(rename = "Name")]
        pub name: String,
        #[serde(rename = "Description")]
        pub description: Option<String>,
        #[serde(rename = "DockerName")]
        pub docker_name: String,
        #[serde(rename = "DockerServiceId")]
        pub docker_service_id: Option<String>,
        #[serde(rename = "Spec")]
        pub spec: Value,
    }
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[schema(as = ActivityChangedField)]
pub struct ActivityChangedFieldSchema {
    #[serde(rename = "Name")]
    pub name: &'static str,
    #[serde(rename = "OldValue")]
    pub old_value: Option<String>,
    #[serde(rename = "NewValue")]
    pub new_value: Option<String>,
}

schema_model! {
    citadel_activities::DeploymentActivitySnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[schema(as = DeploymentActivitySnapshot)]
    pub struct DeploymentActivitySnapshotSchema {
        #[serde(rename = "Id")]
        pub id: Uuid,
        #[serde(rename = "Name")]
        pub name: String,
        #[serde(rename = "PlatformId")]
        pub platform_id: Uuid,
        #[serde(rename = "Description")]
        pub description: Option<String>,
        #[serde(rename = "Spec")]
        pub spec: Value,
    }
}

schema_model! {
    citadel_activities::DeploymentResultActivitySnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[schema(as = DeploymentResultActivitySnapshot)]
    pub struct DeploymentResultActivitySnapshotSchema {
        #[serde(rename = "ContainerIds")]
        pub container_ids: Option<Vec<String>>,
        #[serde(rename = "Message")]
        pub message: Option<String>,
        #[serde(rename = "ResourceBindings")]
        pub resource_bindings: Option<Value>,
    }
}

schema_model! {
    citadel_activities::LicenseActivitySnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[schema(as = LicenseActivitySnapshot)]
    pub struct LicenseActivitySnapshotSchema {
        #[serde(rename = "Schema")]
        pub schema: Option<u8>,
        #[serde(rename = "LicenseId")]
        pub license_id: Option<String>,
        #[serde(rename = "ReplacedLicenseId")]
        pub replaced_license_id: Option<String>,
        #[serde(rename = "LicensedEdition")]
        pub licensed_edition: Option<String>,
        #[serde(rename = "EffectiveEdition")]
        pub effective_edition: String,
        #[serde(rename = "EffectiveCapabilities")]
        #[schema(value_type = Vec < crate::api::resources::schema_models::licensing::LicenseCapabilitySchema >)]
        pub effective_capabilities: Vec<citadel_licensing::LicenseCapability>,
        #[serde(rename = "CustomerId")]
        pub customer_id: Option<String>,
        #[serde(rename = "CustomerName")]
        pub customer_name: Option<String>,
        #[serde(rename = "Fingerprint")]
        pub fingerprint: Option<String>,
        #[serde(rename = "Status")]
        #[schema(value_type = crate::api::resources::schema_models::licensing::LicenseStatusSchema)]
        pub status: citadel_licensing::LicenseStatus,
        #[serde(rename = "ExpiresAt")]
        pub expires_at: Option<DateTime<Utc>>,
        #[serde(rename = "GraceUntil")]
        pub grace_until: Option<DateTime<Utc>>,
    }
}

schema_model! {
    citadel_activities::AlertRuleActivitySnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = AlertRuleActivitySnapshot)]
    pub struct AlertRuleActivitySnapshotSchema {
        pub id: Uuid,
        pub name: String,
        pub description: Option<String>,
        #[serde(rename = "Type")]
        pub alert_type: String,
        pub severity: String,
        pub cooldown_seconds: Option<i32>,
        pub required_matches: Option<i32>,
        # [schema (value_type = Option < f64 >)]
        pub threshold: Option<serde_json::Number>,
        pub status: String,
        pub channel_ids: Vec<Uuid>,
        pub limited_to: Vec<Value>,
        pub quiet_hours: Vec<Value>,
    }
}

schema_model! {
    citadel_activities::BackupPolicyActivitySnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = BackupPolicyActivitySnapshot)]
    pub struct BackupPolicyActivitySnapshotSchema {
        pub id: Uuid,
        pub name: String,
        pub description: Option<String>,
        pub source_type: String,
        pub source_key: String,
        pub backup_repository_id: Uuid,
        pub enabled: bool,
        pub cron: Option<String>,
        pub time_zone: Option<String>,
        pub webhook_enabled: bool,
        pub keep_last_successful: i32,
        pub timeout_seconds: i32,
        pub alert_on_failure: bool,
        pub run_as_actor_id: Uuid,
    }
}

schema_model! {
    citadel_activities::GitRepositoryActivitySnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[schema(as = GitRepositoryActivitySnapshot)]
    pub struct GitRepositoryActivitySnapshotSchema {
        #[serde(rename = "Id")]
        pub id: Uuid,
        #[serde(rename = "Name")]
        pub name: String,
        #[serde(rename = "Description")]
        pub description: Option<String>,
        #[serde(rename = "Url")]
        pub url: String,
        #[serde(rename = "DefaultBranch")]
        pub default_branch: String,
        #[serde(rename = "GitAccountId")]
        pub git_account_id: Option<Uuid>,
        #[serde(rename = "SyncMode")]
        pub sync_mode: String,
        #[serde(rename = "SyncIntervalMinutes")]
        pub sync_interval_minutes: Option<i32>,
        #[serde(rename = "Webhook")]
        #[schema(value_type = Option < crate::api::resources::schema_models::primitives::WebhookConfigSchema >)]
        pub webhook: Option<citadel_primitives::WebhookConfig>,
        #[serde(rename = "OnClone")]
        pub on_clone: Option<Value>,
        #[serde(rename = "OnPull")]
        pub on_pull: Option<Value>,
        #[serde(rename = "ResolvedCommitSha")]
        pub resolved_commit_sha: Option<String>,
    }
}

schema_model! {
    citadel_activities::GitRepositorySyncActivitySnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[schema(as = GitRepositorySyncActivitySnapshot)]
    pub struct GitRepositorySyncActivitySnapshotSchema {
        #[serde(rename = "CommitSha")]
        pub commit_sha: Option<String>,
        #[serde(rename = "Message")]
        pub message: Option<String>,
    }
}

schema_model! {
    citadel_activities::PlatformActivitySnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[schema(as = PlatformActivitySnapshot)]
    pub struct PlatformActivitySnapshotSchema {
        #[serde(rename = "Id")]
        pub id: Uuid,
        #[serde(rename = "Name")]
        pub name: String,
        #[serde(rename = "Address")]
        pub address: String,
        #[serde(rename = "Description")]
        pub description: Option<String>,
        #[serde(rename = "Status")]
        pub status: String,
        #[serde(rename = "ConnectorType")]
        pub connector_type: String,
        #[serde(rename = "NetworkCount")]
        pub network_count: i32,
        #[serde(rename = "VolumeCount")]
        pub volume_count: i32,
        #[serde(rename = "ImageCount")]
        pub image_count: i64,
        #[serde(rename = "CpuCount")]
        pub cpu_count: i64,
        #[serde(rename = "MemTotal")]
        pub mem_total: i64,
        #[serde(rename = "ServerVersion")]
        pub server_version: Option<String>,
        #[serde(rename = "AgentVersion")]
        pub agent_version: Option<String>,
        #[serde(rename = "PlatformDescriptor")]
        pub platform_descriptor: Value,
    }
}

schema_model! {
    citadel_activities::StackActivitySnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[schema(as = StackActivitySnapshot)]
    pub struct StackActivitySnapshotSchema {
        #[serde(rename = "Id")]
        pub id: Uuid,
        #[serde(rename = "Name")]
        pub name: String,
        #[serde(rename = "Description")]
        pub description: Option<String>,
        #[serde(rename = "StackSource")]
        pub stack_source: String,
        #[serde(rename = "DriftPolicy")]
        pub drift_policy: Value,
        #[serde(rename = "StackRelease")]
        pub stack_release: Option<Value>,
    }
}

schema_model! {
    citadel_activities::StackResultActivitySnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[schema(as = StackResultActivitySnapshot)]
    pub struct StackResultActivitySnapshotSchema {
        #[serde(rename = "ContainerIds")]
        pub container_ids: Option<Vec<String>>,
        #[serde(rename = "Message")]
        pub message: Option<String>,
        #[serde(rename = "ResourceBindings")]
        pub resource_bindings: Option<Value>,
    }
}

schema_model! {
    citadel_activities::AutomationActionActivitySnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "PascalCase")]
    #[schema(as = AutomationActionActivitySnapshot)]
    pub struct AutomationActionActivitySnapshotSchema {
        pub id: Uuid,
        pub name: String,
        pub description: Option<String>,
        pub code: String,
        pub default_args_json: String,
        pub enabled: bool,
        pub schedule_enabled: bool,
        pub schedule_cron: Option<String>,
        pub schedule_time_zone: String,
        #[schema(value_type = Option < crate::api::resources::schema_models::primitives::WebhookConfigSchema >)]
        pub webhook: Option<citadel_primitives::WebhookConfig>,
        pub timeout_seconds: i32,
        pub alert_on_failure: bool,
        pub run_as_actor_id: Uuid,
    }
}
