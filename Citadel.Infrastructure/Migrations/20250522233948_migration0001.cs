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
                    Id = table.Column<byte[]>(type: "BLOB", nullable: false),
                    Name = table.Column<string>(type: "TEXT", maxLength: 128, nullable: false),
                    Address = table.Column<string>(type: "TEXT", maxLength: 128, nullable: false),
                    DaemonId = table.Column<string>(type: "TEXT", nullable: false),
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
                    Id = table.Column<byte[]>(type: "BLOB", nullable: false),
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
                    Id = table.Column<byte[]>(type: "BLOB", nullable: false),
                    Name = table.Column<string>(type: "TEXT", maxLength: 128, nullable: false),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2000, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc)),
                    UpdatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2000, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc))
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Roles", x => x.Id);
                });

            migrationBuilder.CreateTable(
                name: "Users",
                columns: table => new
                {
                    Id = table.Column<byte[]>(type: "BLOB", nullable: false),
                    Name = table.Column<string>(type: "TEXT", maxLength: 128, nullable: false),
                    Email = table.Column<string>(type: "TEXT", maxLength: 128, nullable: false),
                    Password = table.Column<string>(type: "TEXT", maxLength: 128, nullable: false),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2000, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc)),
                    UpdatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2000, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc))
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Users", x => x.Id);
                });

            migrationBuilder.CreateTable(
                name: "ContainersInfo",
                columns: table => new
                {
                    Id = table.Column<byte[]>(type: "BLOB", nullable: false),
                    PlatformId = table.Column<byte[]>(type: "BLOB", nullable: false),
                    ContainerId = table.Column<string>(type: "TEXT", maxLength: 64, nullable: false),
                    Name = table.Column<string>(type: "TEXT", nullable: false),
                    Image = table.Column<string>(type: "TEXT", nullable: false),
                    Created = table.Column<long>(type: "INTEGER", nullable: false),
                    State = table.Column<string>(type: "TEXT", nullable: false),
                    Stack = table.Column<string>(type: "TEXT", nullable: true),
                    Status = table.Column<string>(type: "TEXT", nullable: true),
                    Ports = table.Column<string>(type: "TEXT", nullable: false)
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
                    Id = table.Column<byte[]>(type: "BLOB", nullable: false),
                    PlatformId = table.Column<byte[]>(type: "BLOB", nullable: false),
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
                    Id = table.Column<byte[]>(type: "BLOB", nullable: false),
                    PlatformId = table.Column<byte[]>(type: "BLOB", nullable: false),
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
                    Id = table.Column<byte[]>(type: "BLOB", nullable: false),
                    RoleId = table.Column<byte[]>(type: "BLOB", nullable: false),
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
                    Id = table.Column<byte[]>(type: "BLOB", nullable: false),
                    RoleId = table.Column<byte[]>(type: "BLOB", nullable: false),
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
                    Id = table.Column<byte[]>(type: "BLOB", nullable: false),
                    UserId = table.Column<byte[]>(type: "BLOB", nullable: false),
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
                    Id = table.Column<byte[]>(type: "BLOB", nullable: false),
                    ContainerInfoId = table.Column<byte[]>(type: "BLOB", nullable: false),
                    Created = table.Column<long>(type: "INTEGER", nullable: false),
                    MemoryUsage = table.Column<double>(type: "REAL", nullable: true),
                    CpuUsage = table.Column<double>(type: "REAL", nullable: true),
                    MemoryLimit = table.Column<double>(type: "REAL", nullable: true),
                    RxBytes = table.Column<double>(type: "REAL", nullable: true),
                    TxBytes = table.Column<double>(type: "REAL", nullable: true)
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
                    Id = table.Column<byte[]>(type: "BLOB", nullable: false),
                    SwarmInfoId = table.Column<byte[]>(type: "BLOB", nullable: false),
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
                    UserId = table.Column<byte[]>(type: "BLOB", nullable: false),
                    TeamId = table.Column<byte[]>(type: "BLOB", nullable: false)
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
                    { new byte[] { 189, 222, 150, 1, 59, 3, 18, 117, 161, 27, 152, 83, 61, 6, 58, 4 }, new DateTime(2025, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), "Admin" },
                    { new byte[] { 190, 222, 150, 1, 148, 12, 103, 116, 155, 45, 57, 126, 2, 0, 39, 111 }, new DateTime(2025, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), "Dev" },
                    { new byte[] { 190, 222, 150, 1, 128, 45, 221, 118, 179, 81, 173, 227, 143, 162, 145, 105 }, new DateTime(2025, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), "QA" }
                });

            migrationBuilder.InsertData(
                table: "Users",
                columns: new[] { "Id", "CreatedAt", "Email", "Name", "Password" },
                values: new object[,]
                {
                    { new byte[] { 209, 222, 150, 1, 241, 19, 206, 119, 136, 78, 60, 182, 54, 236, 9, 168 }, new DateTime(2025, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), "admin@admin.com", "admin", "MVVL7tb6TKxZwACNu2a4RtOJAYtlE/9fStSFXf9/p1H0XxMm" },
                    { new byte[] { 209, 222, 150, 1, 241, 19, 251, 115, 172, 240, 24, 129, 21, 192, 28, 14 }, new DateTime(2025, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), "dev@dev.com", "dev", "PU5MutM3Zk/QgVpuB1BjYL8ShixKUIYDpUTX2HrcXuikrjp5" },
                    { new byte[] { 209, 222, 150, 1, 241, 19, 58, 116, 138, 27, 94, 36, 48, 72, 199, 126 }, new DateTime(2025, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), "qa@qa.com", "qa", "a+j1Nv1s/Sbwdbs5MpS4lWOfZQAFuDpWOpQQ2IK5w/7I5sQG" }
                });

            migrationBuilder.InsertData(
                table: "Permissions",
                columns: new[] { "Id", "PermissionCode", "RoleId" },
                values: new object[,]
                {
                    { new byte[] { 190, 222, 150, 1, 13, 58, 45, 75, 142, 31, 26, 43, 60, 77, 94, 111 }, 26, new byte[] { 190, 222, 150, 1, 148, 12, 103, 116, 155, 45, 57, 126, 2, 0, 39, 111 } },
                    { new byte[] { 190, 222, 150, 1, 21, 58, 45, 75, 142, 31, 26, 43, 60, 77, 94, 111 }, 13, new byte[] { 190, 222, 150, 1, 128, 45, 221, 118, 179, 81, 173, 227, 143, 162, 145, 105 } },
                    { new byte[] { 190, 222, 150, 1, 20, 58, 45, 75, 142, 31, 26, 43, 60, 77, 94, 111 }, 9, new byte[] { 190, 222, 150, 1, 128, 45, 221, 118, 179, 81, 173, 227, 143, 162, 145, 105 } },
                    { new byte[] { 190, 222, 150, 1, 19, 58, 45, 75, 142, 31, 26, 43, 60, 77, 94, 111 }, 5, new byte[] { 190, 222, 150, 1, 128, 45, 221, 118, 179, 81, 173, 227, 143, 162, 145, 105 } },
                    { new byte[] { 190, 222, 150, 1, 18, 58, 45, 75, 142, 31, 26, 43, 60, 77, 94, 111 }, 1, new byte[] { 190, 222, 150, 1, 128, 45, 221, 118, 179, 81, 173, 227, 143, 162, 145, 105 } },
                    { new byte[] { 190, 222, 150, 1, 24, 58, 45, 75, 142, 31, 26, 43, 60, 77, 94, 111 }, 25, new byte[] { 190, 222, 150, 1, 128, 45, 221, 118, 179, 81, 173, 227, 143, 162, 145, 105 } },
                    { new byte[] { 190, 222, 150, 1, 17, 58, 45, 75, 142, 31, 26, 43, 60, 77, 94, 111 }, 29, new byte[] { 190, 222, 150, 1, 148, 12, 103, 116, 155, 45, 57, 126, 2, 0, 39, 111 } },
                    { new byte[] { 190, 222, 150, 1, 16, 58, 45, 75, 142, 31, 26, 43, 60, 77, 94, 111 }, 30, new byte[] { 190, 222, 150, 1, 148, 12, 103, 116, 155, 45, 57, 126, 2, 0, 39, 111 } },
                    { new byte[] { 190, 222, 150, 1, 15, 58, 45, 75, 142, 31, 26, 43, 60, 77, 94, 111 }, 28, new byte[] { 190, 222, 150, 1, 148, 12, 103, 116, 155, 45, 57, 126, 2, 0, 39, 111 } },
                    { new byte[] { 190, 222, 150, 1, 14, 58, 45, 75, 142, 31, 26, 43, 60, 77, 94, 111 }, 27, new byte[] { 190, 222, 150, 1, 148, 12, 103, 116, 155, 45, 57, 126, 2, 0, 39, 111 } },
                    { new byte[] { 190, 222, 150, 1, 25, 58, 45, 75, 142, 31, 26, 43, 60, 77, 94, 111 }, 28, new byte[] { 190, 222, 150, 1, 128, 45, 221, 118, 179, 81, 173, 227, 143, 162, 145, 105 } },
                    { new byte[] { 190, 222, 150, 1, 22, 58, 45, 75, 142, 31, 26, 43, 60, 77, 94, 111 }, 17, new byte[] { 190, 222, 150, 1, 128, 45, 221, 118, 179, 81, 173, 227, 143, 162, 145, 105 } },
                    { new byte[] { 190, 222, 150, 1, 12, 58, 45, 75, 142, 31, 26, 43, 60, 77, 94, 111 }, 25, new byte[] { 190, 222, 150, 1, 148, 12, 103, 116, 155, 45, 57, 126, 2, 0, 39, 111 } },
                    { new byte[] { 190, 222, 150, 1, 10, 58, 45, 75, 142, 31, 26, 43, 60, 77, 94, 111 }, 22, new byte[] { 190, 222, 150, 1, 148, 12, 103, 116, 155, 45, 57, 126, 2, 0, 39, 111 } },
                    { new byte[] { 190, 222, 150, 1, 9, 58, 45, 75, 142, 31, 26, 43, 60, 77, 94, 111 }, 21, new byte[] { 190, 222, 150, 1, 148, 12, 103, 116, 155, 45, 57, 126, 2, 0, 39, 111 } },
                    { new byte[] { 190, 222, 150, 1, 8, 58, 45, 75, 142, 31, 26, 43, 60, 77, 94, 111 }, 20, new byte[] { 190, 222, 150, 1, 148, 12, 103, 116, 155, 45, 57, 126, 2, 0, 39, 111 } },
                    { new byte[] { 190, 222, 150, 1, 7, 58, 45, 75, 142, 31, 26, 43, 60, 77, 94, 111 }, 19, new byte[] { 190, 222, 150, 1, 148, 12, 103, 116, 155, 45, 57, 126, 2, 0, 39, 111 } },
                    { new byte[] { 190, 222, 150, 1, 6, 58, 45, 75, 142, 31, 26, 43, 60, 77, 94, 111 }, 18, new byte[] { 190, 222, 150, 1, 148, 12, 103, 116, 155, 45, 57, 126, 2, 0, 39, 111 } },
                    { new byte[] { 190, 222, 150, 1, 5, 58, 45, 75, 142, 31, 26, 43, 60, 77, 94, 111 }, 17, new byte[] { 190, 222, 150, 1, 148, 12, 103, 116, 155, 45, 57, 126, 2, 0, 39, 111 } },
                    { new byte[] { 190, 222, 150, 1, 4, 58, 45, 75, 142, 31, 26, 43, 60, 77, 94, 111 }, 16, new byte[] { 190, 222, 150, 1, 148, 12, 103, 116, 155, 45, 57, 126, 2, 0, 39, 111 } },
                    { new byte[] { 190, 222, 150, 1, 3, 58, 45, 75, 142, 31, 26, 43, 60, 77, 94, 111 }, 15, new byte[] { 190, 222, 150, 1, 148, 12, 103, 116, 155, 45, 57, 126, 2, 0, 39, 111 } },
                    { new byte[] { 190, 222, 150, 1, 2, 58, 45, 75, 142, 31, 26, 43, 60, 77, 94, 111 }, 14, new byte[] { 190, 222, 150, 1, 148, 12, 103, 116, 155, 45, 57, 126, 2, 0, 39, 111 } },
                    { new byte[] { 190, 222, 150, 1, 1, 58, 45, 75, 142, 31, 26, 43, 60, 77, 94, 111 }, 13, new byte[] { 190, 222, 150, 1, 148, 12, 103, 116, 155, 45, 57, 126, 2, 0, 39, 111 } },
                    { new byte[] { 190, 222, 150, 1, 11, 58, 45, 75, 142, 31, 26, 43, 60, 77, 94, 111 }, 24, new byte[] { 190, 222, 150, 1, 148, 12, 103, 116, 155, 45, 57, 126, 2, 0, 39, 111 } },
                    { new byte[] { 190, 222, 150, 1, 23, 58, 45, 75, 142, 31, 26, 43, 60, 77, 94, 111 }, 21, new byte[] { 190, 222, 150, 1, 128, 45, 221, 118, 179, 81, 173, 227, 143, 162, 145, 105 } }
                });

            migrationBuilder.InsertData(
                table: "Teams",
                columns: new[] { "Id", "Name", "RoleId" },
                values: new object[,]
                {
                    { new byte[] { 206, 222, 150, 1, 103, 233, 80, 125, 131, 44, 12, 160, 21, 84, 101, 161 }, "Admins", new byte[] { 189, 222, 150, 1, 59, 3, 18, 117, 161, 27, 152, 83, 61, 6, 58, 4 } },
                    { new byte[] { 206, 222, 150, 1, 103, 233, 101, 121, 175, 185, 243, 63, 242, 179, 240, 220 }, "Devs", new byte[] { 190, 222, 150, 1, 148, 12, 103, 116, 155, 45, 57, 126, 2, 0, 39, 111 } },
                    { new byte[] { 206, 222, 150, 1, 103, 233, 95, 117, 156, 106, 91, 165, 163, 69, 119, 241 }, "QA", new byte[] { 190, 222, 150, 1, 128, 45, 221, 118, 179, 81, 173, 227, 143, 162, 145, 105 } }
                });

            migrationBuilder.InsertData(
                table: "UsersTeams",
                columns: new[] { "TeamId", "UserId" },
                values: new object[,]
                {
                    { new byte[] { 206, 222, 150, 1, 103, 233, 80, 125, 131, 44, 12, 160, 21, 84, 101, 161 }, new byte[] { 209, 222, 150, 1, 241, 19, 206, 119, 136, 78, 60, 182, 54, 236, 9, 168 } },
                    { new byte[] { 206, 222, 150, 1, 103, 233, 101, 121, 175, 185, 243, 63, 242, 179, 240, 220 }, new byte[] { 209, 222, 150, 1, 241, 19, 251, 115, 172, 240, 24, 129, 21, 192, 28, 14 } },
                    { new byte[] { 206, 222, 150, 1, 103, 233, 95, 117, 156, 106, 91, 165, 163, 69, 119, 241 }, new byte[] { 209, 222, 150, 1, 241, 19, 58, 116, 138, 27, 94, 36, 48, 72, 199, 126 } }
                });

            migrationBuilder.CreateIndex(
                name: "IX_ContainerStats_ContainerInfoId",
                table: "ContainerStats",
                column: "ContainerInfoId");

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
                name: "IX_Permissions_RoleId",
                table: "Permissions",
                column: "RoleId");

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
                name: "AddressIndex",
                table: "Platforms",
                column: "Address",
                unique: true);

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
