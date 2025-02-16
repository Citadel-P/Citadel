using System;
using Microsoft.EntityFrameworkCore.Migrations;

#nullable disable

#pragma warning disable CA1814 // Prefer jagged arrays over multidimensional

namespace Infrastructure.Migrations
{
    /// <inheritdoc />
    public partial class migration0001 : Migration
    {
        /// <inheritdoc />
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.CreateTable(
                name: "Platforms",
                columns: table => new
                {
                    Id = table.Column<Guid>(type: "TEXT", nullable: false),
                    Name = table.Column<string>(type: "TEXT", maxLength: 128, nullable: true),
                    Address = table.Column<string>(type: "TEXT", maxLength: 128, nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Platforms", x => x.Id);
                });

            migrationBuilder.CreateTable(
                name: "Registries",
                columns: table => new
                {
                    Id = table.Column<Guid>(type: "TEXT", nullable: false),
                    Name = table.Column<string>(type: "TEXT", maxLength: 128, nullable: false),
                    Url = table.Column<string>(type: "TEXT", maxLength: 256, nullable: false),
                    Created = table.Column<DateTime>(type: "TEXT", nullable: false),
                    Discriminator = table.Column<string>(type: "TEXT", nullable: false),
                    Configuration = table.Column<string>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Registries", x => x.Id);
                });

            migrationBuilder.CreateTable(
                name: "Roles",
                columns: table => new
                {
                    Id = table.Column<Guid>(type: "TEXT", nullable: false),
                    Name = table.Column<string>(type: "TEXT", maxLength: 128, nullable: false),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 2, 10, 23, 50, 16, 624, DateTimeKind.Utc).AddTicks(208)),
                    UpdatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 2, 10, 23, 50, 16, 625, DateTimeKind.Utc).AddTicks(3685))
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Roles", x => x.Id);
                });

            migrationBuilder.CreateTable(
                name: "Users",
                columns: table => new
                {
                    Id = table.Column<Guid>(type: "TEXT", nullable: false),
                    Name = table.Column<string>(type: "TEXT", maxLength: 128, nullable: true),
                    Email = table.Column<string>(type: "TEXT", maxLength: 128, nullable: false),
                    Password = table.Column<string>(type: "TEXT", maxLength: 128, nullable: false),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 2, 10, 23, 50, 16, 641, DateTimeKind.Utc).AddTicks(1243)),
                    UpdatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 2, 10, 23, 50, 16, 641, DateTimeKind.Utc).AddTicks(1959))
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Users", x => x.Id);
                });

            migrationBuilder.CreateTable(
                name: "ContainersInfo",
                columns: table => new
                {
                    Id = table.Column<Guid>(type: "TEXT", nullable: false),
                    PlatformId = table.Column<Guid>(type: "TEXT", nullable: false),
                    ContainerId = table.Column<string>(type: "TEXT", maxLength: 64, nullable: true),
                    Name = table.Column<string>(type: "TEXT", nullable: true),
                    Image = table.Column<string>(type: "TEXT", nullable: true),
                    Created = table.Column<long>(type: "INTEGER", nullable: false),
                    State = table.Column<string>(type: "TEXT", nullable: true),
                    Status = table.Column<string>(type: "TEXT", nullable: true),
                    Labels = table.Column<string>(type: "TEXT", nullable: true),
                    Ports = table.Column<string>(type: "TEXT", nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_ContainersInfo", x => x.Id);
                    table.ForeignKey(
                        name: "FK_ContainersInfo_Platforms_PlatformId",
                        column: x => x.PlatformId,
                        principalTable: "Platforms",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "PlatformStats",
                columns: table => new
                {
                    Id = table.Column<Guid>(type: "TEXT", nullable: false),
                    PlatformId = table.Column<Guid>(type: "TEXT", nullable: false),
                    Created = table.Column<long>(type: "INTEGER", nullable: false),
                    MemoryUsage = table.Column<double>(type: "REAL", nullable: false),
                    CpuUsage = table.Column<double>(type: "REAL", nullable: false),
                    RxBytes = table.Column<double>(type: "REAL", nullable: false),
                    TxBytes = table.Column<double>(type: "REAL", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_PlatformStats", x => x.Id);
                    table.ForeignKey(
                        name: "FK_PlatformStats_Platforms_PlatformId",
                        column: x => x.PlatformId,
                        principalTable: "Platforms",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "SystemsInfo",
                columns: table => new
                {
                    Id = table.Column<Guid>(type: "TEXT", nullable: false),
                    PlatformId = table.Column<Guid>(type: "TEXT", nullable: false),
                    DaemonId = table.Column<string>(type: "TEXT", nullable: true),
                    NetworksCount = table.Column<int>(type: "INTEGER", nullable: false),
                    VolumesCount = table.Column<int>(type: "INTEGER", nullable: false),
                    Containers = table.Column<long>(type: "INTEGER", nullable: false),
                    ContainersRunning = table.Column<long>(type: "INTEGER", nullable: false),
                    ContainersPaused = table.Column<long>(type: "INTEGER", nullable: false),
                    ContainersStopped = table.Column<long>(type: "INTEGER", nullable: false),
                    Images = table.Column<long>(type: "INTEGER", nullable: false),
                    Driver = table.Column<string>(type: "TEXT", nullable: true),
                    OperatingSystem = table.Column<string>(type: "TEXT", maxLength: 128, nullable: true),
                    OsVersion = table.Column<string>(type: "TEXT", maxLength: 128, nullable: true),
                    OsType = table.Column<string>(type: "TEXT", maxLength: 128, nullable: true),
                    Architecture = table.Column<string>(type: "TEXT", maxLength: 128, nullable: true),
                    Ncpu = table.Column<long>(type: "INTEGER", nullable: false),
                    MemTotal = table.Column<long>(type: "INTEGER", nullable: false),
                    ServerVersion = table.Column<string>(type: "TEXT", maxLength: 32, nullable: true),
                    AgentVersion = table.Column<string>(type: "TEXT", nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_SystemsInfo", x => x.Id);
                    table.ForeignKey(
                        name: "FK_SystemsInfo_Platforms_PlatformId",
                        column: x => x.PlatformId,
                        principalTable: "Platforms",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "Permissions",
                columns: table => new
                {
                    Id = table.Column<Guid>(type: "TEXT", nullable: false),
                    RoleId = table.Column<Guid>(type: "TEXT", nullable: false),
                    PermissionCode = table.Column<int>(type: "INTEGER", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Permissions", x => x.Id);
                    table.ForeignKey(
                        name: "FK_Permissions_Roles_RoleId",
                        column: x => x.RoleId,
                        principalTable: "Roles",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "Teams",
                columns: table => new
                {
                    Id = table.Column<Guid>(type: "TEXT", nullable: false),
                    RoleId = table.Column<Guid>(type: "TEXT", nullable: false),
                    Name = table.Column<string>(type: "TEXT", maxLength: 128, nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Teams", x => x.Id);
                    table.ForeignKey(
                        name: "FK_Teams_Roles_RoleId",
                        column: x => x.RoleId,
                        principalTable: "Roles",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "RefreshTokens",
                columns: table => new
                {
                    Id = table.Column<Guid>(type: "TEXT", nullable: false),
                    UserId = table.Column<Guid>(type: "TEXT", nullable: false),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_RefreshTokens", x => x.Id);
                    table.ForeignKey(
                        name: "FK_RefreshTokens_Users_UserId",
                        column: x => x.UserId,
                        principalTable: "Users",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "ContainerStats",
                columns: table => new
                {
                    Id = table.Column<Guid>(type: "TEXT", nullable: false),
                    ContainerInfoId = table.Column<Guid>(type: "TEXT", nullable: false),
                    Created = table.Column<long>(type: "INTEGER", nullable: false),
                    MemoryUsage = table.Column<double>(type: "REAL", nullable: true),
                    CpuUsage = table.Column<double>(type: "REAL", nullable: true),
                    MemoryLimit = table.Column<double>(type: "REAL", nullable: true),
                    RxBytes = table.Column<long>(type: "INTEGER", nullable: true),
                    TxBytes = table.Column<long>(type: "INTEGER", nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_ContainerStats", x => x.Id);
                    table.ForeignKey(
                        name: "FK_ContainerStats_ContainersInfo_ContainerInfoId",
                        column: x => x.ContainerInfoId,
                        principalTable: "ContainersInfo",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "SwarmsInfo",
                columns: table => new
                {
                    Id = table.Column<Guid>(type: "TEXT", nullable: false),
                    SystemInfoId = table.Column<Guid>(type: "TEXT", nullable: false),
                    NodeID = table.Column<string>(type: "TEXT", maxLength: 128, nullable: true),
                    NodeAddr = table.Column<string>(type: "TEXT", maxLength: 128, nullable: true),
                    LocalNodeState = table.Column<string>(type: "TEXT", maxLength: 128, nullable: true),
                    ControlAvailable = table.Column<bool>(type: "INTEGER", nullable: false),
                    Error = table.Column<string>(type: "TEXT", nullable: true),
                    Nodes = table.Column<long>(type: "INTEGER", nullable: false),
                    Managers = table.Column<long>(type: "INTEGER", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_SwarmsInfo", x => x.Id);
                    table.ForeignKey(
                        name: "FK_SwarmsInfo_SystemsInfo_SystemInfoId",
                        column: x => x.SystemInfoId,
                        principalTable: "SystemsInfo",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "UsersTeams",
                columns: table => new
                {
                    UserId = table.Column<Guid>(type: "TEXT", nullable: false),
                    TeamId = table.Column<Guid>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_UsersTeams", x => new { x.TeamId, x.UserId });
                    table.ForeignKey(
                        name: "FK_UsersTeams_Teams_TeamId",
                        column: x => x.TeamId,
                        principalTable: "Teams",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Cascade);
                    table.ForeignKey(
                        name: "FK_UsersTeams_Users_UserId",
                        column: x => x.UserId,
                        principalTable: "Users",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "SwarmsPeer",
                columns: table => new
                {
                    Id = table.Column<Guid>(type: "TEXT", nullable: false),
                    SwarmInfoId = table.Column<Guid>(type: "TEXT", nullable: false),
                    NodeID = table.Column<string>(type: "TEXT", maxLength: 64, nullable: true),
                    Addr = table.Column<string>(type: "TEXT", maxLength: 64, nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_SwarmsPeer", x => x.Id);
                    table.ForeignKey(
                        name: "FK_SwarmsPeer_SwarmsInfo_SwarmInfoId",
                        column: x => x.SwarmInfoId,
                        principalTable: "SwarmsInfo",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.InsertData(
                table: "Roles",
                columns: new[] { "Id", "CreatedAt", "Name" },
                values: new object[,]
                {
                    { new Guid("0194f245-5105-782d-811a-0a383b34545c"), new DateTime(2025, 2, 10, 23, 50, 16, 581, DateTimeKind.Utc).AddTicks(7858), "QA" },
                    { new Guid("0194f245-5105-78f5-8a31-ae761f22c91a"), new DateTime(2025, 2, 10, 23, 50, 16, 581, DateTimeKind.Utc).AddTicks(7554), "Administrator" },
                    { new Guid("0194f245-5105-7ca0-8a8c-23762504af10"), new DateTime(2025, 2, 10, 23, 50, 16, 581, DateTimeKind.Utc).AddTicks(7855), "Developer" }
                });

            migrationBuilder.InsertData(
                table: "Users",
                columns: new[] { "Id", "CreatedAt", "Email", "Name", "Password" },
                values: new object[,]
                {
                    { new Guid("0194f245-5106-71d7-bd7c-ca3483dfcd75"), new DateTime(2025, 2, 10, 23, 50, 16, 582, DateTimeKind.Utc).AddTicks(6635), "admin@admin.com", "admin", "6EJq+e3UFqtQXMa+2/ZxFaYyn/nKHjyIYW/ZpGzgJz5V3o+k" },
                    { new Guid("0194f245-5116-7d06-89ee-97ae43361765"), new DateTime(2025, 2, 10, 23, 50, 16, 598, DateTimeKind.Utc).AddTicks(6997), "dev@dev.com", "dev", "bdHMw/kQ/kYRfZ1ikpvF2BHurtom3D33Wa7cJmCoKhuyJm56" },
                    { new Guid("0194f245-511c-7b83-add7-cc843e2d92bc"), new DateTime(2025, 2, 10, 23, 50, 16, 604, DateTimeKind.Utc).AddTicks(3295), "qa@qa.com", "qa", "0ZWDIz/6zoO80i+Lm7LxBMO/3QyDQ1r08+k0JnR2QuyrQ7HA" }
                });

            migrationBuilder.InsertData(
                table: "Permissions",
                columns: new[] { "Id", "PermissionCode", "RoleId" },
                values: new object[,]
                {
                    { new Guid("0194f245-5105-7030-adbb-e066f59fc27f"), 19, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-7037-b5aa-89800f6a22da"), 2, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-7060-be70-1f31f4b4dbde"), 4, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-7084-b56c-87ca341b6c5c"), 1, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-70d3-a2d3-9a50fff38548"), 14, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-7148-887e-c5aba21138f2"), 19, new Guid("0194f245-5105-7ca0-8a8c-23762504af10") },
                    { new Guid("0194f245-5105-714f-b97d-9ab2c5bd9934"), 13, new Guid("0194f245-5105-782d-811a-0a383b34545c") },
                    { new Guid("0194f245-5105-71f9-ba75-29b4d9ffaae1"), 20, new Guid("0194f245-5105-7ca0-8a8c-23762504af10") },
                    { new Guid("0194f245-5105-721a-b481-bf810479a84d"), 16, new Guid("0194f245-5105-7ca0-8a8c-23762504af10") },
                    { new Guid("0194f245-5105-7246-908d-ce248e9af211"), 18, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-7247-bc43-7947827e37a3"), 21, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-7269-afa4-7c92d3ffd677"), 5, new Guid("0194f245-5105-7ca0-8a8c-23762504af10") },
                    { new Guid("0194f245-5105-7321-ab9c-0cf76826ac0f"), 17, new Guid("0194f245-5105-7ca0-8a8c-23762504af10") },
                    { new Guid("0194f245-5105-732b-8d35-3f8f95dcd1c1"), 25, new Guid("0194f245-5105-782d-811a-0a383b34545c") },
                    { new Guid("0194f245-5105-7358-a689-95268f10aa14"), 22, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-737e-b4c7-3252f09563e8"), 9, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-7389-9875-d920b328407b"), 13, new Guid("0194f245-5105-7ca0-8a8c-23762504af10") },
                    { new Guid("0194f245-5105-73bc-aba7-c75da66ffa7a"), 18, new Guid("0194f245-5105-7ca0-8a8c-23762504af10") },
                    { new Guid("0194f245-5105-73d6-a249-61ad258287b8"), 6, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-74b0-a916-b95a0894913e"), 24, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-74b9-b659-82e565a1652e"), 1, new Guid("0194f245-5105-7ca0-8a8c-23762504af10") },
                    { new Guid("0194f245-5105-74dd-949b-4d5cabf6a13e"), 21, new Guid("0194f245-5105-782d-811a-0a383b34545c") },
                    { new Guid("0194f245-5105-752a-bb1e-3c46898d8f3c"), 10, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-7531-a8f2-6c45f65c71b1"), 25, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-755a-ab1b-7fefd6df4d66"), 16, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-75a0-8d64-4057f50f2624"), 5, new Guid("0194f245-5105-782d-811a-0a383b34545c") },
                    { new Guid("0194f245-5105-7677-b28a-f6059916cb8e"), 7, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-768a-a308-3ebd23a385e7"), 1, new Guid("0194f245-5105-782d-811a-0a383b34545c") },
                    { new Guid("0194f245-5105-76c0-bdb0-c25e2db31bbe"), 11, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-76c4-8093-ac3c8e386848"), 13, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-7707-a324-070ec3b9727d"), 27, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-7783-9f56-e8bb539ee38a"), 26, new Guid("0194f245-5105-7ca0-8a8c-23762504af10") },
                    { new Guid("0194f245-5105-77a3-b07e-9b415cbf6be2"), 28, new Guid("0194f245-5105-7ca0-8a8c-23762504af10") },
                    { new Guid("0194f245-5105-780e-ae6b-c58000f3d032"), 17, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-78c4-b810-677776b43aa3"), 23, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-78ec-b52c-7386cd06f67d"), 20, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-79db-a133-a007eeddefb0"), 28, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-7a63-878c-269351dee3b9"), 9, new Guid("0194f245-5105-782d-811a-0a383b34545c") },
                    { new Guid("0194f245-5105-7ab4-a312-0785d8d7455c"), 21, new Guid("0194f245-5105-7ca0-8a8c-23762504af10") },
                    { new Guid("0194f245-5105-7af2-a3b8-d11db798bb25"), 8, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-7b43-9d73-70b87dfbaaae"), 17, new Guid("0194f245-5105-782d-811a-0a383b34545c") },
                    { new Guid("0194f245-5105-7b6d-81c0-19ab12e4579f"), 22, new Guid("0194f245-5105-7ca0-8a8c-23762504af10") },
                    { new Guid("0194f245-5105-7bcc-afc2-c4fc3e415e94"), 12, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-7c69-ac75-2f8254e981ff"), 15, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-7cfd-8b09-934647c27f7b"), 27, new Guid("0194f245-5105-7ca0-8a8c-23762504af10") },
                    { new Guid("0194f245-5105-7e32-9034-725e86893387"), 15, new Guid("0194f245-5105-7ca0-8a8c-23762504af10") },
                    { new Guid("0194f245-5105-7e78-9385-3dab3ad972a6"), 14, new Guid("0194f245-5105-7ca0-8a8c-23762504af10") },
                    { new Guid("0194f245-5105-7f0c-927d-46f543fbaa3f"), 5, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-7f19-9768-94fed7fac0b1"), 26, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-7f26-beb4-0989e7e44a97"), 3, new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") },
                    { new Guid("0194f245-5105-7f63-8cee-8c09f68ada0a"), 9, new Guid("0194f245-5105-7ca0-8a8c-23762504af10") },
                    { new Guid("0194f245-5105-7ffc-9bd5-fbd5170f2177"), 24, new Guid("0194f245-5105-7ca0-8a8c-23762504af10") }
                });

            migrationBuilder.InsertData(
                table: "Teams",
                columns: new[] { "Id", "Name", "RoleId" },
                values: new object[,]
                {
                    { new Guid("0194f245-5106-7205-895a-bdea97f88960"), "Devs", new Guid("0194f245-5105-7ca0-8a8c-23762504af10") },
                    { new Guid("0194f245-5106-7756-b770-75b7bcfb0bef"), "QA", new Guid("0194f245-5105-782d-811a-0a383b34545c") },
                    { new Guid("0194f245-5106-7fef-8188-dc5471e8863a"), "Admins", new Guid("0194f245-5105-78f5-8a31-ae761f22c91a") }
                });

            migrationBuilder.InsertData(
                table: "UsersTeams",
                columns: new[] { "TeamId", "UserId" },
                values: new object[,]
                {
                    { new Guid("0194f245-5106-7205-895a-bdea97f88960"), new Guid("0194f245-5116-7d06-89ee-97ae43361765") },
                    { new Guid("0194f245-5106-7756-b770-75b7bcfb0bef"), new Guid("0194f245-511c-7b83-add7-cc843e2d92bc") },
                    { new Guid("0194f245-5106-7fef-8188-dc5471e8863a"), new Guid("0194f245-5106-71d7-bd7c-ca3483dfcd75") }
                });

            migrationBuilder.CreateIndex(
                name: "IX_ContainersInfo_ContainerId",
                table: "ContainersInfo",
                column: "ContainerId",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_ContainersInfo_PlatformId",
                table: "ContainersInfo",
                column: "PlatformId");

            migrationBuilder.CreateIndex(
                name: "IX_ContainerStats_ContainerInfoId",
                table: "ContainerStats",
                column: "ContainerInfoId");

            migrationBuilder.CreateIndex(
                name: "IX_Permissions_RoleId",
                table: "Permissions",
                column: "RoleId");

            migrationBuilder.CreateIndex(
                name: "AddressIndex",
                table: "Platforms",
                column: "Address",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_PlatformStats_Created",
                table: "PlatformStats",
                column: "Created",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_PlatformStats_PlatformId",
                table: "PlatformStats",
                column: "PlatformId");

            migrationBuilder.CreateIndex(
                name: "IX_RefreshTokens_UserId",
                table: "RefreshTokens",
                column: "UserId");

            migrationBuilder.CreateIndex(
                name: "IX_SwarmsInfo_SystemInfoId",
                table: "SwarmsInfo",
                column: "SystemInfoId",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_SwarmsPeer_SwarmInfoId",
                table: "SwarmsPeer",
                column: "SwarmInfoId");

            migrationBuilder.CreateIndex(
                name: "IX_SystemsInfo_PlatformId",
                table: "SystemsInfo",
                column: "PlatformId",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_Teams_RoleId",
                table: "Teams",
                column: "RoleId");

            migrationBuilder.CreateIndex(
                name: "EmailIndex",
                table: "Users",
                column: "Email",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_UsersTeams_UserId",
                table: "UsersTeams",
                column: "UserId");
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropTable(
                name: "ContainerStats");

            migrationBuilder.DropTable(
                name: "Permissions");

            migrationBuilder.DropTable(
                name: "PlatformStats");

            migrationBuilder.DropTable(
                name: "RefreshTokens");

            migrationBuilder.DropTable(
                name: "Registries");

            migrationBuilder.DropTable(
                name: "SwarmsPeer");

            migrationBuilder.DropTable(
                name: "UsersTeams");

            migrationBuilder.DropTable(
                name: "ContainersInfo");

            migrationBuilder.DropTable(
                name: "SwarmsInfo");

            migrationBuilder.DropTable(
                name: "Teams");

            migrationBuilder.DropTable(
                name: "Users");

            migrationBuilder.DropTable(
                name: "SystemsInfo");

            migrationBuilder.DropTable(
                name: "Roles");

            migrationBuilder.DropTable(
                name: "Platforms");
        }
    }
}
