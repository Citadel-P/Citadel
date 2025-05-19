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
                    Id = table.Column<Guid>(type: "TEXT", nullable: false),
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
                    Id = table.Column<Guid>(type: "TEXT", nullable: false),
                    PlatformId = table.Column<Guid>(type: "TEXT", nullable: false),
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
                    { new Guid("0196debd-033b-7512-a11b-98533d063a04"), new DateTime(2025, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), "Admin" },
                    { new Guid("0196debe-0c94-7467-9b2d-397e0200276f"), new DateTime(2025, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), "Dev" },
                    { new Guid("0196debe-2d80-76dd-b351-ade38fa29169"), new DateTime(2025, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), "QA" }
                });

            migrationBuilder.InsertData(
                table: "Users",
                columns: new[] { "Id", "CreatedAt", "Email", "Name", "Password" },
                values: new object[,]
                {
                    { new Guid("0196ded1-13f1-73fb-acf0-188115c01c0e"), new DateTime(2025, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), "dev@dev.com", "dev", "bstIzQ7Axj+ZtX0eo89tp/8G1+oTO4BTrI+54+Ou7MKVKBwX" },
                    { new Guid("0196ded1-13f1-743a-8a1b-5e243048c77e"), new DateTime(2025, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), "qa@qa.com", "qa", "DZrfPl29P870AAeeCz/rDj2K/68NYT2gECxx5KSnEBZdjZ1i" },
                    { new Guid("0196ded1-13f1-77ce-884e-3cb636ec09a8"), new DateTime(2025, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), "admin@admin.com", "admin", "/VtxS3rzyzEIhP0i6Ehq72mGCarCK+xxCMcMXo4N5WMavIl/" }
                });

            migrationBuilder.InsertData(
                table: "Permissions",
                columns: new[] { "Id", "PermissionCode", "RoleId" },
                values: new object[,]
                {
                    { new Guid("0196debe-3a01-4b2d-8e1f-1a2b3c4d5e6f"), 13, new Guid("0196debe-0c94-7467-9b2d-397e0200276f") },
                    { new Guid("0196debe-3a02-4b2d-8e1f-1a2b3c4d5e6f"), 14, new Guid("0196debe-0c94-7467-9b2d-397e0200276f") },
                    { new Guid("0196debe-3a03-4b2d-8e1f-1a2b3c4d5e6f"), 15, new Guid("0196debe-0c94-7467-9b2d-397e0200276f") },
                    { new Guid("0196debe-3a04-4b2d-8e1f-1a2b3c4d5e6f"), 16, new Guid("0196debe-0c94-7467-9b2d-397e0200276f") },
                    { new Guid("0196debe-3a05-4b2d-8e1f-1a2b3c4d5e6f"), 17, new Guid("0196debe-0c94-7467-9b2d-397e0200276f") },
                    { new Guid("0196debe-3a06-4b2d-8e1f-1a2b3c4d5e6f"), 18, new Guid("0196debe-0c94-7467-9b2d-397e0200276f") },
                    { new Guid("0196debe-3a07-4b2d-8e1f-1a2b3c4d5e6f"), 19, new Guid("0196debe-0c94-7467-9b2d-397e0200276f") },
                    { new Guid("0196debe-3a08-4b2d-8e1f-1a2b3c4d5e6f"), 20, new Guid("0196debe-0c94-7467-9b2d-397e0200276f") },
                    { new Guid("0196debe-3a09-4b2d-8e1f-1a2b3c4d5e6f"), 21, new Guid("0196debe-0c94-7467-9b2d-397e0200276f") },
                    { new Guid("0196debe-3a0a-4b2d-8e1f-1a2b3c4d5e6f"), 22, new Guid("0196debe-0c94-7467-9b2d-397e0200276f") },
                    { new Guid("0196debe-3a0b-4b2d-8e1f-1a2b3c4d5e6f"), 24, new Guid("0196debe-0c94-7467-9b2d-397e0200276f") },
                    { new Guid("0196debe-3a0c-4b2d-8e1f-1a2b3c4d5e6f"), 25, new Guid("0196debe-0c94-7467-9b2d-397e0200276f") },
                    { new Guid("0196debe-3a0d-4b2d-8e1f-1a2b3c4d5e6f"), 26, new Guid("0196debe-0c94-7467-9b2d-397e0200276f") },
                    { new Guid("0196debe-3a0e-4b2d-8e1f-1a2b3c4d5e6f"), 27, new Guid("0196debe-0c94-7467-9b2d-397e0200276f") },
                    { new Guid("0196debe-3a0f-4b2d-8e1f-1a2b3c4d5e6f"), 28, new Guid("0196debe-0c94-7467-9b2d-397e0200276f") },
                    { new Guid("0196debe-3a10-4b2d-8e1f-1a2b3c4d5e6f"), 30, new Guid("0196debe-0c94-7467-9b2d-397e0200276f") },
                    { new Guid("0196debe-3a11-4b2d-8e1f-1a2b3c4d5e6f"), 29, new Guid("0196debe-0c94-7467-9b2d-397e0200276f") },
                    { new Guid("0196debe-3a12-4b2d-8e1f-1a2b3c4d5e6f"), 1, new Guid("0196debe-2d80-76dd-b351-ade38fa29169") },
                    { new Guid("0196debe-3a13-4b2d-8e1f-1a2b3c4d5e6f"), 5, new Guid("0196debe-2d80-76dd-b351-ade38fa29169") },
                    { new Guid("0196debe-3a14-4b2d-8e1f-1a2b3c4d5e6f"), 9, new Guid("0196debe-2d80-76dd-b351-ade38fa29169") },
                    { new Guid("0196debe-3a15-4b2d-8e1f-1a2b3c4d5e6f"), 13, new Guid("0196debe-2d80-76dd-b351-ade38fa29169") },
                    { new Guid("0196debe-3a16-4b2d-8e1f-1a2b3c4d5e6f"), 17, new Guid("0196debe-2d80-76dd-b351-ade38fa29169") },
                    { new Guid("0196debe-3a17-4b2d-8e1f-1a2b3c4d5e6f"), 21, new Guid("0196debe-2d80-76dd-b351-ade38fa29169") },
                    { new Guid("0196debe-3a18-4b2d-8e1f-1a2b3c4d5e6f"), 25, new Guid("0196debe-2d80-76dd-b351-ade38fa29169") },
                    { new Guid("0196debe-3a19-4b2d-8e1f-1a2b3c4d5e6f"), 28, new Guid("0196debe-2d80-76dd-b351-ade38fa29169") }
                });

            migrationBuilder.InsertData(
                table: "Teams",
                columns: new[] { "Id", "Name", "RoleId" },
                values: new object[,]
                {
                    { new Guid("0196dece-e967-755f-9c6a-5ba5a34577f1"), "QA", new Guid("0196debe-2d80-76dd-b351-ade38fa29169") },
                    { new Guid("0196dece-e967-7965-afb9-f33ff2b3f0dc"), "Devs", new Guid("0196debe-0c94-7467-9b2d-397e0200276f") },
                    { new Guid("0196dece-e967-7d50-832c-0ca0155465a1"), "Admins", new Guid("0196debd-033b-7512-a11b-98533d063a04") }
                });

            migrationBuilder.InsertData(
                table: "UsersTeams",
                columns: new[] { "TeamId", "UserId" },
                values: new object[,]
                {
                    { new Guid("0196dece-e967-7965-afb9-f33ff2b3f0dc"), new Guid("0196ded1-13f1-73fb-acf0-188115c01c0e") },
                    { new Guid("0196dece-e967-755f-9c6a-5ba5a34577f1"), new Guid("0196ded1-13f1-743a-8a1b-5e243048c77e") },
                    { new Guid("0196dece-e967-7d50-832c-0ca0155465a1"), new Guid("0196ded1-13f1-77ce-884e-3cb636ec09a8") }
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
