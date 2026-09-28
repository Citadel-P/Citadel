#![forbid(unsafe_code)]

pub mod actors;
pub use actors::{
    ActorDetails, ActorPrincipal, ActorRepository, ActorType, AuthenticatedPrincipalType,
    PatchActorEnabledInput, SYSTEM_ACTOR_ID,
};
pub mod authentication;
pub use authentication::{
    AccessTokenClaims, AuthenticatedBearer, Clock, EntitlementService, IdentityService,
    IdentityStore, InitializeCitadel, Login, LoginNextStep, LoginOutcome,
    MAXIMUM_PASSWORD_CHARACTERS, MAXIMUM_SESSIONS_PER_USER, MINIMUM_PASSWORD_CHARACTERS,
    NewSession, PasswordHasher, PreparedSession, RefreshTokenClaims, ServiceAccountTokenCodec,
    SessionMetadata, SessionTokenCodec, SessionTokens, SetupInitializationMode, SetupStatus,
    SystemClock, UserAuthentication, UserSessionRecord, token_digest, validate_email,
    validate_name, validate_password,
};
pub mod mfa;
pub use mfa::{
    BrowserAuthenticationAction, BrowserAuthenticationResult, ChallengeCompletionCommit,
    ConfirmMandatoryMfaSetupInput, ConfirmProfileMfaSetupInput, DisableMfaCommit,
    DisableProfileMfaInput, MandatoryEnrollmentCommit, MandatoryMfaSetupCompleteDetails,
    MandatoryMfaSetupDetails, MfaChallenge, MfaConfiguration, MfaCredentialAcceptance, MfaPolicy,
    MfaService, MfaSetupSession, MfaStore, MfaVerificationDetails, MfaVerificationInput,
    ProfileEnrollmentCommit, ProfileMfaRecoveryCodesDetails, ProfileMfaSetupDetails,
    ProfileMfaStatusDetails, RecoveryCodeService, RegenerateProfileMfaRecoveryCodesInput,
    RegenerateRecoveryCodesCommit, ResetMfaCommit, SecretProtector, StartProfileMfaSetupInput,
    TotpService, TotpSetup, UserMfaRecoveryCode, UserMfaSettings,
};
pub mod oidc;
pub use oidc::{
    CreateOidcProvider, DEFAULT_OIDC_SCOPES, OidcDiscovery, OidcDiscoveryDetails,
    OidcExternalLogin, OidcIdentity, OidcLoginComplete, OidcLoginProviderDetails,
    OidcLoginProvidersDetails, OidcLoginStart, OidcLoginState, OidcProtocol, OidcProvider,
    OidcProviderDetails, OidcProviderValidationError, OidcProvidersDetails, OidcService, OidcStore,
    PatchOidcProvider, PatchOidcProviderMetadata, RenameOidcProvider, TestOidcDiscovery,
    build_authorization_url, hash_opaque_value, normalize_return_url,
};
pub mod profile;
pub use profile::{
    ChangeCurrentPassword, CurrentProfileAuthenticationDetails, CurrentProfileAuthenticationType,
    CurrentProfileAuthorizationDetails, CurrentProfileDetails, CurrentProfileRecord,
    PasswordChangeOutcome, PatchUserPreferences, ProfileRepository, ProfileResourceInfo,
    ProfileService, ResourceCapabilities, RevokeOtherProfileSessionsDetails, UpdateCurrentProfile,
    UserAppearance, UserContentLayout, UserDateTimeFormat, UserPreferences, UserPreferencesDetails,
    UserPreferencesUpdate, UserSessionSummaryDetails, UserSessionsDetails, UserTheme,
    UserThemeColor, UserUiDensity, UserUiFont, UserUiRadius,
};
pub mod roles;
pub use roles::{
    ADMIN_ROLE_ID, CreateRole, DeleteRoles, NewRoleMutation, PatchRolePermissions, RenameRole,
    Role, RoleDetails, RoleMutationError, RoleMutationService, RolePermission,
    RolePermissionDetails, RolePermissionInput, RoleReadService, RoleReader, RoleRepository,
    RoleType, role_permissions_expand,
};
pub mod service_accounts;
pub use service_accounts::{
    AddServiceAccountResourceAccess, AddServiceAccountRole, ArchiveServiceAccounts,
    CreateServiceAccount, CreateServiceAccountToken, CreatedServiceAccountTokenDetails,
    DEFAULT_SERVICE_ACCOUNT_TOKEN_LIFETIME_DAYS, MAXIMUM_ACTIVE_SERVICE_ACCOUNT_TOKENS,
    MAXIMUM_SERVICE_ACCOUNT_DESCRIPTION_CHARS, MAXIMUM_SERVICE_ACCOUNT_TOKEN_LIFETIME_DAYS,
    NewServiceAccount, NewServiceAccountToken, RenameServiceAccount, RunAsActorUsageDetails,
    ServiceAccountCredential, ServiceAccountDetails, ServiceAccountLimitsDetails,
    ServiceAccountRepository, ServiceAccountResourceAccess, ServiceAccountService,
    ServiceAccountTokenDetails, UpdateServiceAccount,
};
pub mod teams;
pub use teams::{
    AddTeamMember, AddTeamRole, CreateTeam, DeleteTeams, NewTeamMutation, PatchTeam, RenameTeam,
    Team, TeamDetails, TeamMemberDetails, TeamMutationService, TeamPatchMutation, TeamReadService,
    TeamReader, TeamRepository, TeamResourceAccessDetails, TeamResourceAccessInput,
    TeamSearchItemDetails,
};
pub mod users;
pub use users::{
    AddUserRole, CreateUser, DeleteUsers, NewUserMutation, PatchUser, RenameUser, User,
    UserDetails, UserMutationService, UserPasswordContext, UserPatchMutation, UserReadService,
    UserReader, UserRepository, UserResourceAccess, UserResourceAccessDetails,
    UserResourceAccessInput, UserSearchItemDetails,
};
pub mod patch;
pub use patch::PatchField;
pub mod read_models;
pub use read_models::{PagedResult, ResourceInfo, StoredPage};
pub use service_accounts::{
    NoopServiceAccountLastUsedTracker, ServiceAccountLastUsedStore, ServiceAccountLastUsedTracker,
};
pub mod permissions;
pub use permissions::{
    AuthorizationSnapshot, PermissionCapability, PermissionGrant, permission_matrix,
};
pub mod validation;
pub use validation::MAX_NAME_CHARS;

#[cfg(test)]
mod tests;

pub mod error;
pub use error::IdentityError;
