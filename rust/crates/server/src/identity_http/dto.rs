use chrono::{DateTime, Utc};
use citadel_identity::MfaPolicy;

use citadel_identity::{ActorType, RoleType, UserDateTimeFormat, UserTheme};
use citadel_primitives::{ActorId, PermissionLevel, ResourceType, SpecificPermission};

use citadel_identity::PatchField;

use serde::{Deserialize, Serialize};

use uuid::Uuid;

fn enabled_by_default() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceInfo {
    pub id: Uuid,
    pub name: String,
    pub group: Option<String>,
}

impl From<ResourceInfo> for citadel_identity::ResourceInfo {
    fn from(value: ResourceInfo) -> Self {
        Self {
            id: value.id,
            name: value.name,
            group: value.group,
        }
    }
}

impl From<citadel_identity::ResourceInfo> for ResourceInfo {
    fn from(value: citadel_identity::ResourceInfo) -> Self {
        Self {
            id: value.id,
            name: value.name,
            group: value.group,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PagedResult<T> {
    pub items: Vec<T>,
    pub total_count: i64,
    pub page: i64,
    pub page_size: i64,
}

impl<T, V: From<T>> From<citadel_identity::PagedResult<T>> for PagedResult<V> {
    fn from(value: citadel_identity::PagedResult<T>) -> Self {
        Self {
            items: value.items.into_iter().map(V::from).collect(),
            total_count: value.total_count,
            page: value.page,
            page_size: value.page_size,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MfaVerificationInput {
    pub code: Option<String>,
    pub recovery_code: Option<String>,
}

impl From<MfaVerificationInput> for citadel_identity::MfaVerificationInput {
    fn from(value: MfaVerificationInput) -> Self {
        Self {
            code: value.code,
            recovery_code: value.recovery_code,
        }
    }
}

impl From<citadel_identity::MfaVerificationInput> for MfaVerificationInput {
    fn from(value: citadel_identity::MfaVerificationInput) -> Self {
        Self {
            code: value.code,
            recovery_code: value.recovery_code,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfirmMandatoryMfaSetupInput {
    pub code: String,
}

impl From<ConfirmMandatoryMfaSetupInput> for citadel_identity::ConfirmMandatoryMfaSetupInput {
    fn from(value: ConfirmMandatoryMfaSetupInput) -> Self {
        Self { code: value.code }
    }
}

impl From<citadel_identity::ConfirmMandatoryMfaSetupInput> for ConfirmMandatoryMfaSetupInput {
    fn from(value: citadel_identity::ConfirmMandatoryMfaSetupInput) -> Self {
        Self { code: value.code }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartProfileMfaSetupInput {
    pub password: String,
}

impl From<StartProfileMfaSetupInput> for citadel_identity::StartProfileMfaSetupInput {
    fn from(value: StartProfileMfaSetupInput) -> Self {
        Self {
            password: value.password,
        }
    }
}

impl From<citadel_identity::StartProfileMfaSetupInput> for StartProfileMfaSetupInput {
    fn from(value: citadel_identity::StartProfileMfaSetupInput) -> Self {
        Self {
            password: value.password,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfirmProfileMfaSetupInput {
    pub code: String,
}

impl From<ConfirmProfileMfaSetupInput> for citadel_identity::ConfirmProfileMfaSetupInput {
    fn from(value: ConfirmProfileMfaSetupInput) -> Self {
        Self { code: value.code }
    }
}

impl From<citadel_identity::ConfirmProfileMfaSetupInput> for ConfirmProfileMfaSetupInput {
    fn from(value: citadel_identity::ConfirmProfileMfaSetupInput) -> Self {
        Self { code: value.code }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DisableProfileMfaInput {
    pub password: String,
    pub code: Option<String>,
    pub recovery_code: Option<String>,
}

impl From<DisableProfileMfaInput> for citadel_identity::DisableProfileMfaInput {
    fn from(value: DisableProfileMfaInput) -> Self {
        Self {
            password: value.password,
            code: value.code,
            recovery_code: value.recovery_code,
        }
    }
}

impl From<citadel_identity::DisableProfileMfaInput> for DisableProfileMfaInput {
    fn from(value: citadel_identity::DisableProfileMfaInput) -> Self {
        Self {
            password: value.password,
            code: value.code,
            recovery_code: value.recovery_code,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegenerateProfileMfaRecoveryCodesInput {
    pub password: String,
    pub code: String,
}

impl From<RegenerateProfileMfaRecoveryCodesInput>
    for citadel_identity::RegenerateProfileMfaRecoveryCodesInput
{
    fn from(value: RegenerateProfileMfaRecoveryCodesInput) -> Self {
        Self {
            password: value.password,
            code: value.code,
        }
    }
}

impl From<citadel_identity::RegenerateProfileMfaRecoveryCodesInput>
    for RegenerateProfileMfaRecoveryCodesInput
{
    fn from(value: citadel_identity::RegenerateProfileMfaRecoveryCodesInput) -> Self {
        Self {
            password: value.password,
            code: value.code,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MandatoryMfaSetupView {
    pub secret: String,
    pub otp_auth_uri: String,
    pub expires_at: DateTime<Utc>,
}

impl From<MandatoryMfaSetupView> for citadel_identity::MandatoryMfaSetupDetails {
    fn from(value: MandatoryMfaSetupView) -> Self {
        Self {
            secret: value.secret,
            otp_auth_uri: value.otp_auth_uri,
            expires_at: value.expires_at,
        }
    }
}

impl From<citadel_identity::MandatoryMfaSetupDetails> for MandatoryMfaSetupView {
    fn from(value: citadel_identity::MandatoryMfaSetupDetails) -> Self {
        Self {
            secret: value.secret,
            otp_auth_uri: value.otp_auth_uri,
            expires_at: value.expires_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MandatoryMfaSetupCompleteView {
    pub access_token: String,
    pub recovery_codes: Vec<String>,
}

impl From<MandatoryMfaSetupCompleteView> for citadel_identity::MandatoryMfaSetupCompleteDetails {
    fn from(value: MandatoryMfaSetupCompleteView) -> Self {
        Self {
            access_token: value.access_token,
            recovery_codes: value.recovery_codes,
        }
    }
}

impl From<citadel_identity::MandatoryMfaSetupCompleteDetails> for MandatoryMfaSetupCompleteView {
    fn from(value: citadel_identity::MandatoryMfaSetupCompleteDetails) -> Self {
        Self {
            access_token: value.access_token,
            recovery_codes: value.recovery_codes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MfaVerificationView {
    pub access_token: String,
}

impl From<MfaVerificationView> for citadel_identity::MfaVerificationDetails {
    fn from(value: MfaVerificationView) -> Self {
        Self {
            access_token: value.access_token,
        }
    }
}

impl From<citadel_identity::MfaVerificationDetails> for MfaVerificationView {
    fn from(value: citadel_identity::MfaVerificationDetails) -> Self {
        Self {
            access_token: value.access_token,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProfileMfaSetupView {
    pub secret: String,
    pub otp_auth_uri: String,
    pub expires_at: DateTime<Utc>,
}

impl From<ProfileMfaSetupView> for citadel_identity::ProfileMfaSetupDetails {
    fn from(value: ProfileMfaSetupView) -> Self {
        Self {
            secret: value.secret,
            otp_auth_uri: value.otp_auth_uri,
            expires_at: value.expires_at,
        }
    }
}

impl From<citadel_identity::ProfileMfaSetupDetails> for ProfileMfaSetupView {
    fn from(value: citadel_identity::ProfileMfaSetupDetails) -> Self {
        Self {
            secret: value.secret,
            otp_auth_uri: value.otp_auth_uri,
            expires_at: value.expires_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProfileMfaStatusView {
    pub enabled: bool,
    pub remaining_recovery_codes: i32,
    #[schema(value_type = crate::api::vocabulary::MfaPolicySchema)]
    pub policy: MfaPolicy,
    pub can_disable: bool,
}

impl From<ProfileMfaStatusView> for citadel_identity::ProfileMfaStatusDetails {
    fn from(value: ProfileMfaStatusView) -> Self {
        Self {
            enabled: value.enabled,
            remaining_recovery_codes: value.remaining_recovery_codes,
            policy: value.policy,
            can_disable: value.can_disable,
        }
    }
}

impl From<citadel_identity::ProfileMfaStatusDetails> for ProfileMfaStatusView {
    fn from(value: citadel_identity::ProfileMfaStatusDetails) -> Self {
        Self {
            enabled: value.enabled,
            remaining_recovery_codes: value.remaining_recovery_codes,
            policy: value.policy,
            can_disable: value.can_disable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProfileMfaRecoveryCodesView {
    pub enabled: bool,
    pub recovery_codes: Vec<String>,
}

impl From<ProfileMfaRecoveryCodesView> for citadel_identity::ProfileMfaRecoveryCodesDetails {
    fn from(value: ProfileMfaRecoveryCodesView) -> Self {
        Self {
            enabled: value.enabled,
            recovery_codes: value.recovery_codes,
        }
    }
}

impl From<citadel_identity::ProfileMfaRecoveryCodesDetails> for ProfileMfaRecoveryCodesView {
    fn from(value: citadel_identity::ProfileMfaRecoveryCodesDetails) -> Self {
        Self {
            enabled: value.enabled,
            recovery_codes: value.recovery_codes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceAccountResourceAccess {
    pub id: Option<Uuid>,
    pub resource_type: ResourceType,
    pub resource_id: Uuid,
    #[serde(default)]
    pub resource_name: Option<String>,
    pub permission_level: PermissionLevel,
    #[serde(default)]
    pub specific_permissions: Vec<SpecificPermission>,
}

impl From<ServiceAccountResourceAccess> for citadel_identity::ServiceAccountResourceAccess {
    fn from(value: ServiceAccountResourceAccess) -> Self {
        Self {
            id: value.id,
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            resource_name: value.resource_name,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

impl From<citadel_identity::ServiceAccountResourceAccess> for ServiceAccountResourceAccess {
    fn from(value: citadel_identity::ServiceAccountResourceAccess) -> Self {
        Self {
            id: value.id,
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            resource_name: value.resource_name,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateServiceAccountRequest {
    pub name: String,
    pub description: Option<String>,
    #[serde(default = "enabled_by_default")]
    pub is_enabled: bool,
    #[serde(default)]
    pub team_ids: Vec<Uuid>,
    #[serde(default)]
    pub role_ids: Vec<Uuid>,
    #[serde(default)]
    pub resource_accesses: Vec<ServiceAccountResourceAccess>,
}

impl From<CreateServiceAccountRequest> for citadel_identity::CreateServiceAccount {
    fn from(value: CreateServiceAccountRequest) -> Self {
        Self {
            name: value.name,
            description: value.description,
            is_enabled: value.is_enabled,
            team_ids: value.team_ids,
            role_ids: value.role_ids,
            resource_accesses: value
                .resource_accesses
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

impl From<citadel_identity::CreateServiceAccount> for CreateServiceAccountRequest {
    fn from(value: citadel_identity::CreateServiceAccount) -> Self {
        Self {
            name: value.name,
            description: value.description,
            is_enabled: value.is_enabled,
            team_ids: value.team_ids,
            role_ids: value.role_ids,
            resource_accesses: value
                .resource_accesses
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateServiceAccountRequest {
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub description: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<bool>, required = false)]
    pub is_enabled: PatchField<bool>,
}

impl From<UpdateServiceAccountRequest> for citadel_identity::UpdateServiceAccount {
    fn from(value: UpdateServiceAccountRequest) -> Self {
        Self {
            description: value.description,
            is_enabled: value.is_enabled,
        }
    }
}

impl From<citadel_identity::UpdateServiceAccount> for UpdateServiceAccountRequest {
    fn from(value: citadel_identity::UpdateServiceAccount) -> Self {
        Self {
            description: value.description,
            is_enabled: value.is_enabled,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenameServiceAccountRequest {
    pub id: Uuid,
    pub name: String,
}

impl From<RenameServiceAccountRequest> for citadel_identity::RenameServiceAccount {
    fn from(value: RenameServiceAccountRequest) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<citadel_identity::RenameServiceAccount> for RenameServiceAccountRequest {
    fn from(value: citadel_identity::RenameServiceAccount) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddServiceAccountRoleRequest {
    pub role_id: Uuid,
}

impl From<AddServiceAccountRoleRequest> for citadel_identity::AddServiceAccountRole {
    fn from(value: AddServiceAccountRoleRequest) -> Self {
        Self {
            role_id: value.role_id,
        }
    }
}

impl From<citadel_identity::AddServiceAccountRole> for AddServiceAccountRoleRequest {
    fn from(value: citadel_identity::AddServiceAccountRole) -> Self {
        Self {
            role_id: value.role_id,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddServiceAccountResourceAccessRequest {
    pub resource_type: ResourceType,
    pub resource_id: Uuid,
    pub permission_level: PermissionLevel,
    #[serde(default)]
    pub specific_permissions: Vec<SpecificPermission>,
}

impl From<AddServiceAccountResourceAccessRequest>
    for citadel_identity::AddServiceAccountResourceAccess
{
    fn from(value: AddServiceAccountResourceAccessRequest) -> Self {
        Self {
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

impl From<citadel_identity::AddServiceAccountResourceAccess>
    for AddServiceAccountResourceAccessRequest
{
    fn from(value: citadel_identity::AddServiceAccountResourceAccess) -> Self {
        Self {
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveServiceAccountsRequest {
    pub ids: Vec<Uuid>,
}

impl From<ArchiveServiceAccountsRequest> for citadel_identity::ArchiveServiceAccounts {
    fn from(value: ArchiveServiceAccountsRequest) -> Self {
        Self { ids: value.ids }
    }
}

impl From<citadel_identity::ArchiveServiceAccounts> for ArchiveServiceAccountsRequest {
    fn from(value: citadel_identity::ArchiveServiceAccounts) -> Self {
        Self { ids: value.ids }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateServiceAccountTokenRequest {
    pub name: String,
    pub expires_at_utc: Option<DateTime<Utc>>,
    #[serde(default)]
    pub never_expires: bool,
}

impl From<CreateServiceAccountTokenRequest> for citadel_identity::CreateServiceAccountToken {
    fn from(value: CreateServiceAccountTokenRequest) -> Self {
        Self {
            name: value.name,
            expires_at_utc: value.expires_at_utc,
            never_expires: value.never_expires,
        }
    }
}

impl From<citadel_identity::CreateServiceAccountToken> for CreateServiceAccountTokenRequest {
    fn from(value: citadel_identity::CreateServiceAccountToken) -> Self {
        Self {
            name: value.name,
            expires_at_utc: value.expires_at_utc,
            never_expires: value.never_expires,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceAccountView {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub actor_id: ActorId,
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub created_by_actor_id: ActorId,
    pub updated_at: DateTime<Utc>,
    pub archived_at_utc: Option<DateTime<Utc>>,
    pub active_token_count: i64,
    pub last_used_at_utc: Option<DateTime<Utc>>,
    pub teams: Vec<ResourceInfo>,
    pub roles: Vec<ResourceInfo>,
    pub resource_accesses: Vec<ServiceAccountResourceAccess>,
}

impl From<ServiceAccountView> for citadel_identity::ServiceAccountDetails {
    fn from(value: ServiceAccountView) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            actor_id: value.actor_id,
            is_enabled: value.is_enabled,
            created_at: value.created_at,
            created_by_actor_id: value.created_by_actor_id,
            updated_at: value.updated_at,
            archived_at_utc: value.archived_at_utc,
            active_token_count: value.active_token_count,
            last_used_at_utc: value.last_used_at_utc,
            teams: value.teams.into_iter().map(|item| item.into()).collect(),
            roles: value.roles.into_iter().map(|item| item.into()).collect(),
            resource_accesses: value
                .resource_accesses
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

impl From<citadel_identity::ServiceAccountDetails> for ServiceAccountView {
    fn from(value: citadel_identity::ServiceAccountDetails) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            actor_id: value.actor_id,
            is_enabled: value.is_enabled,
            created_at: value.created_at,
            created_by_actor_id: value.created_by_actor_id,
            updated_at: value.updated_at,
            archived_at_utc: value.archived_at_utc,
            active_token_count: value.active_token_count,
            last_used_at_utc: value.last_used_at_utc,
            teams: value.teams.into_iter().map(|item| item.into()).collect(),
            roles: value.roles.into_iter().map(|item| item.into()).collect(),
            resource_accesses: value
                .resource_accesses
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunAsActorUsageView {
    pub id: Uuid,
    pub name: String,
    pub resource_type: ResourceType,
    pub is_active: bool,
}

impl From<RunAsActorUsageView> for citadel_identity::RunAsActorUsageDetails {
    fn from(value: RunAsActorUsageView) -> Self {
        Self {
            id: value.id,
            name: value.name,
            resource_type: value.resource_type,
            is_active: value.is_active,
        }
    }
}

impl From<citadel_identity::RunAsActorUsageDetails> for RunAsActorUsageView {
    fn from(value: citadel_identity::RunAsActorUsageDetails) -> Self {
        Self {
            id: value.id,
            name: value.name,
            resource_type: value.resource_type,
            is_active: value.is_active,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceAccountTokenView {
    pub id: Uuid,
    pub name: String,
    pub hint: String,
    pub expires_at_utc: Option<DateTime<Utc>>,
    pub last_used_at_utc: Option<DateTime<Utc>>,
    pub revoked_at_utc: Option<DateTime<Utc>>,
    pub revoked_by_actor_id: Option<ActorId>,
    pub created_by_actor_id: ActorId,
    pub created_by_name: String,
    pub created_at_utc: DateTime<Utc>,
}

impl From<ServiceAccountTokenView> for citadel_identity::ServiceAccountTokenDetails {
    fn from(value: ServiceAccountTokenView) -> Self {
        Self {
            id: value.id,
            name: value.name,
            hint: value.hint,
            expires_at_utc: value.expires_at_utc,
            last_used_at_utc: value.last_used_at_utc,
            revoked_at_utc: value.revoked_at_utc,
            revoked_by_actor_id: value.revoked_by_actor_id,
            created_by_actor_id: value.created_by_actor_id,
            created_by_name: value.created_by_name,
            created_at_utc: value.created_at_utc,
        }
    }
}

impl From<citadel_identity::ServiceAccountTokenDetails> for ServiceAccountTokenView {
    fn from(value: citadel_identity::ServiceAccountTokenDetails) -> Self {
        Self {
            id: value.id,
            name: value.name,
            hint: value.hint,
            expires_at_utc: value.expires_at_utc,
            last_used_at_utc: value.last_used_at_utc,
            revoked_at_utc: value.revoked_at_utc,
            revoked_by_actor_id: value.revoked_by_actor_id,
            created_by_actor_id: value.created_by_actor_id,
            created_by_name: value.created_by_name,
            created_at_utc: value.created_at_utc,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatedServiceAccountTokenView {
    #[serde(flatten)]
    pub credential: ServiceAccountTokenView,
    pub token: String,
}

impl From<CreatedServiceAccountTokenView> for citadel_identity::CreatedServiceAccountTokenDetails {
    fn from(value: CreatedServiceAccountTokenView) -> Self {
        Self {
            credential: value.credential.into(),
            token: value.token,
        }
    }
}

impl From<citadel_identity::CreatedServiceAccountTokenDetails> for CreatedServiceAccountTokenView {
    fn from(value: citadel_identity::CreatedServiceAccountTokenDetails) -> Self {
        Self {
            credential: value.credential.into(),
            token: value.token,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceAccountLimitsView {
    pub default_token_lifetime_days: i64,
    pub maximum_token_lifetime_days: i64,
    pub maximum_active_tokens_per_account: i64,
}

impl From<ServiceAccountLimitsView> for citadel_identity::ServiceAccountLimitsDetails {
    fn from(value: ServiceAccountLimitsView) -> Self {
        Self {
            default_token_lifetime_days: value.default_token_lifetime_days,
            maximum_token_lifetime_days: value.maximum_token_lifetime_days,
            maximum_active_tokens_per_account: value.maximum_active_tokens_per_account,
        }
    }
}

impl From<citadel_identity::ServiceAccountLimitsDetails> for ServiceAccountLimitsView {
    fn from(value: citadel_identity::ServiceAccountLimitsDetails) -> Self {
        Self {
            default_token_lifetime_days: value.default_token_lifetime_days,
            maximum_token_lifetime_days: value.maximum_token_lifetime_days,
            maximum_active_tokens_per_account: value.maximum_active_tokens_per_account,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub enum CurrentProfileAuthenticationType {
    Local,
    Oidc,
}

impl From<CurrentProfileAuthenticationType> for citadel_identity::CurrentProfileAuthenticationType {
    fn from(value: CurrentProfileAuthenticationType) -> Self {
        match value {
            CurrentProfileAuthenticationType::Local => Self::Local,
            CurrentProfileAuthenticationType::Oidc => Self::Oidc,
        }
    }
}

impl From<citadel_identity::CurrentProfileAuthenticationType> for CurrentProfileAuthenticationType {
    fn from(value: citadel_identity::CurrentProfileAuthenticationType) -> Self {
        match value {
            citadel_identity::CurrentProfileAuthenticationType::Local => Self::Local,
            citadel_identity::CurrentProfileAuthenticationType::Oidc => Self::Oidc,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCurrentProfileRequest {
    pub display_name: String,
}

impl From<UpdateCurrentProfileRequest> for citadel_identity::UpdateCurrentProfile {
    fn from(value: UpdateCurrentProfileRequest) -> Self {
        Self {
            display_name: value.display_name,
        }
    }
}

impl From<citadel_identity::UpdateCurrentProfile> for UpdateCurrentProfileRequest {
    fn from(value: citadel_identity::UpdateCurrentProfile) -> Self {
        Self {
            display_name: value.display_name,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PatchUserPreferencesRequest {
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub time_zone: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<crate::api::vocabulary::UserDateTimeFormatSchema>, required = false)]
    pub date_time_format: PatchField<UserDateTimeFormat>,
    #[serde(default)]
    #[schema(value_type = Option<crate::api::vocabulary::UserThemeSchema>, required = false)]
    pub theme: PatchField<UserTheme>,
}

impl From<PatchUserPreferencesRequest> for citadel_identity::PatchUserPreferences {
    fn from(value: PatchUserPreferencesRequest) -> Self {
        Self {
            time_zone: value.time_zone,
            date_time_format: value.date_time_format,
            theme: value.theme,
        }
    }
}

impl From<citadel_identity::PatchUserPreferences> for PatchUserPreferencesRequest {
    fn from(value: citadel_identity::PatchUserPreferences) -> Self {
        Self {
            time_zone: value.time_zone,
            date_time_format: value.date_time_format,
            theme: value.theme,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChangeCurrentPasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

impl From<ChangeCurrentPasswordRequest> for citadel_identity::ChangeCurrentPassword {
    fn from(value: ChangeCurrentPasswordRequest) -> Self {
        Self {
            current_password: value.current_password,
            new_password: value.new_password,
        }
    }
}

impl From<citadel_identity::ChangeCurrentPassword> for ChangeCurrentPasswordRequest {
    fn from(value: citadel_identity::ChangeCurrentPassword) -> Self {
        Self {
            current_password: value.current_password,
            new_password: value.new_password,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProfileResourceInfo {
    pub id: Uuid,
    pub name: String,
}

impl From<ProfileResourceInfo> for citadel_identity::ProfileResourceInfo {
    fn from(value: ProfileResourceInfo) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<citadel_identity::ProfileResourceInfo> for ProfileResourceInfo {
    fn from(value: citadel_identity::ProfileResourceInfo) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CurrentProfileAuthenticationView {
    pub r#type: CurrentProfileAuthenticationType,
    pub label: String,
    pub can_change_password: bool,
    pub can_use_local_password_mfa: bool,
    pub oidc_provider_id: Option<Uuid>,
    pub oidc_provider_name: Option<String>,
}

impl From<CurrentProfileAuthenticationView>
    for citadel_identity::CurrentProfileAuthenticationDetails
{
    fn from(value: CurrentProfileAuthenticationView) -> Self {
        Self {
            r#type: value.r#type.into(),
            label: value.label,
            can_change_password: value.can_change_password,
            can_use_local_password_mfa: value.can_use_local_password_mfa,
            oidc_provider_id: value.oidc_provider_id,
            oidc_provider_name: value.oidc_provider_name,
        }
    }
}

impl From<citadel_identity::CurrentProfileAuthenticationDetails>
    for CurrentProfileAuthenticationView
{
    fn from(value: citadel_identity::CurrentProfileAuthenticationDetails) -> Self {
        Self {
            r#type: value.r#type.into(),
            label: value.label,
            can_change_password: value.can_change_password,
            can_use_local_password_mfa: value.can_use_local_password_mfa,
            oidc_provider_id: value.oidc_provider_id,
            oidc_provider_name: value.oidc_provider_name,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[schema(as = identity::application::profile::ResourceCapabilities)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCapabilities {
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
}

impl From<ResourceCapabilities> for citadel_identity::ResourceCapabilities {
    fn from(value: ResourceCapabilities) -> Self {
        Self {
            can_read: value.can_read,
            can_write: value.can_write,
            can_execute: value.can_execute,
        }
    }
}

impl From<citadel_identity::ResourceCapabilities> for ResourceCapabilities {
    fn from(value: citadel_identity::ResourceCapabilities) -> Self {
        Self {
            can_read: value.can_read,
            can_write: value.can_write,
            can_execute: value.can_execute,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CurrentProfileAuthorizationView {
    pub is_administrator: bool,
    pub alert_rules: ResourceCapabilities,
    pub bindings: ResourceCapabilities,
    pub tags: ResourceCapabilities,
}

impl From<CurrentProfileAuthorizationView>
    for citadel_identity::CurrentProfileAuthorizationDetails
{
    fn from(value: CurrentProfileAuthorizationView) -> Self {
        Self {
            is_administrator: value.is_administrator,
            alert_rules: value.alert_rules.into(),
            bindings: value.bindings.into(),
            tags: value.tags.into(),
        }
    }
}

impl From<citadel_identity::CurrentProfileAuthorizationDetails>
    for CurrentProfileAuthorizationView
{
    fn from(value: citadel_identity::CurrentProfileAuthorizationDetails) -> Self {
        Self {
            is_administrator: value.is_administrator,
            alert_rules: value.alert_rules.into(),
            bindings: value.bindings.into(),
            tags: value.tags.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CurrentProfileView {
    pub id: Uuid,
    pub display_name: String,
    pub email: String,
    pub authentication: CurrentProfileAuthenticationView,
    pub authorization: CurrentProfileAuthorizationView,
    pub created_at: DateTime<Utc>,
    pub direct_roles: Vec<ProfileResourceInfo>,
    pub teams: Vec<ProfileResourceInfo>,
}

impl From<CurrentProfileView> for citadel_identity::CurrentProfileDetails {
    fn from(value: CurrentProfileView) -> Self {
        Self {
            id: value.id,
            display_name: value.display_name,
            email: value.email,
            authentication: value.authentication.into(),
            authorization: value.authorization.into(),
            created_at: value.created_at,
            direct_roles: value
                .direct_roles
                .into_iter()
                .map(|item| item.into())
                .collect(),
            teams: value.teams.into_iter().map(|item| item.into()).collect(),
        }
    }
}

impl From<citadel_identity::CurrentProfileDetails> for CurrentProfileView {
    fn from(value: citadel_identity::CurrentProfileDetails) -> Self {
        Self {
            id: value.id,
            display_name: value.display_name,
            email: value.email,
            authentication: value.authentication.into(),
            authorization: value.authorization.into(),
            created_at: value.created_at,
            direct_roles: value
                .direct_roles
                .into_iter()
                .map(|item| item.into())
                .collect(),
            teams: value.teams.into_iter().map(|item| item.into()).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserPreferencesView {
    pub time_zone: Option<String>,
    #[schema(value_type = crate::api::vocabulary::UserDateTimeFormatSchema)]
    pub date_time_format: UserDateTimeFormat,
    #[schema(value_type = crate::api::vocabulary::UserThemeSchema)]
    pub theme: UserTheme,
    pub is_persisted: bool,
}

impl From<UserPreferencesView> for citadel_identity::UserPreferencesDetails {
    fn from(value: UserPreferencesView) -> Self {
        Self {
            time_zone: value.time_zone,
            date_time_format: value.date_time_format,
            theme: value.theme,
            is_persisted: value.is_persisted,
        }
    }
}

impl From<citadel_identity::UserPreferencesDetails> for UserPreferencesView {
    fn from(value: citadel_identity::UserPreferencesDetails) -> Self {
        Self {
            time_zone: value.time_zone,
            date_time_format: value.date_time_format,
            theme: value.theme,
            is_persisted: value.is_persisted,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserSessionSummaryView {
    pub id: Uuid,
    pub display_name: String,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub is_current: bool,
}

impl From<UserSessionSummaryView> for citadel_identity::UserSessionSummaryDetails {
    fn from(value: UserSessionSummaryView) -> Self {
        Self {
            id: value.id,
            display_name: value.display_name,
            user_agent: value.user_agent,
            ip_address: value.ip_address,
            created_at: value.created_at,
            last_seen_at: value.last_seen_at,
            expires_at: value.expires_at,
            is_current: value.is_current,
        }
    }
}

impl From<citadel_identity::UserSessionSummaryDetails> for UserSessionSummaryView {
    fn from(value: citadel_identity::UserSessionSummaryDetails) -> Self {
        Self {
            id: value.id,
            display_name: value.display_name,
            user_agent: value.user_agent,
            ip_address: value.ip_address,
            created_at: value.created_at,
            last_seen_at: value.last_seen_at,
            expires_at: value.expires_at,
            is_current: value.is_current,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserSessionsView {
    pub sessions: Vec<UserSessionSummaryView>,
    pub can_revoke_other_sessions: bool,
}

impl From<UserSessionsView> for citadel_identity::UserSessionsDetails {
    fn from(value: UserSessionsView) -> Self {
        Self {
            sessions: value.sessions.into_iter().map(|item| item.into()).collect(),
            can_revoke_other_sessions: value.can_revoke_other_sessions,
        }
    }
}

impl From<citadel_identity::UserSessionsDetails> for UserSessionsView {
    fn from(value: citadel_identity::UserSessionsDetails) -> Self {
        Self {
            sessions: value.sessions.into_iter().map(|item| item.into()).collect(),
            can_revoke_other_sessions: value.can_revoke_other_sessions,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct RevokeOtherProfileSessionsView {
    pub count: i64,
}

impl From<RevokeOtherProfileSessionsView> for citadel_identity::RevokeOtherProfileSessionsDetails {
    fn from(value: RevokeOtherProfileSessionsView) -> Self {
        Self { count: value.count }
    }
}

impl From<citadel_identity::RevokeOtherProfileSessionsDetails> for RevokeOtherProfileSessionsView {
    fn from(value: citadel_identity::RevokeOtherProfileSessionsDetails) -> Self {
        Self { count: value.count }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserResourceAccessInput {
    pub resource_type: ResourceType,
    pub resource_id: Uuid,
    pub permission_level: PermissionLevel,
    #[serde(default)]
    pub specific_permissions: Vec<SpecificPermission>,
}

impl From<UserResourceAccessInput> for citadel_identity::UserResourceAccessInput {
    fn from(value: UserResourceAccessInput) -> Self {
        Self {
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

impl From<citadel_identity::UserResourceAccessInput> for UserResourceAccessInput {
    fn from(value: citadel_identity::UserResourceAccessInput) -> Self {
        Self {
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
    pub password: String,
    #[serde(default = "enabled_by_default")]
    pub is_enabled: bool,
    #[serde(default)]
    pub team_ids: Vec<Uuid>,
    #[serde(default)]
    pub role_ids: Vec<Uuid>,
    #[serde(default)]
    pub resource_accesses: Vec<UserResourceAccessInput>,
}

impl From<CreateUserRequest> for citadel_identity::CreateUser {
    fn from(value: CreateUserRequest) -> Self {
        Self {
            name: value.name,
            email: value.email,
            password: value.password,
            is_enabled: value.is_enabled,
            team_ids: value.team_ids,
            role_ids: value.role_ids,
            resource_accesses: value
                .resource_accesses
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

impl From<citadel_identity::CreateUser> for CreateUserRequest {
    fn from(value: citadel_identity::CreateUser) -> Self {
        Self {
            name: value.name,
            email: value.email,
            password: value.password,
            is_enabled: value.is_enabled,
            team_ids: value.team_ids,
            role_ids: value.role_ids,
            resource_accesses: value
                .resource_accesses
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PatchUserRequest {
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub email: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub password: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<bool>, required = false)]
    pub is_enabled: PatchField<bool>,
    #[serde(default)]
    #[schema(value_type = Option<Vec<Uuid>>, required = false)]
    pub team_ids: PatchField<Vec<Uuid>>,
    #[serde(default)]
    #[schema(value_type = Option<Vec<Uuid>>, required = false)]
    pub role_ids: PatchField<Vec<Uuid>>,
    #[serde(default)]
    #[schema(value_type = Option<Vec<UserResourceAccessInput>>, required = false)]
    pub resource_accesses: PatchField<Vec<UserResourceAccessInput>>,
}

impl From<PatchUserRequest> for citadel_identity::PatchUser {
    fn from(value: PatchUserRequest) -> Self {
        Self {
            email: value.email,
            password: value.password,
            is_enabled: value.is_enabled,
            team_ids: value.team_ids,
            role_ids: value.role_ids,
            resource_accesses: value
                .resource_accesses
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
        }
    }
}

impl From<citadel_identity::PatchUser> for PatchUserRequest {
    fn from(value: citadel_identity::PatchUser) -> Self {
        Self {
            email: value.email,
            password: value.password,
            is_enabled: value.is_enabled,
            team_ids: value.team_ids,
            role_ids: value.role_ids,
            resource_accesses: value
                .resource_accesses
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenameUserRequest {
    pub id: Uuid,
    pub name: String,
}

impl From<RenameUserRequest> for citadel_identity::RenameUser {
    fn from(value: RenameUserRequest) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<citadel_identity::RenameUser> for RenameUserRequest {
    fn from(value: citadel_identity::RenameUser) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddUserRoleRequest {
    pub role_id: Uuid,
}

impl From<AddUserRoleRequest> for citadel_identity::AddUserRole {
    fn from(value: AddUserRoleRequest) -> Self {
        Self {
            role_id: value.role_id,
        }
    }
}

impl From<citadel_identity::AddUserRole> for AddUserRoleRequest {
    fn from(value: citadel_identity::AddUserRole) -> Self {
        Self {
            role_id: value.role_id,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserResourceAccessRequest {
    pub resource_type: ResourceType,
    pub resource_id: Uuid,
    pub permission_level: PermissionLevel,
    #[serde(default)]
    pub specific_permissions: Vec<SpecificPermission>,
}

impl From<UserResourceAccessRequest> for citadel_identity::UserResourceAccess {
    fn from(value: UserResourceAccessRequest) -> Self {
        Self {
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

impl From<citadel_identity::UserResourceAccess> for UserResourceAccessRequest {
    fn from(value: citadel_identity::UserResourceAccess) -> Self {
        Self {
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteUsersRequest {
    pub ids: Vec<Uuid>,
}

impl From<DeleteUsersRequest> for citadel_identity::DeleteUsers {
    fn from(value: DeleteUsersRequest) -> Self {
        Self { ids: value.ids }
    }
}

impl From<citadel_identity::DeleteUsers> for DeleteUsersRequest {
    fn from(value: citadel_identity::DeleteUsers) -> Self {
        Self { ids: value.ids }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserView {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub actor_id: ActorId,
    pub is_enabled: bool,
    pub teams: Option<Vec<ResourceInfo>>,
    pub roles: Option<Vec<ResourceInfo>>,
    pub resource_accesses: Option<Vec<UserResourceAccessView>>,
}

impl From<UserView> for citadel_identity::UserDetails {
    fn from(value: UserView) -> Self {
        Self {
            id: value.id,
            name: value.name,
            email: value.email,
            actor_id: value.actor_id,
            is_enabled: value.is_enabled,
            teams: value
                .teams
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
            roles: value
                .roles
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
            resource_accesses: value
                .resource_accesses
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
        }
    }
}

impl From<citadel_identity::UserDetails> for UserView {
    fn from(value: citadel_identity::UserDetails) -> Self {
        Self {
            id: value.id,
            name: value.name,
            email: value.email,
            actor_id: value.actor_id,
            is_enabled: value.is_enabled,
            teams: value
                .teams
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
            roles: value
                .roles
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
            resource_accesses: value
                .resource_accesses
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSearchItemView {
    pub id: Uuid,
    pub name: String,
    pub email: String,
}

impl From<UserSearchItemView> for citadel_identity::UserSearchItemDetails {
    fn from(value: UserSearchItemView) -> Self {
        Self {
            id: value.id,
            name: value.name,
            email: value.email,
        }
    }
}

impl From<citadel_identity::UserSearchItemDetails> for UserSearchItemView {
    fn from(value: citadel_identity::UserSearchItemDetails) -> Self {
        Self {
            id: value.id,
            name: value.name,
            email: value.email,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserResourceAccessView {
    pub resource_type: ResourceType,
    pub resource_id: Uuid,
    pub resource_name: Option<String>,
    pub permission_level: PermissionLevel,
    pub specific_permissions: Option<Vec<SpecificPermission>>,
    pub id: Option<Uuid>,
}

impl From<UserResourceAccessView> for citadel_identity::UserResourceAccessDetails {
    fn from(value: UserResourceAccessView) -> Self {
        Self {
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            resource_name: value.resource_name,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
            id: value.id,
        }
    }
}

impl From<citadel_identity::UserResourceAccessDetails> for UserResourceAccessView {
    fn from(value: citadel_identity::UserResourceAccessDetails) -> Self {
        Self {
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            resource_name: value.resource_name,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
            id: value.id,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateOidcProviderRequest {
    pub name: String,
    pub description: Option<String>,
    pub display_name: String,
    pub issuer: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub scopes: Option<String>,
    pub enabled: bool,
    pub auto_provision_users: bool,
    pub allow_email_auto_link: bool,
    pub require_email_verified: bool,
    pub allowed_email_domains: Option<String>,
    pub required_claim_name: Option<String>,
    pub required_claim_values: Option<String>,
    pub default_role_id: Option<Uuid>,
}

impl From<CreateOidcProviderRequest> for citadel_identity::CreateOidcProvider {
    fn from(value: CreateOidcProviderRequest) -> Self {
        Self {
            name: value.name,
            description: value.description,
            display_name: value.display_name,
            issuer: value.issuer,
            client_id: value.client_id,
            client_secret: value.client_secret,
            scopes: value.scopes,
            enabled: value.enabled,
            auto_provision_users: value.auto_provision_users,
            allow_email_auto_link: value.allow_email_auto_link,
            require_email_verified: value.require_email_verified,
            allowed_email_domains: value.allowed_email_domains,
            required_claim_name: value.required_claim_name,
            required_claim_values: value.required_claim_values,
            default_role_id: value.default_role_id,
        }
    }
}

impl From<citadel_identity::CreateOidcProvider> for CreateOidcProviderRequest {
    fn from(value: citadel_identity::CreateOidcProvider) -> Self {
        Self {
            name: value.name,
            description: value.description,
            display_name: value.display_name,
            issuer: value.issuer,
            client_id: value.client_id,
            client_secret: value.client_secret,
            scopes: value.scopes,
            enabled: value.enabled,
            auto_provision_users: value.auto_provision_users,
            allow_email_auto_link: value.allow_email_auto_link,
            require_email_verified: value.require_email_verified,
            allowed_email_domains: value.allowed_email_domains,
            required_claim_name: value.required_claim_name,
            required_claim_values: value.required_claim_values,
            default_role_id: value.default_role_id,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PatchOidcProviderRequest {
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub name: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub description: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub display_name: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub issuer: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub client_id: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub client_secret: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub scopes: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<bool>, required = false)]
    pub enabled: PatchField<bool>,
    #[serde(default)]
    #[schema(value_type = Option<bool>, required = false)]
    pub auto_provision_users: PatchField<bool>,
    #[serde(default)]
    #[schema(value_type = Option<bool>, required = false)]
    pub allow_email_auto_link: PatchField<bool>,
    #[serde(default)]
    #[schema(value_type = Option<bool>, required = false)]
    pub require_email_verified: PatchField<bool>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub allowed_email_domains: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub required_claim_name: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub required_claim_values: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<Uuid>, required = false)]
    pub default_role_id: PatchField<Uuid>,
}

impl From<PatchOidcProviderRequest> for citadel_identity::PatchOidcProvider {
    fn from(value: PatchOidcProviderRequest) -> Self {
        Self {
            name: value.name,
            description: value.description,
            display_name: value.display_name,
            issuer: value.issuer,
            client_id: value.client_id,
            client_secret: value.client_secret,
            scopes: value.scopes,
            enabled: value.enabled,
            auto_provision_users: value.auto_provision_users,
            allow_email_auto_link: value.allow_email_auto_link,
            require_email_verified: value.require_email_verified,
            allowed_email_domains: value.allowed_email_domains,
            required_claim_name: value.required_claim_name,
            required_claim_values: value.required_claim_values,
            default_role_id: value.default_role_id,
        }
    }
}

impl From<citadel_identity::PatchOidcProvider> for PatchOidcProviderRequest {
    fn from(value: citadel_identity::PatchOidcProvider) -> Self {
        Self {
            name: value.name,
            description: value.description,
            display_name: value.display_name,
            issuer: value.issuer,
            client_id: value.client_id,
            client_secret: value.client_secret,
            scopes: value.scopes,
            enabled: value.enabled,
            auto_provision_users: value.auto_provision_users,
            allow_email_auto_link: value.allow_email_auto_link,
            require_email_verified: value.require_email_verified,
            allowed_email_domains: value.allowed_email_domains,
            required_claim_name: value.required_claim_name,
            required_claim_values: value.required_claim_values,
            default_role_id: value.default_role_id,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenameOidcProviderRequest {
    pub id: Uuid,
    pub name: String,
}

impl From<RenameOidcProviderRequest> for citadel_identity::RenameOidcProvider {
    fn from(value: RenameOidcProviderRequest) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<citadel_identity::RenameOidcProvider> for RenameOidcProviderRequest {
    fn from(value: citadel_identity::RenameOidcProvider) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PatchOidcProviderMetadataRequest {
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub description: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<Vec<String>>, required = false)]
    pub tags: PatchField<Vec<String>>,
}

impl From<PatchOidcProviderMetadataRequest> for citadel_identity::PatchOidcProviderMetadata {
    fn from(value: PatchOidcProviderMetadataRequest) -> Self {
        Self {
            description: value.description,
            tags: value.tags,
        }
    }
}

impl From<citadel_identity::PatchOidcProviderMetadata> for PatchOidcProviderMetadataRequest {
    fn from(value: citadel_identity::PatchOidcProviderMetadata) -> Self {
        Self {
            description: value.description,
            tags: value.tags,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TestOidcDiscoveryRequest {
    pub provider_id: Option<Uuid>,
    pub issuer: Option<String>,
}

impl From<TestOidcDiscoveryRequest> for citadel_identity::TestOidcDiscovery {
    fn from(value: TestOidcDiscoveryRequest) -> Self {
        Self {
            provider_id: value.provider_id,
            issuer: value.issuer,
        }
    }
}

impl From<citadel_identity::TestOidcDiscovery> for TestOidcDiscoveryRequest {
    fn from(value: citadel_identity::TestOidcDiscovery) -> Self {
        Self {
            provider_id: value.provider_id,
            issuer: value.issuer,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OidcProviderView {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub display_name: String,
    pub issuer: String,
    pub client_id: String,
    pub scopes: String,
    pub enabled: bool,
    pub auto_provision_users: bool,
    pub allow_email_auto_link: bool,
    pub require_email_verified: bool,
    pub allowed_email_domains: Option<String>,
    pub required_claim_name: Option<String>,
    pub required_claim_values: Option<String>,
    pub default_role_id: Option<Uuid>,
    pub has_client_secret: bool,
    pub created_by_actor_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<OidcProviderView> for citadel_identity::OidcProviderDetails {
    fn from(value: OidcProviderView) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            display_name: value.display_name,
            issuer: value.issuer,
            client_id: value.client_id,
            scopes: value.scopes,
            enabled: value.enabled,
            auto_provision_users: value.auto_provision_users,
            allow_email_auto_link: value.allow_email_auto_link,
            require_email_verified: value.require_email_verified,
            allowed_email_domains: value.allowed_email_domains,
            required_claim_name: value.required_claim_name,
            required_claim_values: value.required_claim_values,
            default_role_id: value.default_role_id,
            has_client_secret: value.has_client_secret,
            created_by_actor_id: value.created_by_actor_id,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<citadel_identity::OidcProviderDetails> for OidcProviderView {
    fn from(value: citadel_identity::OidcProviderDetails) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            display_name: value.display_name,
            issuer: value.issuer,
            client_id: value.client_id,
            scopes: value.scopes,
            enabled: value.enabled,
            auto_provision_users: value.auto_provision_users,
            allow_email_auto_link: value.allow_email_auto_link,
            require_email_verified: value.require_email_verified,
            allowed_email_domains: value.allowed_email_domains,
            required_claim_name: value.required_claim_name,
            required_claim_values: value.required_claim_values,
            default_role_id: value.default_role_id,
            has_client_secret: value.has_client_secret,
            created_by_actor_id: value.created_by_actor_id,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OidcProvidersView {
    pub providers: Vec<OidcProviderView>,
}

impl From<OidcProvidersView> for citadel_identity::OidcProvidersDetails {
    fn from(value: OidcProvidersView) -> Self {
        Self {
            providers: value
                .providers
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

impl From<citadel_identity::OidcProvidersDetails> for OidcProvidersView {
    fn from(value: citadel_identity::OidcProvidersDetails) -> Self {
        Self {
            providers: value
                .providers
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OidcLoginProviderView {
    pub id: Uuid,
    pub display_name: String,
}

impl From<OidcLoginProviderView> for citadel_identity::OidcLoginProviderDetails {
    fn from(value: OidcLoginProviderView) -> Self {
        Self {
            id: value.id,
            display_name: value.display_name,
        }
    }
}

impl From<citadel_identity::OidcLoginProviderDetails> for OidcLoginProviderView {
    fn from(value: citadel_identity::OidcLoginProviderDetails) -> Self {
        Self {
            id: value.id,
            display_name: value.display_name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OidcLoginProvidersView {
    pub providers: Vec<OidcLoginProviderView>,
}

impl From<OidcLoginProvidersView> for citadel_identity::OidcLoginProvidersDetails {
    fn from(value: OidcLoginProvidersView) -> Self {
        Self {
            providers: value
                .providers
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

impl From<citadel_identity::OidcLoginProvidersDetails> for OidcLoginProvidersView {
    fn from(value: citadel_identity::OidcLoginProvidersDetails) -> Self {
        Self {
            providers: value
                .providers
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OidcDiscoveryView {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub jwks_uri: String,
}

impl From<OidcDiscoveryView> for citadel_identity::OidcDiscoveryDetails {
    fn from(value: OidcDiscoveryView) -> Self {
        Self {
            issuer: value.issuer,
            authorization_endpoint: value.authorization_endpoint,
            token_endpoint: value.token_endpoint,
            jwks_uri: value.jwks_uri,
        }
    }
}

impl From<citadel_identity::OidcDiscoveryDetails> for OidcDiscoveryView {
    fn from(value: citadel_identity::OidcDiscoveryDetails) -> Self {
        Self {
            issuer: value.issuer,
            authorization_endpoint: value.authorization_endpoint,
            token_endpoint: value.token_endpoint,
            jwks_uri: value.jwks_uri,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamResourceAccessInput {
    pub resource_type: ResourceType,
    pub resource_id: Uuid,
    pub permission_level: PermissionLevel,
    #[serde(default)]
    pub specific_permissions: Vec<SpecificPermission>,
}

impl From<TeamResourceAccessInput> for citadel_identity::TeamResourceAccessInput {
    fn from(value: TeamResourceAccessInput) -> Self {
        Self {
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

impl From<citadel_identity::TeamResourceAccessInput> for TeamResourceAccessInput {
    fn from(value: citadel_identity::TeamResourceAccessInput) -> Self {
        Self {
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTeamRequest {
    pub name: String,
    #[serde(default)]
    pub user_ids: Vec<Uuid>,
    #[serde(default)]
    pub role_ids: Vec<Uuid>,
    #[serde(default)]
    pub resource_accesses: Vec<TeamResourceAccessInput>,
}

impl From<CreateTeamRequest> for citadel_identity::CreateTeam {
    fn from(value: CreateTeamRequest) -> Self {
        Self {
            name: value.name,
            user_ids: value.user_ids,
            role_ids: value.role_ids,
            resource_accesses: value
                .resource_accesses
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

impl From<citadel_identity::CreateTeam> for CreateTeamRequest {
    fn from(value: citadel_identity::CreateTeam) -> Self {
        Self {
            name: value.name,
            user_ids: value.user_ids,
            role_ids: value.role_ids,
            resource_accesses: value
                .resource_accesses
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PatchTeamRequest {
    #[serde(default)]
    #[schema(value_type = Option<bool>, required = false)]
    pub is_enabled: PatchField<bool>,
    #[serde(default)]
    #[schema(value_type = Option<Vec<Uuid>>, required = false)]
    pub user_ids: PatchField<Vec<Uuid>>,
    #[serde(default)]
    #[schema(value_type = Option<Vec<Uuid>>, required = false)]
    pub role_ids: PatchField<Vec<Uuid>>,
    #[serde(default)]
    #[schema(value_type = Option<Vec<TeamResourceAccessInput>>, required = false)]
    pub resource_accesses: PatchField<Vec<TeamResourceAccessInput>>,
}

impl From<PatchTeamRequest> for citadel_identity::PatchTeam {
    fn from(value: PatchTeamRequest) -> Self {
        Self {
            is_enabled: value.is_enabled,
            user_ids: value.user_ids,
            role_ids: value.role_ids,
            resource_accesses: value
                .resource_accesses
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
        }
    }
}

impl From<citadel_identity::PatchTeam> for PatchTeamRequest {
    fn from(value: citadel_identity::PatchTeam) -> Self {
        Self {
            is_enabled: value.is_enabled,
            user_ids: value.user_ids,
            role_ids: value.role_ids,
            resource_accesses: value
                .resource_accesses
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenameTeamRequest {
    pub id: Uuid,
    pub name: String,
}

impl From<RenameTeamRequest> for citadel_identity::RenameTeam {
    fn from(value: RenameTeamRequest) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<citadel_identity::RenameTeam> for RenameTeamRequest {
    fn from(value: citadel_identity::RenameTeam) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddTeamRoleRequest {
    pub role_id: Uuid,
}

impl From<AddTeamRoleRequest> for citadel_identity::AddTeamRole {
    fn from(value: AddTeamRoleRequest) -> Self {
        Self {
            role_id: value.role_id,
        }
    }
}

impl From<citadel_identity::AddTeamRole> for AddTeamRoleRequest {
    fn from(value: citadel_identity::AddTeamRole) -> Self {
        Self {
            role_id: value.role_id,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddTeamMemberRequest {
    pub member_actor_id: Uuid,
}

impl From<AddTeamMemberRequest> for citadel_identity::AddTeamMember {
    fn from(value: AddTeamMemberRequest) -> Self {
        Self {
            member_actor_id: value.member_actor_id,
        }
    }
}

impl From<citadel_identity::AddTeamMember> for AddTeamMemberRequest {
    fn from(value: citadel_identity::AddTeamMember) -> Self {
        Self {
            member_actor_id: value.member_actor_id,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTeamsRequest {
    pub ids: Vec<Uuid>,
}

impl From<DeleteTeamsRequest> for citadel_identity::DeleteTeams {
    fn from(value: DeleteTeamsRequest) -> Self {
        Self { ids: value.ids }
    }
}

impl From<citadel_identity::DeleteTeams> for DeleteTeamsRequest {
    fn from(value: citadel_identity::DeleteTeams) -> Self {
        Self { ids: value.ids }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamView {
    pub id: Uuid,
    pub name: String,
    pub actor_id: ActorId,
    pub is_enabled: bool,
    pub total_members: i32,
    pub users: Option<Vec<ResourceInfo>>,
    pub roles: Option<Vec<ResourceInfo>>,
    pub resource_accesses: Option<Vec<TeamResourceAccessView>>,
    pub members: Option<Vec<TeamMemberView>>,
}

impl From<TeamView> for citadel_identity::TeamDetails {
    fn from(value: TeamView) -> Self {
        Self {
            id: value.id,
            name: value.name,
            actor_id: value.actor_id,
            is_enabled: value.is_enabled,
            total_members: value.total_members,
            users: value
                .users
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
            roles: value
                .roles
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
            resource_accesses: value
                .resource_accesses
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
            members: value
                .members
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
        }
    }
}

impl From<citadel_identity::TeamDetails> for TeamView {
    fn from(value: citadel_identity::TeamDetails) -> Self {
        Self {
            id: value.id,
            name: value.name,
            actor_id: value.actor_id,
            is_enabled: value.is_enabled,
            total_members: value.total_members,
            users: value
                .users
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
            roles: value
                .roles
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
            resource_accesses: value
                .resource_accesses
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
            members: value
                .members
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamMemberView {
    pub actor_id: ActorId,
    pub resource_id: Uuid,
    pub name: String,
    #[schema(value_type = crate::api::vocabulary::ActorTypeSchema)]
    pub principal_type: ActorType,
}

impl From<TeamMemberView> for citadel_identity::TeamMemberDetails {
    fn from(value: TeamMemberView) -> Self {
        Self {
            actor_id: value.actor_id,
            resource_id: value.resource_id,
            name: value.name,
            principal_type: value.principal_type,
        }
    }
}

impl From<citadel_identity::TeamMemberDetails> for TeamMemberView {
    fn from(value: citadel_identity::TeamMemberDetails) -> Self {
        Self {
            actor_id: value.actor_id,
            resource_id: value.resource_id,
            name: value.name,
            principal_type: value.principal_type,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamSearchItemView {
    pub id: Uuid,
    pub name: String,
}

impl From<TeamSearchItemView> for citadel_identity::TeamSearchItemDetails {
    fn from(value: TeamSearchItemView) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<citadel_identity::TeamSearchItemDetails> for TeamSearchItemView {
    fn from(value: citadel_identity::TeamSearchItemDetails) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamResourceAccessView {
    pub resource_type: ResourceType,
    pub resource_id: Uuid,
    pub resource_name: Option<String>,
    pub permission_level: PermissionLevel,
    pub specific_permissions: Option<Vec<SpecificPermission>>,
    pub id: Option<Uuid>,
}

impl From<TeamResourceAccessView> for citadel_identity::TeamResourceAccessDetails {
    fn from(value: TeamResourceAccessView) -> Self {
        Self {
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            resource_name: value.resource_name,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
            id: value.id,
        }
    }
}

impl From<citadel_identity::TeamResourceAccessDetails> for TeamResourceAccessView {
    fn from(value: citadel_identity::TeamResourceAccessDetails) -> Self {
        Self {
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            resource_name: value.resource_name,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
            id: value.id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RolePermissionInput {
    pub resource_type: ResourceType,
    pub permission_level: PermissionLevel,
    pub specific_permissions: Option<Vec<SpecificPermission>>,
}

impl From<RolePermissionInput> for citadel_identity::RolePermissionInput {
    fn from(value: RolePermissionInput) -> Self {
        Self {
            resource_type: value.resource_type,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

impl From<citadel_identity::RolePermissionInput> for RolePermissionInput {
    fn from(value: citadel_identity::RolePermissionInput) -> Self {
        Self {
            resource_type: value.resource_type,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateRoleRequest {
    pub name: String,
    pub permissions: Option<Vec<RolePermissionInput>>,
}

impl From<CreateRoleRequest> for citadel_identity::CreateRole {
    fn from(value: CreateRoleRequest) -> Self {
        Self {
            name: value.name,
            permissions: value
                .permissions
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
        }
    }
}

impl From<citadel_identity::CreateRole> for CreateRoleRequest {
    fn from(value: citadel_identity::CreateRole) -> Self {
        Self {
            name: value.name,
            permissions: value
                .permissions
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PatchRolePermissionsRequest {
    #[serde(default)]
    #[schema(value_type = Option<Vec<RolePermissionInput>>, required = false)]
    pub permissions: PatchField<Vec<RolePermissionInput>>,
}

impl From<PatchRolePermissionsRequest> for citadel_identity::PatchRolePermissions {
    fn from(value: PatchRolePermissionsRequest) -> Self {
        Self {
            permissions: value
                .permissions
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
        }
    }
}

impl From<citadel_identity::PatchRolePermissions> for PatchRolePermissionsRequest {
    fn from(value: citadel_identity::PatchRolePermissions) -> Self {
        Self {
            permissions: value
                .permissions
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenameRoleRequest {
    pub id: Uuid,
    pub name: String,
}

impl From<RenameRoleRequest> for citadel_identity::RenameRole {
    fn from(value: RenameRoleRequest) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<citadel_identity::RenameRole> for RenameRoleRequest {
    fn from(value: citadel_identity::RenameRole) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteRolesRequest {
    pub ids: Vec<Uuid>,
}

impl From<DeleteRolesRequest> for citadel_identity::DeleteRoles {
    fn from(value: DeleteRolesRequest) -> Self {
        Self { ids: value.ids }
    }
}

impl From<citadel_identity::DeleteRoles> for DeleteRolesRequest {
    fn from(value: citadel_identity::DeleteRoles) -> Self {
        Self { ids: value.ids }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RolePermissionView {
    pub resource_type: ResourceType,
    pub permission_level: PermissionLevel,
    #[serde(default)]
    pub specific_permissions: Vec<SpecificPermission>,
}

impl From<RolePermissionView> for citadel_identity::RolePermissionDetails {
    fn from(value: RolePermissionView) -> Self {
        Self {
            resource_type: value.resource_type,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

impl From<citadel_identity::RolePermissionDetails> for RolePermissionView {
    fn from(value: citadel_identity::RolePermissionDetails) -> Self {
        Self {
            resource_type: value.resource_type,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RoleView {
    pub id: Uuid,
    pub name: String,
    #[schema(value_type = crate::api::vocabulary::RoleTypeSchema)]
    pub role_type: RoleType,
    pub permissions: Vec<RolePermissionView>,
}

impl From<RoleView> for citadel_identity::RoleDetails {
    fn from(value: RoleView) -> Self {
        Self {
            id: value.id,
            name: value.name,
            role_type: value.role_type,
            permissions: value
                .permissions
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

impl From<citadel_identity::RoleDetails> for RoleView {
    fn from(value: citadel_identity::RoleDetails) -> Self {
        Self {
            id: value.id,
            name: value.name,
            role_type: value.role_type,
            permissions: value
                .permissions
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PatchActorEnabledInput {
    pub is_enabled: bool,
}

impl From<PatchActorEnabledInput> for citadel_identity::PatchActorEnabledInput {
    fn from(value: PatchActorEnabledInput) -> Self {
        Self {
            is_enabled: value.is_enabled,
        }
    }
}

impl From<citadel_identity::PatchActorEnabledInput> for PatchActorEnabledInput {
    fn from(value: citadel_identity::PatchActorEnabledInput) -> Self {
        Self {
            is_enabled: value.is_enabled,
        }
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActorView {
    pub id: Uuid,
    pub name: String,
    #[serde(rename = "type")]
    #[schema(value_type = crate::api::vocabulary::ActorTypeSchema)]
    pub actor_type: ActorType,
    pub is_enabled: bool,
}

impl From<ActorView> for citadel_identity::ActorDetails {
    fn from(value: ActorView) -> Self {
        Self {
            id: value.id,
            name: value.name,
            actor_type: value.actor_type,
            is_enabled: value.is_enabled,
        }
    }
}

impl From<citadel_identity::ActorDetails> for ActorView {
    fn from(value: citadel_identity::ActorDetails) -> Self {
        Self {
            id: value.id,
            name: value.name,
            actor_type: value.actor_type,
            is_enabled: value.is_enabled,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub enum LoginNextStep {
    Completed,
    VerifyMfa,
    EnrollMfa,
}

impl From<LoginNextStep> for citadel_identity::LoginNextStep {
    fn from(value: LoginNextStep) -> Self {
        match value {
            LoginNextStep::Completed => Self::Completed,
            LoginNextStep::VerifyMfa => Self::VerifyMfa,
            LoginNextStep::EnrollMfa => Self::EnrollMfa,
        }
    }
}

impl From<citadel_identity::LoginNextStep> for LoginNextStep {
    fn from(value: citadel_identity::LoginNextStep) -> Self {
        match value {
            citadel_identity::LoginNextStep::Completed => Self::Completed,
            citadel_identity::LoginNextStep::VerifyMfa => Self::VerifyMfa,
            citadel_identity::LoginNextStep::EnrollMfa => Self::EnrollMfa,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InitializeCitadelRequest {
    pub name: String,
    pub email: String,
    pub password: String,
}

impl From<InitializeCitadelRequest> for citadel_identity::InitializeCitadel {
    fn from(value: InitializeCitadelRequest) -> Self {
        Self {
            name: value.name,
            email: value.email,
            password: value.password,
        }
    }
}

impl From<citadel_identity::InitializeCitadel> for InitializeCitadelRequest {
    fn from(value: citadel_identity::InitializeCitadel) -> Self {
        Self {
            name: value.name,
            email: value.email,
            password: value.password,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub email_or_name: String,
    pub password: String,
}

impl From<LoginRequest> for citadel_identity::Login {
    fn from(value: LoginRequest) -> Self {
        Self {
            email_or_name: value.email_or_name,
            password: value.password,
        }
    }
}

impl From<citadel_identity::Login> for LoginRequest {
    fn from(value: citadel_identity::Login) -> Self {
        Self {
            email_or_name: value.email_or_name,
            password: value.password,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub access_token: Option<String>,
    pub next_step: LoginNextStep,
}

impl From<LoginResponse> for citadel_identity::LoginOutcome {
    fn from(value: LoginResponse) -> Self {
        Self {
            access_token: value.access_token,
            next_step: value.next_step.into(),
        }
    }
}

impl From<citadel_identity::LoginOutcome> for LoginResponse {
    fn from(value: citadel_identity::LoginOutcome) -> Self {
        Self {
            access_token: value.access_token,
            next_step: value.next_step.into(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupStatusView {
    pub requires_setup: bool,
}
impl From<citadel_identity::SetupStatus> for SetupStatusView {
    fn from(value: citadel_identity::SetupStatus) -> Self {
        Self {
            requires_setup: value.requires_setup,
        }
    }
}
