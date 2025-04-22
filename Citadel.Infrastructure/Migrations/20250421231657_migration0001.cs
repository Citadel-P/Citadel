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
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 4, 21, 23, 16, 56, 971, DateTimeKind.Utc).AddTicks(5168)),
                    UpdatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 4, 21, 23, 16, 56, 973, DateTimeKind.Utc).AddTicks(2676))
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
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 4, 21, 23, 16, 56, 990, DateTimeKind.Utc).AddTicks(8232)),
                    UpdatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 4, 21, 23, 16, 56, 990, DateTimeKind.Utc).AddTicks(9474))
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
                    { new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180"), new DateTime(2025, 4, 21, 23, 16, 56, 828, DateTimeKind.Utc).AddTicks(9331), "Administrator" },
                    { new Guid("01965aa3-f57c-737f-89e2-b6dbd0d8891b"), new DateTime(2025, 4, 21, 23, 16, 56, 828, DateTimeKind.Utc).AddTicks(9848), "QA" },
                    { new Guid("01965aa3-f57c-751a-8aa4-940b56927228"), new DateTime(2025, 4, 21, 23, 16, 56, 828, DateTimeKind.Utc).AddTicks(9845), "Developer" }
                });

            migrationBuilder.InsertData(
                table: "Users",
                columns: new[] { "Id", "CreatedAt", "Email", "Name", "Password" },
                values: new object[,]
                {
                    { new Guid("01965aa3-f57d-706c-a9d0-8c9d0f54fd4b"), new DateTime(2025, 4, 21, 23, 16, 56, 829, DateTimeKind.Utc).AddTicks(8673), "admin@admin.com", "admin", "2yxl248L4MWO00eEmgv0rgAWzLxlGUwrWwwVMPh8yUC5SNKZ" },
                    { new Guid("01965aa3-f58c-71fd-b992-2881891b6448"), new DateTime(2025, 4, 21, 23, 16, 56, 844, DateTimeKind.Utc).AddTicks(5554), "dev@dev.com", "dev", "p61wp7aQfcFNaWqddB2Jf7FM1UUryp9Kq7zVplC2DdSxv7ax" },
                    { new Guid("01965aa3-f592-7fa5-97b9-4d5c0f4467a9"), new DateTime(2025, 4, 21, 23, 16, 56, 850, DateTimeKind.Utc).AddTicks(8679), "qa@qa.com", "qa", "Af4Xz2XGq5XqI83lxfVt+b1xCo06yeVscrq0kobBLn3eLe6A" }
                });

            migrationBuilder.InsertData(
                table: "Permissions",
                columns: new[] { "Id", "PermissionCode", "RoleId" },
                values: new object[,]
                {
                    { new Guid("01965aa3-f57d-7073-8bad-f6ec9c8c1128"), 28, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-7091-9220-eb432e08acef"), 1, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-70e5-b6d7-cd1381a2c7ac"), 27, new Guid("01965aa3-f57c-751a-8aa4-940b56927228") },
                    { new Guid("01965aa3-f57d-71a8-a1b7-040d52d3a64c"), 2, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-71b3-ba96-843a5e0fde96"), 20, new Guid("01965aa3-f57c-751a-8aa4-940b56927228") },
                    { new Guid("01965aa3-f57d-71b8-83f2-c483044073ce"), 6, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-71c9-816d-30143ea0a52f"), 13, new Guid("01965aa3-f57c-737f-89e2-b6dbd0d8891b") },
                    { new Guid("01965aa3-f57d-721f-add8-0f94f066c659"), 15, new Guid("01965aa3-f57c-751a-8aa4-940b56927228") },
                    { new Guid("01965aa3-f57d-7233-9a7e-21a4a58cb3f3"), 14, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-7241-8399-138a37b7c8b6"), 8, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-7253-b7ee-b944904346c7"), 4, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-725e-b6c8-2ca4cf49694e"), 20, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-73ba-8fe7-ecffc2b0a836"), 28, new Guid("01965aa3-f57c-751a-8aa4-940b56927228") },
                    { new Guid("01965aa3-f57d-73c2-b0bf-3970c03c6dbb"), 13, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-73e1-89f2-6de13cc0b9ce"), 19, new Guid("01965aa3-f57c-751a-8aa4-940b56927228") },
                    { new Guid("01965aa3-f57d-7407-833e-f3b75f768883"), 5, new Guid("01965aa3-f57c-751a-8aa4-940b56927228") },
                    { new Guid("01965aa3-f57d-741d-a5c3-b93fffea4067"), 22, new Guid("01965aa3-f57c-751a-8aa4-940b56927228") },
                    { new Guid("01965aa3-f57d-74a0-abef-aa62dfc71f8d"), 18, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-74eb-8ac7-9ec55efc70a3"), 5, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-74f3-90e6-0e3c9e9ce68b"), 9, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-756d-a9e9-24cb8ba511f7"), 21, new Guid("01965aa3-f57c-737f-89e2-b6dbd0d8891b") },
                    { new Guid("01965aa3-f57d-7570-9823-667e6ba8d900"), 12, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-7573-808f-b9d0f1603c46"), 15, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-7576-bbb4-6a6843149b92"), 25, new Guid("01965aa3-f57c-737f-89e2-b6dbd0d8891b") },
                    { new Guid("01965aa3-f57d-758c-ad1f-eb058770b9f8"), 14, new Guid("01965aa3-f57c-751a-8aa4-940b56927228") },
                    { new Guid("01965aa3-f57d-75b3-9e03-589baeeb352e"), 16, new Guid("01965aa3-f57c-751a-8aa4-940b56927228") },
                    { new Guid("01965aa3-f57d-75c4-84a7-738c53dc6c95"), 19, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-7638-9086-6f0d64e3109e"), 9, new Guid("01965aa3-f57c-751a-8aa4-940b56927228") },
                    { new Guid("01965aa3-f57d-763d-b61a-b01e46c1992d"), 22, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-767b-917c-66e74d0a65d7"), 24, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-76c0-8a27-f8a588bbe184"), 11, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-76fd-8f0f-36c898cbdac9"), 7, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-77ca-b968-88eecaf9b3c9"), 21, new Guid("01965aa3-f57c-751a-8aa4-940b56927228") },
                    { new Guid("01965aa3-f57d-78d3-903a-b9df31ab59c7"), 26, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-7986-94b9-2ca308c644dd"), 10, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-79b6-9aea-26967376ecb5"), 25, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-7a1f-886b-eb6bd589917c"), 3, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-7b9b-a7f9-3bba17e3d292"), 5, new Guid("01965aa3-f57c-737f-89e2-b6dbd0d8891b") },
                    { new Guid("01965aa3-f57d-7ba7-8940-445e0e48da6e"), 17, new Guid("01965aa3-f57c-751a-8aa4-940b56927228") },
                    { new Guid("01965aa3-f57d-7bf1-a3b3-4f4bbf0e4b76"), 27, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-7c16-94e7-0e1c88c89a5e"), 24, new Guid("01965aa3-f57c-751a-8aa4-940b56927228") },
                    { new Guid("01965aa3-f57d-7cec-93f8-6c89020939e0"), 1, new Guid("01965aa3-f57c-737f-89e2-b6dbd0d8891b") },
                    { new Guid("01965aa3-f57d-7d91-80dd-7b4aa1ee89f4"), 18, new Guid("01965aa3-f57c-751a-8aa4-940b56927228") },
                    { new Guid("01965aa3-f57d-7daf-a9a3-5cd081a573d9"), 23, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-7df3-acc8-0c401faee48c"), 1, new Guid("01965aa3-f57c-751a-8aa4-940b56927228") },
                    { new Guid("01965aa3-f57d-7e1b-bb0d-859ea469f615"), 9, new Guid("01965aa3-f57c-737f-89e2-b6dbd0d8891b") },
                    { new Guid("01965aa3-f57d-7e23-bcbc-ff8761567b99"), 17, new Guid("01965aa3-f57c-737f-89e2-b6dbd0d8891b") },
                    { new Guid("01965aa3-f57d-7eb9-a39f-cdf41f9a8d98"), 17, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-7ed2-9566-1a7164f11a3d"), 13, new Guid("01965aa3-f57c-751a-8aa4-940b56927228") },
                    { new Guid("01965aa3-f57d-7f2d-be54-46c4ea17bf28"), 26, new Guid("01965aa3-f57c-751a-8aa4-940b56927228") },
                    { new Guid("01965aa3-f57d-7f39-8f3c-24dad5e9e469"), 21, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-7f45-925d-8809581dfa3f"), 16, new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") }
                });

            migrationBuilder.InsertData(
                table: "Teams",
                columns: new[] { "Id", "Name", "RoleId" },
                values: new object[,]
                {
                    { new Guid("01965aa3-f57d-7637-9ee0-e0d049d25925"), "Admins", new Guid("01965aa3-f57c-732e-89ea-3c085b4bc180") },
                    { new Guid("01965aa3-f57d-7ab6-a160-e731f2fc5b7b"), "QA", new Guid("01965aa3-f57c-737f-89e2-b6dbd0d8891b") },
                    { new Guid("01965aa3-f57d-7fba-b981-2204cbb46545"), "Devs", new Guid("01965aa3-f57c-751a-8aa4-940b56927228") }
                });

            migrationBuilder.InsertData(
                table: "UsersTeams",
                columns: new[] { "TeamId", "UserId" },
                values: new object[,]
                {
                    { new Guid("01965aa3-f57d-7637-9ee0-e0d049d25925"), new Guid("01965aa3-f57d-706c-a9d0-8c9d0f54fd4b") },
                    { new Guid("01965aa3-f57d-7ab6-a160-e731f2fc5b7b"), new Guid("01965aa3-f592-7fa5-97b9-4d5c0f4467a9") },
                    { new Guid("01965aa3-f57d-7fba-b981-2204cbb46545"), new Guid("01965aa3-f58c-71fd-b992-2881891b6448") }
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
