using System.Text.Json.Serialization;

namespace Infrastructure;

public enum AppPermission
{
    None = 0,
    ListUsers,
    AddUsers,
    EditUsers,
    DeleteUsers,
    ListRoles,
    AddRoles,
    EditRoles,
    DeleteRoles,
    ListTeams,
    AddTeams,
    EditTeams,
    DeleteTeams,
    ListPlatforms,
    AddPlatforms,
    EditPlatforms,
    DeletePlatforms,
    ListContainers,
    AddContainers,
    EditContainers,
    DeleteContainers,
    ListNetworks,
    AddNetworks,
    EditNetworks,
    DeleteNetworks,
    ListVolumes,
    AddVolumes,
    EditVolumes,
    DeleteVolumes,
}

[JsonConverter(typeof(JsonStringEnumConverter))]
public enum PlatformStatus
{
    [JsonStringEnumMemberName("Offline")]
    Offline,
    [JsonStringEnumMemberName("Online")]
    Online
}

[JsonConverter(typeof(JsonStringEnumConverter))]
public enum RegistryDiscriminator
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

[JsonConverter(typeof(JsonStringEnumConverter))]
public enum GhcrAccountType
{
    [JsonStringEnumMemberName("Organization")]
    Organization,
    [JsonStringEnumMemberName("User")]
    User
}

[JsonConverter(typeof(JsonStringEnumConverter))]
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