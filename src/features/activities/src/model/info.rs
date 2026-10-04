use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "$type")]
// This closed activity enum is serialized immediately at mutation
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
        mode: String,
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
    BackupPolicyCreated {
        #[serde(rename = "Policy")]
        policy: BackupPolicyActivitySnapshot,
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
    ActionWebhookReceived(WebhookActivityDetails),
    BackupPolicyWebhookReceived(WebhookActivityDetails),
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
        #[serde(rename = "ServiceNames", default)]
        service_names: Vec<String>,
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
        if changes.iter().any(|change| {
            !matches!(
                change.name(),
                "TimeZone"
                    | "DateTimeFormat"
                    | "Theme"
                    | "ThemeColor"
                    | "Font"
                    | "Radius"
                    | "ContentLayout"
                    | "Density"
            )
        }) {
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
            Self::ActionWebhookReceived(_) => ActivityEventType::ActionWebhookReceived,
            Self::BackupPolicyWebhookReceived(_) => ActivityEventType::BackupPolicyWebhookReceived,
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
            Self::BackupPolicyCreated { .. } => ActivityEventType::BackupPolicyCreated,
            Self::BackupRunQueued { .. } => ActivityEventType::BackupRunQueued,
            Self::BackupRunStarted { .. } => ActivityEventType::BackupRunStarted,
            Self::BackupRunCompleted { .. } => ActivityEventType::BackupRunCompleted,
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
            Self::BuildAgentPoolConnected { .. } => ActivityEventType::BuildAgentPoolConnected,
            Self::BuildAgentPoolDisconnected { .. } => {
                ActivityEventType::BuildAgentPoolDisconnected
            }
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
