using System.Text.Json.Serialization;

namespace Domain;

public enum PlatformType
{
    Docker,
    DockerSwarm,
    Kubernetes
}

public enum PlatformStatus
{
    Offline,
    Online
}

public enum RegistryType
{
    Custom,
    DockerHub,
    Azure,
    AWS,
    Gitlab,
    GitHub
}

public enum GhcrAccountType
{
    Organization,
    User
}

public enum ContainerStateStatus
{
    Unknown,
    Created,
    Running,
    Paused,
    Restarting,
    Exited,
    Removing,
    Dead,
    Offline,
}

public enum AppPermission
{
    None = 0,
    // Users
    User_View,
    User_Create,
    User_Update,
    User_Delete,
    // Roles
    Role_View,
    Role_Create,
    Role_Update,
    Role_Delete,
    // Teams
    Team_View,
    Team_Create,
    Team_Update,
    Team_Delete,
    // Platforms
    Platform_View,
    Platform_Create,
    Platform_Update,
    Platform_Delete,
    // Containers
    Container_View,
    Container_Create,
    Container_Update,
    Container_Delete,
    // Networks
    Network_View,
    Network_Create,
    Network_Update,
    Network_Delete,
    // Volumes
    Volume_View,
    Volume_Create,
    Volume_Delete,
    // Images
    Image_View,
    Image_Create,
    Image_Delete,
    // Registries
    Registry_View,
    Registry_Create,
    Registry_Update,
    Registry_Delete,
    // Deployments
    Deployment_View,
    Deploymen_Create,
    Deploymen_Update,
    Deploymen_Delete,
}

public enum DockerHubTagStatus
{
    Active = 0,
    Inactive = 1,
}

public enum DockerHubImageStatus
{
    Active = 0,
    Inactive = 1,
}

public enum ContainerAction
{
    START = 0,
    RESTART,
    STOP,
    PAUSE,
    UNPAUSE
}

public enum DeploymentAction
{
    STOP,
    PAUSE,
    UNPAUSE,
    RESTART,
    START
}

public enum ContainerEventType
{
    Unknown = 0,
    Builder,
    Config,
    Container,
    Daemon,
    Image,
    Network,
    Node,
    Plugin,
    Secret,
    Service,
    Volume,
}

public enum VolumeScope
{
    Single = 0,
    Multi
}

public enum VolumeSharing
{
    None = 0,
    ReadOnly,
    OneWriter,
    All
}

public enum PlatformConnectorType
{
    Unknown,
    Local,
    Agent
}

public enum ContainerRestartPolicy
{
    No = 0,
    Always,
    OnFailure,
    UnlessStopped
}

public enum LoggingDriverType
{
    [JsonStringEnumMemberName("none")]
    None = 0,
    [JsonStringEnumMemberName("local")]
    Local,
    [JsonStringEnumMemberName("json-file")]
    JsonFile,
    [JsonStringEnumMemberName("syslog")]
    Syslog,
    [JsonStringEnumMemberName("journald")]
    Journald,
    [JsonStringEnumMemberName("gelf")]
    Gelf,
    [JsonStringEnumMemberName("fluentd")]
    Fluentd,
    [JsonStringEnumMemberName("awslogs")]
    Awslogs,
    [JsonStringEnumMemberName("splunk")]
    Splunk,
    [JsonStringEnumMemberName("etwlogs")]
    Etwlogs,
    [JsonStringEnumMemberName("gcplogs")]
    Gcplogs
}

public enum DeploymentStatus
{
    /// <summary>
    /// Fallback for serialization errors or agent disconnects.
    /// </summary>
    Unknown = 0,
    /// <summary>
    /// Spec saved in DB, but the runner hasn't picked it up yet.
    /// </summary>
    Created,
    /// <summary>
    /// Paused or restarting container, waiting to become healthy.
    /// </summary>
    Pending,
    /// <summary>
    /// Pulling images, creating containers, starting networking.
    /// </summary>
    Applying,
    /// <summary>
    /// Container is up, health checks passing (if any), exit code 0.
    /// </summary>
    Healthy,
    /// <summary>
    /// Container is running but drift detected, or health checks failing, 
    /// or in a restart loop (CrashLoopBackOff), or paused.
    /// </summary>
    Degraded,
    /// <summary>
    /// Container exited with non-zero code, or image pull failed.
    /// </summary>
    Failed,
    /// <summary>
    /// Intentionally stopped by user. Resources exist but are not running.
    /// </summary>
    Stopped
}

