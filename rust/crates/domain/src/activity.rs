use std::fmt;

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    ActivityEventType, ActivityResourceType, ActivityStatus, ActorId, LicenseCapability,
    LicenseStatus, PermissionLevel, ResourceType, RoleType, UserDateTimeFormat, UserTheme,
};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "$type")]
pub enum ActivityEventInfo {
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
    pub const fn event_type(&self) -> ActivityEventType {
        match self {
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
        Self::new_identity_event(
            resource_id,
            resource_name,
            ActivityResourceType::User,
            actor_id,
            info,
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
        Self::new_identity_event(
            resource_id,
            resource_name,
            ActivityResourceType::Team,
            actor_id,
            info,
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
        Self::new_identity_event(
            resource_id,
            resource_name,
            ActivityResourceType::Role,
            actor_id,
            info,
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
        Self::new_identity_event(
            resource_id,
            resource_name,
            ActivityResourceType::ServiceAccount,
            actor_id,
            info,
            created_at,
        )
    }

    pub fn new_license_event(
        instance_id: Uuid,
        actor_id: ActorId,
        info: ActivityEventInfo,
        created_at: DateTime<Utc>,
    ) -> Result<Self, ActivityInvariantError> {
        Self::new_identity_event(
            instance_id,
            "License".to_owned(),
            ActivityResourceType::License,
            actor_id,
            info,
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
        Self::new_identity_event(
            resource_id,
            resource_name,
            ActivityResourceType::OidcProvider,
            actor_id,
            info,
            created_at,
        )
    }

    fn new_identity_event(
        resource_id: Uuid,
        resource_name: String,
        expected_resource_type: ActivityResourceType,
        actor_id: ActorId,
        info: ActivityEventInfo,
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
            status: ActivityStatus::Success,
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
}
