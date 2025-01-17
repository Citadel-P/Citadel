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
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 1, 16, 20, 50, 19, 825, DateTimeKind.Utc).AddTicks(896)),
                    UpdatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 1, 16, 20, 50, 19, 826, DateTimeKind.Utc).AddTicks(7281))
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
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 1, 16, 20, 50, 19, 843, DateTimeKind.Utc).AddTicks(6496)),
                    UpdatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 1, 16, 20, 50, 19, 843, DateTimeKind.Utc).AddTicks(7492))
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
                    Created = table.Column<int>(type: "INTEGER", nullable: false),
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
                    { new Guid("019470e1-9608-7333-a132-489ef66a32a7"), new DateTime(2025, 1, 16, 20, 50, 19, 785, DateTimeKind.Utc).AddTicks(115), "Administrator" },
                    { new Guid("019470e1-9609-70dc-8fbd-4db035745107"), new DateTime(2025, 1, 16, 20, 50, 19, 785, DateTimeKind.Utc).AddTicks(426), "Developer" },
                    { new Guid("019470e1-9609-7471-bc9e-aeeffb8a7560"), new DateTime(2025, 1, 16, 20, 50, 19, 785, DateTimeKind.Utc).AddTicks(429), "QA" }
                });

            migrationBuilder.InsertData(
                table: "Users",
                columns: new[] { "Id", "CreatedAt", "Email", "Name", "Password" },
                values: new object[,]
                {
                    { new Guid("019470e1-9609-7b34-b239-748d9cef4b76"), new DateTime(2025, 1, 16, 20, 50, 19, 785, DateTimeKind.Utc).AddTicks(8091), "admin@admin.com", "admin", "XKrZCuNutAAm0KgY2F+9nvjlLyG6D/oFtSYweCrUHDhyVHn/" },
                    { new Guid("019470e1-9617-7194-add2-37adb9fdb1c2"), new DateTime(2025, 1, 16, 20, 50, 19, 799, DateTimeKind.Utc).AddTicks(2073), "dev@dev.com", "dev", "YPcK7rxDPhupAY9RJft4H5eNtrpEcGQxl1awe64++avmRSBA" },
                    { new Guid("019470e1-961c-753a-83f5-3416cd953d7a"), new DateTime(2025, 1, 16, 20, 50, 19, 804, DateTimeKind.Utc).AddTicks(2689), "qa@qa.com", "qa", "C0wzuO5W0LOE4txT39FO8A4upUKMVKQcZBRB64M92XVxJqx4" }
                });

            migrationBuilder.InsertData(
                table: "Permissions",
                columns: new[] { "Id", "PermissionCode", "RoleId" },
                values: new object[,]
                {
                    { new Guid("019470e1-9609-7074-8bd1-748afce461f5"), 22, new Guid("019470e1-9609-70dc-8fbd-4db035745107") },
                    { new Guid("019470e1-9609-7074-a129-89846ed894b9"), 26, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-71a4-aeb3-9d52195de5b1"), 28, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-71c5-bcd7-076712361591"), 9, new Guid("019470e1-9609-70dc-8fbd-4db035745107") },
                    { new Guid("019470e1-9609-71c8-8b27-18b2ee0e5090"), 21, new Guid("019470e1-9609-70dc-8fbd-4db035745107") },
                    { new Guid("019470e1-9609-71da-baa0-cfda48af5281"), 27, new Guid("019470e1-9609-70dc-8fbd-4db035745107") },
                    { new Guid("019470e1-9609-71dc-9412-8a3c2f10e52b"), 27, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-7231-b781-dd4b5c241d4b"), 28, new Guid("019470e1-9609-70dc-8fbd-4db035745107") },
                    { new Guid("019470e1-9609-726c-8d7f-0a3604a4c9d4"), 16, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-7284-8d2e-e36a3011165d"), 19, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-728a-b183-10a1bea29309"), 1, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-72d1-a3bd-0901f799640f"), 2, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-7334-95b6-4ed0e649af7a"), 21, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-7346-a9a2-916aa471c102"), 9, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-737c-8cc3-d3dff18ac984"), 25, new Guid("019470e1-9609-7471-bc9e-aeeffb8a7560") },
                    { new Guid("019470e1-9609-739a-9a04-eda8b1dd0266"), 22, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-73a6-a2c1-c7a48e15de41"), 13, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-73cd-abc0-7dea8994b26e"), 18, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-7427-861a-c37ccc2c2263"), 21, new Guid("019470e1-9609-7471-bc9e-aeeffb8a7560") },
                    { new Guid("019470e1-9609-7447-b823-1cb0f29a9daf"), 17, new Guid("019470e1-9609-7471-bc9e-aeeffb8a7560") },
                    { new Guid("019470e1-9609-74d5-adea-e3b4880d12c7"), 5, new Guid("019470e1-9609-7471-bc9e-aeeffb8a7560") },
                    { new Guid("019470e1-9609-7526-b7e3-3b6cc2f2c1ae"), 17, new Guid("019470e1-9609-70dc-8fbd-4db035745107") },
                    { new Guid("019470e1-9609-7566-81c9-5342d202619d"), 1, new Guid("019470e1-9609-70dc-8fbd-4db035745107") },
                    { new Guid("019470e1-9609-75c0-829b-229ce7931fda"), 14, new Guid("019470e1-9609-70dc-8fbd-4db035745107") },
                    { new Guid("019470e1-9609-76f7-b46b-c6aac3f5deae"), 24, new Guid("019470e1-9609-70dc-8fbd-4db035745107") },
                    { new Guid("019470e1-9609-7765-9542-346514e363cc"), 9, new Guid("019470e1-9609-7471-bc9e-aeeffb8a7560") },
                    { new Guid("019470e1-9609-78b0-bd06-8622f20fcf50"), 25, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-78c4-aa5f-9de3bc8d5d1a"), 5, new Guid("019470e1-9609-70dc-8fbd-4db035745107") },
                    { new Guid("019470e1-9609-78db-9e3b-3b04da4142b7"), 13, new Guid("019470e1-9609-7471-bc9e-aeeffb8a7560") },
                    { new Guid("019470e1-9609-78e9-a8d0-57cb341e6771"), 3, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-78f2-85e1-4f6d69665fd7"), 7, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-7976-ab4d-e62787236e38"), 16, new Guid("019470e1-9609-70dc-8fbd-4db035745107") },
                    { new Guid("019470e1-9609-799a-b5f4-2d31d98e0864"), 20, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-79b2-804f-a023f10f8eca"), 14, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-7a33-96a5-a8ee2fe5f304"), 17, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-7a65-ab80-a93171929ef1"), 18, new Guid("019470e1-9609-70dc-8fbd-4db035745107") },
                    { new Guid("019470e1-9609-7a6a-9086-05b33de61785"), 10, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-7b48-a9bc-702b95b20eed"), 26, new Guid("019470e1-9609-70dc-8fbd-4db035745107") },
                    { new Guid("019470e1-9609-7b8c-b938-334d4ee21bc9"), 15, new Guid("019470e1-9609-70dc-8fbd-4db035745107") },
                    { new Guid("019470e1-9609-7c54-b7d1-f0b1e3362a4e"), 12, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-7c6c-8727-64384a850ef9"), 1, new Guid("019470e1-9609-7471-bc9e-aeeffb8a7560") },
                    { new Guid("019470e1-9609-7ce7-a553-b1e399ec1428"), 19, new Guid("019470e1-9609-70dc-8fbd-4db035745107") },
                    { new Guid("019470e1-9609-7d26-8c7d-936b7616f609"), 15, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-7d43-8061-3c4c144ccfea"), 11, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-7da2-8f18-5b40f909a449"), 6, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-7ded-a592-4b41b2bc8c14"), 8, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-7e92-a6bd-be5c28be2e3c"), 23, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-7f0f-b2a1-fd5015e31ba2"), 13, new Guid("019470e1-9609-70dc-8fbd-4db035745107") },
                    { new Guid("019470e1-9609-7f40-95bb-b3bb7a67749a"), 4, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-7f75-8cc5-dda61131638a"), 24, new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-7f93-8d21-03656f40e761"), 20, new Guid("019470e1-9609-70dc-8fbd-4db035745107") },
                    { new Guid("019470e1-9609-7fe9-ba98-79ab15d0221a"), 5, new Guid("019470e1-9608-7333-a132-489ef66a32a7") }
                });

            migrationBuilder.InsertData(
                table: "Teams",
                columns: new[] { "Id", "Name", "RoleId" },
                values: new object[,]
                {
                    { new Guid("019470e1-9609-71e3-a2a3-b4895845c0c9"), "QA", new Guid("019470e1-9609-7471-bc9e-aeeffb8a7560") },
                    { new Guid("019470e1-9609-7c59-85d9-8e4f15e12209"), "Admins", new Guid("019470e1-9608-7333-a132-489ef66a32a7") },
                    { new Guid("019470e1-9609-7e11-8245-aa6a3031b7ca"), "Devs", new Guid("019470e1-9609-70dc-8fbd-4db035745107") }
                });

            migrationBuilder.InsertData(
                table: "UsersTeams",
                columns: new[] { "TeamId", "UserId" },
                values: new object[,]
                {
                    { new Guid("019470e1-9609-71e3-a2a3-b4895845c0c9"), new Guid("019470e1-961c-753a-83f5-3416cd953d7a") },
                    { new Guid("019470e1-9609-7c59-85d9-8e4f15e12209"), new Guid("019470e1-9609-7b34-b239-748d9cef4b76") },
                    { new Guid("019470e1-9609-7e11-8245-aa6a3031b7ca"), new Guid("019470e1-9617-7194-add2-37adb9fdb1c2") }
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
