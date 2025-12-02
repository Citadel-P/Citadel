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
    Etwlogs
}

public enum DeploymentStatus
{
    /// <summary>
    /// Spec written but not applied
    /// </summary>
    Created,
    /// <summary>
    /// Queued to be applied
    /// </summary>
    Pending,
    /// <summary>
    /// Actively applying
    /// </summary>
    Applying,
    /// <summary>
    /// Running OK
    /// </summary>
    Healthy,
    /// <summary>
    /// Drift / errors
    /// </summary>
    Degraded,
    Failed,
    /// <summary>
    /// Replaced by older version
    /// </summary>
    RolledBack
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