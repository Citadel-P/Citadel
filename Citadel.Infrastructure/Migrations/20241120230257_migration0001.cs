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
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2024, 11, 20, 23, 2, 57, 64, DateTimeKind.Utc).AddTicks(4541)),
                    UpdatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2024, 11, 20, 23, 2, 57, 65, DateTimeKind.Utc).AddTicks(8826))
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
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2024, 11, 20, 23, 2, 57, 84, DateTimeKind.Utc).AddTicks(906)),
                    UpdatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2024, 11, 20, 23, 2, 57, 84, DateTimeKind.Utc).AddTicks(1648))
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
                    Created = table.Column<DateTimeOffset>(type: "TEXT", nullable: false),
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
                    MemoryUsage = table.Column<double>(type: "REAL", nullable: false),
                    CpuUsage = table.Column<double>(type: "REAL", nullable: false),
                    CreatedAtUtc = table.Column<DateTime>(type: "TEXT", nullable: false)
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
                    NetworksCount = table.Column<short>(type: "INTEGER", nullable: false),
                    VolumesCount = table.Column<short>(type: "INTEGER", nullable: false),
                    Containers = table.Column<long>(type: "INTEGER", nullable: false),
                    ContainersRunning = table.Column<long>(type: "INTEGER", nullable: false),
                    ContainersPaused = table.Column<long>(type: "INTEGER", nullable: false),
                    ContainersStopped = table.Column<long>(type: "INTEGER", nullable: false),
                    Images = table.Column<long>(type: "INTEGER", nullable: false),
                    Driver = table.Column<string>(type: "TEXT", nullable: true),
                    OperatingSystem = table.Column<string>(type: "TEXT", maxLength: 128, nullable: true),
                    OSVersion = table.Column<string>(type: "TEXT", maxLength: 128, nullable: true),
                    OSType = table.Column<string>(type: "TEXT", maxLength: 128, nullable: true),
                    Architecture = table.Column<string>(type: "TEXT", maxLength: 128, nullable: true),
                    NCPU = table.Column<long>(type: "INTEGER", nullable: false),
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
                    CreatedAtUtc = table.Column<DateTime>(type: "TEXT", nullable: false),
                    MemoryUsage = table.Column<double>(type: "REAL", nullable: true),
                    CpuUsage = table.Column<double>(type: "REAL", nullable: true),
                    MemoryLimit = table.Column<double>(type: "REAL", nullable: true),
                    RxBytes = table.Column<ulong>(type: "INTEGER", nullable: true),
                    TxBytes = table.Column<ulong>(type: "INTEGER", nullable: true)
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
                    { new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d"), new DateTime(2024, 11, 20, 23, 2, 57, 22, DateTimeKind.Utc).AddTicks(9950), "Administrator" },
                    { new Guid("01934bd0-84ff-7a0f-9030-3f9352a83511"), new DateTime(2024, 11, 20, 23, 2, 57, 23, DateTimeKind.Utc).AddTicks(317), "QA" },
                    { new Guid("01934bd0-84ff-7a25-8b54-265589aeb00c"), new DateTime(2024, 11, 20, 23, 2, 57, 23, DateTimeKind.Utc).AddTicks(315), "Developer" }
                });

            migrationBuilder.InsertData(
                table: "Users",
                columns: new[] { "Id", "CreatedAt", "Email", "Name", "Password" },
                values: new object[,]
                {
                    { new Guid("01934bd0-84ff-7a0b-b89e-57b1ec7b8db5"), new DateTime(2024, 11, 20, 23, 2, 57, 23, DateTimeKind.Utc).AddTicks(7516), "admin@admin.com", "admin", "+HRQtRdSCAW2Z+JxWK4bcNBaD05BooUjNoXDiorhBeLGycZ1" },
                    { new Guid("01934bd0-850c-7727-9ca7-862904d5b3b1"), new DateTime(2024, 11, 20, 23, 2, 57, 36, DateTimeKind.Utc).AddTicks(7790), "dev@dev.com", "dev", "Juv8e61mds1If1G7yC7jGoKkhTdO3daGy/6zhF12gW9lrkVz" },
                    { new Guid("01934bd0-8512-7776-8973-8797d0d8979f"), new DateTime(2024, 11, 20, 23, 2, 57, 42, DateTimeKind.Utc).AddTicks(1704), "qa@qa.com", "qa", "Wo2c8VG1ZFmAkHDwIQq3OoPRzJwLhjkJ0xcWwQO4IrXGN6sb" }
                });

            migrationBuilder.InsertData(
                table: "Permissions",
                columns: new[] { "Id", "PermissionCode", "RoleId" },
                values: new object[,]
                {
                    { new Guid("01934bd0-84ff-7038-83e2-dffb0a4a9ec3"), 24, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-705c-a010-f6976e6c46c9"), 20, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-706d-9354-5c13c1796742"), 15, new Guid("01934bd0-84ff-7a25-8b54-265589aeb00c") },
                    { new Guid("01934bd0-84ff-70cf-b826-2720d66ec960"), 22, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-7119-a41f-3ed0e5719bac"), 2, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-716e-ac32-ba8b700a140b"), 9, new Guid("01934bd0-84ff-7a0f-9030-3f9352a83511") },
                    { new Guid("01934bd0-84ff-7235-a492-1a47a0c4161b"), 25, new Guid("01934bd0-84ff-7a0f-9030-3f9352a83511") },
                    { new Guid("01934bd0-84ff-736d-8195-19da8444e24a"), 13, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-7386-b878-ec08a9e97da0"), 13, new Guid("01934bd0-84ff-7a25-8b54-265589aeb00c") },
                    { new Guid("01934bd0-84ff-73e6-b3c6-f095b7d442f1"), 5, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-742a-a532-4f0e795a685f"), 18, new Guid("01934bd0-84ff-7a25-8b54-265589aeb00c") },
                    { new Guid("01934bd0-84ff-7433-a8c4-d18740b14abf"), 7, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-7443-9bc4-d9d9add22b2e"), 17, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-7450-a7d8-ceb0c1e4f7ce"), 23, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-7461-b148-97c8da6dc960"), 9, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-746f-9d14-60ece28db8c7"), 6, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-74ac-b533-5381345ccaf2"), 13, new Guid("01934bd0-84ff-7a0f-9030-3f9352a83511") },
                    { new Guid("01934bd0-84ff-7565-85af-727e4ff31641"), 12, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-75cc-8fd2-271909639e41"), 19, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-75e8-be36-d906e8141cd9"), 10, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-75f2-8341-d26eaef6e5a9"), 27, new Guid("01934bd0-84ff-7a25-8b54-265589aeb00c") },
                    { new Guid("01934bd0-84ff-7697-b581-0beb2a01a41e"), 11, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-76b3-ac33-27c3ec8ec7a6"), 5, new Guid("01934bd0-84ff-7a0f-9030-3f9352a83511") },
                    { new Guid("01934bd0-84ff-76c2-8cb0-9bfe6625f1ba"), 4, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-76d1-963e-eaf6a087b10b"), 9, new Guid("01934bd0-84ff-7a25-8b54-265589aeb00c") },
                    { new Guid("01934bd0-84ff-774b-b176-9c2bce6d3af4"), 1, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-7755-8a82-3c293ea14e99"), 14, new Guid("01934bd0-84ff-7a25-8b54-265589aeb00c") },
                    { new Guid("01934bd0-84ff-7810-95a5-a786db9926bb"), 15, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-7818-8b53-b8fad16c2573"), 18, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-78cb-816e-afa8bb9627a6"), 25, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-7949-ac0e-a9fdbba5f8f6"), 19, new Guid("01934bd0-84ff-7a25-8b54-265589aeb00c") },
                    { new Guid("01934bd0-84ff-799e-9a45-0655cbb0580c"), 8, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-7a26-bd12-a22388209e5d"), 27, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-7a73-a283-6c606edd2e0f"), 1, new Guid("01934bd0-84ff-7a0f-9030-3f9352a83511") },
                    { new Guid("01934bd0-84ff-7b0d-a6df-583de2db2068"), 5, new Guid("01934bd0-84ff-7a25-8b54-265589aeb00c") },
                    { new Guid("01934bd0-84ff-7b51-99de-118314688e8e"), 26, new Guid("01934bd0-84ff-7a25-8b54-265589aeb00c") },
                    { new Guid("01934bd0-84ff-7b6e-b89a-77d3d3cc08bb"), 21, new Guid("01934bd0-84ff-7a25-8b54-265589aeb00c") },
                    { new Guid("01934bd0-84ff-7bb7-8815-96c5df844155"), 17, new Guid("01934bd0-84ff-7a0f-9030-3f9352a83511") },
                    { new Guid("01934bd0-84ff-7c4d-a8fc-f9e8a893dcb3"), 21, new Guid("01934bd0-84ff-7a0f-9030-3f9352a83511") },
                    { new Guid("01934bd0-84ff-7ce1-9f27-70fab0efe872"), 16, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-7d3d-ae46-c179701a14bb"), 1, new Guid("01934bd0-84ff-7a25-8b54-265589aeb00c") },
                    { new Guid("01934bd0-84ff-7d4d-9cc9-e47d96169a5e"), 17, new Guid("01934bd0-84ff-7a25-8b54-265589aeb00c") },
                    { new Guid("01934bd0-84ff-7d65-b32a-405fe79e936c"), 22, new Guid("01934bd0-84ff-7a25-8b54-265589aeb00c") },
                    { new Guid("01934bd0-84ff-7dc8-9aa3-b6171441c3db"), 14, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-7dfd-803b-a6f0a02c15f9"), 21, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-7e8c-b1e0-3a77a98ce822"), 16, new Guid("01934bd0-84ff-7a25-8b54-265589aeb00c") },
                    { new Guid("01934bd0-84ff-7ed8-9081-77e3b2a87392"), 28, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-7f13-a311-ef4cfd942777"), 3, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") },
                    { new Guid("01934bd0-84ff-7f76-89b2-bd38396a2c67"), 20, new Guid("01934bd0-84ff-7a25-8b54-265589aeb00c") },
                    { new Guid("01934bd0-84ff-7f7a-95bb-e11717b20b65"), 28, new Guid("01934bd0-84ff-7a25-8b54-265589aeb00c") },
                    { new Guid("01934bd0-84ff-7f8e-acfa-250be43757cc"), 24, new Guid("01934bd0-84ff-7a25-8b54-265589aeb00c") },
                    { new Guid("01934bd0-84ff-7fe1-9486-63c267475087"), 26, new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") }
                });

            migrationBuilder.InsertData(
                table: "Teams",
                columns: new[] { "Id", "Name", "RoleId" },
                values: new object[,]
                {
                    { new Guid("01934bd0-84ff-790e-98b6-d720d47ca01e"), "QA", new Guid("01934bd0-84ff-7a0f-9030-3f9352a83511") },
                    { new Guid("01934bd0-84ff-7a01-a9b7-19a84c97c4f5"), "Devs", new Guid("01934bd0-84ff-7a25-8b54-265589aeb00c") },
                    { new Guid("01934bd0-84ff-7a20-ae88-888f274c88a7"), "Admins", new Guid("01934bd0-84fe-7abe-a03a-e2d9919bfa2d") }
                });

            migrationBuilder.InsertData(
                table: "UsersTeams",
                columns: new[] { "TeamId", "UserId" },
                values: new object[,]
                {
                    { new Guid("01934bd0-84ff-790e-98b6-d720d47ca01e"), new Guid("01934bd0-8512-7776-8973-8797d0d8979f") },
                    { new Guid("01934bd0-84ff-7a01-a9b7-19a84c97c4f5"), new Guid("01934bd0-850c-7727-9ca7-862904d5b3b1") },
                    { new Guid("01934bd0-84ff-7a20-ae88-888f274c88a7"), new Guid("01934bd0-84ff-7a0b-b89e-57b1ec7b8db5") }
                });

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
