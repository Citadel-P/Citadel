using Infrastructure.Entities.Identity;

namespace Infrastructure.EntityFramework.Seed;

internal static class DbSeed
{
    public static Role[] Roles =
    [
        Role.Create("Administrator"),
        Role.Create("Developer"),
        Role.Create("QA")
    ];

    public static Permission[] Permissions =
    [
        // Admin permissions (all)
        Permission.Create(Roles[0].Id, AppPermission.ListUsers),
        Permission.Create(Roles[0].Id, AppPermission.AddUsers),
        Permission.Create(Roles[0].Id, AppPermission.EditUsers),
        Permission.Create(Roles[0].Id, AppPermission.DeleteUsers),
        Permission.Create(Roles[0].Id, AppPermission.ListRoles),
        Permission.Create(Roles[0].Id, AppPermission.AddRoles),
        Permission.Create(Roles[0].Id, AppPermission.EditRoles),
        Permission.Create(Roles[0].Id, AppPermission.DeleteRoles),
        Permission.Create(Roles[0].Id, AppPermission.ListTeams),
        Permission.Create(Roles[0].Id, AppPermission.AddTeams),
        Permission.Create(Roles[0].Id, AppPermission.EditTeams),
        Permission.Create(Roles[0].Id, AppPermission.DeleteTeams),
        Permission.Create(Roles[0].Id, AppPermission.ListPlatforms),
        Permission.Create(Roles[0].Id, AppPermission.AddPlatforms),
        Permission.Create(Roles[0].Id, AppPermission.EditPlatforms),
        Permission.Create(Roles[0].Id, AppPermission.DeletePlatforms),
        Permission.Create(Roles[0].Id, AppPermission.ListContainers),
        Permission.Create(Roles[0].Id, AppPermission.AddContainers),
        Permission.Create(Roles[0].Id, AppPermission.EditContainers),
        Permission.Create(Roles[0].Id, AppPermission.DeleteContainers),
        Permission.Create(Roles[0].Id, AppPermission.ListNetworks),
        Permission.Create(Roles[0].Id, AppPermission.AddNetworks),
        Permission.Create(Roles[0].Id, AppPermission.EditNetworks),
        Permission.Create(Roles[0].Id, AppPermission.DeleteNetworks),
        Permission.Create(Roles[0].Id, AppPermission.ListVolumes),
        Permission.Create(Roles[0].Id, AppPermission.AddVolumes),
        Permission.Create(Roles[0].Id, AppPermission.EditVolumes),
        Permission.Create(Roles[0].Id, AppPermission.DeleteVolumes),

        // Dev
        Permission.Create(Roles[1].Id, AppPermission.ListUsers),
        Permission.Create(Roles[1].Id, AppPermission.ListRoles),
        Permission.Create(Roles[1].Id, AppPermission.ListTeams),
        Permission.Create(Roles[1].Id, AppPermission.ListPlatforms),
        Permission.Create(Roles[1].Id, AppPermission.AddPlatforms),
        Permission.Create(Roles[1].Id, AppPermission.EditPlatforms),
        Permission.Create(Roles[1].Id, AppPermission.DeletePlatforms),
        Permission.Create(Roles[1].Id, AppPermission.ListContainers),
        Permission.Create(Roles[1].Id, AppPermission.AddContainers),
        Permission.Create(Roles[1].Id, AppPermission.EditContainers),
        Permission.Create(Roles[1].Id, AppPermission.DeleteContainers),
        Permission.Create(Roles[1].Id, AppPermission.ListNetworks),
        Permission.Create(Roles[1].Id, AppPermission.AddNetworks),
        Permission.Create(Roles[1].Id, AppPermission.DeleteNetworks),
        Permission.Create(Roles[1].Id, AppPermission.AddVolumes),
        Permission.Create(Roles[1].Id, AppPermission.EditVolumes),
        Permission.Create(Roles[1].Id, AppPermission.DeleteVolumes),

        // QA
        Permission.Create(Roles[2].Id, AppPermission.ListUsers),
        Permission.Create(Roles[2].Id, AppPermission.ListRoles),
        Permission.Create(Roles[2].Id, AppPermission.ListTeams),
        Permission.Create(Roles[2].Id, AppPermission.ListPlatforms),
        Permission.Create(Roles[2].Id, AppPermission.ListContainers),
        Permission.Create(Roles[2].Id, AppPermission.ListNetworks),
        Permission.Create(Roles[2].Id, AppPermission.ListVolumes)
    ];

    public static Team[] Teams =
    [
        Team.Create("Admins", Roles[0].Id),
        Team.Create("Devs", Roles[1].Id),
        Team.Create("QA", Roles[2].Id)
    ];

    public static User[] Users =
    [
        User.Create("admin", "admin@admin.com", "admin123"),
        User.Create("dev", "dev@dev.com", "dev123"),
        User.Create("qa", "qa@qa.com", "qa123")
    ];

    public static UserTeam[] UsersTeams =
    [
        UserTeam.Create(Users[0].Id, Teams[0].Id),
        UserTeam.Create(Users[1].Id, Teams[1].Id),
        UserTeam.Create(Users[2].Id, Teams[2].Id)
    ];
}