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

public enum PlatformStatus
{
    Offline,
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