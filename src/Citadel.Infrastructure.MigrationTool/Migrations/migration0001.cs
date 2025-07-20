using FluentMigrator;

namespace Infrastructure.MigrationTool.Migrations;

[Migration(20250701084457)]
public class Migration0001 : Migration
{
    public override void Up()
    {
        PlatformsConfiguration();
        PlatformStatsConfiguration();
        RegistriesConfiguration();
        RolesConfiguration();
        UsersConfiguration();
        PermissionsConfiguration();
        TeamsConfiguration();
        RefreshTokensConfiguration();
        UsersTeamsConfiguration();
        ContainersConfiguration();
        ContainerStatsConfiguration();
    }

    public override void Down()
    {
        Delete.Table("UsersTeams");
        Delete.Table("RefreshTokens");
        Delete.Table("Teams");
        Delete.Table("Permissions");
        Delete.Table("PlatformStats");
        Delete.Table("ContainerStats");
        Delete.Table("Containers");
        Delete.Table("Users");
        Delete.Table("Roles");
        Delete.Table("Registries");
        Delete.Table("Platforms");
    }

    private void PlatformsConfiguration()
    {
        Create.Table("Platforms")
            .WithColumn("Id").AsCustom("TEXT").PrimaryKey()
            .WithColumn("Name").AsString().NotNullable()
            .WithColumn("Address").AsString().NotNullable()
            .WithColumn("Status").AsString().NotNullable()
            .WithColumn("ConnectorType").AsString().NotNullable()
            .WithColumn("NetworkCount").AsInt32().NotNullable()
            .WithColumn("VolumeCount").AsInt32().NotNullable()
            .WithColumn("ImageCount").AsInt32().NotNullable()
            .WithColumn("CpuCount").AsInt32().NotNullable()
            .WithColumn("MemTotal").AsInt64().NotNullable()
            .WithColumn("AgentVersion").AsString().Nullable()
            .WithColumn("ServerVersion").AsString().Nullable()
            .WithColumn("PlatformDescriptor").AsString().NotNullable();

        Create.Index("AddressIndex").OnTable("Platforms").OnColumn("Address").Unique();
    }

    private void RegistriesConfiguration()
    {
        Create.Table("Registries")
           .WithColumn("Id").AsCustom("TEXT").PrimaryKey()
           .WithColumn("Name").AsString().NotNullable()
           .WithColumn("Url").AsString().NotNullable()
           .WithColumn("Created").AsString().NotNullable()
           .WithColumn("Type").AsString().NotNullable()
           .WithColumn("Configuration").AsString().NotNullable();

        Create.Index("IX_Registries_Name").OnTable("Registries").OnColumn("Name").Unique();
    }

    private void RolesConfiguration()
    {
        Create.Table("Roles")
            .WithColumn("Id").AsCustom("TEXT").PrimaryKey()
            .WithColumn("Name").AsString().NotNullable()
            .WithColumn("CreatedAt").AsString().NotNullable().WithDefaultValue("2000-01-01 00:00:00")
            .WithColumn("UpdatedAt").AsString().NotNullable().WithDefaultValue("2000-01-01 00:00:00");
    }

    private void UsersConfiguration()
    {
        Create.Table("Users")
            .WithColumn("Id").AsCustom("TEXT").PrimaryKey()
            .WithColumn("Name").AsString().NotNullable()
            .WithColumn("Email").AsString().NotNullable()
            .WithColumn("Password").AsString().NotNullable()
            .WithColumn("CreatedAt").AsString().NotNullable().WithDefaultValue("2000-01-01 00:00:00")
            .WithColumn("UpdatedAt").AsString().NotNullable().WithDefaultValue("2000-01-01 00:00:00");

        Create.Index("EmailIndex").OnTable("Users").OnColumn("Email").Unique();
    }

    private void PermissionsConfiguration()
    {
        Create.Table("Permissions")
            .WithColumn("Id").AsCustom("TEXT").PrimaryKey()
            .WithColumn("RoleId").AsCustom("TEXT").NotNullable()
                .ForeignKey("Roles", "Id").OnDeleteOrUpdate(System.Data.Rule.Cascade)
            .WithColumn("PermissionCode").AsInt32().NotNullable();

        Create.Index("IX_Permissions_RoleId").OnTable("Permissions").OnColumn("RoleId");
    }

    private void TeamsConfiguration()
    {
        Create.Table("Teams")
            .WithColumn("Id").AsCustom("TEXT").PrimaryKey()
            .WithColumn("RoleId").AsCustom("TEXT").NotNullable()
                .ForeignKey("Roles", "Id").OnDeleteOrUpdate(System.Data.Rule.Cascade)
            .WithColumn("Name").AsString().NotNullable();

        Create.Index("IX_Teams_RoleId").OnTable("Teams").OnColumn("RoleId");
    }

