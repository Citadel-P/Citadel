use std::fmt;

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use crate::{
    ActivityEventType, ActivityResourceType, ActivityStatus, ActorId, LicenseCapability,
    LicenseStatus, PermissionLevel, ResourceType, RoleType, UserDateTimeFormat, UserTheme,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BuildProjectActivitySnapshot {
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
    pub registry_id: Uuid,
    pub image_repository: String,
    pub tag_templates: Vec<String>,
    pub webhook: Option<Value>,
    pub timeout_seconds: i32,
    pub retention_run_count: i32,
    pub build_secrets: Vec<BuildSecretActivitySnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BuildSecretActivitySnapshot {
    pub id: String,
    pub secret_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct AutomationActionActivitySnapshot {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub code: String,
    pub default_args_json: String,
    pub enabled: bool,
    pub schedule_enabled: bool,
    pub schedule_cron: Option<String>,
    pub schedule_time_zone: String,
    pub webhook: Option<Value>,
    pub timeout_seconds: i32,
    pub alert_on_failure: bool,
    pub run_as_actor_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BuildAgentPoolActivitySnapshot {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IdentityResourceAccessSnapshot {
    #[serde(rename = "ResourceType")]
    pub resource_type: ResourceType,
    #[serde(rename = "ResourceId")]
    pub resource_id: Uuid,
    #[serde(rename = "PermissionLevel")]
    pub permission_level: PermissionLevel,
    #[serde(rename = "SpecificPermissions")]
    pub specific_permissions: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UserActivitySnapshot {
    #[serde(rename = "Email")]
    pub email: String,
    #[serde(rename = "IsEnabled")]
    pub is_enabled: bool,
    #[serde(rename = "TeamIds")]
    pub team_ids: Vec<Uuid>,
    #[serde(rename = "RoleIds")]
    pub role_ids: Vec<Uuid>,
    #[serde(rename = "ResourceAccesses")]
    pub resource_accesses: Vec<IdentityResourceAccessSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TeamActivitySnapshot {
    #[serde(rename = "IsEnabled")]
    pub is_enabled: bool,
    #[serde(rename = "MemberActorIds")]
    pub member_actor_ids: Vec<Uuid>,
    #[serde(rename = "RoleIds")]
    pub role_ids: Vec<Uuid>,
    #[serde(rename = "ResourceAccesses")]
    pub resource_accesses: Vec<IdentityResourceAccessSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RolePermissionActivitySnapshot {
    #[serde(rename = "ResourceType")]
    pub resource_type: ResourceType,
    #[serde(rename = "PermissionLevel")]
    pub permission_level: PermissionLevel,
    #[serde(rename = "SpecificPermissions")]
    pub specific_permissions: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoleActivitySnapshot {
    #[serde(rename = "RoleType")]
    pub role_type: RoleType,
    #[serde(rename = "Permissions")]
    pub permissions: Vec<RolePermissionActivitySnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServiceAccountResourceAccessSnapshot {
    #[serde(rename = "ResourceType")]
    pub resource_type: ResourceType,
    #[serde(rename = "ResourceId")]
    pub resource_id: Uuid,
    #[serde(rename = "PermissionLevel")]
    pub permission_level: PermissionLevel,
    #[serde(rename = "SpecificPermissions")]
    pub specific_permissions: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServiceAccountActivitySnapshot {
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
    pub resource_accesses: Vec<ServiceAccountResourceAccessSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LicenseActivitySnapshot {
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
    pub effective_capabilities: Vec<LicenseCapability>,
    #[serde(rename = "CustomerId")]
    pub customer_id: Option<String>,
    #[serde(rename = "CustomerName")]
    pub customer_name: Option<String>,
    #[serde(rename = "Fingerprint")]
    pub fingerprint: Option<String>,
    #[serde(rename = "Status")]
    pub status: LicenseStatus,
    #[serde(rename = "ExpiresAt")]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(rename = "GraceUntil")]
    pub grace_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OidcProviderActivitySnapshot {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegistryActivitySnapshot {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GitRepositoryActivitySnapshot {
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
    pub webhook: Option<Value>,
    #[serde(rename = "OnClone")]
    pub on_clone: Option<Value>,
    #[serde(rename = "OnPull")]
    pub on_pull: Option<Value>,
    #[serde(rename = "ResolvedCommitSha")]
    pub resolved_commit_sha: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GitRepositorySyncActivitySnapshot {
    #[serde(rename = "CommitSha")]
    pub commit_sha: Option<String>,
    #[serde(rename = "Message")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeploymentActivitySnapshot {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeploymentResultActivitySnapshot {
    #[serde(rename = "ContainerIds")]
    pub container_ids: Option<Vec<String>>,
    #[serde(rename = "Message")]
    pub message: Option<String>,
    #[serde(rename = "ResourceBindings")]
    pub resource_bindings: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SwarmServiceActivitySnapshot {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StackActivitySnapshot {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StackResultActivitySnapshot {
    #[serde(rename = "ContainerIds")]
    pub container_ids: Option<Vec<String>>,
    #[serde(rename = "Message")]
    pub message: Option<String>,
    #[serde(rename = "ResourceBindings")]
    pub resource_bindings: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlatformActivitySnapshot {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActivitySourceResource {
    #[serde(rename = "ResourceType")]
    pub resource_type: ActivityResourceType,
    #[serde(rename = "ResourceId")]
    pub resource_id: Uuid,
    #[serde(rename = "ResourceName")]
    pub resource_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct AlertRuleActivitySnapshot {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "Type")]
    pub alert_type: String,
    pub severity: String,
    pub cooldown_seconds: Option<i32>,
    pub required_matches: Option<i32>,
    pub threshold: Option<serde_json::Number>,
    pub status: String,
    pub channel_ids: Vec<Uuid>,
    pub limited_to: Vec<Value>,
    pub quiet_hours: Vec<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityChangedFieldName {
    DisplayName,
    TimeZone,
    DateTimeFormat,
    Theme,
}

impl ActivityChangedFieldName {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DisplayName => "DisplayName",
            Self::TimeZone => "TimeZone",
            Self::DateTimeFormat => "DateTimeFormat",
            Self::Theme => "Theme",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActivityChangedField {
    #[serde(rename = "Name")]
    name: &'static str,
    #[serde(rename = "OldValue")]
    old_value: Option<String>,
    #[serde(rename = "NewValue")]
    new_value: Option<String>,
}

impl ActivityChangedField {
    #[must_use]
    pub fn display_name(old_value: String, new_value: String) -> Self {
        Self::new(
            ActivityChangedFieldName::DisplayName,
            Some(old_value),
            Some(new_value),
        )
    }

    #[must_use]
    pub fn time_zone(old_value: Option<String>, new_value: String) -> Self {
        Self::new(
            ActivityChangedFieldName::TimeZone,
            old_value,
            Some(new_value),
        )
    }

    #[must_use]
    pub fn date_time_format(old_value: UserDateTimeFormat, new_value: UserDateTimeFormat) -> Self {
        Self::new(
            ActivityChangedFieldName::DateTimeFormat,
            Some(old_value.as_database_str().to_owned()),
            Some(new_value.as_database_str().to_owned()),
        )
    }

    #[must_use]
    pub fn theme(old_value: UserTheme, new_value: UserTheme) -> Self {
        Self::new(
            ActivityChangedFieldName::Theme,
            Some(old_value.as_database_str().to_owned()),
            Some(new_value.as_database_str().to_owned()),
        )
    }

    fn new(
        name: ActivityChangedFieldName,
        old_value: Option<String>,
        new_value: Option<String>,
    ) -> Self {
        Self {
            name: name.as_str(),
            old_value,
            new_value,
        }
    }

    #[must_use]
    pub const fn name(&self) -> &str {
        self.name
    }

    #[must_use]
    pub fn old_value(&self) -> Option<&str> {
        self.old_value.as_deref()
    }

    #[must_use]
    pub fn new_value(&self) -> Option<&str> {
        self.new_value.as_deref()
    }
}

/// Safe webhook audit metadata. Authentication headers, credentials and request
/// bodies are deliberately not representable in the persisted event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct WebhookActivityDetails {
    pub request_id: Uuid,
    pub auth_type: String,
    pub execution: String,
    pub status: &'static str,
    pub reason: Option<&'static str>,
    #[serde(flatten)]
    pub source: WebhookActivitySource,
    pub dispatched_branch: Option<String>,
    pub dispatched_commit_sha: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct WebhookActivitySource {
    pub delivery_id: Option<String>,
    pub event_type: Option<String>,
    pub branch: Option<String>,
    pub commit_sha: Option<String>,
    pub repository_full_name: Option<String>,
}

impl ActivityEvent {
    pub fn new_webhook_event(
        id: Uuid,
        name: String,
        platform_id: Option<Uuid>,
        resource_type: ActivityResourceType,
        details: WebhookActivityDetails,
        now: DateTime<Utc>,
    ) -> Result<Self, ActivityInvariantError> {
        let status = match details.status {
            "queued" => ActivityStatus::Success,
            "noop" => ActivityStatus::Information,
            _ => ActivityStatus::Failure,
        };
        let info = match resource_type {
            ActivityResourceType::GitRepository => {
                ActivityEventInfo::GitRepoWebhookReceived(details)
            }
            ActivityResourceType::Stack => ActivityEventInfo::StackWebhookReceived(details),
            ActivityResourceType::Build => ActivityEventInfo::BuildWebhookReceived(details),
            ActivityResourceType::SwarmService => {
                ActivityEventInfo::SwarmServiceWebhookReceived(details)
            }
            _ => return Err(ActivityInvariantError::MismatchedResourceType),
        };
        let mut event = Self::new_resource_event(
            id,
            name,
            resource_type,
            ActorId::new(Uuid::from_u128(1)),
            info,
            status,
            now,
        )?;
        event.platform_id = platform_id;
        Ok(event)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "PascalCase")]
pub struct VolumeContentDownloaded {
    pub volume_name: String,
    pub path: String,
    pub is_directory: bool,
    pub file_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BackupPolicyActivitySnapshot {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "$type")]
// This closed compatibility enum is serialized immediately at mutation
// boundaries. Boxing its larger snapshots would add heap allocations to every
// Activity construction solely to reduce the enum's stack size.
#[allow(clippy::large_enum_variant)]
pub enum ActivityEventInfo {
    InitialAdministratorCreated {
        #[serde(rename = "UserId")]
        user_id: Uuid,
        #[serde(rename = "UserName")]
        user_name: String,
        #[serde(rename = "Mode")]
        mode: crate::SetupInitializationMode,
    },
    AlertRuleCreated {
        #[serde(rename = "AlertRule")]
        alert_rule: AlertRuleActivitySnapshot,
    },
    AlertRuleUpdated {
        #[serde(rename = "OldRule")]
        old_rule: AlertRuleActivitySnapshot,
        #[serde(rename = "NewRule")]
        new_rule: AlertRuleActivitySnapshot,
    },
    BackupPolicyRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },
    BackupPolicyUpdated {
        #[serde(rename = "OldPolicy")]
        old_policy: BackupPolicyActivitySnapshot,
        #[serde(rename = "NewPolicy")]
        new_policy: BackupPolicyActivitySnapshot,
    },
    AlertRuleRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },
    VolumeContentDownloaded(VolumeContentDownloaded),
    GitRepoWebhookReceived(WebhookActivityDetails),
    StackWebhookReceived(WebhookActivityDetails),
    BuildWebhookReceived(WebhookActivityDetails),
    SwarmServiceWebhookReceived(WebhookActivityDetails),
    UserProfileUpdated {
        #[serde(rename = "Changes")]
        changes: Vec<ActivityChangedField>,
    },
    UserPreferencesUpdated {
        #[serde(rename = "Changes")]
        changes: Vec<ActivityChangedField>,
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
        user: UserActivitySnapshot,
    },
    UserUpdated {
        #[serde(rename = "OldUser")]
        old_user: UserActivitySnapshot,
        #[serde(rename = "NewUser")]
        new_user: UserActivitySnapshot,
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
        user: UserActivitySnapshot,
    },
    TeamCreated {
        #[serde(rename = "Team")]
        team: TeamActivitySnapshot,
    },
    TeamUpdated {
        #[serde(rename = "OldTeam")]
        old_team: TeamActivitySnapshot,
        #[serde(rename = "NewTeam")]
        new_team: TeamActivitySnapshot,
    },
    TeamRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },
    TeamDeleted {
        #[serde(rename = "Team")]
        team: TeamActivitySnapshot,
    },
    RoleCreated {
        #[serde(rename = "Role")]
        role: RoleActivitySnapshot,
    },
    RoleUpdated {
        #[serde(rename = "OldRole")]
        old_role: RoleActivitySnapshot,
        #[serde(rename = "NewRole")]
        new_role: RoleActivitySnapshot,
    },
    RoleRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },
    RoleDeleted {
        #[serde(rename = "Role")]
        role: RoleActivitySnapshot,
    },
    ServiceAccountCreated {
        #[serde(rename = "Account")]
        account: ServiceAccountActivitySnapshot,
    },
    ServiceAccountUpdated {
        #[serde(rename = "OldAccount")]
        old_account: ServiceAccountActivitySnapshot,
        #[serde(rename = "NewAccount")]
        new_account: ServiceAccountActivitySnapshot,
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
        license: Box<LicenseActivitySnapshot>,
    },
    LicenseReplaced {
        #[serde(rename = "OldLicense")]
        old_license: Box<LicenseActivitySnapshot>,
        #[serde(rename = "NewLicense")]
        new_license: Box<LicenseActivitySnapshot>,
    },
    LicenseRemoved {
        #[serde(rename = "License")]
        license: Box<LicenseActivitySnapshot>,
    },
    LicenseEnteredGracePeriod {
        #[serde(rename = "License")]
        license: Box<LicenseActivitySnapshot>,
    },
    LicenseExpired {
        #[serde(rename = "License")]
        license: Box<LicenseActivitySnapshot>,
    },
    LicenseValidationFailed {
        #[serde(rename = "Fingerprint")]
        fingerprint: Option<String>,
        #[serde(rename = "Status")]
        status: LicenseStatus,
        #[serde(rename = "ErrorCode")]
        error_code: Option<String>,
    },
    OidcProviderCreated {
        #[serde(rename = "Provider")]
        provider: Box<OidcProviderActivitySnapshot>,
    },
    OidcProviderUpdated {
        #[serde(rename = "OldProvider")]
        old_provider: Box<OidcProviderActivitySnapshot>,
        #[serde(rename = "NewProvider")]
        new_provider: Box<OidcProviderActivitySnapshot>,
    },
    OidcProviderRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },
    OidcProviderDeleted {
        #[serde(rename = "Provider")]
        provider: Box<OidcProviderActivitySnapshot>,
    },
    RegistryCreated {
        #[serde(rename = "Registry")]
        registry: RegistryActivitySnapshot,
    },
    RegistryUpdated {
        #[serde(rename = "OldRegistry")]
        old_registry: RegistryActivitySnapshot,
        #[serde(rename = "NewRegistry")]
        new_registry: RegistryActivitySnapshot,
    },
    RegistryRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },
    RegistryDeleted {
        #[serde(rename = "Registry")]
        registry: RegistryActivitySnapshot,
    },
    DeploymentCreated {
        #[serde(rename = "Deployment")]
        deployment: DeploymentActivitySnapshot,
    },
    DeploymentAdopted {
        #[serde(rename = "Deployment")]
        deployment: DeploymentActivitySnapshot,
        #[serde(rename = "ContainerId")]
        container_id: String,
        #[serde(rename = "ContainerName")]
        container_name: String,
    },
    DeploymentDuplicated {
        #[serde(rename = "Deployment")]
        deployment: DeploymentActivitySnapshot,
        #[serde(rename = "Source")]
        source: ActivitySourceResource,
    },
    DeploymentUpdated {
        #[serde(rename = "OldDeployment")]
        old_deployment: DeploymentActivitySnapshot,
        #[serde(rename = "NewDeployment")]
        new_deployment: DeploymentActivitySnapshot,
    },
    DeploymentRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },
    DeploymentDeleted {
        #[serde(rename = "Deployment")]
        deployment: DeploymentActivitySnapshot,
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
        deployment: Option<DeploymentActivitySnapshot>,
        #[serde(rename = "Result")]
        result: DeploymentResultActivitySnapshot,
    },
    StackCreated {
        #[serde(rename = "Stack")]
        stack: StackActivitySnapshot,
    },
    StackDuplicated {
        #[serde(rename = "Stack")]
        stack: StackActivitySnapshot,
        #[serde(rename = "Source")]
        source: ActivitySourceResource,
    },
    StackUpdated {
        #[serde(rename = "OldStack")]
        old_stack: StackActivitySnapshot,
        #[serde(rename = "NewStack")]
        new_stack: StackActivitySnapshot,
    },
    StackRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },
    StackDeleted {
        #[serde(rename = "Stack")]
        stack: StackActivitySnapshot,
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
        stack: Option<StackActivitySnapshot>,
        #[serde(rename = "Result")]
        result: StackResultActivitySnapshot,
    },
    StackRollback {
        #[serde(rename = "OldStack")]
        old_stack: Option<StackActivitySnapshot>,
        #[serde(rename = "NewStack")]
        new_stack: Option<StackActivitySnapshot>,
        #[serde(rename = "Result")]
        result: StackResultActivitySnapshot,
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
        stack: StackActivitySnapshot,
        #[serde(rename = "ProjectName")]
        project_name: String,
    },
    SwarmServiceCreated {
        #[serde(rename = "Service")]
        service: SwarmServiceActivitySnapshot,
    },
    SwarmServiceDuplicated {
        #[serde(rename = "Service")]
        service: SwarmServiceActivitySnapshot,
        #[serde(rename = "Source")]
        source: ActivitySourceResource,
    },
    SwarmServiceAdopted {
        #[serde(rename = "Service")]
        service: SwarmServiceActivitySnapshot,
        #[serde(rename = "DockerServiceId")]
        docker_service_id: String,
    },
    SwarmServiceUpdated {
        #[serde(rename = "OldService")]
        old_service: SwarmServiceActivitySnapshot,
        #[serde(rename = "NewService")]
        new_service: SwarmServiceActivitySnapshot,
    },
    SwarmServiceRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },
    SwarmServiceDeleted {
        #[serde(rename = "Service")]
        service: SwarmServiceActivitySnapshot,
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
        platform: PlatformActivitySnapshot,
        #[serde(rename = "PreviousStatus")]
        previous_status: String,
    },
    PlatformDisconnected {
        #[serde(rename = "Platform")]
        platform: PlatformActivitySnapshot,
        #[serde(rename = "PreviousStatus")]
        previous_status: String,
    },
    PlatformCreated {
        #[serde(rename = "Platform")]
        platform: PlatformActivitySnapshot,
    },
    PlatformRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },
    PlatformDeleted {
        #[serde(rename = "Platform")]
        platform: PlatformActivitySnapshot,
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
        git_repo: GitRepositoryActivitySnapshot,
    },
    GitRepoUpdated {
        #[serde(rename = "OldGitRepo")]
        old_git_repo: GitRepositoryActivitySnapshot,
        #[serde(rename = "NewGitRepo")]
        new_git_repo: GitRepositoryActivitySnapshot,
    },
    GitRepoRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },
    GitRepoDeleted {
        #[serde(rename = "GitRepo")]
        git_repo: GitRepositoryActivitySnapshot,
    },
    GitRepoPulled {
        #[serde(rename = "GitRepo")]
        git_repo: GitRepositoryActivitySnapshot,
        #[serde(rename = "Result")]
        result: GitRepositorySyncActivitySnapshot,
    },
    GitRepoCloned {
        #[serde(rename = "GitRepo")]
        git_repo: GitRepositoryActivitySnapshot,
        #[serde(rename = "Result")]
        result: GitRepositorySyncActivitySnapshot,
    },
    ActionCreated {
        #[serde(rename = "Action")]
        action: AutomationActionActivitySnapshot,
    },
    ActionUpdated {
        #[serde(rename = "OldAction")]
        old_action: AutomationActionActivitySnapshot,
        #[serde(rename = "NewAction")]
        new_action: AutomationActionActivitySnapshot,
    },
    ActionRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },
    ActionDeleted {
        #[serde(rename = "Action")]
        action: AutomationActionActivitySnapshot,
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
        build: BuildProjectActivitySnapshot,
    },
    BuildUpdated {
        #[serde(rename = "OldBuild")]
        old_build: BuildProjectActivitySnapshot,
        #[serde(rename = "NewBuild")]
        new_build: BuildProjectActivitySnapshot,
    },
    BuildRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },
    BuildDeleted {
        #[serde(rename = "Build")]
        build: BuildProjectActivitySnapshot,
    },
    BuildAgentPoolCreated {
        #[serde(rename = "Pool")]
        pool: BuildAgentPoolActivitySnapshot,
    },
    BuildAgentPoolUpdated {
        #[serde(rename = "OldPool")]
        old_pool: BuildAgentPoolActivitySnapshot,
        #[serde(rename = "NewPool")]
        new_pool: BuildAgentPoolActivitySnapshot,
    },
    BuildAgentPoolRenamed {
        #[serde(rename = "OldName")]
        old_name: String,
        #[serde(rename = "NewName")]
        new_name: String,
    },
    BuildAgentPoolDeleted {
        #[serde(rename = "Pool")]
        pool: BuildAgentPoolActivitySnapshot,
    },
    BuildAgentPoolTested {
        #[serde(rename = "Pool")]
        pool: BuildAgentPoolActivitySnapshot,
        #[serde(rename = "Status")]
        status: String,
        #[serde(rename = "Message")]
        message: String,
    },
}

impl ActivityEventInfo {
    #[must_use]
    pub fn user_profile_updated(old_name: String, new_name: String) -> Self {
        Self::UserProfileUpdated {
            changes: vec![ActivityChangedField::display_name(old_name, new_name)],
        }
    }

    pub fn user_preferences_updated(
        changes: Vec<ActivityChangedField>,
    ) -> Result<Self, ActivityInvariantError> {
        if changes.is_empty() {
            return Err(ActivityInvariantError::EmptyChanges);
        }
        if changes
            .iter()
            .any(|change| !matches!(change.name(), "TimeZone" | "DateTimeFormat" | "Theme"))
        {
            return Err(ActivityInvariantError::InvalidChangedField);
        }
        Ok(Self::UserPreferencesUpdated { changes })
    }

    #[must_use]
    pub const fn user_password_changed() -> Self {
        Self::UserPasswordChanged
    }

    #[must_use]
    pub const fn user_session_revoked(session_id: Uuid) -> Self {
        Self::UserSessionRevoked { session_id }
    }

    pub fn user_other_sessions_revoked(count: i64) -> Result<Self, ActivityInvariantError> {
        let count = i32::try_from(count).map_err(|_| ActivityInvariantError::InvalidCount)?;
        if count <= 0 {
            return Err(ActivityInvariantError::InvalidCount);
        }
        Ok(Self::UserOtherSessionsRevoked { count })
    }

    #[must_use]
    pub const fn user_mfa_enabled() -> Self {
        Self::UserMfaEnabled
    }

    #[must_use]
    pub const fn user_mfa_disabled() -> Self {
        Self::UserMfaDisabled
    }

    #[must_use]
    pub const fn user_mfa_verification_failed() -> Self {
        Self::UserMfaVerificationFailed
    }

    #[must_use]
    pub const fn user_mfa_recovery_code_used() -> Self {
        Self::UserMfaRecoveryCodeUsed
    }

    #[must_use]
    pub const fn user_mfa_recovery_codes_regenerated() -> Self {
        Self::UserMfaRecoveryCodesRegenerated
    }

    #[must_use]
    pub const fn user_mfa_reset_by_administrator(target_user_id: Uuid) -> Self {
        Self::UserMfaResetByAdministrator { target_user_id }
    }

    #[must_use]
    pub const fn user_created(user: UserActivitySnapshot) -> Self {
        Self::UserCreated { user }
    }

    #[must_use]
    pub const fn user_updated(
        old_user: UserActivitySnapshot,
        new_user: UserActivitySnapshot,
        password_changed: bool,
    ) -> Self {
        Self::UserUpdated {
            old_user,
            new_user,
            password_changed,
        }
    }

    #[must_use]
    pub fn user_renamed(old_name: String, new_name: String) -> Self {
        Self::UserRenamed { old_name, new_name }
    }

    #[must_use]
    pub const fn user_deleted(user: UserActivitySnapshot) -> Self {
        Self::UserDeleted { user }
    }

    #[must_use]
    pub const fn team_created(team: TeamActivitySnapshot) -> Self {
        Self::TeamCreated { team }
    }

    #[must_use]
    pub const fn team_updated(
        old_team: TeamActivitySnapshot,
        new_team: TeamActivitySnapshot,
    ) -> Self {
        Self::TeamUpdated { old_team, new_team }
    }

    #[must_use]
    pub fn team_renamed(old_name: String, new_name: String) -> Self {
        Self::TeamRenamed { old_name, new_name }
    }

    #[must_use]
    pub const fn team_deleted(team: TeamActivitySnapshot) -> Self {
        Self::TeamDeleted { team }
    }

    #[must_use]
    pub const fn role_created(role: RoleActivitySnapshot) -> Self {
        Self::RoleCreated { role }
    }

    #[must_use]
    pub const fn role_updated(
        old_role: RoleActivitySnapshot,
        new_role: RoleActivitySnapshot,
    ) -> Self {
        Self::RoleUpdated { old_role, new_role }
    }

    #[must_use]
    pub fn role_renamed(old_name: String, new_name: String) -> Self {
        Self::RoleRenamed { old_name, new_name }
    }

    #[must_use]
    pub const fn role_deleted(role: RoleActivitySnapshot) -> Self {
        Self::RoleDeleted { role }
    }

    #[must_use]
    pub const fn service_account_created(account: ServiceAccountActivitySnapshot) -> Self {
        Self::ServiceAccountCreated { account }
    }

    #[must_use]
    pub const fn service_account_updated(
        old_account: ServiceAccountActivitySnapshot,
        new_account: ServiceAccountActivitySnapshot,
    ) -> Self {
        Self::ServiceAccountUpdated {
            old_account,
            new_account,
        }
    }

    #[must_use]
    pub fn service_account_renamed(old_name: String, new_name: String) -> Self {
        Self::ServiceAccountRenamed { old_name, new_name }
    }

    #[must_use]
    pub const fn service_account_enabled(account_id: Uuid) -> Self {
        Self::ServiceAccountEnabled { account_id }
    }

    #[must_use]
    pub const fn service_account_disabled(account_id: Uuid) -> Self {
        Self::ServiceAccountDisabled { account_id }
    }

    #[must_use]
    pub const fn service_account_archived(account_id: Uuid) -> Self {
        Self::ServiceAccountArchived { account_id }
    }

    #[must_use]
    pub fn service_account_token_created(
        account_id: Uuid,
        token_id: Uuid,
        token_name: String,
        public_hint: String,
        expires_at_utc: Option<DateTime<Utc>>,
    ) -> Self {
        Self::ServiceAccountTokenCreated {
            account_id,
            token_id,
            token_name,
            public_hint,
            expires_at_utc,
        }
    }

    #[must_use]
    pub fn service_account_token_revoked(
        account_id: Uuid,
        token_id: Uuid,
        public_hint: String,
    ) -> Self {
        Self::ServiceAccountTokenRevoked {
            account_id,
            token_id,
            public_hint,
        }
    }

    #[must_use]
    pub fn license_installed(license: LicenseActivitySnapshot) -> Self {
        Self::LicenseInstalled {
            license: Box::new(license),
        }
    }

    #[must_use]
    pub fn license_replaced(
        old_license: LicenseActivitySnapshot,
        new_license: LicenseActivitySnapshot,
    ) -> Self {
        Self::LicenseReplaced {
            old_license: Box::new(old_license),
            new_license: Box::new(new_license),
        }
    }

    #[must_use]
    pub fn license_removed(license: LicenseActivitySnapshot) -> Self {
        Self::LicenseRemoved {
            license: Box::new(license),
        }
    }

    #[must_use]
    pub fn license_entered_grace_period(license: LicenseActivitySnapshot) -> Self {
        Self::LicenseEnteredGracePeriod {
            license: Box::new(license),
        }
    }

    #[must_use]
    pub fn license_expired(license: LicenseActivitySnapshot) -> Self {
        Self::LicenseExpired {
            license: Box::new(license),
        }
    }

    #[must_use]
    pub fn license_validation_failed(
        fingerprint: Option<String>,
        status: LicenseStatus,
        error_code: Option<String>,
    ) -> Self {
        Self::LicenseValidationFailed {
            fingerprint,
            status,
            error_code,
        }
    }

    #[must_use]
    pub fn oidc_provider_created(provider: OidcProviderActivitySnapshot) -> Self {
        Self::OidcProviderCreated {
            provider: Box::new(provider),
        }
    }

    #[must_use]
    pub fn oidc_provider_updated(
        old_provider: OidcProviderActivitySnapshot,
        new_provider: OidcProviderActivitySnapshot,
    ) -> Self {
        Self::OidcProviderUpdated {
            old_provider: Box::new(old_provider),
            new_provider: Box::new(new_provider),
        }
    }

    #[must_use]
    pub fn oidc_provider_renamed(old_name: String, new_name: String) -> Self {
        Self::OidcProviderRenamed { old_name, new_name }
    }

    #[must_use]
    pub fn oidc_provider_deleted(provider: OidcProviderActivitySnapshot) -> Self {
        Self::OidcProviderDeleted {
            provider: Box::new(provider),
        }
    }

    #[must_use]
    pub const fn registry_created(registry: RegistryActivitySnapshot) -> Self {
        Self::RegistryCreated { registry }
    }

    #[must_use]
    pub const fn registry_updated(
        old_registry: RegistryActivitySnapshot,
        new_registry: RegistryActivitySnapshot,
    ) -> Self {
        Self::RegistryUpdated {
            old_registry,
            new_registry,
        }
    }

    #[must_use]
    pub fn registry_renamed(old_name: String, new_name: String) -> Self {
        Self::RegistryRenamed { old_name, new_name }
    }

    #[must_use]
    pub const fn registry_deleted(registry: RegistryActivitySnapshot) -> Self {
        Self::RegistryDeleted { registry }
    }

    #[must_use]
    pub const fn deployment_created(deployment: DeploymentActivitySnapshot) -> Self {
        Self::DeploymentCreated { deployment }
    }

    #[must_use]
    pub const fn deployment_duplicated(
        deployment: DeploymentActivitySnapshot,
        source: ActivitySourceResource,
    ) -> Self {
        Self::DeploymentDuplicated { deployment, source }
    }

    #[must_use]
    pub const fn deployment_updated(
        old_deployment: DeploymentActivitySnapshot,
        new_deployment: DeploymentActivitySnapshot,
    ) -> Self {
        Self::DeploymentUpdated {
            old_deployment,
            new_deployment,
        }
    }

    #[must_use]
    pub fn deployment_renamed(old_name: String, new_name: String) -> Self {
        Self::DeploymentRenamed { old_name, new_name }
    }

    #[must_use]
    pub const fn deployment_deleted(deployment: DeploymentActivitySnapshot) -> Self {
        Self::DeploymentDeleted { deployment }
    }

    #[must_use]
    pub const fn deployment_applied(
        deployment: Option<DeploymentActivitySnapshot>,
        result: DeploymentResultActivitySnapshot,
    ) -> Self {
        Self::DeploymentApplied { deployment, result }
    }

    #[must_use]
    pub const fn swarm_service_created(service: SwarmServiceActivitySnapshot) -> Self {
        Self::SwarmServiceCreated { service }
    }

    #[must_use]
    pub const fn swarm_service_updated(
        old_service: SwarmServiceActivitySnapshot,
        new_service: SwarmServiceActivitySnapshot,
    ) -> Self {
        Self::SwarmServiceUpdated {
            old_service,
            new_service,
        }
    }

    #[must_use]
    pub fn swarm_service_renamed(old_name: String, new_name: String) -> Self {
        Self::SwarmServiceRenamed { old_name, new_name }
    }

    #[must_use]
    pub const fn swarm_service_deleted(service: SwarmServiceActivitySnapshot) -> Self {
        Self::SwarmServiceDeleted { service }
    }

    #[must_use]
    pub fn swarm_service_completed(
        kind: &str,
        operation_id: Uuid,
        replicas: Option<i32>,
        warnings: Vec<String>,
    ) -> Self {
        match kind {
            "Scale" => Self::SwarmServiceScaled {
                operation_id,
                replicas: replicas.unwrap_or_default(),
                warnings,
            },
            "ForceUpdate" => Self::SwarmServiceForceUpdated {
                operation_id,
                warnings,
            },
            _ => Self::SwarmServiceApplied {
                operation_id,
                warnings,
            },
        }
    }

    #[must_use]
    pub fn swarm_service_operation_failed(
        operation_id: Uuid,
        kind: String,
        reason: String,
    ) -> Self {
        Self::SwarmServiceOperationFailed {
            operation_id,
            kind,
            reason,
        }
    }

    #[must_use]
    pub const fn platform_created(platform: PlatformActivitySnapshot) -> Self {
        Self::PlatformCreated { platform }
    }

    #[must_use]
    pub const fn git_repo_created(git_repo: GitRepositoryActivitySnapshot) -> Self {
        Self::GitRepoCreated { git_repo }
    }

    #[must_use]
    pub const fn git_repo_updated(
        old_git_repo: GitRepositoryActivitySnapshot,
        new_git_repo: GitRepositoryActivitySnapshot,
    ) -> Self {
        Self::GitRepoUpdated {
            old_git_repo,
            new_git_repo,
        }
    }

    #[must_use]
    pub fn git_repo_renamed(old_name: String, new_name: String) -> Self {
        Self::GitRepoRenamed { old_name, new_name }
    }

    #[must_use]
    pub const fn git_repo_deleted(git_repo: GitRepositoryActivitySnapshot) -> Self {
        Self::GitRepoDeleted { git_repo }
    }

    #[must_use]
    pub const fn git_repo_synchronized(
        git_repo: GitRepositoryActivitySnapshot,
        result: GitRepositorySyncActivitySnapshot,
        cloned: bool,
    ) -> Self {
        if cloned {
            Self::GitRepoCloned { git_repo, result }
        } else {
            Self::GitRepoPulled { git_repo, result }
        }
    }

    #[must_use]
    pub const fn event_type(&self) -> ActivityEventType {
        match self {
            Self::VolumeContentDownloaded(_) => ActivityEventType::VolumeContentDownloaded,
            Self::GitRepoWebhookReceived(_) => ActivityEventType::GitRepoWebhookReceived,
            Self::StackWebhookReceived(_) => ActivityEventType::StackWebhookReceived,
            Self::BuildWebhookReceived(_) => ActivityEventType::BuildWebhookReceived,
            Self::SwarmServiceWebhookReceived(_) => ActivityEventType::SwarmServiceWebhookReceived,
            Self::UserProfileUpdated { .. } => ActivityEventType::UserProfileUpdated,
            Self::UserPreferencesUpdated { .. } => ActivityEventType::UserPreferencesUpdated,
            Self::UserPasswordChanged => ActivityEventType::UserPasswordChanged,
            Self::UserSessionRevoked { .. } => ActivityEventType::UserSessionRevoked,
            Self::UserOtherSessionsRevoked { .. } => ActivityEventType::UserOtherSessionsRevoked,
            Self::UserMfaEnabled => ActivityEventType::UserMfaEnabled,
            Self::UserMfaDisabled => ActivityEventType::UserMfaDisabled,
            Self::UserMfaVerificationFailed => ActivityEventType::UserMfaVerificationFailed,
            Self::UserMfaRecoveryCodeUsed => ActivityEventType::UserMfaRecoveryCodeUsed,
            Self::UserMfaRecoveryCodesRegenerated => {
                ActivityEventType::UserMfaRecoveryCodesRegenerated
            }
            Self::UserMfaResetByAdministrator { .. } => {
                ActivityEventType::UserMfaResetByAdministrator
            }
            Self::UserCreated { .. } => ActivityEventType::UserCreated,
            Self::UserUpdated { .. } => ActivityEventType::UserUpdated,
            Self::UserRenamed { .. } => ActivityEventType::UserRenamed,
            Self::UserDeleted { .. } => ActivityEventType::UserDeleted,
            Self::TeamCreated { .. } => ActivityEventType::TeamCreated,
            Self::TeamUpdated { .. } => ActivityEventType::TeamUpdated,
            Self::TeamRenamed { .. } => ActivityEventType::TeamRenamed,
            Self::TeamDeleted { .. } => ActivityEventType::TeamDeleted,
            Self::RoleCreated { .. } => ActivityEventType::RoleCreated,
            Self::RoleUpdated { .. } => ActivityEventType::RoleUpdated,
            Self::RoleRenamed { .. } => ActivityEventType::RoleRenamed,
            Self::RoleDeleted { .. } => ActivityEventType::RoleDeleted,
            Self::ServiceAccountCreated { .. } => ActivityEventType::ServiceAccountCreated,
            Self::ServiceAccountUpdated { .. } => ActivityEventType::ServiceAccountUpdated,
            Self::ServiceAccountRenamed { .. } => ActivityEventType::ServiceAccountRenamed,
            Self::ServiceAccountEnabled { .. } => ActivityEventType::ServiceAccountEnabled,
            Self::ServiceAccountDisabled { .. } => ActivityEventType::ServiceAccountDisabled,
            Self::ServiceAccountArchived { .. } => ActivityEventType::ServiceAccountArchived,
            Self::ServiceAccountTokenCreated { .. } => {
                ActivityEventType::ServiceAccountTokenCreated
            }
            Self::ServiceAccountTokenRevoked { .. } => {
                ActivityEventType::ServiceAccountTokenRevoked
            }
            Self::LicenseInstalled { .. } => ActivityEventType::LicenseInstalled,
            Self::LicenseReplaced { .. } => ActivityEventType::LicenseReplaced,
            Self::LicenseRemoved { .. } => ActivityEventType::LicenseRemoved,
            Self::LicenseEnteredGracePeriod { .. } => ActivityEventType::LicenseEnteredGracePeriod,
            Self::LicenseExpired { .. } => ActivityEventType::LicenseExpired,
            Self::LicenseValidationFailed { .. } => ActivityEventType::LicenseValidationFailed,
            Self::OidcProviderCreated { .. } => ActivityEventType::OidcProviderCreated,
            Self::OidcProviderUpdated { .. } => ActivityEventType::OidcProviderUpdated,
            Self::OidcProviderRenamed { .. } => ActivityEventType::OidcProviderRenamed,
            Self::OidcProviderDeleted { .. } => ActivityEventType::OidcProviderDeleted,
            Self::RegistryCreated { .. } => ActivityEventType::RegistryCreated,
            Self::RegistryUpdated { .. } => ActivityEventType::RegistryUpdated,
            Self::RegistryRenamed { .. } => ActivityEventType::RegistryRenamed,
            Self::AlertRuleRenamed { .. } => ActivityEventType::AlertRuleRenamed,
            Self::InitialAdministratorCreated { .. } => {
                ActivityEventType::InitialAdministratorCreated
            }
            Self::AlertRuleCreated { .. } => ActivityEventType::AlertRuleCreated,
            Self::AlertRuleUpdated { .. } => ActivityEventType::AlertRuleUpdated,
            Self::BackupPolicyRenamed { .. } => ActivityEventType::BackupPolicyRenamed,
            Self::BackupPolicyUpdated { .. } => ActivityEventType::BackupPolicyUpdated,
            Self::RegistryDeleted { .. } => ActivityEventType::RegistryDeleted,
            Self::DeploymentCreated { .. } => ActivityEventType::DeploymentCreated,
            Self::DeploymentAdopted { .. } => ActivityEventType::DeploymentAdopted,
            Self::DeploymentDuplicated { .. } => ActivityEventType::DeploymentDuplicated,
            Self::DeploymentUpdated { .. } => ActivityEventType::DeploymentUpdated,
            Self::DeploymentRenamed { .. } => ActivityEventType::DeploymentRenamed,
            Self::DeploymentDeleted { .. } => ActivityEventType::DeploymentDeleted,
            Self::DeploymentStarted { .. } => ActivityEventType::DeploymentStarted,
            Self::DeploymentStopped { .. } => ActivityEventType::DeploymentStopped,
            Self::DeploymentPaused { .. } => ActivityEventType::DeploymentPaused,
            Self::DeploymentDegraded { .. } => ActivityEventType::DeploymentDegraded,
            Self::StackDegraded { .. } => ActivityEventType::StackDegraded,
            Self::DeploymentApplied { .. } => ActivityEventType::DeploymentApplied,
            Self::StackCreated { .. } => ActivityEventType::StackCreated,
            Self::StackDuplicated { .. } => ActivityEventType::StackDuplicated,
            Self::StackUpdated { .. } => ActivityEventType::StackUpdated,
            Self::StackRenamed { .. } => ActivityEventType::StackRenamed,
            Self::StackDeleted { .. } => ActivityEventType::StackDeleted,
            Self::StackStarted { .. } => ActivityEventType::StackStarted,
            Self::StackStopped { .. } => ActivityEventType::StackStopped,
            Self::StackPaused { .. } => ActivityEventType::StackPaused,
            Self::StackApplied { .. } => ActivityEventType::StackApplied,
            Self::StackRollback { .. } => ActivityEventType::StackRollback,
            Self::StackImported { .. } => ActivityEventType::StackImported,
            Self::StackDriftDetected { .. } => ActivityEventType::StackDriftDetected,
            Self::StackDriftResolved { .. } => ActivityEventType::StackDriftResolved,
            Self::SwarmServiceCreated { .. } => ActivityEventType::SwarmServiceCreated,
            Self::SwarmServiceDuplicated { .. } => ActivityEventType::SwarmServiceDuplicated,
            Self::SwarmServiceAdopted { .. } => ActivityEventType::SwarmServiceAdopted,
            Self::SwarmServiceUpdated { .. } => ActivityEventType::SwarmServiceUpdated,
            Self::SwarmServiceRenamed { .. } => ActivityEventType::SwarmServiceRenamed,
            Self::SwarmServiceDeleted { .. } => ActivityEventType::SwarmServiceDeleted,
            Self::SwarmServiceApplied { .. } => ActivityEventType::SwarmServiceApplied,
            Self::SwarmServiceScaled { .. } => ActivityEventType::SwarmServiceScaled,
            Self::SwarmServiceForceUpdated { .. } => ActivityEventType::SwarmServiceForceUpdated,
            Self::SwarmServiceOperationFailed { .. } => {
                ActivityEventType::SwarmServiceOperationFailed
            }
            Self::PlatformCreated { .. } => ActivityEventType::PlatformCreated,
            Self::PlatformConnected { .. } => ActivityEventType::PlatformConnected,
            Self::PlatformDisconnected { .. } => ActivityEventType::PlatformDisconnected,
            Self::PlatformRenamed { .. } => ActivityEventType::PlatformRenamed,
            Self::PlatformDeleted { .. } => ActivityEventType::PlatformDeleted,
            Self::PlatformNodeAgentLifecycle { .. } => {
                ActivityEventType::PlatformNodeAgentLifecycle
            }
            Self::GitRepoCreated { .. } => ActivityEventType::GitRepoCreated,
            Self::GitRepoUpdated { .. } => ActivityEventType::GitRepoUpdated,
            Self::GitRepoRenamed { .. } => ActivityEventType::GitRepoRenamed,
            Self::GitRepoDeleted { .. } => ActivityEventType::GitRepoDeleted,
            Self::GitRepoPulled { .. } => ActivityEventType::GitRepoPulled,
            Self::GitRepoCloned { .. } => ActivityEventType::GitRepoCloned,
            Self::ActionCreated { .. } => ActivityEventType::ActionCreated,
            Self::ActionUpdated { .. } => ActivityEventType::ActionUpdated,
            Self::ActionRenamed { .. } => ActivityEventType::ActionRenamed,
            Self::ActionDeleted { .. } => ActivityEventType::ActionDeleted,
            Self::ActionRunQueued { .. } => ActivityEventType::ActionRunQueued,
            Self::ActionRunStarted { .. } => ActivityEventType::ActionRunStarted,
            Self::ActionRunSucceeded { .. } => ActivityEventType::ActionRunSucceeded,
            Self::ActionRunFailed { .. } => ActivityEventType::ActionRunFailed,
            Self::ActionRunTimedOut { .. } => ActivityEventType::ActionRunTimedOut,
            Self::ActionRunCancelled { .. } => ActivityEventType::ActionRunCancelled,
            Self::ActionRunRejected { .. } => ActivityEventType::ActionRunRejected,
            Self::BuildAgentPoolTested { .. } => ActivityEventType::BuildAgentPoolTested,
            Self::BuildCreated { .. } => ActivityEventType::BuildCreated,
            Self::BuildRunQueued { .. } => ActivityEventType::BuildRunQueued,
            Self::BuildRunStarted { .. } => ActivityEventType::BuildRunStarted,
            Self::BuildRunSucceeded { .. } => ActivityEventType::BuildRunSucceeded,
            Self::BuildRunFailed { .. } => ActivityEventType::BuildRunFailed,
            Self::BuildRunTimedOut { .. } => ActivityEventType::BuildRunTimedOut,
            Self::BuildRunCancelled { .. } => ActivityEventType::BuildRunCancelled,
            Self::BuildUpdated { .. } => ActivityEventType::BuildUpdated,
            Self::BuildRenamed { .. } => ActivityEventType::BuildRenamed,
            Self::BuildDeleted { .. } => ActivityEventType::BuildDeleted,
            Self::BuildAgentPoolCreated { .. } => ActivityEventType::BuildAgentPoolCreated,
            Self::BuildAgentPoolUpdated { .. } => ActivityEventType::BuildAgentPoolUpdated,
            Self::BuildAgentPoolRenamed { .. } => ActivityEventType::BuildAgentPoolRenamed,
            Self::BuildAgentPoolDeleted { .. } => ActivityEventType::BuildAgentPoolDeleted,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityEvent {
    id: Uuid,
    platform_id: Option<Uuid>,
    resource_id: Uuid,
    resource_name: String,
    resource_type: ActivityResourceType,
    status: ActivityStatus,
    event_type: ActivityEventType,
    info: ActivityEventInfo,
    created_by_actor_id: ActorId,
    created_at: DateTime<Utc>,
}

impl ActivityEvent {
    pub fn new_user_event(
        resource_id: Uuid,
        resource_name: String,
        actor_id: ActorId,
        info: ActivityEventInfo,
        created_at: DateTime<Utc>,
    ) -> Result<Self, ActivityInvariantError> {
        Self::new_resource_event(
            resource_id,
            resource_name,
            ActivityResourceType::User,
            actor_id,
            info,
            ActivityStatus::Success,
            created_at,
        )
    }

    pub fn new_team_event(
        resource_id: Uuid,
        resource_name: String,
        actor_id: ActorId,
        info: ActivityEventInfo,
        created_at: DateTime<Utc>,
    ) -> Result<Self, ActivityInvariantError> {
        Self::new_resource_event(
            resource_id,
            resource_name,
            ActivityResourceType::Team,
            actor_id,
            info,
            ActivityStatus::Success,
            created_at,
        )
    }

    pub fn new_role_event(
        resource_id: Uuid,
        resource_name: String,
        actor_id: ActorId,
        info: ActivityEventInfo,
        created_at: DateTime<Utc>,
    ) -> Result<Self, ActivityInvariantError> {
        Self::new_resource_event(
            resource_id,
            resource_name,
            ActivityResourceType::Role,
            actor_id,
            info,
            ActivityStatus::Success,
            created_at,
        )
    }

    pub fn new_service_account_event(
        resource_id: Uuid,
        resource_name: String,
        actor_id: ActorId,
        info: ActivityEventInfo,
        created_at: DateTime<Utc>,
    ) -> Result<Self, ActivityInvariantError> {
        Self::new_resource_event(
            resource_id,
            resource_name,
            ActivityResourceType::ServiceAccount,
            actor_id,
            info,
            ActivityStatus::Success,
            created_at,
        )
    }

    pub fn new_license_event(
        instance_id: Uuid,
        actor_id: ActorId,
        info: ActivityEventInfo,
        created_at: DateTime<Utc>,
    ) -> Result<Self, ActivityInvariantError> {
        Self::new_resource_event(
            instance_id,
            "License".to_owned(),
            ActivityResourceType::License,
            actor_id,
            info,
            ActivityStatus::Success,
            created_at,
        )
    }

    pub fn new_oidc_provider_event(
        resource_id: Uuid,
        resource_name: String,
        actor_id: ActorId,
        info: ActivityEventInfo,
        created_at: DateTime<Utc>,
    ) -> Result<Self, ActivityInvariantError> {
        Self::new_resource_event(
            resource_id,
            resource_name,
            ActivityResourceType::OidcProvider,
            actor_id,
            info,
            ActivityStatus::Success,
            created_at,
        )
    }

    pub fn new_registry_event(
        resource_id: Uuid,
        resource_name: String,
        actor_id: ActorId,
        info: ActivityEventInfo,
        created_at: DateTime<Utc>,
    ) -> Result<Self, ActivityInvariantError> {
        Self::new_resource_event(
            resource_id,
            resource_name,
            ActivityResourceType::Registry,
            actor_id,
            info,
            ActivityStatus::Success,
            created_at,
        )
    }

    pub fn new_git_repository_event(
        resource_id: Uuid,
        resource_name: String,
        actor_id: ActorId,
        info: ActivityEventInfo,
        created_at: DateTime<Utc>,
    ) -> Result<Self, ActivityInvariantError> {
        let status = if matches!(&info, ActivityEventInfo::GitRepoCreated { .. }) {
            ActivityStatus::Information
        } else {
            ActivityStatus::Success
        };
        Self::new_resource_event(
            resource_id,
            resource_name,
            ActivityResourceType::GitRepository,
            actor_id,
            info,
            status,
            created_at,
        )
    }

    pub fn new_git_repository_sync_event(
        resource_id: Uuid,
        resource_name: String,
        actor_id: ActorId,
        info: ActivityEventInfo,
        success: bool,
        created_at: DateTime<Utc>,
    ) -> Result<Self, ActivityInvariantError> {
        Self::new_resource_event(
            resource_id,
            resource_name,
            ActivityResourceType::GitRepository,
            actor_id,
            info,
            if success {
                ActivityStatus::Success
            } else {
                ActivityStatus::Failure
            },
            created_at,
        )
    }

    pub fn new_backup_policy_event(
        resource_id: Uuid,
        resource_name: String,
        actor_id: ActorId,
        info: ActivityEventInfo,
        created_at: DateTime<Utc>,
    ) -> Result<Self, ActivityInvariantError> {
        Self::new_resource_event(
            resource_id,
            resource_name,
            ActivityResourceType::BackupPolicy,
            actor_id,
            info,
            ActivityStatus::Success,
            created_at,
        )
    }

    pub fn new_alert_rule_event(
        resource_id: Uuid,
        resource_name: String,
        actor_id: ActorId,
        info: ActivityEventInfo,
        created_at: DateTime<Utc>,
    ) -> Result<Self, ActivityInvariantError> {
        Self::new_resource_event(
            resource_id,
            resource_name,
            ActivityResourceType::AlertRule,
            actor_id,
            info,
            ActivityStatus::Success,
            created_at,
        )
    }

    pub fn new_deployment_event(
        resource_id: Uuid,
        resource_name: String,
        platform_id: Uuid,
        actor_id: ActorId,
        info: ActivityEventInfo,
        created_at: DateTime<Utc>,
    ) -> Result<Self, ActivityInvariantError> {
        if platform_id.is_nil() {
            return Err(ActivityInvariantError::MissingResourceId);
        }
        let status = if matches!(
            &info,
            ActivityEventInfo::DeploymentCreated { .. }
                | ActivityEventInfo::DeploymentAdopted { .. }
                | ActivityEventInfo::DeploymentDuplicated { .. }
        ) {
            ActivityStatus::Information
        } else {
            ActivityStatus::Success
        };
        let mut event = Self::new_resource_event(
            resource_id,
            resource_name,
            ActivityResourceType::Deployment,
            actor_id,
            info,
            status,
            created_at,
        )?;
        event.platform_id = Some(platform_id);
        Ok(event)
    }

    pub fn new_automation_event(
        resource_id: Uuid,
        resource_name: String,
        actor_id: ActorId,
        info: ActivityEventInfo,
        created_at: DateTime<Utc>,
    ) -> Result<Self, ActivityInvariantError> {
        let status = match &info {
            ActivityEventInfo::ActionRunQueued { .. }
            | ActivityEventInfo::ActionRunStarted { .. } => ActivityStatus::Information,
            ActivityEventInfo::ActionRunFailed { .. }
            | ActivityEventInfo::ActionRunTimedOut { .. } => ActivityStatus::Failure,
            ActivityEventInfo::ActionRunCancelled { .. }
            | ActivityEventInfo::ActionRunRejected { .. } => ActivityStatus::Warning,
            _ => ActivityStatus::Success,
        };
        Self::new_resource_event(
            resource_id,
            resource_name,
            ActivityResourceType::AutomationAction,
            actor_id,
            info,
            status,
            created_at,
        )
    }

    pub fn new_build_pool_event(
        resource_id: Uuid,
        resource_name: String,
        actor_id: ActorId,
        info: ActivityEventInfo,
        created_at: DateTime<Utc>,
    ) -> Result<Self, ActivityInvariantError> {
        Self::new_resource_event(
            resource_id,
            resource_name,
            ActivityResourceType::BuildAgentPool,
            actor_id,
            info,
            ActivityStatus::Success,
            created_at,
        )
    }

    pub fn new_build_event(
        resource_id: Uuid,
        resource_name: String,
        actor_id: ActorId,
        info: ActivityEventInfo,
        created_at: DateTime<Utc>,
    ) -> Result<Self, ActivityInvariantError> {
        let status = match &info {
            ActivityEventInfo::BuildRunFailed { .. }
            | ActivityEventInfo::BuildRunTimedOut { .. } => ActivityStatus::Failure,
            _ => ActivityStatus::Success,
        };
        Self::new_resource_event(
            resource_id,
            resource_name,
            ActivityResourceType::Build,
            actor_id,
            info,
            status,
            created_at,
        )
    }

    pub fn new_platform_event(
        resource_id: Uuid,
        resource_name: String,
        actor_id: ActorId,
        info: ActivityEventInfo,
        created_at: DateTime<Utc>,
    ) -> Result<Self, ActivityInvariantError> {
        let status = if matches!(info, ActivityEventInfo::PlatformDisconnected { .. }) {
            ActivityStatus::Warning
        } else {
            ActivityStatus::Success
        };
        let mut event = Self::new_resource_event(
            resource_id,
            resource_name,
            ActivityResourceType::Platform,
            actor_id,
            info,
            status,
            created_at,
        )?;
        event.platform_id = Some(resource_id);
        Ok(event)
    }

    pub fn volume_downloaded(
        platform_id: Uuid,
        actor_id: ActorId,
        details: VolumeContentDownloaded,
        created_at: DateTime<Utc>,
    ) -> Result<Self, ActivityInvariantError> {
        let mut event = Self::new_resource_event(
            platform_id,
            details.volume_name.clone(),
            ActivityResourceType::Volume,
            actor_id,
            ActivityEventInfo::VolumeContentDownloaded(details),
            ActivityStatus::Success,
            created_at,
        )?;
        event.platform_id = Some(platform_id);
        Ok(event)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn node_agent_lifecycle(
        resource_id: Uuid,
        resource_name: String,
        actor: ActorId,
        operation_id: Uuid,
        status: ActivityStatus,
        message: String,
        created_at: DateTime<Utc>,
        kind: &'static str,
    ) -> Result<Self, ActivityInvariantError> {
        let state = match status {
            ActivityStatus::Success => "Completed",
            ActivityStatus::Information => "Running",
            _ => "Failed",
        };
        let mut event = Self::new_platform_event(
            resource_id,
            resource_name,
            actor,
            ActivityEventInfo::PlatformNodeAgentLifecycle {
                operation_id,
                kind,
                state,
                message,
            },
            created_at,
        )?;
        event.status = status;
        Ok(event)
    }

    pub fn new_swarm_service_event(
        resource_id: Uuid,
        resource_name: String,
        platform_id: Uuid,
        actor_id: ActorId,
        info: ActivityEventInfo,
        status: ActivityStatus,
        created_at: DateTime<Utc>,
    ) -> Result<Self, ActivityInvariantError> {
        if platform_id.is_nil() {
            return Err(ActivityInvariantError::MissingResourceId);
        }
        let mut event = Self::new_resource_event(
            resource_id,
            resource_name,
            ActivityResourceType::SwarmService,
            actor_id,
            info,
            status,
            created_at,
        )?;
        event.platform_id = Some(platform_id);
        Ok(event)
    }

    pub fn new_stack_event(
        resource_id: Uuid,
        resource_name: String,
        platform_id: Uuid,
        actor_id: ActorId,
        info: ActivityEventInfo,
        status: ActivityStatus,
        created_at: DateTime<Utc>,
    ) -> Result<Self, ActivityInvariantError> {
        if platform_id.is_nil() {
            return Err(ActivityInvariantError::MissingResourceId);
        }
        let mut event = Self::new_resource_event(
            resource_id,
            resource_name,
            ActivityResourceType::Stack,
            actor_id,
            info,
            status,
            created_at,
        )?;
        event.platform_id = Some(platform_id);
        Ok(event)
    }

    pub fn new_deployment_result_event(
        resource_id: Uuid,
        resource_name: String,
        platform_id: Uuid,
        actor_id: ActorId,
        info: ActivityEventInfo,
        status: ActivityStatus,
        created_at: DateTime<Utc>,
    ) -> Result<Self, ActivityInvariantError> {
        if platform_id.is_nil() {
            return Err(ActivityInvariantError::MissingResourceId);
        }
        let mut event = Self::new_resource_event(
            resource_id,
            resource_name,
            ActivityResourceType::Deployment,
            actor_id,
            info,
            status,
            created_at,
        )?;
        event.platform_id = Some(platform_id);
        Ok(event)
    }

    fn new_resource_event(
        resource_id: Uuid,
        resource_name: String,
        expected_resource_type: ActivityResourceType,
        actor_id: ActorId,
        info: ActivityEventInfo,
        status: ActivityStatus,
        created_at: DateTime<Utc>,
    ) -> Result<Self, ActivityInvariantError> {
        if resource_id.is_nil() {
            return Err(ActivityInvariantError::MissingResourceId);
        }
        if actor_id.value().is_nil() {
            return Err(ActivityInvariantError::MissingActorId);
        }
        let resource_name = resource_name.trim();
        if resource_name.is_empty() {
            return Err(ActivityInvariantError::MissingResourceName);
        }
        let event_type = info.event_type();
        let resource_type = event_type.resource_type();
        if resource_type != expected_resource_type {
            return Err(ActivityInvariantError::MismatchedResourceType);
        }
        Ok(Self {
            id: Uuid::now_v7(),
            platform_id: None,
            resource_id,
            resource_name: resource_name.to_owned(),
            resource_type,
            status,
            event_type,
            info,
            created_by_actor_id: actor_id,
            created_at,
        })
    }

    #[must_use]
    pub const fn id(&self) -> Uuid {
        self.id
    }

    #[must_use]
    pub const fn platform_id(&self) -> Option<Uuid> {
        self.platform_id
    }

    #[must_use]
    pub const fn resource_id(&self) -> Uuid {
        self.resource_id
    }

    #[must_use]
    pub fn resource_name(&self) -> &str {
        &self.resource_name
    }

    #[must_use]
    pub const fn resource_type(&self) -> ActivityResourceType {
        self.resource_type
    }

    #[must_use]
    pub const fn status(&self) -> ActivityStatus {
        self.status
    }

    #[must_use]
    pub const fn event_type(&self) -> ActivityEventType {
        self.event_type
    }

    #[must_use]
    pub const fn info(&self) -> &ActivityEventInfo {
        &self.info
    }

    #[must_use]
    pub const fn created_by_actor_id(&self) -> ActorId {
        self.created_by_actor_id
    }

    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityInvariantError {
    MissingResourceId,
    MissingActorId,
    MissingResourceName,
    EmptyChanges,
    InvalidChangedField,
    InvalidCount,
    MismatchedResourceType,
}

impl fmt::Display for ActivityInvariantError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::MissingResourceId => "activity resource ID is required",
            Self::MissingActorId => "activity actor ID is required",
            Self::MissingResourceName => "activity resource name is required",
            Self::EmptyChanges => "activity changes cannot be empty",
            Self::InvalidChangedField => "activity contains a non-allow-listed changed field",
            Self::InvalidCount => "activity count must be a positive 32-bit integer",
            Self::MismatchedResourceType => "activity event and resource types do not match",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ActivityInvariantError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automation_activity_preserves_wire_identity_and_status() {
        let resource = Uuid::now_v7();
        let actor = ActorId::new(Uuid::now_v7());
        let run = Uuid::now_v7();
        for (info, expected) in [
            (
                ActivityEventInfo::ActionRunQueued {
                    run_id: run,
                    trigger: "Manual".into(),
                },
                ActivityStatus::Information,
            ),
            (
                ActivityEventInfo::ActionRunSucceeded {
                    run_id: run,
                    trigger: "Manual".into(),
                    exit_code: Some(0),
                    duration_ms: Some(12),
                },
                ActivityStatus::Success,
            ),
            (
                ActivityEventInfo::ActionRunFailed {
                    run_id: run,
                    trigger: "Manual".into(),
                    exit_code: Some(7),
                    duration_ms: Some(12),
                    error_message: Some("failed".into()),
                },
                ActivityStatus::Failure,
            ),
            (
                ActivityEventInfo::ActionRunCancelled {
                    run_id: run,
                    trigger: "Manual".into(),
                },
                ActivityStatus::Warning,
            ),
        ] {
            let event = ActivityEvent::new_automation_event(
                resource,
                "Action".into(),
                actor,
                info,
                Utc::now(),
            )
            .unwrap();
            assert_eq!(event.status(), expected);
            assert_eq!(
                event.resource_type(),
                ActivityResourceType::AutomationAction
            );
            let info = serde_json::to_value(event.info()).unwrap();
            assert_eq!(info["RunId"], run.to_string());
            assert_eq!(info["Trigger"], "Manual");
        }
    }

    #[test]
    fn profile_activity_derives_its_discriminators() {
        let event = ActivityEvent::new_user_event(
            Uuid::now_v7(),
            "Owner".to_owned(),
            ActorId::new(Uuid::now_v7()),
            ActivityEventInfo::user_profile_updated("Old owner".to_owned(), "Owner".to_owned()),
            Utc::now(),
        )
        .unwrap();

        assert_eq!(event.event_type(), ActivityEventType::UserProfileUpdated);
        assert_eq!(event.resource_type(), ActivityResourceType::User);
        assert_eq!(event.status(), ActivityStatus::Success);
    }

    #[test]
    fn empty_preference_changes_are_rejected() {
        assert_eq!(
            ActivityEventInfo::user_preferences_updated(Vec::new()),
            Err(ActivityInvariantError::EmptyChanges)
        );
    }

    #[test]
    fn revoked_other_sessions_requires_a_positive_count() {
        assert_eq!(
            ActivityEventInfo::user_other_sessions_revoked(0),
            Err(ActivityInvariantError::InvalidCount)
        );
    }

    #[test]
    fn user_lifecycle_activity_uses_the_compatible_safe_payload_shape() {
        let snapshot = UserActivitySnapshot {
            email: "owner@example.test".to_owned(),
            is_enabled: true,
            team_ids: Vec::new(),
            role_ids: vec![Uuid::now_v7()],
            resource_accesses: Vec::new(),
        };
        let info = ActivityEventInfo::user_updated(snapshot.clone(), snapshot, true);
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["$type"], "UserUpdated");
        assert_eq!(json["PasswordChanged"], true);
        assert!(json.get("Password").is_none());
        assert!(json.get("PasswordHash").is_none());
        assert_eq!(info.event_type(), ActivityEventType::UserUpdated);
    }

    #[test]
    fn mfa_activity_payloads_cannot_contain_credentials() {
        let target = Uuid::now_v7();
        let info = ActivityEventInfo::user_mfa_reset_by_administrator(target);
        let json = serde_json::to_value(&info).unwrap();

        assert_eq!(json["$type"], "UserMfaResetByAdministrator");
        assert_eq!(json["TargetUserId"], target.to_string());
        assert!(json.get("Code").is_none());
        assert!(json.get("Secret").is_none());
        assert!(json.get("RecoveryCode").is_none());
    }

    #[test]
    fn team_lifecycle_activity_uses_the_compatible_safe_payload_shape() {
        let snapshot = TeamActivitySnapshot {
            is_enabled: true,
            member_actor_ids: vec![Uuid::now_v7()],
            role_ids: vec![Uuid::now_v7()],
            resource_accesses: Vec::new(),
        };
        let info = ActivityEventInfo::team_updated(snapshot.clone(), snapshot);
        let event = ActivityEvent::new_team_event(
            Uuid::now_v7(),
            "Operations".to_owned(),
            ActorId::new(Uuid::now_v7()),
            info,
            Utc::now(),
        )
        .unwrap();
        let json = serde_json::to_value(event.info()).unwrap();

        assert_eq!(event.event_type(), ActivityEventType::TeamUpdated);
        assert_eq!(event.resource_type(), ActivityResourceType::Team);
        assert_eq!(json["$type"], "TeamUpdated");
        assert!(json.get("Password").is_none());
        assert!(json.get("Token").is_none());
    }

    #[test]
    fn deployment_activity_uses_the_existing_dotnet_payload_shape() {
        let deployment_id = Uuid::now_v7();
        let platform_id = Uuid::now_v7();
        let event = ActivityEvent::new_deployment_event(
            deployment_id,
            "web".to_owned(),
            platform_id,
            ActorId::new(Uuid::now_v7()),
            ActivityEventInfo::deployment_created(DeploymentActivitySnapshot {
                id: deployment_id,
                name: "web".to_owned(),
                platform_id,
                description: None,
                spec: serde_json::json!({"Image":{"$type":"Local","ImageId":"image"}}),
            }),
            Utc::now(),
        )
        .unwrap();
        let info = serde_json::to_value(event.info()).unwrap();

        assert_eq!(event.platform_id(), Some(platform_id));
        assert_eq!(event.resource_type(), ActivityResourceType::Deployment);
        assert_eq!(event.event_type(), ActivityEventType::DeploymentCreated);
        assert_eq!(event.status(), ActivityStatus::Information);
        assert_eq!(info["$type"], "DeploymentCreated");
        assert_eq!(info["Deployment"]["Id"], deployment_id.to_string());
        assert_eq!(info["Deployment"]["PlatformId"], platform_id.to_string());
    }

    #[test]
    fn swarm_service_activity_uses_the_existing_dotnet_payload_shape() {
        let service_id = Uuid::now_v7();
        let platform_id = Uuid::now_v7();
        let operation_id = Uuid::now_v7();
        let event = ActivityEvent::new_swarm_service_event(
            service_id,
            "redis".to_owned(),
            platform_id,
            ActorId::new(Uuid::now_v7()),
            ActivityEventInfo::swarm_service_completed(
                "Scale",
                operation_id,
                Some(3),
                vec!["warning".to_owned()],
            ),
            ActivityStatus::Success,
            Utc::now(),
        )
        .unwrap();
        let json = serde_json::to_value(event.info()).unwrap();

        assert_eq!(event.resource_type(), ActivityResourceType::SwarmService);
        assert_eq!(event.event_type(), ActivityEventType::SwarmServiceScaled);
        assert_eq!(event.platform_id(), Some(platform_id));
        assert_eq!(json["$type"], "SwarmServiceScaled");
        assert_eq!(json["OperationId"], operation_id.to_string());
        assert_eq!(json["Replicas"], 3);
        assert_eq!(json["Warnings"], serde_json::json!(["warning"]));
    }

    #[test]
    fn webhook_activities_keep_dotnet_discriminators_and_safe_shared_metadata() {
        for (resource, discriminator) in [
            (
                ActivityResourceType::GitRepository,
                "GitRepoWebhookReceived",
            ),
            (ActivityResourceType::Stack, "StackWebhookReceived"),
            (ActivityResourceType::Build, "BuildWebhookReceived"),
            (
                ActivityResourceType::SwarmService,
                "SwarmServiceWebhookReceived",
            ),
        ] {
            let request = Uuid::now_v7();
            let event = ActivityEvent::new_webhook_event(
                Uuid::now_v7(),
                "resource".into(),
                None,
                resource,
                WebhookActivityDetails {
                    request_id: request,
                    auth_type: "generic".into(),
                    execution: "update".into(),
                    status: "queued",
                    reason: None,
                    source: WebhookActivitySource::default(),
                    dispatched_branch: None,
                    dispatched_commit_sha: None,
                },
                Utc::now(),
            )
            .unwrap();
            let json = serde_json::to_value(event.info()).unwrap();
            assert_eq!(json["$type"], discriminator);
            assert_eq!(json["RequestId"], request.to_string());
            assert_eq!(json["Status"], "queued");
            assert_eq!(event.status(), ActivityStatus::Success);
            assert_eq!(event.resource_type(), resource);
            assert!(json.get("Headers").is_none() && json.get("Body").is_none());
        }
    }

    #[test]
    fn role_lifecycle_activity_uses_the_compatible_safe_payload_shape() {
        let snapshot = RoleActivitySnapshot {
            role_type: RoleType::Custom,
            permissions: vec![RolePermissionActivitySnapshot {
                resource_type: ResourceType::Registry,
                permission_level: PermissionLevel::Read,
                specific_permissions: 0,
            }],
        };
        let info = ActivityEventInfo::role_updated(snapshot.clone(), snapshot);
        let event = ActivityEvent::new_role_event(
            Uuid::now_v7(),
            "Registry reader".to_owned(),
            ActorId::new(Uuid::now_v7()),
            info,
            Utc::now(),
        )
        .unwrap();
        let json = serde_json::to_value(event.info()).unwrap();

        assert_eq!(event.event_type(), ActivityEventType::RoleUpdated);
        assert_eq!(event.resource_type(), ActivityResourceType::Role);
        assert_eq!(json["$type"], "RoleUpdated");
        assert_eq!(json["NewRole"]["RoleType"], "Custom");
        assert!(json.get("Password").is_none());
        assert!(json.get("Token").is_none());
    }

    #[test]
    fn oidc_provider_activity_never_contains_client_credentials() {
        let snapshot = OidcProviderActivitySnapshot {
            id: Uuid::now_v7(),
            name: "corporate".to_owned(),
            description: None,
            display_name: "Corporate login".to_owned(),
            issuer: "https://issuer.example.test".to_owned(),
            client_id: "citadel-client".to_owned(),
            scopes: "openid profile email".to_owned(),
            enabled: true,
            auto_provision_users: false,
            allow_email_auto_link: true,
            require_email_verified: true,
            allowed_email_domains: Some("example.test".to_owned()),
            required_claim_name: None,
            required_claim_values: None,
            default_role_id: None,
        };
        let info = ActivityEventInfo::oidc_provider_created(snapshot);
        let event = ActivityEvent::new_oidc_provider_event(
            Uuid::now_v7(),
            "corporate".to_owned(),
            ActorId::new(Uuid::now_v7()),
            info,
            Utc::now(),
        )
        .unwrap();
        let json = serde_json::to_value(event.info()).unwrap();

        assert_eq!(event.event_type(), ActivityEventType::OidcProviderCreated);
        assert_eq!(event.resource_type(), ActivityResourceType::OidcProvider);
        assert_eq!(json["$type"], "OidcProviderCreated");
        assert!(json["Provider"].get("ClientSecret").is_none());
        assert!(json["Provider"].get("ClientSecretCiphertext").is_none());
    }

    #[test]
    fn catalog_activity_uses_compatible_fields_and_git_creation_status() {
        let snapshot = GitRepositoryActivitySnapshot {
            id: Uuid::now_v7(),
            name: "repository".to_owned(),
            description: None,
            url: "https://git.example.test/team/repository".to_owned(),
            default_branch: "main".to_owned(),
            git_account_id: None,
            sync_mode: "Manual".to_owned(),
            sync_interval_minutes: None,
            webhook: None,
            on_clone: None,
            on_pull: None,
            resolved_commit_sha: None,
        };
        let event = ActivityEvent::new_git_repository_event(
            snapshot.id,
            snapshot.name.clone(),
            ActorId::new(Uuid::now_v7()),
            ActivityEventInfo::git_repo_created(snapshot),
            Utc::now(),
        )
        .unwrap();
        let json = serde_json::to_value(event.info()).unwrap();

        assert_eq!(event.status(), ActivityStatus::Information);
        assert_eq!(json["$type"], "GitRepoCreated");
        assert!(json.get("GitRepo").is_some());
        assert!(json.get("Repository").is_none());
    }
}
