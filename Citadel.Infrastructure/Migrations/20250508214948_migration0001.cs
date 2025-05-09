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
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 5, 8, 21, 49, 48, 237, DateTimeKind.Utc).AddTicks(9522)),
                    UpdatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 5, 8, 21, 49, 48, 239, DateTimeKind.Utc).AddTicks(4092))
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
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 5, 8, 21, 49, 48, 247, DateTimeKind.Utc).AddTicks(2411)),
                    UpdatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 5, 8, 21, 49, 48, 247, DateTimeKind.Utc).AddTicks(3352))
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
                    table.PrimaryKey("PK_UsersTeams", x => new { x.UserId, x.TeamId });
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
                    { new Guid("0196b1e0-492b-76de-befb-6a62e6c3cbb1"), new DateTime(2025, 5, 8, 21, 49, 48, 203, DateTimeKind.Utc).AddTicks(1365), "Developer" },
                    { new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5"), new DateTime(2025, 5, 8, 21, 49, 48, 203, DateTimeKind.Utc).AddTicks(1097), "Administrator" },
                    { new Guid("0196b1e0-492b-7f03-91bc-4a84099966e3"), new DateTime(2025, 5, 8, 21, 49, 48, 203, DateTimeKind.Utc).AddTicks(1368), "QA" }
                });

            migrationBuilder.InsertData(
                table: "Users",
                columns: new[] { "Id", "CreatedAt", "Email", "Name", "Password" },
                values: new object[,]
                {
                    { new Guid("0196b1e0-492b-7ca4-8cde-26605ba867c0"), new DateTime(2025, 5, 8, 21, 49, 48, 203, DateTimeKind.Utc).AddTicks(8175), "admin@admin.com", "admin", "UC6Q2farx8LScvyfNsHA99c8ZA7vItC4rfFHQYuN4Or00cry" },
                    { new Guid("0196b1e0-4936-79ea-941f-7666e1922dad"), new DateTime(2025, 5, 8, 21, 49, 48, 214, DateTimeKind.Utc).AddTicks(8986), "dev@dev.com", "dev", "CUro6w0VVGmrSwf1IXcneYi/1liBoSZxp4et9I5xkowrSuEM" },
                    { new Guid("0196b1e0-493c-75c0-a56c-058af194621b"), new DateTime(2025, 5, 8, 21, 49, 48, 220, DateTimeKind.Utc).AddTicks(1297), "qa@qa.com", "qa", "dnby9q+gleI5kInorTiRNmBj3nx8Rv7+p84g/Oxu010CRnTT" }
                });

            migrationBuilder.InsertData(
                table: "Permissions",
                columns: new[] { "Id", "PermissionCode", "RoleId" },
                values: new object[,]
                {
                    { new Guid("0196b1e0-492b-700d-9497-9371c945a1de"), 15, new Guid("0196b1e0-492b-76de-befb-6a62e6c3cbb1") },
                    { new Guid("0196b1e0-492b-703a-a18d-df37845d4a1a"), 9, new Guid("0196b1e0-492b-7f03-91bc-4a84099966e3") },
                    { new Guid("0196b1e0-492b-704b-8b5d-4588441a02f3"), 9, new Guid("0196b1e0-492b-76de-befb-6a62e6c3cbb1") },
                    { new Guid("0196b1e0-492b-70b1-a22e-5638d8cacd75"), 24, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-70bf-a199-7d1864a0b525"), 1, new Guid("0196b1e0-492b-76de-befb-6a62e6c3cbb1") },
                    { new Guid("0196b1e0-492b-70d2-b54c-1772d4405564"), 5, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-70d3-9db0-3e2de176cdbe"), 20, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-70fb-8ca0-9d3eb60f3461"), 21, new Guid("0196b1e0-492b-76de-befb-6a62e6c3cbb1") },
                    { new Guid("0196b1e0-492b-71d2-bdd4-b7359f539384"), 25, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-71e3-9341-fb5bdd5fc539"), 3, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-71ee-81d6-79e231bb8f6f"), 2, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-7216-b4fb-037f6797681f"), 15, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-726c-a739-46f3a66d5466"), 22, new Guid("0196b1e0-492b-76de-befb-6a62e6c3cbb1") },
                    { new Guid("0196b1e0-492b-726e-9fc3-fd5034e731c7"), 26, new Guid("0196b1e0-492b-76de-befb-6a62e6c3cbb1") },
                    { new Guid("0196b1e0-492b-7291-8994-92f17b0769ac"), 13, new Guid("0196b1e0-492b-76de-befb-6a62e6c3cbb1") },
                    { new Guid("0196b1e0-492b-730a-8f7a-9a64a32d8735"), 16, new Guid("0196b1e0-492b-76de-befb-6a62e6c3cbb1") },
                    { new Guid("0196b1e0-492b-7347-a657-1714d2889aa4"), 5, new Guid("0196b1e0-492b-76de-befb-6a62e6c3cbb1") },
                    { new Guid("0196b1e0-492b-736c-a524-d5f0721a14e6"), 13, new Guid("0196b1e0-492b-7f03-91bc-4a84099966e3") },
                    { new Guid("0196b1e0-492b-73c9-b1d7-2a538f10e560"), 19, new Guid("0196b1e0-492b-76de-befb-6a62e6c3cbb1") },
                    { new Guid("0196b1e0-492b-73de-a204-72647423c2a6"), 18, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-7415-be6f-99f83b24c145"), 4, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-7471-be83-d10b94434bae"), 17, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-754f-8e24-83e3e6e703f4"), 27, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-7585-b815-32e824a89b33"), 1, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-75a2-94d4-be7421b0d246"), 5, new Guid("0196b1e0-492b-7f03-91bc-4a84099966e3") },
                    { new Guid("0196b1e0-492b-75a4-a02c-4fa4fca755ee"), 13, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-75a4-b2de-ef47a39a4cc1"), 24, new Guid("0196b1e0-492b-76de-befb-6a62e6c3cbb1") },
                    { new Guid("0196b1e0-492b-7626-8981-3935f25c1a08"), 14, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-7711-a0d6-920a4124327b"), 21, new Guid("0196b1e0-492b-7f03-91bc-4a84099966e3") },
                    { new Guid("0196b1e0-492b-7738-9197-f8b9854f317a"), 22, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-77aa-b692-4c05dcf61ffa"), 20, new Guid("0196b1e0-492b-76de-befb-6a62e6c3cbb1") },
                    { new Guid("0196b1e0-492b-7814-9760-710c2afaa026"), 8, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-78a6-89d1-e0e18555f533"), 6, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-78be-af08-868a0659f8e7"), 23, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-78ec-9b2d-2076d73c6f51"), 19, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-795f-80a9-daaaa92b0073"), 12, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-7969-807e-b296e8ccc6a4"), 10, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-79ba-acf6-f95fa8e855f6"), 25, new Guid("0196b1e0-492b-7f03-91bc-4a84099966e3") },
                    { new Guid("0196b1e0-492b-7a60-b71b-8eea68d52d24"), 17, new Guid("0196b1e0-492b-76de-befb-6a62e6c3cbb1") },
                    { new Guid("0196b1e0-492b-7a71-b140-c72779dcf1b0"), 28, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-7a98-8f09-169a65e58599"), 9, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-7b3f-93b5-84a008729064"), 1, new Guid("0196b1e0-492b-7f03-91bc-4a84099966e3") },
                    { new Guid("0196b1e0-492b-7b60-9904-b8e036fa195f"), 17, new Guid("0196b1e0-492b-7f03-91bc-4a84099966e3") },
                    { new Guid("0196b1e0-492b-7b91-a176-243bda513513"), 21, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-7d9d-8c84-11fc2cb82a89"), 7, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-7db5-bae1-9c0d7854e530"), 11, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-7e05-a019-766067b1824b"), 14, new Guid("0196b1e0-492b-76de-befb-6a62e6c3cbb1") },
                    { new Guid("0196b1e0-492b-7ed9-b15a-a080ecd69686"), 26, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-7f15-ab8a-f67df34e6a2f"), 28, new Guid("0196b1e0-492b-76de-befb-6a62e6c3cbb1") },
                    { new Guid("0196b1e0-492b-7f39-b85f-ab3482c67d0f"), 27, new Guid("0196b1e0-492b-76de-befb-6a62e6c3cbb1") },
                    { new Guid("0196b1e0-492b-7f5b-ab40-577a46f08869"), 16, new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-7ff0-bced-7c61179fadb3"), 18, new Guid("0196b1e0-492b-76de-befb-6a62e6c3cbb1") }
                });

            migrationBuilder.InsertData(
                table: "Teams",
                columns: new[] { "Id", "Name", "RoleId" },
                values: new object[,]
                {
                    { new Guid("0196b1e0-492b-71ee-9a1a-022d58d8c8cc"), "Devs", new Guid("0196b1e0-492b-76de-befb-6a62e6c3cbb1") },
                    { new Guid("0196b1e0-492b-79ff-984d-62a33392544c"), "Admins", new Guid("0196b1e0-492b-7a8a-92f9-bc554a2558c5") },
                    { new Guid("0196b1e0-492b-7e5d-9395-10e838cb3622"), "QA", new Guid("0196b1e0-492b-7f03-91bc-4a84099966e3") }
                });

            migrationBuilder.InsertData(
                table: "UsersTeams",
                columns: new[] { "TeamId", "UserId" },
                values: new object[,]
                {
                    { new Guid("0196b1e0-492b-79ff-984d-62a33392544c"), new Guid("0196b1e0-492b-7ca4-8cde-26605ba867c0") },
                    { new Guid("0196b1e0-492b-71ee-9a1a-022d58d8c8cc"), new Guid("0196b1e0-4936-79ea-941f-7666e1922dad") },
                    { new Guid("0196b1e0-492b-7e5d-9395-10e838cb3622"), new Guid("0196b1e0-493c-75c0-a56c-058af194621b") }
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
                name: "IX_UsersTeams_TeamId",
                table: "UsersTeams",
                column: "TeamId");
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
