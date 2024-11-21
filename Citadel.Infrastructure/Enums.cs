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

public enum RegistryDiscriminator
{
    DockerHub,
    Azure,
    AWS,
    Gitlab,
    Custom
}