use super::*;
impl ActivityEvent {
    pub fn new_webhook_event(
        id: Uuid,
        name: String,
        platform_id: Option<Uuid>,
        resource_type: ActivityResourceType,
        details: WebhookActivityDetails,
        now: DateTime<Utc>,
    ) -> Result<Self, ActivityInvariantError> {
        let status = details.activity_status();
        let info = match resource_type {
            ActivityResourceType::GitRepository => {
                ActivityEventInfo::GitRepoWebhookReceived(details)
            }
            ActivityResourceType::Stack => ActivityEventInfo::StackWebhookReceived(details),
            ActivityResourceType::Build => ActivityEventInfo::BuildWebhookReceived(details),
            ActivityResourceType::AutomationAction => {
                ActivityEventInfo::ActionWebhookReceived(details)
            }
            ActivityResourceType::BackupPolicy => {
                ActivityEventInfo::BackupPolicyWebhookReceived(details)
            }
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
        let status = match &info {
            ActivityEventInfo::BackupRunQueued { .. }
            | ActivityEventInfo::BackupRunStarted { .. } => ActivityStatus::Information,
            ActivityEventInfo::BackupRunCompleted { status, .. } => match status.as_str() {
                "Succeeded" => ActivityStatus::Success,
                "SucceededWithWarnings" | "Cancelled" => ActivityStatus::Warning,
                _ => ActivityStatus::Failure,
            },
            _ => ActivityStatus::Success,
        };
        Self::new_resource_event(
            resource_id,
            resource_name,
            ActivityResourceType::BackupPolicy,
            actor_id,
            info,
            status,
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
        let status = if matches!(info, ActivityEventInfo::BuildAgentPoolDisconnected { .. }) {
            ActivityStatus::Warning
        } else {
            ActivityStatus::Success
        };
        Self::new_resource_event(
            resource_id,
            resource_name,
            ActivityResourceType::BuildAgentPool,
            actor_id,
            info,
            status,
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
