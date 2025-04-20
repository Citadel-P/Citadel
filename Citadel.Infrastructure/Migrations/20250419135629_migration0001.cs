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
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 4, 19, 13, 56, 28, 364, DateTimeKind.Utc).AddTicks(102)),
                    UpdatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 4, 19, 13, 56, 28, 365, DateTimeKind.Utc).AddTicks(7855))
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
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 4, 19, 13, 56, 28, 384, DateTimeKind.Utc).AddTicks(3345)),
                    UpdatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 4, 19, 13, 56, 28, 384, DateTimeKind.Utc).AddTicks(4543))
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
                    { new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1"), new DateTime(2025, 4, 19, 13, 56, 28, 189, DateTimeKind.Utc).AddTicks(8965), "Administrator" },
                    { new Guid("01964e56-1b9d-73e9-89cd-af2b59107c13"), new DateTime(2025, 4, 19, 13, 56, 28, 189, DateTimeKind.Utc).AddTicks(9300), "Developer" },
                    { new Guid("01964e56-1b9d-7ae4-8db6-e6a4cc20b964"), new DateTime(2025, 4, 19, 13, 56, 28, 189, DateTimeKind.Utc).AddTicks(9303), "QA" }
                });

            migrationBuilder.InsertData(
                table: "Users",
                columns: new[] { "Id", "CreatedAt", "Email", "Name", "Password" },
                values: new object[,]
                {
                    { new Guid("01964e56-1b9e-758e-a638-46291bc0ca18"), new DateTime(2025, 4, 19, 13, 56, 28, 190, DateTimeKind.Utc).AddTicks(7249), "admin@admin.com", "admin", "YGBr0SW+slBJzgzt1XQVs4/PhG1FA/U3IKbrQbeaT0bqi946" },
                    { new Guid("01964e56-1bac-78ab-a75c-1cac1eca4dd1"), new DateTime(2025, 4, 19, 13, 56, 28, 204, DateTimeKind.Utc).AddTicks(6941), "dev@dev.com", "dev", "ESHhX3C3nOAZIEmK7Mso73a4lRijWyv72kboe5lREodbFtK7" },
                    { new Guid("01964e56-1bb3-785f-a3f7-f9906cd16ba8"), new DateTime(2025, 4, 19, 13, 56, 28, 211, DateTimeKind.Utc).AddTicks(5349), "qa@qa.com", "qa", "MC5fWQvrm0O2hksBgb7+/1Y7J3pCsU/i17z9mE1mcNRCMokO" }
                });

            migrationBuilder.InsertData(
                table: "Permissions",
                columns: new[] { "Id", "PermissionCode", "RoleId" },
                values: new object[,]
                {
                    { new Guid("01964e56-1b9e-7006-8b7b-a8f5334b285a"), 11, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-709d-93c1-2727232c529e"), 27, new Guid("01964e56-1b9d-73e9-89cd-af2b59107c13") },
                    { new Guid("01964e56-1b9e-7162-983e-7f2920e3257e"), 13, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-71ef-a43c-835fedcf1bf3"), 25, new Guid("01964e56-1b9d-7ae4-8db6-e6a4cc20b964") },
                    { new Guid("01964e56-1b9e-7200-8dc1-2ce4c556a341"), 23, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-7207-be6b-fa4f9adab8ea"), 19, new Guid("01964e56-1b9d-73e9-89cd-af2b59107c13") },
                    { new Guid("01964e56-1b9e-7212-ae9c-88d6ac8abf47"), 28, new Guid("01964e56-1b9d-73e9-89cd-af2b59107c13") },
                    { new Guid("01964e56-1b9e-721c-8b6c-2c79e9f5875a"), 20, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-7268-84e9-7e2c9586af7c"), 16, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-72a2-93b2-b3eb53aa5782"), 27, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-72b3-a2fd-4830e8347c6b"), 9, new Guid("01964e56-1b9d-73e9-89cd-af2b59107c13") },
                    { new Guid("01964e56-1b9e-72e5-86f5-6947cf26d57f"), 5, new Guid("01964e56-1b9d-73e9-89cd-af2b59107c13") },
                    { new Guid("01964e56-1b9e-72fb-8046-15e12b50d5b5"), 26, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-7348-b55c-ff379ea0c9a3"), 10, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-736d-9032-cc65293c416e"), 28, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-7513-89e1-70e11a133ca5"), 17, new Guid("01964e56-1b9d-73e9-89cd-af2b59107c13") },
                    { new Guid("01964e56-1b9e-7597-afb4-fb558963b057"), 4, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-76a3-ba49-60694e3608e9"), 16, new Guid("01964e56-1b9d-73e9-89cd-af2b59107c13") },
                    { new Guid("01964e56-1b9e-76a7-8aca-af2a5b04136e"), 9, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-7758-95c4-270112aa00f0"), 13, new Guid("01964e56-1b9d-7ae4-8db6-e6a4cc20b964") },
                    { new Guid("01964e56-1b9e-7798-be9b-b23be1ab1a1b"), 20, new Guid("01964e56-1b9d-73e9-89cd-af2b59107c13") },
                    { new Guid("01964e56-1b9e-77aa-8947-732cd1a362d6"), 5, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-7863-803c-beabe4d8b171"), 6, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-7874-8756-923167467b53"), 24, new Guid("01964e56-1b9d-73e9-89cd-af2b59107c13") },
                    { new Guid("01964e56-1b9e-787b-97e8-e888bbae7b4b"), 5, new Guid("01964e56-1b9d-7ae4-8db6-e6a4cc20b964") },
                    { new Guid("01964e56-1b9e-7885-bf98-9d5b06e6d8b4"), 1, new Guid("01964e56-1b9d-73e9-89cd-af2b59107c13") },
                    { new Guid("01964e56-1b9e-7890-bcef-7d3fbbdcc261"), 19, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-7895-9ed6-17196ad1b3d2"), 25, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-78b5-ab5e-c1c14c5be343"), 21, new Guid("01964e56-1b9d-73e9-89cd-af2b59107c13") },
                    { new Guid("01964e56-1b9e-78eb-9877-e5eaddb25d27"), 14, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-7983-9628-cef5e52af171"), 18, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-79dd-b591-ab17c31e3f0f"), 14, new Guid("01964e56-1b9d-73e9-89cd-af2b59107c13") },
                    { new Guid("01964e56-1b9e-79e5-8598-3e409680aa8b"), 13, new Guid("01964e56-1b9d-73e9-89cd-af2b59107c13") },
                    { new Guid("01964e56-1b9e-79f8-bd81-3e7f64728cfa"), 2, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-7a86-b6a9-44874a23546e"), 8, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-7aa4-8d65-bd80e239c366"), 12, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-7aac-8587-47cc2a1009b8"), 21, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-7ad6-b35b-a340bcbdecb5"), 7, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-7ad8-8892-b131a8af9934"), 24, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-7b5d-9496-7fdb57a94e45"), 3, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-7bac-817f-d72e025551a4"), 1, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-7c4e-96e7-d42b8f08f086"), 15, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-7c4e-a756-846f43679da3"), 15, new Guid("01964e56-1b9d-73e9-89cd-af2b59107c13") },
                    { new Guid("01964e56-1b9e-7c68-972f-5c0d30a659b6"), 26, new Guid("01964e56-1b9d-73e9-89cd-af2b59107c13") },
                    { new Guid("01964e56-1b9e-7ccb-8a0f-290547b63580"), 17, new Guid("01964e56-1b9d-7ae4-8db6-e6a4cc20b964") },
                    { new Guid("01964e56-1b9e-7cd3-a825-1e48cbb0b8ea"), 22, new Guid("01964e56-1b9d-73e9-89cd-af2b59107c13") },
                    { new Guid("01964e56-1b9e-7dc0-898f-aca2d34367e9"), 22, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-7e20-b8ef-9928d602cb42"), 9, new Guid("01964e56-1b9d-7ae4-8db6-e6a4cc20b964") },
                    { new Guid("01964e56-1b9e-7e80-8d56-712a412b2ee3"), 17, new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-7ec5-a78c-2adabe9e39b6"), 1, new Guid("01964e56-1b9d-7ae4-8db6-e6a4cc20b964") },
                    { new Guid("01964e56-1b9e-7f22-92d7-b0491659c084"), 21, new Guid("01964e56-1b9d-7ae4-8db6-e6a4cc20b964") },
                    { new Guid("01964e56-1b9e-7f48-b0a5-cfcfa7adced5"), 18, new Guid("01964e56-1b9d-73e9-89cd-af2b59107c13") }
                });

            migrationBuilder.InsertData(
                table: "Teams",
                columns: new[] { "Id", "Name", "RoleId" },
                values: new object[,]
                {
                    { new Guid("01964e56-1b9e-7400-8310-904fce120a28"), "Admins", new Guid("01964e56-1b9d-71f4-a3f2-779f7aa0b6b1") },
                    { new Guid("01964e56-1b9e-7d83-bd80-cdccc08f885a"), "Devs", new Guid("01964e56-1b9d-73e9-89cd-af2b59107c13") },
                    { new Guid("01964e56-1b9e-7f9b-889b-5a6a3c8a8e93"), "QA", new Guid("01964e56-1b9d-7ae4-8db6-e6a4cc20b964") }
                });

            migrationBuilder.InsertData(
                table: "UsersTeams",
                columns: new[] { "TeamId", "UserId" },
                values: new object[,]
                {
                    { new Guid("01964e56-1b9e-7400-8310-904fce120a28"), new Guid("01964e56-1b9e-758e-a638-46291bc0ca18") },
                    { new Guid("01964e56-1b9e-7d83-bd80-cdccc08f885a"), new Guid("01964e56-1bac-78ab-a75c-1cac1eca4dd1") },
                    { new Guid("01964e56-1b9e-7f9b-889b-5a6a3c8a8e93"), new Guid("01964e56-1bb3-785f-a3f7-f9906cd16ba8") }
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
