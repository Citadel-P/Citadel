using Infrastructure.Entities.Identity;

namespace Infrastructure.EntityFramework.Seed;

internal static class DbSeed
{
    private static readonly DateTime dateTime = new (2025, 1, 1, 0, 0, 0, DateTimeKind.Utc);
    public static Role[] Roles =
    [
        Role.Create("Admin", Guid.Parse("0196debd-033b-7512-a11b-98533d063a04"), dateTime),
        Role.Create("Dev", Guid.Parse("0196debe-0c94-7467-9b2d-397e0200276f"), dateTime),
        Role.Create("QA", Guid.Parse("0196debe-2d80-76dd-b351-ade38fa29169"), dateTime)
    ];

    public static Permission[] Permissions =
    [
        // Admin has all permissions
        
        // Dev
        Permission.Create(Roles[1].Id, AppPermission.Platform_View, Guid.Parse("0196debe-3a01-4b2d-8e1f-1a2b3c4d5e6f")),
        Permission.Create(Roles[1].Id, AppPermission.Platform_Create, Guid.Parse("0196debe-3a02-4b2d-8e1f-1a2b3c4d5e6f")),
        Permission.Create(Roles[1].Id, AppPermission.Platform_Update, Guid.Parse("0196debe-3a03-4b2d-8e1f-1a2b3c4d5e6f")),
        Permission.Create(Roles[1].Id, AppPermission.Platform_Delete, Guid.Parse("0196debe-3a04-4b2d-8e1f-1a2b3c4d5e6f")),
        Permission.Create(Roles[1].Id, AppPermission.Container_View, Guid.Parse("0196debe-3a05-4b2d-8e1f-1a2b3c4d5e6f")),
        Permission.Create(Roles[1].Id, AppPermission.Container_Create, Guid.Parse("0196debe-3a06-4b2d-8e1f-1a2b3c4d5e6f")),
        Permission.Create(Roles[1].Id, AppPermission.Container_Update, Guid.Parse("0196debe-3a07-4b2d-8e1f-1a2b3c4d5e6f")),
        Permission.Create(Roles[1].Id, AppPermission.Container_Delete, Guid.Parse("0196debe-3a08-4b2d-8e1f-1a2b3c4d5e6f")),
        Permission.Create(Roles[1].Id, AppPermission.Network_View, Guid.Parse("0196debe-3a09-4b2d-8e1f-1a2b3c4d5e6f")),
        Permission.Create(Roles[1].Id, AppPermission.Network_Create, Guid.Parse("0196debe-3a0a-4b2d-8e1f-1a2b3c4d5e6f")),
        Permission.Create(Roles[1].Id, AppPermission.Network_Delete, Guid.Parse("0196debe-3a0b-4b2d-8e1f-1a2b3c4d5e6f")),
        Permission.Create(Roles[1].Id, AppPermission.Volume_View, Guid.Parse("0196debe-3a0c-4b2d-8e1f-1a2b3c4d5e6f")),
        Permission.Create(Roles[1].Id, AppPermission.Volume_Create, Guid.Parse("0196debe-3a0d-4b2d-8e1f-1a2b3c4d5e6f")),
        Permission.Create(Roles[1].Id, AppPermission.Volume_Delete, Guid.Parse("0196debe-3a0e-4b2d-8e1f-1a2b3c4d5e6f")),
        Permission.Create(Roles[1].Id, AppPermission.Image_View, Guid.Parse("0196debe-3a0f-4b2d-8e1f-1a2b3c4d5e6f")),
        Permission.Create(Roles[1].Id, AppPermission.Image_Delete, Guid.Parse("0196debe-3a10-4b2d-8e1f-1a2b3c4d5e6f")),
        Permission.Create(Roles[1].Id, AppPermission.Image_Create, Guid.Parse("0196debe-3a11-4b2d-8e1f-1a2b3c4d5e6f")),

        // QA
        Permission.Create(Roles[2].Id, AppPermission.User_View, Guid.Parse("0196debe-3a12-4b2d-8e1f-1a2b3c4d5e6f")),
        Permission.Create(Roles[2].Id, AppPermission.Role_View, Guid.Parse("0196debe-3a13-4b2d-8e1f-1a2b3c4d5e6f")),
        Permission.Create(Roles[2].Id, AppPermission.Team_View, Guid.Parse("0196debe-3a14-4b2d-8e1f-1a2b3c4d5e6f")),
        Permission.Create(Roles[2].Id, AppPermission.Platform_View, Guid.Parse("0196debe-3a15-4b2d-8e1f-1a2b3c4d5e6f")),
        Permission.Create(Roles[2].Id, AppPermission.Container_View, Guid.Parse("0196debe-3a16-4b2d-8e1f-1a2b3c4d5e6f")),
        Permission.Create(Roles[2].Id, AppPermission.Network_View, Guid.Parse("0196debe-3a17-4b2d-8e1f-1a2b3c4d5e6f")),
        Permission.Create(Roles[2].Id, AppPermission.Volume_View, Guid.Parse("0196debe-3a18-4b2d-8e1f-1a2b3c4d5e6f")),
        Permission.Create(Roles[2].Id, AppPermission.Image_View, Guid.Parse("0196debe-3a19-4b2d-8e1f-1a2b3c4d5e6f")),
    ];

    public static Team[] Teams =
    [
        Team.Create("Admins", Roles[0].Id, Guid.Parse("0196dece-e967-7d50-832c-0ca0155465a1")),
        Team.Create("Devs", Roles[1].Id, Guid.Parse("0196dece-e967-7965-afb9-f33ff2b3f0dc")),
        Team.Create("QA", Roles[2].Id, Guid.Parse("0196dece-e967-755f-9c6a-5ba5a34577f1"))
    ];

    public static User[] Users =
    [
        User.Create("admin", "admin@admin.com", "admin123", Guid.Parse("0196ded1-13f1-77ce-884e-3cb636ec09a8"), dateTime),
        User.Create("dev", "dev@dev.com", "dev123dev", Guid.Parse("0196ded1-13f1-73fb-acf0-188115c01c0e"), dateTime),
        User.Create("qa", "qa@qa.com", "qa123qa@", Guid.Parse("0196ded1-13f1-743a-8a1b-5e243048c77e"), dateTime)
    ];

    public static UserTeam[] UsersTeams =
    [
        UserTeam.Create(Users[0].Id, Teams[0].Id),
        UserTeam.Create(Users[1].Id, Teams[1].Id),
        UserTeam.Create(Users[2].Id, Teams[2].Id)
    ];
}