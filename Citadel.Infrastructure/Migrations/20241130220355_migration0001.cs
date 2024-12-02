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
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2024, 11, 30, 22, 3, 54, 813, DateTimeKind.Utc).AddTicks(6845)),
                    UpdatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2024, 11, 30, 22, 3, 54, 815, DateTimeKind.Utc).AddTicks(587))
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
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2024, 11, 30, 22, 3, 54, 830, DateTimeKind.Utc).AddTicks(4772)),
                    UpdatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValue: new DateTime(2024, 11, 30, 22, 3, 54, 830, DateTimeKind.Utc).AddTicks(5508))
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
                    { new Guid("01937f1a-1018-7384-a380-83286832d06b"), new DateTime(2024, 11, 30, 22, 3, 54, 776, DateTimeKind.Utc).AddTicks(8262), "Administrator" },
                    { new Guid("01937f1a-1018-74c9-bff3-21486353e14b"), new DateTime(2024, 11, 30, 22, 3, 54, 776, DateTimeKind.Utc).AddTicks(8524), "QA" },
                    { new Guid("01937f1a-1018-777c-bc3d-672acab83347"), new DateTime(2024, 11, 30, 22, 3, 54, 776, DateTimeKind.Utc).AddTicks(8522), "Developer" }
                });

            migrationBuilder.InsertData(
                table: "Users",
                columns: new[] { "Id", "CreatedAt", "Email", "Name", "Password" },
                values: new object[,]
                {
                    { new Guid("01937f1a-1019-7964-9bad-83de9b370a7c"), new DateTime(2024, 11, 30, 22, 3, 54, 777, DateTimeKind.Utc).AddTicks(4279), "admin@admin.com", "admin", "sxXMX2aY6Obc2ZvZofa3DFlDjqd/P7qzYrhAoMhUZFEugmH2" },
                    { new Guid("01937f1a-1024-7282-af3b-5ab0994c5e14"), new DateTime(2024, 11, 30, 22, 3, 54, 788, DateTimeKind.Utc).AddTicks(6357), "dev@dev.com", "dev", "lBfctv6K1XlYlbEvyT7dlTGx4sPUM7DGaAD3HPmUAYjD/jTO" },
                    { new Guid("01937f1a-1029-7659-8cb5-bd29715ed99b"), new DateTime(2024, 11, 30, 22, 3, 54, 793, DateTimeKind.Utc).AddTicks(7191), "qa@qa.com", "qa", "QPLDVMTGOn/n1n32S8mi+Ho4HNs8UnRERbiCSQGZQpfJ/1As" }
                });

            migrationBuilder.InsertData(
                table: "Permissions",
                columns: new[] { "Id", "PermissionCode", "RoleId" },
                values: new object[,]
                {
                    { new Guid("01937f1a-1018-7ffb-af8f-3fd6849983f6"), 1, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-701b-9451-979979ade515"), 3, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-7123-95b7-3014de7882ea"), 26, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-72be-b914-31af0fedf03e"), 5, new Guid("01937f1a-1018-74c9-bff3-21486353e14b") },
                    { new Guid("01937f1a-1019-72c7-9755-96a03a9eafb5"), 1, new Guid("01937f1a-1018-777c-bc3d-672acab83347") },
                    { new Guid("01937f1a-1019-732d-87b7-2d1686c629cc"), 14, new Guid("01937f1a-1018-777c-bc3d-672acab83347") },
                    { new Guid("01937f1a-1019-73c5-93b5-f6b4025d9179"), 21, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-7476-8313-6e7126699047"), 22, new Guid("01937f1a-1018-777c-bc3d-672acab83347") },
                    { new Guid("01937f1a-1019-7482-916a-13634db08665"), 18, new Guid("01937f1a-1018-777c-bc3d-672acab83347") },
                    { new Guid("01937f1a-1019-74d3-9841-374159192566"), 18, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-74fc-86c0-6f664b89c4d6"), 17, new Guid("01937f1a-1018-74c9-bff3-21486353e14b") },
                    { new Guid("01937f1a-1019-7594-91af-f5cd1549434f"), 26, new Guid("01937f1a-1018-777c-bc3d-672acab83347") },
                    { new Guid("01937f1a-1019-75ab-8527-038bf0be0a2e"), 20, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-7601-999d-c8fe39bd7abe"), 5, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-7610-9d0f-d95884aa3375"), 8, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-7628-9e51-7cab8f3172ce"), 15, new Guid("01937f1a-1018-777c-bc3d-672acab83347") },
                    { new Guid("01937f1a-1019-765d-a377-1de6ec3549d8"), 17, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-7695-9fb5-f3706adc71ed"), 21, new Guid("01937f1a-1018-777c-bc3d-672acab83347") },
                    { new Guid("01937f1a-1019-7697-91ae-b53def3ba90f"), 1, new Guid("01937f1a-1018-74c9-bff3-21486353e14b") },
                    { new Guid("01937f1a-1019-76b1-a4b0-202aed8ecd48"), 9, new Guid("01937f1a-1018-74c9-bff3-21486353e14b") },
                    { new Guid("01937f1a-1019-76d8-8adc-1d01d56544df"), 27, new Guid("01937f1a-1018-777c-bc3d-672acab83347") },
                    { new Guid("01937f1a-1019-76f7-816f-cf8b049107e7"), 5, new Guid("01937f1a-1018-777c-bc3d-672acab83347") },
                    { new Guid("01937f1a-1019-7793-9f0d-019ae6f3a4ac"), 20, new Guid("01937f1a-1018-777c-bc3d-672acab83347") },
                    { new Guid("01937f1a-1019-77bc-8c05-a4a529fb7508"), 13, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-77bd-bfbf-5f5f31a80669"), 17, new Guid("01937f1a-1018-777c-bc3d-672acab83347") },
                    { new Guid("01937f1a-1019-77fa-8023-9feec44c259c"), 15, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-781d-93be-10c779863841"), 10, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-7820-9de1-982f11bb5992"), 4, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-7840-9346-f6f57f40ab01"), 25, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-78b2-a7f6-9898494b87d5"), 7, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-78b9-b284-c4b9da2ca9b6"), 24, new Guid("01937f1a-1018-777c-bc3d-672acab83347") },
                    { new Guid("01937f1a-1019-78db-b5b2-3abdf4fe6c1b"), 14, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-7910-b324-6e8573002284"), 6, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-791b-97d7-573141cab081"), 24, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-79aa-8637-c2d20cbd7494"), 13, new Guid("01937f1a-1018-777c-bc3d-672acab83347") },
                    { new Guid("01937f1a-1019-7a5d-9ba3-3f8b91b627f5"), 28, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-7ad5-9b03-59f3f1e27d9a"), 19, new Guid("01937f1a-1018-777c-bc3d-672acab83347") },
                    { new Guid("01937f1a-1019-7bc6-b9f9-02fd86b22dcc"), 11, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-7bee-812c-700aba9dcd33"), 2, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-7c48-9556-85ebd15d7ed6"), 21, new Guid("01937f1a-1018-74c9-bff3-21486353e14b") },
                    { new Guid("01937f1a-1019-7c57-8cd8-947384bcffb2"), 13, new Guid("01937f1a-1018-74c9-bff3-21486353e14b") },
                    { new Guid("01937f1a-1019-7c84-9dc0-fc57d181ddb2"), 28, new Guid("01937f1a-1018-777c-bc3d-672acab83347") },
                    { new Guid("01937f1a-1019-7d2f-b820-fa6108206797"), 9, new Guid("01937f1a-1018-777c-bc3d-672acab83347") },
                    { new Guid("01937f1a-1019-7d3e-91eb-624371da1b3c"), 9, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-7db5-83c3-37dacfa7a7bd"), 22, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-7dcc-8594-eff75babb19b"), 16, new Guid("01937f1a-1018-777c-bc3d-672acab83347") },
                    { new Guid("01937f1a-1019-7dea-885f-8bf719dcd194"), 16, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-7e4f-9eb3-3431898c12c5"), 23, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-7eb7-b2a6-d39b4bcabc16"), 19, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-7f26-81d4-762e0a7e88ad"), 27, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-7f72-8109-993b4408fe17"), 12, new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-7fb0-877d-d247e79f1381"), 25, new Guid("01937f1a-1018-74c9-bff3-21486353e14b") }
                });

            migrationBuilder.InsertData(
                table: "Teams",
                columns: new[] { "Id", "Name", "RoleId" },
                values: new object[,]
                {
                    { new Guid("01937f1a-1019-750b-aa8d-880f29f3aff0"), "QA", new Guid("01937f1a-1018-74c9-bff3-21486353e14b") },
                    { new Guid("01937f1a-1019-76c5-8851-be3dba2306d4"), "Admins", new Guid("01937f1a-1018-7384-a380-83286832d06b") },
                    { new Guid("01937f1a-1019-7df8-a365-5ead27a06491"), "Devs", new Guid("01937f1a-1018-777c-bc3d-672acab83347") }
                });

            migrationBuilder.InsertData(
                table: "UsersTeams",
                columns: new[] { "TeamId", "UserId" },
                values: new object[,]
                {
                    { new Guid("01937f1a-1019-750b-aa8d-880f29f3aff0"), new Guid("01937f1a-1029-7659-8cb5-bd29715ed99b") },
                    { new Guid("01937f1a-1019-76c5-8851-be3dba2306d4"), new Guid("01937f1a-1019-7964-9bad-83de9b370a7c") },
                    { new Guid("01937f1a-1019-7df8-a365-5ead27a06491"), new Guid("01937f1a-1024-7282-af3b-5ab0994c5e14") }
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
