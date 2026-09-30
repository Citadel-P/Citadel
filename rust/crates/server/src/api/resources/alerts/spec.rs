//! Typed alert HTTP values. OpenAPI derives from the same types used for decoding.
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub enum AlertDestination {
    Generic,
    Bark,
    Discord,
    Gotify,
    #[serde(rename = "Google_Chat")]
    GoogleChat,
    #[serde(rename = "IFTTT")]
    Ifttt,
    Join,
    Lark,
    Mattermost,
    Matrix,
    Ntfy,
    OpsGenie,
    Pushbullet,
    Pushover,
    Rocketchat,
    Signal,
    Slack,
    Teams,
    Telegram,
    WeCom,
    #[serde(rename = "Zulip_Chat")]
    ZulipChat,
}
impl AlertDestination {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Generic => "Generic",
            Self::Bark => "Bark",
            Self::Discord => "Discord",
            Self::Gotify => "Gotify",
            Self::GoogleChat => "Google_Chat",
            Self::Ifttt => "IFTTT",
            Self::Join => "Join",
            Self::Lark => "Lark",
            Self::Mattermost => "Mattermost",
            Self::Matrix => "Matrix",
            Self::Ntfy => "Ntfy",
            Self::OpsGenie => "OpsGenie",
            Self::Pushbullet => "Pushbullet",
            Self::Pushover => "Pushover",
            Self::Rocketchat => "Rocketchat",
            Self::Signal => "Signal",
            Self::Slack => "Slack",
            Self::Teams => "Teams",
            Self::Telegram => "Telegram",
            Self::WeCom => "WeCom",
            Self::ZulipChat => "Zulip_Chat",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub enum AlertType {
    PlatformCpuHigh,
    PlatformRamHigh,
    PlatformDiskHigh,
    PlatformUnreachable,
    PlatformVersionMismatch,
    UnmanagedContainerCreated,
    DeploymentImageUpdateAvailable,
    DeploymentAutoDeployFailed,
    DeploymentAutoUpdated,
    SwarmServiceOperationFailed,
    StackImageUpdateAvailable,
    StackAutoDeployFailed,
    StackAutoUpdated,
    StackServiceAutoDeployFailed,
    StackServiceAutoUpdated,
    StackDriftDetected,
    StackDriftAutoReconciled,
    StackGitUpdateAvailable,
    StackGitAutoUpdated,
    StackGitAutoDeployFailed,
    StackConfigurationResolutionFailed,
    DeploymentConfigurationResolutionFailed,
    WebhookAuthenticationFailed,
    WebhookDispatchFailed,
    WebhookGitRepoSyncFailed,
    WebhookStackGitDeployFailed,
    AutomationActionRunFailed,
    BuildRunFailed,
    LicenseEnteredGracePeriod,
    LicenseExpired,
}
impl AlertType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PlatformCpuHigh => "PlatformCpuHigh",
            Self::PlatformRamHigh => "PlatformRamHigh",
            Self::PlatformDiskHigh => "PlatformDiskHigh",
            Self::PlatformUnreachable => "PlatformUnreachable",
            Self::PlatformVersionMismatch => "PlatformVersionMismatch",
            Self::UnmanagedContainerCreated => "UnmanagedContainerCreated",
            Self::DeploymentImageUpdateAvailable => "DeploymentImageUpdateAvailable",
            Self::DeploymentAutoDeployFailed => "DeploymentAutoDeployFailed",
            Self::DeploymentAutoUpdated => "DeploymentAutoUpdated",
            Self::SwarmServiceOperationFailed => "SwarmServiceOperationFailed",
            Self::StackImageUpdateAvailable => "StackImageUpdateAvailable",
            Self::StackAutoDeployFailed => "StackAutoDeployFailed",
            Self::StackAutoUpdated => "StackAutoUpdated",
            Self::StackServiceAutoDeployFailed => "StackServiceAutoDeployFailed",
            Self::StackServiceAutoUpdated => "StackServiceAutoUpdated",
            Self::StackDriftDetected => "StackDriftDetected",
            Self::StackDriftAutoReconciled => "StackDriftAutoReconciled",
            Self::StackGitUpdateAvailable => "StackGitUpdateAvailable",
            Self::StackGitAutoUpdated => "StackGitAutoUpdated",
            Self::StackGitAutoDeployFailed => "StackGitAutoDeployFailed",
            Self::StackConfigurationResolutionFailed => "StackConfigurationResolutionFailed",
            Self::DeploymentConfigurationResolutionFailed => {
                "DeploymentConfigurationResolutionFailed"
            }
            Self::WebhookAuthenticationFailed => "WebhookAuthenticationFailed",
            Self::WebhookDispatchFailed => "WebhookDispatchFailed",
            Self::WebhookGitRepoSyncFailed => "WebhookGitRepoSyncFailed",
            Self::WebhookStackGitDeployFailed => "WebhookStackGitDeployFailed",
            Self::AutomationActionRunFailed => "AutomationActionRunFailed",
            Self::BuildRunFailed => "BuildRunFailed",
            Self::LicenseEnteredGracePeriod => "LicenseEnteredGracePeriod",
            Self::LicenseExpired => "LicenseExpired",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}
impl AlertSeverity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Warning => "Warning",
            Self::Critical => "Critical",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub enum AlertRuleStatus {
    Enabled,
    Disabled,
}
impl AlertRuleStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Enabled => "Enabled",
            Self::Disabled => "Disabled",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub enum AlertEventStatus {
    Active,
    Acknowledged,
    Resolved,
}
impl AlertEventStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::Acknowledged => "Acknowledged",
            Self::Resolved => "Resolved",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub enum AlertResourceType {
    Platform,
    Deployment,
    Stack,
    GitRepository,
    Webhook,
    AutomationAction,
    Build,
    License,
    SwarmService,
}
impl AlertResourceType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Platform => "Platform",
            Self::Deployment => "Deployment",
            Self::Stack => "Stack",
            Self::GitRepository => "GitRepository",
            Self::Webhook => "Webhook",
            Self::AutomationAction => "AutomationAction",
            Self::Build => "Build",
            Self::License => "License",
            Self::SwarmService => "SwarmService",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub enum DayOfWeek {
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}
impl DayOfWeek {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sunday => "Sunday",
            Self::Monday => "Monday",
            Self::Tuesday => "Tuesday",
            Self::Wednesday => "Wednesday",
            Self::Thursday => "Thursday",
            Self::Friday => "Friday",
            Self::Saturday => "Saturday",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AlertResourceScope {
    pub resource_id: Uuid,
    pub resource_type: AlertResourceType,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(tag = "$type")]
pub enum AlertQuietHour {
    Daily {
        #[serde(default)]
        #[schema(required = true)]
        name: String,
        description: Option<String>,
        #[serde(rename = "startTime")]
        start_time: String,
        #[serde(rename = "endTime")]
        end_time: String,
        timezone: String,
    },
    Weekly {
        #[serde(default)]
        #[schema(required = true)]
        name: String,
        description: Option<String>,
        #[serde(rename = "startTime")]
        start_time: String,
        #[serde(rename = "endTime")]
        end_time: String,
        timezone: String,
        #[serde(rename = "dayOfWeek")]
        day_of_week: DayOfWeek,
    },
}

pub(super) fn decode<T: DeserializeOwned>(value: Value) -> Result<T, citadel_alerts::AlertError> {
    serde_json::from_value(value).map_err(|error| {
        citadel_alerts::AlertError::Storage(format!("Invalid stored alert data: {error}"))
    })
}
