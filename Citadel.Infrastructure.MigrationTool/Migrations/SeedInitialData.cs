using FluentMigrator;

namespace Infrastructure.MigrationTool.Migrations;

[Migration(20250701084487)]
public class SeedInitialData : Migration
{
    public override void Up()
    {
        // --- Roles ---
        Insert.IntoTable("Roles").Row(new
        {
            Id = "bdde9601-3b03-1275-a11b-98533d063a04",
            Name = "Admin",
            CreatedAt = "2025-01-01 00:00:00",
            UpdatedAt = "2025-01-01 00:00:00"
        });

        Insert.IntoTable("Roles").Row(new
        {
            Id = "bede9601-940c-6774-9b2d-397e0200276f",
            Name = "Dev",
            CreatedAt = "2025-01-01 00:00:00",
            UpdatedAt = "2025-01-01 00:00:00"
        });

        Insert.IntoTable("Roles").Row(new
        {
            Id = "bede9601-802d-dd76-b351-ade38fa29169",
            Name = "QA",
            CreatedAt = "2025-01-01 00:00:00",
            UpdatedAt = "2025-01-01 00:00:00"
        });

        // --- Users ---
        Insert.IntoTable("Users").Row(new
        {
            Id = "d1de9601-f113-ce77-884e-3cb636ec09a8",
            Name = "admin",
            Email = "admin@admin.com",
            Password = "o6hWzZ+DIuSZoHNjf5D1t6101vfm4w2kmPRiAZ3Xq53JMMl1",
            CreatedAt = "2025-01-01 00:00:00",
            UpdatedAt = "2025-01-01 00:00:00"
        });

        Insert.IntoTable("Users").Row(new
        {
            Id = "d1de9601-f113-fb73-acf0-188115c01c0e",
            Name = "dev",
            Email = "dev@dev.com",
            Password = "eBnEWmOx4+wNHGR/Tunt+Sz5y7y3CQxufbe3lO1vOKwFCrft",
            CreatedAt = "2025-01-01 00:00:00",
            UpdatedAt = "2025-01-01 00:00:00"
        });

        Insert.IntoTable("Users").Row(new
        {
            Id = "d1de9601-f113-3a74-8a1b-5e243048c77e",
            Name = "qa",
            Email = "qa@qa.com",
            Password = "MmjMzZZgclu4JrBkm1SbuTKP52DJncsGDuj+/NJe1VW3alHk",
            CreatedAt = "2025-01-01 00:00:00",
            UpdatedAt = "2025-01-01 00:00:00"
        });

        // --- Teams ---
        Insert.IntoTable("Teams").Row(new
        {
            Id = "cede9601-67e9-507d-832c-0ca0155465a1",
            Name = "Admins",
            RoleId = "bdde9601-3b03-1275-a11b-98533d063a04"
        });

        Insert.IntoTable("Teams").Row(new
        {
            Id = "cede9601-67e9-6579-afb9-f33ff2b3f0dc",
            Name = "Devs",
            RoleId = "bede9601-940c-6774-9b2d-397e0200276f"
        });

        Insert.IntoTable("Teams").Row(new
        {
            Id = "cede9601-67e9-5f75-9c6a-5ba5a34577f1",
            Name = "QA",
            RoleId = "bede9601-802d-dd76-b351-ade38fa29169"
        });

        // --- UsersTeams ---
        Insert.IntoTable("UsersTeams").Row(new
        {
            UserId = "d1de9601-f113-ce77-884e-3cb636ec09a8",
            TeamId = "cede9601-67e9-507d-832c-0ca0155465a1"
        });

        Insert.IntoTable("UsersTeams").Row(new
        {
            UserId = "d1de9601-f113-fb73-acf0-188115c01c0e",
            TeamId = "cede9601-67e9-6579-afb9-f33ff2b3f0dc"
        });

        Insert.IntoTable("UsersTeams").Row(new
        {
            UserId = "d1de9601-f113-3a74-8a1b-5e243048c77e",
            TeamId = "cede9601-67e9-5f75-9c6a-5ba5a34577f1"
        });

        // --- Permissions (sample) ---
        Insert.IntoTable("Permissions").Row(new
        {
            Id = "bede9601-0d3a-2d4b-8e1f-1a2b3c4d5e6f",
            RoleId = "bede9601-940c-6774-9b2d-397e0200276f",
            PermissionCode = 26
        });

        Insert.IntoTable("Permissions").Row(new
        {
            Id = "bede9601-153a-2d4b-8e1f-1a2b3c4d5e6f",
            RoleId = "bede9601-802d-dd76-b351-ade38fa29169",
            PermissionCode = 13
        });

    }

    public override void Down()
    {
        Delete.FromTable("UsersTeams").AllRows();
        Delete.FromTable("Permissions").AllRows();
        Delete.FromTable("Teams").AllRows();
        Delete.FromTable("Users").AllRows();
        Delete.FromTable("Roles").AllRows();
    }
}