public enum DeploymentSource 
{
    UI,
    Git, 
    API, 
    CLI 
}

public enum ScalingStrategy
{
    RollingUpdate, 
    Recreate
}

public enum ImageSource
{
    Local,
    External
}

public enum StopSignal {     
    SIGTERM,
    SIGKILL,
    SIGINT,
}
public enum ActorType
{
    User = 0,
    System,
    Agent,
    Service
}

public enum RegistryStatus
{
    Active,
    Disabled,
    Deprecated
}

public enum AutoUpdateMode
{
    /// <summary>
    /// The platform periodically checks the registry (Default).
    /// </summary>
    Poll,
    /// <summary>
    /// The registry calls a webhook on this platform to trigger update.
    /// </summary>
    Webhook
}

public enum AutoUpdateStrategy
{
    /// <summary>
    /// The standard "Watchtower" behavior. 
    /// Checks if the Digest for the currently defined Tag has changed.
    /// Used for mutable tags like 'latest', 'dev', 'stable'.
    /// </summary>
    RecreateOnNewDigest,

    /// <summary>
    /// Scans registry for newer tags matching a SemVer pattern.
    /// E.g. currently 'v1.0.1', found 'v1.0.2'.
    /// </summary>
    SemVerBump
}

public enum AutoUpdateStatus
{
    Unknown,
    UpToDate,
    /// <summary>
    /// A new digest/tag was found, waiting for update window/approval
    /// </summary>
    UpdateAvailable,
    /// <summary>
    /// Deployment is currently restarting with new image
    /// </summary>
    Updating,
    Failed
}

public enum UpdateBehavior
{
    /// <summary>
    /// Do not check for updates.
    /// </summary>
    Disabled,
    /// <summary>
    /// Periodically check for updates and alert me, but do not redeploy.
    /// </summary>
    Notify,
    /// <summary>
    /// Periodically check and automatically redeploy when a new image is found.
    /// </summary>
    AutoDeploy
}

public enum DeployedContainerState
{
    Running,
    Exited,
    Timeout
}

public enum ResourceControlState
{
    Idle,
    Processing
}

public enum ActivityStatus
{
    Success,
    Failure,
    Warning,
    Information
}

public enum ActivityResourceType
{
    Platform,
    Registry,
    Deployment,
    Stack,
    AlertRule,
}

public enum ActivityEventType
{
    #region Deployment Events
    DeploymentCreated,
    DeploymentUpdated,
    DeploymentRenamed,
    DeploymentDeleted,
    DeploymentStarted,
    DeploymentStopped,
    DeploymentPaused,
    DeploymentApplied,
    DeploymentDegraded,
    #endregion

    #region Platform Events
    PlatformConnected,
    PlatformDisconnected,
    PlatformRenamed,
    #endregion

    #region Registry Events
    RegistryCreated,
    RegistryRenamed,
    RegistryUpdated,
    RegistryDeleted,
    #endregion

    #region AlertRule Events
    AlertRuleCreated,
    AlertRuleUpdated,
    AlertRuleDeleted,
    AlertRuleRenamed
    #endregion
}

public enum AlertType
{
    #region Platform Alerts
    PlatformCpuHigh,
    PlatformRamHigh,
    PlatformUnreachable,
    PlatformVersionMismatch,
    UnmanagedContainerCreated,
    #endregion

    #region Deployment Alerts
    DeploymentImageUpdateAvailable,
    DeploymentAutoDeployFailed,
    DeploymentAutoUpdated,
    #endregion

    #region Stack Alerts
    StackImageUpdateAvailable,
    StackAutoDeployFailed,
    StackAutoUpdated,
    #endregion
}

public enum AlertSeverity
{
    Info,
    Warning,
    Critical
}

public enum AlertRuleStatus
{
    Enabled,
    Disabled
}

public enum ScheduleType
{
    Daily,
    Weekly
}

public enum AlertResourceType
{
    Platform,
    Deployment,
    Stack
}

public enum AlertEventStatus
{
    Active,
    Acknowledged,
    Resolved
}

public enum AlertDestination
{
    Generic,
    Bark,
    Discord,
    Gotify,
    Google_Chat,
    IFTTT,
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
    Zulip_Chat
}

public enum GitAuthType
{
    None,
    Https,
    Ssh
}