    private void RefreshTokensConfiguration()
    {
        Create.Table("RefreshTokens")
            .WithColumn("Id").AsCustom("TEXT").PrimaryKey()
            .WithColumn("UserId").AsCustom("TEXT").NotNullable()
                .ForeignKey("Users", "Id").OnDeleteOrUpdate(System.Data.Rule.Cascade)
            .WithColumn("CreatedAt").AsString().NotNullable();

        Create.Index("IX_RefreshTokens_UserId").OnTable("RefreshTokens").OnColumn("UserId");
    }

    private void UsersTeamsConfiguration()
    {
        Create.Table("UsersTeams")
            .WithColumn("UserId").AsCustom("TEXT").NotNullable().PrimaryKey()
                .ForeignKey("Users", "Id").OnDeleteOrUpdate(System.Data.Rule.Cascade)
            .WithColumn("TeamId").AsCustom("TEXT").NotNullable().PrimaryKey()
                .ForeignKey("Teams", "Id").OnDeleteOrUpdate(System.Data.Rule.Cascade);

        Create.Index("IX_UsersTeams_TeamId").OnTable("UsersTeams").OnColumn("TeamId");
        Create.Index("IX_UsersTeams_UserId").OnTable("UsersTeams").OnColumn("UserId");
    }

    private void ContainersConfiguration()
    {
        Create.Table("Containers")
            .WithColumn("Id").AsCustom("TEXT").PrimaryKey()
            .WithColumn("PlatformId").AsCustom("TEXT").NotNullable()
                .ForeignKey("FK_Containers_Platforms", "Platforms", "Id").OnDeleteOrUpdate(System.Data.Rule.Cascade)
            .WithColumn("ContainerId").AsString(64).NotNullable()
            .WithColumn("Name").AsString().NotNullable()
            .WithColumn("Image").AsString().NotNullable()
            .WithColumn("Created").AsInt64().NotNullable()
            .WithColumn("Updated").AsInt64().NotNullable()
            .WithColumn("State").AsString().NotNullable()
            .WithColumn("Stack").AsString().Nullable()
            .WithColumn("Ports").AsCustom("TEXT").NotNullable();

        Create.Index("IX_Containers_ContainerId").OnTable("Containers").OnColumn("ContainerId").Unique();
        Create.Index("IX_Containers_PlatformId").OnTable("Containers").OnColumn("PlatformId");
    }

    private void ContainerStatsConfiguration()
    {
        Create.Table("ContainerStats")
            .WithColumn("Id").AsCustom("TEXT").PrimaryKey()
            .WithColumn("ContainerId").AsCustom("TEXT").NotNullable()
                .ForeignKey("Containers", "Id").OnDeleteOrUpdate(System.Data.Rule.Cascade)
            .WithColumn("Created").AsInt64().NotNullable()
            .WithColumn("MemoryUsage").AsCustom("REAL").Nullable()
            .WithColumn("CpuUsage").AsCustom("REAL").Nullable()
            .WithColumn("MemoryLimit").AsCustom("REAL").Nullable()
            .WithColumn("RxBytes").AsCustom("REAL").Nullable()
            .WithColumn("TxBytes").AsCustom("REAL").Nullable();

        Create.Index("IX_ContainerStats_ContainerId").OnTable("ContainerStats").OnColumn("ContainerId");
    }

    private void PlatformStatsConfiguration()
    {
        Create.Table("PlatformStats")
            .WithColumn("Id").AsCustom("TEXT").PrimaryKey()
            .WithColumn("PlatformId").AsCustom("TEXT").NotNullable()
                .ForeignKey("FK_PlatformStats_Platforms", "Platforms", "Id").OnDeleteOrUpdate(System.Data.Rule.Cascade)
            .WithColumn("Created").AsInt64().NotNullable()
            .WithColumn("MemoryUsage").AsCustom("REAL").NotNullable()
            .WithColumn("CpuUsage").AsCustom("REAL").NotNullable()
            .WithColumn("RxBytes").AsCustom("REAL").NotNullable()
            .WithColumn("TxBytes").AsCustom("REAL").NotNullable();

        Create.Index("IX_PlatformStats_PlatformId").OnTable("PlatformStats").OnColumn("PlatformId");
        Create.Index("IX_PlatformStats_Created").OnTable("PlatformStats").OnColumn("Created").Unique();
    }
}