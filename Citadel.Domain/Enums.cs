using System.Text.Json.Serialization;

namespace Domain;

[JsonConverter(typeof(JsonStringEnumConverter<PlatformType>))]
public enum PlatformType
{
    [JsonStringEnumMemberName("Docker")]
    Docker,
    [JsonStringEnumMemberName("DockerSwarm")]
    DockerSwarm,
    [JsonStringEnumMemberName("Kubernetes")]
    Kubernetes
}

[JsonConverter(typeof(JsonStringEnumConverter<PlatformStatus>))]
public enum PlatformStatus
{
    [JsonStringEnumMemberName("Offline")]
    Offline,
    [JsonStringEnumMemberName("Online")]
    Online
}

[JsonConverter(typeof(JsonStringEnumConverter<RegistryType>))]
public enum RegistryType
{
    [JsonStringEnumMemberName("DockerHub")] 
    DockerHub,
    [JsonStringEnumMemberName("Azure")]
    Azure,
    [JsonStringEnumMemberName("AWS")]
    AWS,
    [JsonStringEnumMemberName("Gitlab")]
    Gitlab,
    [JsonStringEnumMemberName("GitHub")]
    GitHub
}

[JsonConverter(typeof(JsonStringEnumConverter<GhcrAccountType>))]
public enum GhcrAccountType
{
    [JsonStringEnumMemberName("Organization")]
    Organization,
    [JsonStringEnumMemberName("User")]
    User
}

[JsonConverter(typeof(JsonStringEnumConverter<ContainerStateStatus>))]
public enum ContainerStateStatus
{
    [JsonStringEnumMemberName("Unknown")]
    Unknown,
    [JsonStringEnumMemberName("Created")]
    Created,
    [JsonStringEnumMemberName("Running")]
    Running,
    [JsonStringEnumMemberName("Paused")]
    Paused,
    [JsonStringEnumMemberName("Restarting")]
    Restarting,
    [JsonStringEnumMemberName("Exited")]
    Exited,
    [JsonStringEnumMemberName("Removing")]
    Removing,
    [JsonStringEnumMemberName("Dead")]
    Dead,
    [JsonStringEnumMemberName("Offline")]
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
}

public enum DockerHubTagStatus
{
    [JsonStringEnumMemberName("active")]
    Active = 0,
    [JsonStringEnumMemberName("inactive")]
    Inactive = 1,
}

public enum DockerHubImageStatus
{
    [JsonStringEnumMemberName("active")]
    Active = 0,
    [JsonStringEnumMemberName("inactive")]
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

[JsonConverter(typeof(JsonStringEnumConverter<PlatformConnectorType>))]
public enum PlatformConnectorType
{
    [JsonStringEnumMemberName("unknown")]
    Unknown,
    [JsonStringEnumMemberName("local")]
    Local,
    [JsonStringEnumMemberName("agent")]
    Agent
}