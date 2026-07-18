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
    Image_Pull,
    Image_Create,
    Image_Delete,
    // Registries
    Registry_View,
    Registry_Create,
    Registry_Update,
    Registry_Delete,
    // Alerts
    Alert_View,
    Alert_Create,
    Alert_Update,
    Alert_Delete,
    // Alert Channel
    AlertChannel_View,
    AlertChannel_Create,
    AlertChannel_Update,
    AlertChannel_Delete,
    // Git Account
    GitAccount_View,
    GitAccount_Create,
    GitAccount_Update,
    GitAccount_Delete,
    // Git Repository
    GitRepository_View,
    GitRepository_Create,
    GitRepository_Update,
    GitRepository_Delete,
    // Activity
    Activity_View,
    // Deployments
    Deployment_Apply,
    Deployment_View,
    Deployment_Create,
    Deployment_Update,
    Deployment_Delete,
    // Stacks
    Stack_Apply,
    Stack_View,
    Stack_Create,
    Stack_Update,
    Stack_Delete,
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
    Agent,
    EdgeAgent
}

public enum LicenseLimit
{
    CustomRoles,
    ActiveUsers,
    Platforms,
    BackupPolicies,
    AutomationActions
}

public enum LicenseStatus
{
    Community,
    Valid,
    GracePeriod,
    NotYetValid,
    Expired,
    Invalid,
    InstanceMismatch,
    UnsupportedSchema,
    UnknownSigningKey
}

public enum PruneResource
{
    All,
    Volume,
    Network,
    Image,
    Build
}

public enum EdgeAgentConnectionStatus
{
    Offline = 0,
    Connected,
    Revoked
}

public enum EdgeAgentCommandKind
{
    Unspecified = 0,
    PlatformCheckHealth = 1,
    PlatformGetInfo = 2,
    PlatformStatsStream = 3,
    PlatformDaemonEventsStream = 4,
    PlatformPrune = 5,
    ContainerList = 10,
    ContainerLogsStream = 11,
    ContainerInspect = 12,
    ContainerCreate = 13,
    ContainerStart = 14,
    ContainerStop = 15,
    ContainerPause = 16,
    ContainerUnpause = 17,
    ContainerRestart = 18,
    ContainerDelete = 19,
    ContainerStatsStream = 20,
    ContainersStatsStream = 21,
    ContainerExec = 22,
    ContainerExecBinary = 23,
    ImageGet = 30,
    ImageList = 31,
    ImageInspect = 32,
    ImageDelete = 33,
    ImageHistory = 34,
    ImageExposedPorts = 35,
    ImageDistributionInspect = 36,
    ImagePullStream = 37,
    VolumeList = 50,
    VolumeInspect = 51,
    VolumeCreate = 52,
    VolumeDelete = 53,
    NetworkList = 60,
    NetworkInspect = 61,
    NetworkCreate = 62,
    NetworkDelete = 63,
    StackApplyStream = 70,
    DeploymentApply = 80
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

public enum StackReleaseStatus
{
    Unknown = 0,
    Created,
    Applying,
    Healthy,
    Pending,
    Paused,
    Degraded,
    Failed,
    Stopped
}

public enum StackSource
{
    WebEditor = 0,
    Git
}

public enum StackDriftMode
{
    Disabled = 0,
    DetectOnly = 1,
    AutoFix = 2
}

public enum StackReconciliationStatus
{
    NoDrift = 0,
    Reconciled = 1,
    Partial = 2,
    RequiresReapply = 3,
    Disabled = 4,
    Failed = 5
}

public enum StackReconciliationActionType
{
    StartContainer = 0,
    ResumeContainer = 1,
    RemoveContainer = 2
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
    Service,
    Team
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

public enum StackUpdateBehavior
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
    /// Periodically check and automatically redeploy the service when a new image is found.
    /// </summary>
    ServiceAutoDeploy,
    /// <summary>
    /// Periodically check and automatically redeploy the entire stack instead of just specific services
    /// </summary>
    StackAutoDeploy
}

public enum  UpdateTrigger
{
    Poll,
    Webhook
}

public enum WebHookAuthStyle
{
    Github,
    Gitlab
}

public enum WebhookProvider
{
    GitHub = 1,
    GitLab = 2
}

public enum WebhookAuthScheme
{
    GitHubHmacSha256 = 1,
    GitLabSignedToken = 2,
    GitLabLegacyToken = 3
}

public enum WebhookExecution
{
    RepoPull = 1,
    StackDeploy = 2,
    AutomationActionRun = 3,
    BackupPolicyRun = 4
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
    GitRepository,
    OidcProvider,
    AutomationAction,
    User,
    License,
    Volume
}

public enum ActivityEventType
{
    #region Deployment Events
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
    #endregion

    #region Platform Events
    PlatformCreated,
    PlatformDeleted,
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
    AlertRuleRenamed,
    #endregion

