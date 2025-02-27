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
                    Status = table.Column<int>(type: "INTEGER", nullable: false),
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
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 2, 26, 18, 44, 26, 609, DateTimeKind.Utc).AddTicks(6441)),
                    UpdatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 2, 26, 18, 44, 26, 611, DateTimeKind.Utc).AddTicks(5341))
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
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 2, 26, 18, 44, 26, 628, DateTimeKind.Utc).AddTicks(4843)),
                    UpdatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2025, 2, 26, 18, 44, 26, 628, DateTimeKind.Utc).AddTicks(6444))
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
                    { new Guid("01954393-1144-7744-8a53-f7da8e48854e"), new DateTime(2025, 2, 26, 18, 44, 26, 564, DateTimeKind.Utc).AddTicks(6446), "Administrator" },
                    { new Guid("01954393-1144-7794-80e4-4288a0838a66"), new DateTime(2025, 2, 26, 18, 44, 26, 564, DateTimeKind.Utc).AddTicks(6779), "QA" },
                    { new Guid("01954393-1144-7c5f-9f4e-6c85e1710e31"), new DateTime(2025, 2, 26, 18, 44, 26, 564, DateTimeKind.Utc).AddTicks(6776), "Developer" }
                });

            migrationBuilder.InsertData(
                table: "Users",
                columns: new[] { "Id", "CreatedAt", "Email", "Name", "Password" },
                values: new object[,]
                {
                    { new Guid("01954393-1145-7e43-83d3-e4ad4319577a"), new DateTime(2025, 2, 26, 18, 44, 26, 565, DateTimeKind.Utc).AddTicks(5330), "admin@admin.com", "admin", "QCyhWv+FCZtWUTcJBncklPXX4zpKhats4zffnYCieG/Ha18e" },
                    { new Guid("01954393-1153-7814-b5f6-69c4cf9940c7"), new DateTime(2025, 2, 26, 18, 44, 26, 579, DateTimeKind.Utc).AddTicks(212), "dev@dev.com", "dev", "qYet1K5Yq84sfH48GPH+fdR5PIZ8Ko1PuyGvavvBSC4kS22e" },
                    { new Guid("01954393-1158-7b61-b844-ac41263a1572"), new DateTime(2025, 2, 26, 18, 44, 26, 584, DateTimeKind.Utc).AddTicks(8843), "qa@qa.com", "qa", "RisHKpxJqHIE8i8i7xWhGbn+5uyZkVeIYVnKQwAc/Qv9GUg5" }
                });

            migrationBuilder.InsertData(
                table: "Permissions",
                columns: new[] { "Id", "PermissionCode", "RoleId" },
                values: new object[,]
                {
                    { new Guid("01954393-1144-7030-a3e2-83050558825c"), 15, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-7044-9185-c4dc887491e0"), 28, new Guid("01954393-1144-7c5f-9f4e-6c85e1710e31") },
                    { new Guid("01954393-1144-7186-8020-ef89ec439775"), 17, new Guid("01954393-1144-7c5f-9f4e-6c85e1710e31") },
                    { new Guid("01954393-1144-71c8-aedc-d30b62fab491"), 9, new Guid("01954393-1144-7794-80e4-4288a0838a66") },
                    { new Guid("01954393-1144-71df-ab70-407f51d827dc"), 21, new Guid("01954393-1144-7c5f-9f4e-6c85e1710e31") },
                    { new Guid("01954393-1144-7216-b6ea-195bf2500ad0"), 1, new Guid("01954393-1144-7794-80e4-4288a0838a66") },
                    { new Guid("01954393-1144-7229-bc29-16db0482c877"), 21, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-722d-b7ff-d7c5ce92fc56"), 14, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-727e-9668-85b4ab149f2e"), 26, new Guid("01954393-1144-7c5f-9f4e-6c85e1710e31") },
                    { new Guid("01954393-1144-729d-9abf-0c1a5b85575d"), 24, new Guid("01954393-1144-7c5f-9f4e-6c85e1710e31") },
                    { new Guid("01954393-1144-72b2-822d-3c91fe4e9dce"), 9, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-7356-b8cc-49ea7113d924"), 28, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-73ee-9337-267921f9206a"), 17, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-73f2-a976-7fcde05f73e2"), 18, new Guid("01954393-1144-7c5f-9f4e-6c85e1710e31") },
                    { new Guid("01954393-1144-7403-97cc-64c51ee5bb24"), 5, new Guid("01954393-1144-7c5f-9f4e-6c85e1710e31") },
                    { new Guid("01954393-1144-740c-b2d3-c3d889032058"), 13, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-7467-ab98-957de6b089b9"), 1, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-7478-8c3a-d6f5b59d7978"), 13, new Guid("01954393-1144-7c5f-9f4e-6c85e1710e31") },
                    { new Guid("01954393-1144-748e-9379-a4684971957d"), 5, new Guid("01954393-1144-7794-80e4-4288a0838a66") },
                    { new Guid("01954393-1144-7519-a68f-27fa1a50e5bd"), 3, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-753c-a2b9-59722f336b08"), 14, new Guid("01954393-1144-7c5f-9f4e-6c85e1710e31") },
                    { new Guid("01954393-1144-75e7-b152-83485e7cb0ec"), 13, new Guid("01954393-1144-7794-80e4-4288a0838a66") },
                    { new Guid("01954393-1144-762f-9f3d-66cc58f7c13f"), 25, new Guid("01954393-1144-7794-80e4-4288a0838a66") },
                    { new Guid("01954393-1144-763e-82a7-7f1a78c625ff"), 25, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-767e-8d85-00bbde6e1d19"), 11, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-76cb-ad84-c0c13ffc6502"), 27, new Guid("01954393-1144-7c5f-9f4e-6c85e1710e31") },
                    { new Guid("01954393-1144-76e0-9f7c-1612077a5b06"), 8, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-7779-b678-89c187239be9"), 5, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-7822-84c6-8de3acad2951"), 23, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-78c3-b8d3-ec567a1a5088"), 2, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-792b-8a31-c3a3986fd3b3"), 10, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-7930-a092-18021fff7d98"), 27, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-7972-883e-d68094ea6d28"), 19, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-79e0-a195-be4ac824dd24"), 18, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-7a09-8bbe-83c8eaf2d701"), 16, new Guid("01954393-1144-7c5f-9f4e-6c85e1710e31") },
                    { new Guid("01954393-1144-7a13-b396-524a763c1869"), 19, new Guid("01954393-1144-7c5f-9f4e-6c85e1710e31") },
                    { new Guid("01954393-1144-7aa1-b953-484029a56073"), 26, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-7aef-89cf-47824c28e28c"), 4, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-7aef-a3e3-605319b4b5fd"), 24, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-7b26-a4d0-433e2f3e400f"), 17, new Guid("01954393-1144-7794-80e4-4288a0838a66") },
                    { new Guid("01954393-1144-7b46-b5a1-01724e44d847"), 12, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-7b67-884b-750cb824dfbe"), 16, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-7bbd-a34b-24cb3dc9ebdb"), 21, new Guid("01954393-1144-7794-80e4-4288a0838a66") },
                    { new Guid("01954393-1144-7be0-8495-43c6812b3e8a"), 9, new Guid("01954393-1144-7c5f-9f4e-6c85e1710e31") },
                    { new Guid("01954393-1144-7c44-b6ef-efe84e6f9bdc"), 20, new Guid("01954393-1144-7c5f-9f4e-6c85e1710e31") },
                    { new Guid("01954393-1144-7c70-b0ec-2aa0a485351c"), 22, new Guid("01954393-1144-7c5f-9f4e-6c85e1710e31") },
                    { new Guid("01954393-1144-7cae-83a4-f2ff7d427b6e"), 7, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-7cd5-a984-dfd9339ab241"), 20, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-7e64-82e6-de074c287e76"), 6, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-7eb2-85d9-737526e3b357"), 22, new Guid("01954393-1144-7744-8a53-f7da8e48854e") },
                    { new Guid("01954393-1144-7f47-8759-adf88a868b1e"), 1, new Guid("01954393-1144-7c5f-9f4e-6c85e1710e31") },
                    { new Guid("01954393-1144-7f9a-b162-69cbc87d08ae"), 15, new Guid("01954393-1144-7c5f-9f4e-6c85e1710e31") }
                });

            migrationBuilder.InsertData(
                table: "Teams",
                columns: new[] { "Id", "Name", "RoleId" },
                values: new object[,]
                {
                    { new Guid("01954393-1145-7313-97fe-4709d3a28d40"), "QA", new Guid("01954393-1144-7794-80e4-4288a0838a66") },
                    { new Guid("01954393-1145-788e-835d-8159b4011015"), "Devs", new Guid("01954393-1144-7c5f-9f4e-6c85e1710e31") },
                    { new Guid("01954393-1145-7d0e-8c40-e3b960e09bb1"), "Admins", new Guid("01954393-1144-7744-8a53-f7da8e48854e") }
                });

            migrationBuilder.InsertData(
                table: "UsersTeams",
                columns: new[] { "TeamId", "UserId" },
                values: new object[,]
                {
                    { new Guid("01954393-1145-7313-97fe-4709d3a28d40"), new Guid("01954393-1158-7b61-b844-ac41263a1572") },
                    { new Guid("01954393-1145-788e-835d-8159b4011015"), new Guid("01954393-1153-7814-b5f6-69c4cf9940c7") },
                    { new Guid("01954393-1145-7d0e-8c40-e3b960e09bb1"), new Guid("01954393-1145-7e43-83d3-e4ad4319577a") }
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
