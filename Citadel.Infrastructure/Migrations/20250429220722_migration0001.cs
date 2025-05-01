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
                    Address = table.Column<string>(type: "TEXT", maxLength: 128, nullable: true),
                    DaemonId = table.Column<string>(type: "TEXT", nullable: true),
                    Status = table.Column<string>(type: "TEXT", nullable: false),
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
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 4, 29, 22, 7, 21, 972, DateTimeKind.Utc).AddTicks(5232)),
                    UpdatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 4, 29, 22, 7, 21, 974, DateTimeKind.Utc).AddTicks(3070))
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
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 4, 29, 22, 7, 21, 992, DateTimeKind.Utc).AddTicks(2321)),
                    UpdatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 4, 29, 22, 7, 21, 992, DateTimeKind.Utc).AddTicks(3244))
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
                    State = table.Column<string>(type: "TEXT", nullable: false),
                    Stack = table.Column<string>(type: "TEXT", nullable: true),
                    Status = table.Column<string>(type: "TEXT", nullable: true),
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
                name: "SwarmsInfo",
                columns: table => new
                {
                    Id = table.Column<Guid>(type: "TEXT", nullable: false),
                    PlatformId = table.Column<Guid>(type: "TEXT", nullable: false),
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
                        name: "FK_SwarmsInfo_Platforms_PlatformId",
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

            migrationBuilder.InsertData(
                table: "Roles",
                columns: new[] { "Id", "CreatedAt", "Name" },
                values: new object[,]
                {
                    { new Guid("01968397-20e1-7549-bd8f-2783ebd4a988"), new DateTime(2025, 4, 29, 22, 7, 21, 825, DateTimeKind.Utc).AddTicks(3568), "QA" },
                    { new Guid("01968397-20e1-768b-ba1e-75aca20fd1e0"), new DateTime(2025, 4, 29, 22, 7, 21, 825, DateTimeKind.Utc).AddTicks(3565), "Developer" },
                    { new Guid("01968397-20e1-78f5-ba61-f6d18b58e966"), new DateTime(2025, 4, 29, 22, 7, 21, 825, DateTimeKind.Utc).AddTicks(3257), "Administrator" }
                });

            migrationBuilder.InsertData(
                table: "Users",
                columns: new[] { "Id", "CreatedAt", "Email", "Name", "Password" },
                values: new object[,]
                {
                    { new Guid("01968397-20e1-75ce-9292-2b1010dcd92c"), new DateTime(2025, 4, 29, 22, 7, 21, 826, DateTimeKind.Utc).AddTicks(584), "admin@admin.com", "admin", "z3+rEL+8qBOywW3oL7ibQ1XzZeYenTfhqoQBDICpKUqqHiWg" },
                    { new Guid("01968397-20ed-73e1-905f-dc138a2c0bde"), new DateTime(2025, 4, 29, 22, 7, 21, 837, DateTimeKind.Utc).AddTicks(8537), "dev@dev.com", "dev", "6kQsexQTUZFZhmDTmCjL/y8D3SXPsULBd81ivyKZdVNbrnzz" },
                    { new Guid("01968397-20f3-7ab4-b0b3-649133fac306"), new DateTime(2025, 4, 29, 22, 7, 21, 843, DateTimeKind.Utc).AddTicks(9246), "qa@qa.com", "qa", "GKgD3xbni671kItljE6VOEhjuprtOIHBIRdMRPd0OQBZABzI" }
                });

            migrationBuilder.InsertData(
                table: "Permissions",
                columns: new[] { "Id", "PermissionCode", "RoleId" },
                values: new object[,]
                {
                    { new Guid("01968397-20e1-7055-9b45-c053caf66230"), 19, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-709b-bd47-0b8166ecf686"), 12, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-70c5-892b-ef4f871506bd"), 1, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-70c5-be1b-6c87796d12b5"), 5, new Guid("01968397-20e1-768b-ba1e-75aca20fd1e0") },
                    { new Guid("01968397-20e1-717b-af0e-8822f6729da2"), 26, new Guid("01968397-20e1-768b-ba1e-75aca20fd1e0") },
                    { new Guid("01968397-20e1-71dc-aa7f-b46897fa667b"), 13, new Guid("01968397-20e1-768b-ba1e-75aca20fd1e0") },
                    { new Guid("01968397-20e1-71e3-9cf0-f225dcffb977"), 14, new Guid("01968397-20e1-768b-ba1e-75aca20fd1e0") },
                    { new Guid("01968397-20e1-71e3-b1f8-a529651df6e1"), 22, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-720e-8af7-fdbfc39c5c62"), 6, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-721f-b65c-9b2d5d2cffe5"), 21, new Guid("01968397-20e1-7549-bd8f-2783ebd4a988") },
                    { new Guid("01968397-20e1-7221-aa45-68b9514b36c6"), 15, new Guid("01968397-20e1-768b-ba1e-75aca20fd1e0") },
                    { new Guid("01968397-20e1-7314-a253-c35f0ae4d117"), 20, new Guid("01968397-20e1-768b-ba1e-75aca20fd1e0") },
                    { new Guid("01968397-20e1-7326-b0ef-4611b5727c7f"), 14, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-7329-8fa6-0b102d7a2507"), 17, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-734a-bdeb-8158e83d5732"), 28, new Guid("01968397-20e1-768b-ba1e-75aca20fd1e0") },
                    { new Guid("01968397-20e1-7359-a096-f9e26bd136b6"), 1, new Guid("01968397-20e1-768b-ba1e-75aca20fd1e0") },
                    { new Guid("01968397-20e1-736b-bf4e-82375c3b3088"), 13, new Guid("01968397-20e1-7549-bd8f-2783ebd4a988") },
                    { new Guid("01968397-20e1-738d-a232-335814efec6d"), 8, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-7398-a54a-fcf8769d4be8"), 24, new Guid("01968397-20e1-768b-ba1e-75aca20fd1e0") },
                    { new Guid("01968397-20e1-73fb-ac2d-160cd970793b"), 17, new Guid("01968397-20e1-7549-bd8f-2783ebd4a988") },
                    { new Guid("01968397-20e1-74f6-9db0-bc4dd2960510"), 16, new Guid("01968397-20e1-768b-ba1e-75aca20fd1e0") },
                    { new Guid("01968397-20e1-74fa-aa81-31a5e5e3d327"), 28, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-754f-84a8-bcc1e310ea9e"), 9, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-7624-9991-3bb45e2960dd"), 21, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-763a-8ab0-cf36a8c77ba8"), 1, new Guid("01968397-20e1-7549-bd8f-2783ebd4a988") },
                    { new Guid("01968397-20e1-76eb-989f-a83a04a5d558"), 9, new Guid("01968397-20e1-7549-bd8f-2783ebd4a988") },
                    { new Guid("01968397-20e1-7761-987c-b26ee9f21dd9"), 11, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-7797-9522-09da5270fef8"), 13, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-779b-a217-a6b7b9589844"), 27, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-77db-9e82-4efce9d44505"), 9, new Guid("01968397-20e1-768b-ba1e-75aca20fd1e0") },
                    { new Guid("01968397-20e1-782b-a657-33d489003914"), 2, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-782e-b20f-fc9b67c5c167"), 22, new Guid("01968397-20e1-768b-ba1e-75aca20fd1e0") },
                    { new Guid("01968397-20e1-78ef-aa95-963dd4cd7409"), 5, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-7951-8ad3-b89889f890e0"), 15, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-7a66-ba32-c1fa2938b6c1"), 27, new Guid("01968397-20e1-768b-ba1e-75aca20fd1e0") },
                    { new Guid("01968397-20e1-7a80-9fa3-b23e80513292"), 17, new Guid("01968397-20e1-768b-ba1e-75aca20fd1e0") },
                    { new Guid("01968397-20e1-7b81-86ea-5cbe6b42f7b8"), 4, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-7bc2-ae0f-7d582c3d670d"), 26, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-7cbb-933e-a79690e3cb0d"), 24, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-7d43-951e-24a7f5d5fb65"), 3, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-7da8-89d8-22936106577f"), 23, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-7db9-9d60-8c574eab49b9"), 10, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-7dc3-8751-baefcae9d7fe"), 20, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-7df6-82c2-3135354a918e"), 18, new Guid("01968397-20e1-768b-ba1e-75aca20fd1e0") },
                    { new Guid("01968397-20e1-7e03-88ef-22f9bad08236"), 25, new Guid("01968397-20e1-7549-bd8f-2783ebd4a988") },
                    { new Guid("01968397-20e1-7e8c-8ba8-c87f58218a7b"), 7, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-7ea0-bb14-21dd58b3b73f"), 21, new Guid("01968397-20e1-768b-ba1e-75aca20fd1e0") },
                    { new Guid("01968397-20e1-7eeb-95ce-352efdc93abd"), 18, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-7f63-95a2-71a7fbdbcda6"), 25, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-7fa4-8679-c49c558fe2ee"), 16, new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-7fae-b1a7-86cee69a959c"), 19, new Guid("01968397-20e1-768b-ba1e-75aca20fd1e0") },
                    { new Guid("01968397-20e1-7fb9-a066-d14799ee0131"), 5, new Guid("01968397-20e1-7549-bd8f-2783ebd4a988") }
                });

            migrationBuilder.InsertData(
                table: "Teams",
                columns: new[] { "Id", "Name", "RoleId" },
                values: new object[,]
                {
                    { new Guid("01968397-20e1-7023-948b-68d5b20307f6"), "Admins", new Guid("01968397-20e1-78f5-ba61-f6d18b58e966") },
                    { new Guid("01968397-20e1-7226-b60b-140d2e6e6d1c"), "Devs", new Guid("01968397-20e1-768b-ba1e-75aca20fd1e0") },
                    { new Guid("01968397-20e1-72a2-91d9-3cddf518d12a"), "QA", new Guid("01968397-20e1-7549-bd8f-2783ebd4a988") }
                });

            migrationBuilder.InsertData(
                table: "UsersTeams",
                columns: new[] { "TeamId", "UserId" },
                values: new object[,]
                {
                    { new Guid("01968397-20e1-7023-948b-68d5b20307f6"), new Guid("01968397-20e1-75ce-9292-2b1010dcd92c") },
                    { new Guid("01968397-20e1-7226-b60b-140d2e6e6d1c"), new Guid("01968397-20ed-73e1-905f-dc138a2c0bde") },
                    { new Guid("01968397-20e1-72a2-91d9-3cddf518d12a"), new Guid("01968397-20f3-7ab4-b0b3-649133fac306") }
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
                name: "IX_Registries_Name",
                table: "Registries",
                column: "Name",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_SwarmsInfo_PlatformId",
                table: "SwarmsInfo",
                column: "PlatformId",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_SwarmsPeer_SwarmInfoId",
                table: "SwarmsPeer",
                column: "SwarmInfoId");

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
                name: "Platforms");

            migrationBuilder.DropTable(
                name: "Roles");
        }
    }
}