    #region GitRepository Events
    GitRepoCreated,
    GitRepoUpdated,
    GitRepoDeleted,
    GitRepoRenamed,
    GitRepoPulled,
    GitRepoCloned,
    GitRepoWebhookReceived,
    #endregion

    #region OIDC Provider Events
    OidcProviderCreated,
    OidcProviderUpdated,
    OidcProviderRenamed,
    OidcProviderDeleted,
    #endregion

    #region Automation Action Events
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
    #endregion

    #region Stack Events
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
    StackWebhookReceived,
    #endregion

    #region User Events
    UserProfileUpdated,
    UserPreferencesUpdated,
    UserPasswordChanged,
    UserSessionRevoked,
    UserOtherSessionsRevoked,
    #endregion

    #region License Events
    LicenseInstalled,
    LicenseReplaced,
    LicenseRemoved,
    LicenseEnteredGracePeriod,
    LicenseExpired,
    LicenseValidationFailed,
    #endregion

    #region Volume Events
    VolumeContentDownloaded,
    #endregion
}

public enum UserDateTimeFormat
{
    System,
    TwentyFourHour,
    TwelveHour
}

public enum UserTheme
{
    System,
    Light,
    Dark
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
    #endregion

    #region Automation Alerts
    AutomationActionRunFailed,
    #endregion

    #region License Alerts
    LicenseEnteredGracePeriod,
    LicenseExpired,
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
    Stack,
    GitRepository,
    Webhook,
    AutomationAction,
    License
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
    Basic,
    Token,
    SshKey,
}

public enum GitTransport
{
    Http,
    Https,
    Ssh
}

public enum GitReposStatus
{
    Unknown,
    Pending,
    Created,
    Healthy,
    Degraded
}

public enum GitRepositorySyncMode
{
    Manual,
    PullInterval
}

public enum GitOperation
{ 
    Authenticate = 0,
    Clone,
    Pull
}

public enum RoleType
{
    System,
    Custom
}

public enum TargetResource
{
    Container,
    Deployment,
    Stack
}

public enum LookupResourceType
{
    Platform,
    Deployment,
    Stack,
    Image,
    Network,
    Volume,
    Registry,
    GitRepository,
    GitAccount,
    OidcProvider,
    AutomationAction,
    Alert,
    AlertChannel,
    User,
    UserActor,
    Team,
    Role,
    ResourceBinding,
    License,
    BackupRepository,
    BackupPolicy,
}

public enum StackApplyEventType
{
    StdOut = 1,
    StdErr,
    SystemMessage,
    CommandCompleted
}

public enum SecretProviderType
{
    InternalEncrypted,
    VaultCompatibleKvV2
}

public enum ResourceBindingKind
{
    Variable,
    Secret
}

public enum ResourceBindingScope
{
    Global,
    Stack,
    Deployment
}

public enum TaggableResourceType
{
    Deployment,
    Stack,
    Platform,
    GitRepository,
    Registry,
    AutomationAction,
    BackupPolicy
}

public enum SecretDeliveryMode
{
    EnvironmentVariable,
    MountedFile,
    NativePlatformSecret
}

public enum ActionRunTrigger
{
    Manual,
    Test,
    Schedule,
    Webhook
}

public enum ActionRunStatus
{
    Queued,
    Running,
    Succeeded,
    Failed,
    TimedOut,
    Cancelled,
    Rejected
}

public enum BackupSourceType
{
    DockerVolume,
    CitadelSystem,
    Stack,
    Deployment
}

public enum VolumeBackupConsistency
{
    Live,
    StopAttachedContainers
}

public enum BackupRepositoryType
{
    FileSystem,
    S3Compatible
}

public enum BackupExecutionLocation
{
    Core,
    Platform
}

public enum S3BucketLookup
{
    Auto,
    Path,
    Dns
}

public enum BackupRepositoryStatus
{
    Unknown,
    Uninitialized,
    Ready
}

public enum BackupRepositoryValidationStatus
{
    Unknown,
    Ready,
    Uninitialized,
    Unavailable,
    InvalidPassword,
    InvalidConfiguration
}

public enum BackupRunTrigger
{
    Manual,
    Schedule,
    Automation,
    Webhook
}

public enum BackupRunStatus
{
    Queued,
    Preparing,
    Running,
    ApplyingRetention,
    Succeeded,
    SucceededWithWarnings,
    Failed,
    TimedOut,
    Cancelled,
    Rejected,
    Interrupted
}

public enum BackupRunItemStatus
{
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled
}

public enum BackupSnapshotAvailability
{
    Pending,
    Available,
    Expired,
    Missing,
    NotCreated
}

public enum BackupCoverageStatus
{
    NotApplicable,
    Unprotected,
    Protected,
    Warning,
    Failed
}

public enum BackupCoverageResourceType
{
    Platform,
    Volume,
    Container,
    Deployment,
    Stack
}

public enum BackupRestoreStatus
{
    Queued,
    Preparing,
    Running,
    Succeeded,
    SucceededWithWarnings,
    Failed,
    TimedOut,
    Cancelled,
    Rejected,
    Interrupted
}
