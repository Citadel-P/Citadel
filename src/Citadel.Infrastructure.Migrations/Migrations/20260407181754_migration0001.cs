using System;
using Microsoft.EntityFrameworkCore.Migrations;

#nullable disable

#pragma warning disable CA1814 // Prefer jagged arrays over multidimensional

namespace Infrastructure.Migrations.Migrations
{
    /// <inheritdoc />
    public partial class migration0001 : Migration
    {
        /// <inheritdoc />
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.CreateTable(
                name: "Actors",
                columns: table => new
                {
                    Id = table.Column<string>(type: "TEXT", nullable: false),
                    IsEnabled = table.Column<bool>(type: "INTEGER", nullable: false, defaultValue: true),
                    Type = table.Column<string>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Actors", x => x.Id);
                });

            migrationBuilder.CreateTable(
                name: "Platforms",
                columns: table => new
                {
                    Id = table.Column<string>(type: "TEXT", nullable: false),
                    Address = table.Column<string>(type: "TEXT", nullable: false),
                    AgentVersion = table.Column<string>(type: "TEXT", nullable: true),
                    ConnectorType = table.Column<string>(type: "TEXT", nullable: false),
                    CpuCount = table.Column<int>(type: "REAL", nullable: false),
                    ImageCount = table.Column<int>(type: "REAL", nullable: false),
                    MemTotal = table.Column<long>(type: "REAL", nullable: false),
                    Name = table.Column<string>(type: "TEXT", nullable: false),
                    NetworkCount = table.Column<int>(type: "REAL", nullable: false),
                    PlatformDescriptor = table.Column<string>(type: "TEXT", nullable: false),
                    ServerVersion = table.Column<string>(type: "TEXT", nullable: true),
                    Status = table.Column<string>(type: "TEXT", nullable: false),
                    VolumeCount = table.Column<int>(type: "REAL", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Platforms", x => x.Id);
                });

            migrationBuilder.CreateTable(
                name: "ResourceAccesses",
                columns: table => new
                {
                    Id = table.Column<string>(type: "TEXT", nullable: false),
                    Action = table.Column<string>(type: "TEXT", nullable: false),
                    ActorId = table.Column<string>(type: "TEXT", nullable: false),
                    ResourceId = table.Column<string>(type: "TEXT", nullable: false),
                    ResourceType = table.Column<string>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_ResourceAccesses", x => x.Id);
                });

            migrationBuilder.CreateTable(
                name: "Roles",
                columns: table => new
                {
                    Id = table.Column<string>(type: "TEXT", nullable: false),
                    Name = table.Column<string>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Roles", x => x.Id);
                });

            migrationBuilder.CreateTable(
                name: "AlertChannels",
                columns: table => new
                {
                    Id = table.Column<string>(type: "TEXT", nullable: false),
                    AlertDestination = table.Column<string>(type: "TEXT", nullable: false),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    CreatedByActorId = table.Column<string>(type: "TEXT", nullable: false),
                    IsActive = table.Column<bool>(type: "INTEGER", nullable: false, defaultValue: true),
                    Name = table.Column<string>(type: "TEXT", nullable: false),
                    Url = table.Column<string>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_AlertChannels", x => x.Id);
                    table.ForeignKey(
                        name: "FK_AlertChannels_Actors_CreatedByActorId",
                        column: x => x.CreatedByActorId,
                        principalTable: "Actors",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "AlertRules",
                columns: table => new
                {
                    Id = table.Column<string>(type: "TEXT", nullable: false),
                    CooldownSeconds = table.Column<int>(type: "INTEGER", nullable: true),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    CreatedByActorId = table.Column<string>(type: "TEXT", nullable: false),
                    Description = table.Column<string>(type: "TEXT", maxLength: 600, nullable: true),
                    LimitedTo = table.Column<string>(type: "TEXT", nullable: false),
                    Name = table.Column<string>(type: "TEXT", maxLength: 120, nullable: false),
                    QuietHours = table.Column<string>(type: "TEXT", nullable: false),
                    RequiredMatches = table.Column<int>(type: "INTEGER", nullable: true),
                    Severity = table.Column<string>(type: "TEXT", nullable: false),
                    Status = table.Column<string>(type: "TEXT", nullable: false, defaultValue: "Enabled"),
                    Threshold = table.Column<double>(type: "REAL", nullable: true),
                    Type = table.Column<string>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_AlertRules", x => x.Id);
                    table.ForeignKey(
                        name: "FK_AlertRules_Actors_CreatedByActorId",
                        column: x => x.CreatedByActorId,
                        principalTable: "Actors",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "GitAccounts",
                columns: table => new
                {
                    Id = table.Column<string>(type: "TEXT", nullable: false),
                    AuthType = table.Column<string>(type: "TEXT", nullable: false),
                    Configuration = table.Column<string>(type: "TEXT", nullable: false),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    CreatedByActorId = table.Column<string>(type: "TEXT", nullable: false),
                    Domain = table.Column<string>(type: "TEXT", nullable: false),
                    Name = table.Column<string>(type: "TEXT", nullable: false),
                    Transport = table.Column<string>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_GitAccounts", x => x.Id);
                    table.ForeignKey(
                        name: "FK_GitAccounts_Actors_CreatedByActorId",
                        column: x => x.CreatedByActorId,
                        principalTable: "Actors",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "Registries",
                columns: table => new
                {
                    Id = table.Column<string>(type: "TEXT", nullable: false),
                    Configuration = table.Column<string>(type: "TEXT", nullable: false),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    CreatedByActorId = table.Column<string>(type: "TEXT", nullable: false),
                    Description = table.Column<string>(type: "TEXT", maxLength: 600, nullable: true),
                    Name = table.Column<string>(type: "TEXT", nullable: false),
                    RegistryHost = table.Column<string>(type: "TEXT", nullable: false),
                    Status = table.Column<string>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Registries", x => x.Id);
                    table.ForeignKey(
                        name: "FK_Registries_Actors_CreatedByActorId",
                        column: x => x.CreatedByActorId,
                        principalTable: "Actors",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "Stacks",
                columns: table => new
                {
                    Id = table.Column<string>(type: "TEXT", nullable: false),
                    ControlStartedAt = table.Column<long>(type: "INTEGER", nullable: true),
                    ControlState = table.Column<string>(type: "TEXT", maxLength: 64, nullable: true, defaultValue: "Idle"),
                    ControlTriggeredBy = table.Column<string>(type: "TEXT", nullable: true),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    CreatedByActorId = table.Column<string>(type: "TEXT", nullable: false),
                    CurrentStackReleaseId = table.Column<string>(type: "TEXT", nullable: true),
                    Description = table.Column<string>(type: "TEXT", maxLength: 600, nullable: true),
                    Name = table.Column<string>(type: "TEXT", nullable: false),
                    RowVersion = table.Column<long>(type: "INTEGER", nullable: false, defaultValue: 0L),
                    StackSource = table.Column<string>(type: "TEXT", nullable: false),
                    StackUpdateState = table.Column<string>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Stacks", x => x.Id);
                    table.ForeignKey(
                        name: "FK_Stacks_Actors_ControlTriggeredBy",
                        column: x => x.ControlTriggeredBy,
                        principalTable: "Actors",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_Stacks_Actors_CreatedByActorId",
                        column: x => x.CreatedByActorId,
                        principalTable: "Actors",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "Teams",
                columns: table => new
                {
                    Id = table.Column<string>(type: "TEXT", nullable: false),
                    ActorId = table.Column<string>(type: "TEXT", nullable: false),
                    Name = table.Column<string>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Teams", x => x.Id);
                    table.ForeignKey(
                        name: "FK_Teams_Actors_ActorId",
                        column: x => x.ActorId,
                        principalTable: "Actors",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "Users",
                columns: table => new
                {
                    Id = table.Column<string>(type: "TEXT", nullable: false),
                    ActorId = table.Column<string>(type: "TEXT", nullable: false),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    CreatedByActorId = table.Column<string>(type: "TEXT", nullable: false),
                    Email = table.Column<string>(type: "TEXT", nullable: true),
                    Name = table.Column<string>(type: "TEXT", maxLength: 100, nullable: false),
                    Password = table.Column<string>(type: "TEXT", nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Users", x => x.Id);
                    table.ForeignKey(
                        name: "FK_Users_Actors_ActorId",
                        column: x => x.ActorId,
                        principalTable: "Actors",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_Users_Actors_CreatedByActorId",
                        column: x => x.CreatedByActorId,
                        principalTable: "Actors",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "ActivityEvents",
                columns: table => new
                {
                    Id = table.Column<string>(type: "TEXT", nullable: false),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    CreatedByActorId = table.Column<string>(type: "TEXT", nullable: false),
                    EventType = table.Column<string>(type: "TEXT", nullable: false),
                    Info = table.Column<string>(type: "TEXT", nullable: false),
                    PlatformId = table.Column<string>(type: "TEXT", nullable: true),
                    ResourceId = table.Column<string>(type: "TEXT", nullable: true),
                    ResourceName = table.Column<string>(type: "TEXT", nullable: false),
                    ResourceType = table.Column<string>(type: "TEXT", nullable: false),
                    Status = table.Column<string>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_ActivityEvents", x => x.Id);
                    table.ForeignKey(
                        name: "FK_ActivityEvents_Actors_CreatedByActorId",
                        column: x => x.CreatedByActorId,
                        principalTable: "Actors",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_ActivityEvents_Platforms_PlatformId",
                        column: x => x.PlatformId,
                        principalTable: "Platforms",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "Deployments",
                columns: table => new
                {
                    Id = table.Column<string>(type: "TEXT", nullable: false),
                    AutoUpdateState_CurrentDigest = table.Column<string>(type: "TEXT", nullable: true),
                    AutoUpdateState_LastCheckedAt = table.Column<DateTime>(type: "TEXT", nullable: true),
                    AutoUpdateState_LastError = table.Column<string>(type: "TEXT", maxLength: 2000, nullable: true),
                    AutoUpdateState_RemoteDigest = table.Column<string>(type: "TEXT", nullable: true),
                    AutoUpdateState_Status = table.Column<string>(type: "TEXT", nullable: true),
                    ControlStartedAt = table.Column<long>(type: "INTEGER", nullable: true),
                    ControlState = table.Column<string>(type: "TEXT", maxLength: 64, nullable: true, defaultValue: "Idle"),
                    ControlTriggeredBy = table.Column<string>(type: "TEXT", nullable: true),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    CreatedByActorId = table.Column<string>(type: "TEXT", nullable: false),
                    Description = table.Column<string>(type: "TEXT", maxLength: 600, nullable: true),
                    Name = table.Column<string>(type: "TEXT", nullable: false),
                    PlatformId = table.Column<string>(type: "TEXT", nullable: false),
                    RowVersion = table.Column<long>(type: "INTEGER", nullable: false, defaultValue: 0L),
                    Spec = table.Column<string>(type: "TEXT", nullable: false),
                    Status = table.Column<string>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Deployments", x => x.Id);
                    table.ForeignKey(
                        name: "FK_Deployments_Actors_ControlTriggeredBy",
                        column: x => x.ControlTriggeredBy,
                        principalTable: "Actors",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_Deployments_Actors_CreatedByActorId",
                        column: x => x.CreatedByActorId,
                        principalTable: "Actors",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_Deployments_Platforms_PlatformId",
                        column: x => x.PlatformId,
                        principalTable: "Platforms",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "PlatformStats",
                columns: table => new
                {
                    Id = table.Column<string>(type: "TEXT", nullable: false),
                    CpuUsage = table.Column<double>(type: "REAL", nullable: false),
                    Created = table.Column<long>(type: "REAL", nullable: false),
                    MemoryUsage = table.Column<double>(type: "REAL", nullable: false),
                    PlatformId = table.Column<string>(type: "TEXT", nullable: false),
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
                name: "ActorRoles",
                columns: table => new
                {
                    ActorId = table.Column<string>(type: "TEXT", nullable: false),
                    RoleId = table.Column<string>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_ActorRoles", x => new { x.ActorId, x.RoleId });
                    table.ForeignKey(
                        name: "FK_ActorRoles_Actors_ActorId",
                        column: x => x.ActorId,
                        principalTable: "Actors",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Cascade);
                    table.ForeignKey(
                        name: "FK_ActorRoles_Roles_RoleId",
                        column: x => x.RoleId,
                        principalTable: "Roles",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "Permissions",
                columns: table => new
                {
                    Id = table.Column<string>(type: "TEXT", nullable: false),
                    ResourceAction = table.Column<string>(type: "TEXT", nullable: false),
                    ResourceType = table.Column<string>(type: "TEXT", nullable: false),
                    RoleId = table.Column<string>(type: "TEXT", nullable: false)
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
                name: "AlertEvents",
                columns: table => new
                {
                    Id = table.Column<string>(type: "TEXT", nullable: false),
                    AcknowledgedAt = table.Column<DateTime>(type: "TEXT", nullable: true),
                    AcknowledgedByActorId = table.Column<string>(type: "TEXT", nullable: true),
                    AlertRuleId = table.Column<string>(type: "TEXT", nullable: false),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    DeduplicationKey = table.Column<string>(type: "TEXT", nullable: false),
                    Info = table.Column<string>(type: "TEXT", nullable: false),
                    OpenIncidentKey = table.Column<string>(type: "TEXT", nullable: true),
                    ResolutionNote = table.Column<string>(type: "TEXT", nullable: true),
                    ResolvedAt = table.Column<DateTime>(type: "TEXT", nullable: true),
                    ResolvedByActorId = table.Column<string>(type: "TEXT", nullable: true),
                    ResourceId = table.Column<string>(type: "TEXT", nullable: true),
                    ResourceName = table.Column<string>(type: "TEXT", nullable: false),
                    ResourceType = table.Column<string>(type: "TEXT", nullable: false),
                    Severity = table.Column<string>(type: "TEXT", nullable: false),
                    Type = table.Column<string>(type: "TEXT", nullable: false),
                    UpdatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP")
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_AlertEvents", x => x.Id);
                    table.ForeignKey(
                        name: "FK_AlertEvents_AlertRules_AlertRuleId",
                        column: x => x.AlertRuleId,
                        principalTable: "AlertRules",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "AlertRuleChannels",
                columns: table => new
                {
                    AlertRuleId = table.Column<string>(type: "TEXT", nullable: false),
                    AlertChannelId = table.Column<string>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_AlertRuleChannels", x => new { x.AlertRuleId, x.AlertChannelId });
                    table.ForeignKey(
                        name: "FK_AlertRuleChannels_AlertChannels_AlertChannelId",
                        column: x => x.AlertChannelId,
                        principalTable: "AlertChannels",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Cascade);
                    table.ForeignKey(
                        name: "FK_AlertRuleChannels_AlertRules_AlertRuleId",
                        column: x => x.AlertRuleId,
                        principalTable: "AlertRules",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "AlertRuleStates",
                columns: table => new
                {
                    AlertRuleId = table.Column<string>(type: "TEXT", nullable: false),
                    ResourceId = table.Column<string>(type: "TEXT", nullable: false),
                    ConsecutiveMatches = table.Column<int>(type: "INTEGER", nullable: false, defaultValue: 3),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    CreatedByActorId = table.Column<string>(type: "TEXT", nullable: false),
                    LastTriggeredAt = table.Column<DateTime>(type: "TEXT", nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_AlertRuleStates", x => new { x.AlertRuleId, x.ResourceId });
                    table.ForeignKey(
                        name: "FK_AlertRuleStates_Actors_CreatedByActorId",
                        column: x => x.CreatedByActorId,
                        principalTable: "Actors",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_AlertRuleStates_AlertRules_AlertRuleId",
                        column: x => x.AlertRuleId,
                        principalTable: "AlertRules",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "GitRepositories",
                columns: table => new
                {
                    Id = table.Column<string>(type: "TEXT", nullable: false),
                    ControlStartedAt = table.Column<long>(type: "INTEGER", nullable: true),
                    ControlState = table.Column<string>(type: "TEXT", maxLength: 64, nullable: true, defaultValue: "Idle"),
                    ControlTriggeredBy = table.Column<string>(type: "TEXT", nullable: true),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    CreatedByActorId = table.Column<string>(type: "TEXT", nullable: false),
                    DefaultBranch = table.Column<string>(type: "TEXT", nullable: false),
                    Description = table.Column<string>(type: "TEXT", maxLength: 600, nullable: true),
                    GitAccountId = table.Column<string>(type: "TEXT", nullable: true),
                    Name = table.Column<string>(type: "TEXT", nullable: false),
                    OnClone = table.Column<string>(type: "TEXT", nullable: true),
                    OnPull = table.Column<string>(type: "TEXT", nullable: true),
                    RowVersion = table.Column<long>(type: "INTEGER", nullable: false, defaultValue: 0L),
                    Status = table.Column<string>(type: "TEXT", nullable: false),
                    Url = table.Column<string>(type: "TEXT", nullable: false),
                    WebHookEnabled = table.Column<bool>(type: "INTEGER", nullable: false, defaultValue: false),
                    WebHookSecret = table.Column<string>(type: "TEXT", nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_GitRepositories", x => x.Id);
                    table.ForeignKey(
                        name: "FK_GitRepositories_Actors_ControlTriggeredBy",
                        column: x => x.ControlTriggeredBy,
                        principalTable: "Actors",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_GitRepositories_Actors_CreatedByActorId",
                        column: x => x.CreatedByActorId,
                        principalTable: "Actors",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_GitRepositories_GitAccounts_GitAccountId",
                        column: x => x.GitAccountId,
                        principalTable: "GitAccounts",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.SetNull);
                });

            migrationBuilder.CreateTable(
                name: "Images",
                columns: table => new
                {
                    Id = table.Column<string>(type: "TEXT", nullable: false),
                    Containers = table.Column<int>(type: "INTEGER", nullable: false, defaultValue: 0),
                    ControlStartedAt = table.Column<long>(type: "INTEGER", nullable: true),
                    ControlState = table.Column<string>(type: "TEXT", maxLength: 64, nullable: true, defaultValue: "Idle"),
                    ControlTriggeredBy = table.Column<string>(type: "TEXT", nullable: true),
                    CreatedAt = table.Column<string>(type: "TEXT", nullable: false),
                    DockerImageId = table.Column<string>(type: "TEXT", nullable: false),
                    Name = table.Column<string>(type: "TEXT", nullable: false),
                    PlatformId = table.Column<string>(type: "TEXT", nullable: false),
                    RegistryId = table.Column<string>(type: "TEXT", nullable: true),
                    RowVersion = table.Column<long>(type: "INTEGER", nullable: false, defaultValue: 0L),
                    Size = table.Column<double>(type: "REAL", nullable: false, defaultValue: 0.0),
                    Tags = table.Column<string>(type: "TEXT", nullable: false),
                    UpdatedAt = table.Column<string>(type: "TEXT", nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Images", x => x.Id);
                    table.ForeignKey(
                        name: "FK_Images_Actors_ControlTriggeredBy",
                        column: x => x.ControlTriggeredBy,
                        principalTable: "Actors",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_Images_Platforms_PlatformId",
                        column: x => x.PlatformId,
                        principalTable: "Platforms",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Cascade);
                    table.ForeignKey(
                        name: "FK_Images_Registries_RegistryId",
                        column: x => x.RegistryId,
                        principalTable: "Registries",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.SetNull);
                });

            migrationBuilder.CreateTable(
                name: "StackReleases",
                columns: table => new
                {
                    Id = table.Column<string>(type: "TEXT", nullable: false),
                    CreatedAt = table.Column<DateTime>(type: "TEXT", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    CreatedByActorId = table.Column<string>(type: "TEXT", nullable: false),
                    PlatformId = table.Column<string>(type: "TEXT", nullable: false),
                    Spec = table.Column<string>(type: "TEXT", nullable: false),
                    StackId = table.Column<string>(type: "TEXT", nullable: false),
                    Status = table.Column<string>(type: "TEXT", nullable: false),
                    Version = table.Column<string>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_StackReleases", x => x.Id);
                    table.ForeignKey(
                        name: "FK_StackReleases_Actors_CreatedByActorId",
                        column: x => x.CreatedByActorId,
                        principalTable: "Actors",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_StackReleases_Platforms_PlatformId",
                        column: x => x.PlatformId,
                        principalTable: "Platforms",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_StackReleases_Stacks_StackId",
                        column: x => x.StackId,
                        principalTable: "Stacks",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "RefreshTokens",
                columns: table => new
                {
                    Id = table.Column<string>(type: "TEXT", nullable: false),
                    CreatedAt = table.Column<string>(type: "TEXT", nullable: false),
                    UserId = table.Column<string>(type: "TEXT", nullable: false)
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
                name: "UsersTeams",
                columns: table => new
                {
                    UserId = table.Column<string>(type: "TEXT", nullable: false),
                    TeamId = table.Column<string>(type: "TEXT", nullable: false)
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

            migrationBuilder.CreateTable(
                name: "Containers",
                columns: table => new
                {
                    Id = table.Column<string>(type: "TEXT", nullable: false),
                    ControlStartedAt = table.Column<long>(type: "INTEGER", nullable: true),
                    ControlState = table.Column<string>(type: "TEXT", maxLength: 64, nullable: true, defaultValue: "Idle"),
                    ControlTriggeredBy = table.Column<string>(type: "TEXT", nullable: true),
                    Created = table.Column<long>(type: "REAL", nullable: false),
                    DeploymentId = table.Column<string>(type: "TEXT", nullable: true),
                    DockerContainerId = table.Column<string>(type: "TEXT", maxLength: 64, nullable: false),
                    DockerImageId = table.Column<string>(type: "TEXT", nullable: false),
                    ImageId = table.Column<string>(type: "TEXT", nullable: true),
                    Name = table.Column<string>(type: "TEXT", nullable: false),
                    PlatformId = table.Column<string>(type: "TEXT", nullable: false),
                    Ports = table.Column<string>(type: "TEXT", nullable: false),
                    RowVersion = table.Column<long>(type: "INTEGER", nullable: false, defaultValue: 0L),
                    Stack = table.Column<string>(type: "TEXT", nullable: true),
                    State = table.Column<string>(type: "TEXT", nullable: false),
                    Updated = table.Column<string>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Containers", x => x.Id);
                    table.ForeignKey(
                        name: "FK_Containers_Actors_ControlTriggeredBy",
                        column: x => x.ControlTriggeredBy,
                        principalTable: "Actors",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "FK_Containers_Deployments_DeploymentId",
                        column: x => x.DeploymentId,
                        principalTable: "Deployments",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.SetNull);
                    table.ForeignKey(
                        name: "FK_Containers_Images_ImageId",
                        column: x => x.ImageId,
                        principalTable: "Images",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.SetNull);
                    table.ForeignKey(
                        name: "FK_Containers_Platforms_PlatformId",
                        column: x => x.PlatformId,
                        principalTable: "Platforms",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "ContainerStats",
                columns: table => new
                {
                    Id = table.Column<string>(type: "TEXT", nullable: false),
                    ContainerId = table.Column<string>(type: "TEXT", nullable: false),
                    CpuUsage = table.Column<double>(type: "REAL", nullable: false),
                    Created = table.Column<long>(type: "REAL", nullable: false),
                    MemoryActive = table.Column<double>(type: "REAL", nullable: false),
                    MemoryCache = table.Column<double>(type: "REAL", nullable: false),
                    MemoryLimit = table.Column<double>(type: "REAL", nullable: false),
                    RxBytes = table.Column<double>(type: "REAL", nullable: false),
                    TxBytes = table.Column<double>(type: "REAL", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_ContainerStats", x => x.Id);
                    table.ForeignKey(
                        name: "FK_ContainerStats_Containers_ContainerId",
                        column: x => x.ContainerId,
                        principalTable: "Containers",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.InsertData(
                table: "Actors",
                columns: new[] { "Id", "IsEnabled", "Type" },
                values: new object[,]
                {
                    { "00000000-0000-0000-0000-000000000001", true, "System" },
                    { "00000000-0000-0000-0000-000000000002", true, "User" },
                    { "00000000-0000-0000-0000-000000000003", true, "Team" }
                });

            migrationBuilder.InsertData(
                table: "Roles",
                columns: new[] { "Id", "Name" },
                values: new object[,]
                {
                    { "30000000-0000-0000-0000-000000000001", "Admin" },
                    { "30000000-0000-0000-0000-000000000002", "Operator" },
                    { "30000000-0000-0000-0000-000000000003", "Viewer" }
                });

            migrationBuilder.InsertData(
                table: "ActorRoles",
                columns: new[] { "ActorId", "RoleId" },
                values: new object[,]
                {
                    { "00000000-0000-0000-0000-000000000002", "30000000-0000-0000-0000-000000000001" },
                    { "00000000-0000-0000-0000-000000000003", "30000000-0000-0000-0000-000000000002" }
                });

            migrationBuilder.InsertData(
                table: "AlertRules",
                columns: new[] { "Id", "CooldownSeconds", "CreatedAt", "CreatedByActorId", "Description", "LimitedTo", "Name", "QuietHours", "RequiredMatches", "Severity", "Threshold", "Type" },
                values: new object[,]
                {
                    { "019d0000-0001-7000-8001-000000000001", 300, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Unspecified), "00000000-0000-0000-0000-000000000001", null, "[]", "CPU > 90% - Platform", "[]", 3, "Critical", 90.0, "PlatformCpuHigh" },
                    { "019d0000-0001-7000-8001-000000000002", 300, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Unspecified), "00000000-0000-0000-0000-000000000001", null, "[]", "RAM > 90% - Platform", "[]", 3, "Critical", 90.0, "PlatformRamHigh" },
                    { "019d0000-0001-7000-8001-000000000003", 600, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Unspecified), "00000000-0000-0000-0000-000000000001", null, "[]", "Platform Unreachable", "[]", null, "Critical", null, "PlatformUnreachable" },
                    { "019d0000-0001-7000-8001-000000000004", 3600, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Unspecified), "00000000-0000-0000-0000-000000000001", null, "[]", "Platform Version Mismatch", "[]", null, "Warning", null, "PlatformVersionMismatch" },
                    { "019d0000-0001-7000-8001-000000000005", null, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Unspecified), "00000000-0000-0000-0000-000000000001", null, "[]", "Unmanaged Container Created", "[]", null, "Info", null, "UnmanagedContainerCreated" },
                    { "019d0000-0001-7000-8001-000000000006", 86400, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Unspecified), "00000000-0000-0000-0000-000000000001", null, "[]", "Image Update Available - Deployment", "[]", null, "Info", null, "DeploymentImageUpdateAvailable" },
                    { "019d0000-0001-7000-8001-000000000007", null, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Unspecified), "00000000-0000-0000-0000-000000000001", null, "[]", "Auto Deploy Failed - Deployment", "[]", null, "Critical", null, "DeploymentAutoDeployFailed" },
                    { "019d0000-0001-7000-8001-000000000008", null, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Unspecified), "00000000-0000-0000-0000-000000000001", null, "[]", "Deployment Auto Updated", "[]", null, "Info", null, "DeploymentAutoUpdated" },
                    { "019d0000-0001-7000-8001-000000000009", 86400, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Unspecified), "00000000-0000-0000-0000-000000000001", null, "[]", "Image Update Available - Stack", "[]", null, "Info", null, "StackImageUpdateAvailable" },
                    { "019d0000-0001-7000-8001-00000000000a", null, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Unspecified), "00000000-0000-0000-0000-000000000001", null, "[]", "Auto Deploy Failed - Stack", "[]", null, "Critical", null, "StackAutoDeployFailed" },
                    { "019d0000-0001-7000-8001-00000000000b", null, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Unspecified), "00000000-0000-0000-0000-000000000001", null, "[]", "Stack Auto Updated", "[]", null, "Info", null, "StackAutoUpdated" },
                    { "019d0000-0001-7000-8001-000000000011", 300, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Unspecified), "00000000-0000-0000-0000-000000000001", null, "[]", "CPU > 80% - Platform", "[]", 3, "Warning", 80.0, "PlatformCpuHigh" },
                    { "019d0000-0001-7000-8001-000000000022", 300, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Unspecified), "00000000-0000-0000-0000-000000000001", null, "[]", "RAM > 80% - Platform", "[]", 3, "Warning", 80.0, "PlatformRamHigh" }
                });

            migrationBuilder.InsertData(
                table: "Permissions",
                columns: new[] { "Id", "ResourceAction", "ResourceType", "RoleId" },
                values: new object[,]
                {
                    { "017b5a65-e312-8a67-b7fd-6124f6ddeeee", "View", "GitRepository", "30000000-0000-0000-0000-000000000001" },
                    { "0292fdeb-9a33-5a4c-60e9-395eac821cdc", "Delete", "User", "30000000-0000-0000-0000-000000000001" },
                    { "0309bcb2-05ec-623d-e45b-ec10cfddee24", "Create", "Role", "30000000-0000-0000-0000-000000000002" },
                    { "052e45fb-cb15-9380-6a33-c233fde703a6", "Update", "GitAccount", "30000000-0000-0000-0000-000000000001" },
                    { "0854c122-21bc-147b-506b-6caf72ac48ca", "View", "Team", "30000000-0000-0000-0000-000000000003" },
                    { "0dd6ded1-5ef7-c1b5-a36a-d0de73459d8a", "Apply", "Registry", "30000000-0000-0000-0000-000000000002" },
                    { "0eda225f-cb5b-1bb0-9525-be92b14fc322", "Apply", "GitRepository", "30000000-0000-0000-0000-000000000001" },
                    { "0edd69d5-bb37-653c-9b25-5ff32a2b8243", "Apply", "Role", "30000000-0000-0000-0000-000000000001" },
                    { "0f030749-53d1-bade-8ef3-30112991786d", "Create", "Stack", "30000000-0000-0000-0000-000000000002" },
                    { "117176b6-ca23-e53d-d996-83affab7ed48", "View", "Stack", "30000000-0000-0000-0000-000000000001" },
                    { "12bbcf07-7237-3afb-65f7-1fc8e2de4939", "Apply", "Registry", "30000000-0000-0000-0000-000000000001" },
                    { "15923c89-875e-7d0b-b80b-96af1f0cd1f2", "Apply", "Role", "30000000-0000-0000-0000-000000000002" },
                    { "18255099-e963-802b-21a3-d115440e9322", "Pull", "Deployment", "30000000-0000-0000-0000-000000000002" },
                    { "18b8b740-528c-6366-9902-ebf6a025d063", "Apply", "Alert", "30000000-0000-0000-0000-000000000002" },
                    { "19357866-b0f7-4c0b-fb01-c3a6556d1e5f", "Apply", "AlertChannel", "30000000-0000-0000-0000-000000000001" },
                    { "1a0b6504-d508-0f6d-4513-12b80c3ab4d8", "Update", "Platform", "30000000-0000-0000-0000-000000000002" },
                    { "1f00afd4-a4d1-94cd-3a94-44d420eef066", "Apply", "GitRepository", "30000000-0000-0000-0000-000000000002" },
                    { "21d8c7f7-5480-5509-e2ab-e3c8fdbb5ab8", "Update", "AlertChannel", "30000000-0000-0000-0000-000000000001" },
                    { "2548763c-c9b7-5359-a80a-5ce706c5c42c", "Update", "Alert", "30000000-0000-0000-0000-000000000001" },
                    { "258c5870-adb0-f28f-bed2-5c993e5d11da", "Pull", "GitRepository", "30000000-0000-0000-0000-000000000002" },
                    { "26bb8bc9-e526-dfd5-c3cc-2ebc0fb9837b", "Apply", "Alert", "30000000-0000-0000-0000-000000000001" },
                    { "2e0582c8-9569-22cd-c777-69f03893b8ec", "Delete", "GitRepository", "30000000-0000-0000-0000-000000000001" },
                    { "30f18293-2d41-2525-3210-e0f83bafa13d", "Update", "Stack", "30000000-0000-0000-0000-000000000001" },
                    { "3a14f868-33fb-3a2e-92e1-5579da6962ce", "Pull", "Role", "30000000-0000-0000-0000-000000000002" },
                    { "3a8c08b1-d033-1580-65f9-a1cb3ed3fc6a", "Create", "Registry", "30000000-0000-0000-0000-000000000002" },
                    { "3d84b4f0-2433-c34e-45e1-84d18b6c155d", "Pull", "Platform", "30000000-0000-0000-0000-000000000002" },
                    { "3e1abbe9-b2f2-21b8-bf02-38d4c10cd79d", "Create", "Role", "30000000-0000-0000-0000-000000000001" },
                    { "3e5365c6-7759-a0ad-e7fa-32263d00682c", "View", "GitAccount", "30000000-0000-0000-0000-000000000003" },
                    { "40aadb71-124b-7c1b-44b9-f507a69ade11", "Pull", "Stack", "30000000-0000-0000-0000-000000000001" },
                    { "4711987b-af34-12f7-4cf4-795f51049571", "Pull", "Platform", "30000000-0000-0000-0000-000000000001" },
                    { "47a25f07-9bb1-361d-788e-4d99fd0e50ee", "Update", "User", "30000000-0000-0000-0000-000000000002" },
                    { "47c763f4-71e9-2992-3223-8ab97876b727", "Apply", "Platform", "30000000-0000-0000-0000-000000000001" },
                    { "49ce8531-88f1-5ed2-a1c7-95d40cc72c47", "Create", "User", "30000000-0000-0000-0000-000000000001" },
                    { "4aedeb0a-de43-b841-5970-9743bcde952b", "Create", "Stack", "30000000-0000-0000-0000-000000000001" },
                    { "4b68a9cf-0af5-b0d6-8c05-e8b7e98b3919", "View", "Stack", "30000000-0000-0000-0000-000000000002" },
                    { "4cfe0dcb-ce92-0500-981f-7d79b3782877", "Create", "Alert", "30000000-0000-0000-0000-000000000002" },
                    { "4fa196f3-a9e5-7716-b1dd-aa574061e1f7", "Pull", "GitAccount", "30000000-0000-0000-0000-000000000002" },
                    { "510688a3-f3e5-851a-29fb-8c8ae66a06d5", "View", "User", "30000000-0000-0000-0000-000000000001" },
                    { "55228106-7ae9-6748-5a33-73e253ad940d", "Apply", "AlertChannel", "30000000-0000-0000-0000-000000000002" },
                    { "57de5bf0-3ba6-3067-d4d0-c8c6a889507b", "View", "Role", "30000000-0000-0000-0000-000000000003" },
                    { "58e018b1-ba4c-56bb-c56c-b9473688127b", "Pull", "GitRepository", "30000000-0000-0000-0000-000000000001" },
                    { "5e169a67-b789-1d24-db33-ea48a69f362e", "Create", "Team", "30000000-0000-0000-0000-000000000002" },
                    { "5e8afc50-270c-4413-c5f1-bd25dac435d9", "Apply", "Stack", "30000000-0000-0000-0000-000000000002" },
                    { "5ede29a8-8e33-2d7c-48c3-1e5d733edd73", "View", "AlertChannel", "30000000-0000-0000-0000-000000000003" },
                    { "613e9000-da2c-b4e7-9e02-b4bef349f0f7", "Pull", "AlertChannel", "30000000-0000-0000-0000-000000000001" },
                    { "62d97329-3b51-37c0-abe7-aba92734e97e", "Pull", "Team", "30000000-0000-0000-0000-000000000002" },
                    { "62e8e1fe-c911-e1b9-cc70-7efff6e08327", "Pull", "Deployment", "30000000-0000-0000-0000-000000000001" },
                    { "6395043d-510b-dc85-f19d-2a57463f4e8f", "Pull", "GitAccount", "30000000-0000-0000-0000-000000000001" },
                    { "65cb87df-133d-b577-21f6-2f20190ce45f", "View", "Registry", "30000000-0000-0000-0000-000000000003" },
                    { "6deaa9b4-66e6-22bb-3f49-62584ccd9e1f", "View", "Platform", "30000000-0000-0000-0000-000000000001" },
                    { "6e332b06-35e2-fbbd-e12c-bb7f02ab2474", "Create", "Alert", "30000000-0000-0000-0000-000000000001" },
                    { "6ed1c4e3-9d28-23d8-2939-6a414aa0f53d", "Apply", "Platform", "30000000-0000-0000-0000-000000000002" },
                    { "6f3a7126-e96a-d249-5e58-108bd0bd1f0f", "View", "AlertChannel", "30000000-0000-0000-0000-000000000002" },
                    { "7238045c-c070-0ba5-b133-6083ea208d1b", "Apply", "Stack", "30000000-0000-0000-0000-000000000001" },
                    { "723bb5cb-0c68-80e7-d890-7c4f6e3ce23a", "Delete", "Team", "30000000-0000-0000-0000-000000000001" },
                    { "73174250-b459-3def-d4e6-82d09d07ece9", "Update", "GitRepository", "30000000-0000-0000-0000-000000000002" },
                    { "75575fd4-2d45-4302-12ba-1ebf9e9ea17f", "Create", "GitRepository", "30000000-0000-0000-0000-000000000002" },
                    { "780d5066-5b19-668e-9f2f-0103f6cb23be", "View", "Deployment", "30000000-0000-0000-0000-000000000002" },
                    { "783ca30a-d153-8f1b-27db-c3e722f7f34d", "Apply", "GitAccount", "30000000-0000-0000-0000-000000000002" },
                    { "7ad3c461-3f87-fe3c-a1e7-4a490906600e", "Delete", "Platform", "30000000-0000-0000-0000-000000000001" },
                    { "7ba78b50-a388-1606-e334-30c69758e60f", "View", "Deployment", "30000000-0000-0000-0000-000000000003" },
                    { "7ccb3b9e-a09a-d3c9-82ed-3f71646d2576", "View", "GitAccount", "30000000-0000-0000-0000-000000000001" },
                    { "7fddc3c2-6d6e-cb92-a7a3-59a7a18b7c08", "View", "GitRepository", "30000000-0000-0000-0000-000000000003" },
                    { "83ac5b37-8bd4-e093-3cdd-7e9f61300108", "View", "Alert", "30000000-0000-0000-0000-000000000002" },
                    { "8469f325-f132-73f4-0fa4-42131875a5ed", "Update", "Role", "30000000-0000-0000-0000-000000000001" },
                    { "8487254d-0383-5b91-fa5f-816cfdc29054", "View", "Team", "30000000-0000-0000-0000-000000000001" },
                    { "8658afec-8be0-2f4b-7b1e-478ac44341ec", "Create", "Registry", "30000000-0000-0000-0000-000000000001" },
                    { "8722b0da-7d07-7c14-0f9c-161e0c39a751", "Pull", "User", "30000000-0000-0000-0000-000000000002" },
                    { "88dc9733-349d-635c-7ef6-829065f4f87b", "Create", "AlertChannel", "30000000-0000-0000-0000-000000000002" },
                    { "8b1633bc-d6ba-a419-38ea-e4d7e4b48cbe", "Create", "Deployment", "30000000-0000-0000-0000-000000000001" },
                    { "8ce09606-e435-8a9b-2dab-8d1dfc91a198", "Delete", "Alert", "30000000-0000-0000-0000-000000000001" },
                    { "8de4cd72-b2ce-4e18-f148-b93892845825", "View", "Alert", "30000000-0000-0000-0000-000000000003" },
                    { "9308eb9b-7faa-e3d3-ffa8-86fc7946fae0", "View", "Team", "30000000-0000-0000-0000-000000000002" },
                    { "9365df99-cab8-01da-f3c7-dc17a54e8801", "Delete", "GitAccount", "30000000-0000-0000-0000-000000000001" },
                    { "961c1641-93aa-54ca-9b00-6608da3ae4c8", "Pull", "Registry", "30000000-0000-0000-0000-000000000001" },
                    { "969a37a2-af1e-fa24-35b8-4857f802e001", "Apply", "Deployment", "30000000-0000-0000-0000-000000000002" },
                    { "99a0ba24-900d-448e-70f0-6e5eda10f8fc", "Apply", "Deployment", "30000000-0000-0000-0000-000000000001" },
                    { "99b23eff-a5c8-0cbe-6383-b99e43a43b23", "Create", "AlertChannel", "30000000-0000-0000-0000-000000000001" },
                    { "9aa863b1-84ad-e6c5-738f-41425290cbb8", "Create", "User", "30000000-0000-0000-0000-000000000002" },
                    { "9e087c4f-e933-37c9-3941-b1803f83ede3", "Create", "Platform", "30000000-0000-0000-0000-000000000001" },
                    { "9f3d1be4-d19e-9453-86aa-2ae06eae6d18", "Delete", "AlertChannel", "30000000-0000-0000-0000-000000000001" },
                    { "9fd296b8-cb43-5a62-9672-cf562aa6efd1", "Update", "GitRepository", "30000000-0000-0000-0000-000000000001" },
                    { "9feb50a6-6f53-b270-5e7c-f679e7d85ed5", "Update", "Role", "30000000-0000-0000-0000-000000000002" },
                    { "a178007d-0c14-258e-bb6a-8828a5c28db7", "Update", "Deployment", "30000000-0000-0000-0000-000000000001" },
                    { "a1a791a5-9c37-88ad-c0e2-9a6717191298", "Update", "Stack", "30000000-0000-0000-0000-000000000002" },
                    { "a7ac62e2-2a4d-50c6-6700-af7a5a345bf7", "Update", "Team", "30000000-0000-0000-0000-000000000002" },
                    { "ab32d40c-3859-6377-1634-a84b67e820dc", "Apply", "Team", "30000000-0000-0000-0000-000000000001" },
                    { "ab3dfa51-423f-716d-a623-760c9f72f791", "Pull", "Role", "30000000-0000-0000-0000-000000000001" },
                    { "acab0152-cf67-d16c-fe1e-c579972ad2df", "Pull", "Registry", "30000000-0000-0000-0000-000000000002" },
                    { "acece9ff-20d1-f3d5-c304-3a07adb9a03b", "Update", "Alert", "30000000-0000-0000-0000-000000000002" },
                    { "b60ccb85-3aaa-0077-b0e8-7adbb9f4a596", "View", "User", "30000000-0000-0000-0000-000000000002" },
                    { "b678aa42-8c01-b706-1332-b85adb4e3096", "Delete", "Registry", "30000000-0000-0000-0000-000000000001" },
                    { "bf01fa5c-a0a9-749e-b6c9-2d04af953b2b", "Create", "GitRepository", "30000000-0000-0000-0000-000000000001" },
                    { "c1184544-a092-3f80-c4d6-6f778db56b26", "Update", "GitAccount", "30000000-0000-0000-0000-000000000002" },
                    { "c12c9075-9343-73ec-322a-cc41a23230db", "Delete", "Deployment", "30000000-0000-0000-0000-000000000001" },
                    { "c3e7230a-af2b-ba29-57ae-3c4043b66d61", "View", "Deployment", "30000000-0000-0000-0000-000000000001" },
                    { "c71643e0-9549-edeb-c7ff-496efa58260c", "Create", "Platform", "30000000-0000-0000-0000-000000000002" },
                    { "c717528a-5a83-69a2-6893-4ab5fbc14d1b", "View", "Platform", "30000000-0000-0000-0000-000000000002" },
                    { "c75f0cb6-117e-8929-d133-c45f363c1610", "Delete", "Role", "30000000-0000-0000-0000-000000000001" },
                    { "cbcc1ffb-e622-9159-df1e-d0d05e50385c", "Pull", "Team", "30000000-0000-0000-0000-000000000001" },
                    { "cdd4e750-840f-2a08-7a9a-3bd65a4360e9", "Delete", "Stack", "30000000-0000-0000-0000-000000000001" },
                    { "ce566660-f041-32b6-0942-2f8abb92b17d", "View", "Registry", "30000000-0000-0000-0000-000000000002" },
                    { "d23893f2-7f44-a593-83de-22ca4a8c80a1", "Update", "Registry", "30000000-0000-0000-0000-000000000002" },
                    { "d5ba4edc-5278-a21e-613d-91350e52bce7", "Apply", "GitAccount", "30000000-0000-0000-0000-000000000001" },
                    { "d868c269-b60f-2be0-2451-9b81b3c95674", "View", "AlertChannel", "30000000-0000-0000-0000-000000000001" },
                    { "d990b800-123d-a0ec-9b6a-239915235880", "Create", "GitAccount", "30000000-0000-0000-0000-000000000002" },
                    { "ddcb3cb1-e44f-0ab8-9e1e-698ed0352dc6", "Apply", "User", "30000000-0000-0000-0000-000000000001" },
                    { "deb33289-b4e7-0111-9e93-079c08f09cf3", "Create", "Team", "30000000-0000-0000-0000-000000000001" },
                    { "df77eb4e-7860-5319-431e-481bfe08baeb", "Pull", "AlertChannel", "30000000-0000-0000-0000-000000000002" },
                    { "e0494cd9-b3ec-b088-0532-089d029accca", "Update", "Team", "30000000-0000-0000-0000-000000000001" },
                    { "e08e5c0a-2e94-f112-22dd-e06d44dd2d9b", "View", "Stack", "30000000-0000-0000-0000-000000000003" },
                    { "e0e6d40a-91ae-d947-56a3-6e71df38581a", "Update", "User", "30000000-0000-0000-0000-000000000001" },
                    { "e13e141d-8322-f5f5-0488-cf961e919143", "Update", "Deployment", "30000000-0000-0000-0000-000000000002" },
                    { "e3b44e1b-f776-b5ce-509d-2b529768a528", "View", "Registry", "30000000-0000-0000-0000-000000000001" },
                    { "e6a62042-68c0-d60f-2888-f685176a8f4f", "Create", "Deployment", "30000000-0000-0000-0000-000000000002" },
                    { "e92ef59e-f1ab-51fc-abc8-4d90c98e5bf9", "Update", "Platform", "30000000-0000-0000-0000-000000000001" },
                    { "ea5f48d2-2719-0a78-dcb4-2efd447e674f", "Pull", "Alert", "30000000-0000-0000-0000-000000000001" },
                    { "ec427e29-a8ed-8604-59cc-7eda3268fc30", "View", "Alert", "30000000-0000-0000-0000-000000000001" },
                    { "eef93be9-d327-6cfe-3bc9-5c5290f4b686", "Apply", "User", "30000000-0000-0000-0000-000000000002" },
                    { "f12e902b-29c0-d404-41ef-6c7731211a70", "Pull", "Stack", "30000000-0000-0000-0000-000000000002" },
                    { "f1782e79-808e-d628-a83a-10b4639b9a68", "View", "GitRepository", "30000000-0000-0000-0000-000000000002" },
                    { "f30ff44f-86d5-8215-2c0f-60ed6ebd6234", "Update", "Registry", "30000000-0000-0000-0000-000000000001" },
                    { "f43aa780-94d7-54b7-ceef-0cee3a2922df", "View", "Role", "30000000-0000-0000-0000-000000000001" },
                    { "f4595acc-f991-34d8-834c-4a1136010e17", "View", "Platform", "30000000-0000-0000-0000-000000000003" },
                    { "f482aa00-8a5a-30da-4d1b-f0dfe770bcb3", "Pull", "User", "30000000-0000-0000-0000-000000000001" },
                    { "f7be80a4-dffa-1049-994a-5314ee03efaf", "Create", "GitAccount", "30000000-0000-0000-0000-000000000001" },
                    { "f8833b18-d702-70b1-f75a-33f732e5ac29", "Apply", "Team", "30000000-0000-0000-0000-000000000002" },
                    { "f893a35b-7eac-bd31-921e-ec48bd5335e8", "View", "Role", "30000000-0000-0000-0000-000000000002" },
                    { "f9af8f42-cf9c-21a8-43ba-793b4dd1bd3f", "Pull", "Alert", "30000000-0000-0000-0000-000000000002" },
                    { "fc6bc1bc-bb69-098b-b09e-07a96444f57e", "Update", "AlertChannel", "30000000-0000-0000-0000-000000000002" },
                    { "ff6e9bea-dbf2-e811-d4ba-6702a55c3f92", "View", "GitAccount", "30000000-0000-0000-0000-000000000002" },
                    { "ffc7419f-9c54-80fa-cac0-9e52ebeda6d3", "View", "User", "30000000-0000-0000-0000-000000000003" }
                });

            migrationBuilder.InsertData(
                table: "Registries",
                columns: new[] { "Id", "Configuration", "CreatedAt", "CreatedByActorId", "Description", "Name", "RegistryHost", "Status" },
                values: new object[] { "00000000-0000-0000-0000-000000000100", "{\r\n    \"$type\": \"DockerHub\"\r\n}", new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Unspecified), "00000000-0000-0000-0000-000000000001", "Public Docker Hub Registry", "Docker Hub", "hub.docker.com", "Active" });

            migrationBuilder.InsertData(
                table: "Teams",
                columns: new[] { "Id", "ActorId", "Name" },
                values: new object[] { "20000000-0000-0000-0000-000000000001", "00000000-0000-0000-0000-000000000003", "Default Team" });

            migrationBuilder.InsertData(
                table: "Users",
                columns: new[] { "Id", "ActorId", "CreatedAt", "CreatedByActorId", "Email", "Name", "Password" },
                values: new object[] { "10000000-0000-0000-0000-000000000001", "00000000-0000-0000-0000-000000000002", new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Unspecified), "00000000-0000-0000-0000-000000000001", "admin@citadel.local", "Admin", "o6hWzZ+DIuSZoHNjf5D1t6101vfm4w2kmPRiAZ3Xq53JMMl1" });

            migrationBuilder.CreateIndex(
                name: "IX_ActivityEvents_CreatedByActorId",
                table: "ActivityEvents",
                column: "CreatedByActorId");

            migrationBuilder.CreateIndex(
                name: "IX_ActivityEvents_EventType",
                table: "ActivityEvents",
                column: "EventType");

            migrationBuilder.CreateIndex(
                name: "IX_ActivityEvents_Platform_CreatedAt",
                table: "ActivityEvents",
                columns: new[] { "PlatformId", "CreatedAt" });

            migrationBuilder.CreateIndex(
                name: "IX_ActivityEvents_Resource_CreatedAt",
                table: "ActivityEvents",
                columns: new[] { "ResourceId", "CreatedAt" });

            migrationBuilder.CreateIndex(
                name: "IX_ActivityEvents_Status",
                table: "ActivityEvents",
                column: "Status");

            migrationBuilder.CreateIndex(
                name: "IX_ActorRoles_ActorId",
                table: "ActorRoles",
                column: "ActorId");

            migrationBuilder.CreateIndex(
                name: "IX_ActorRoles_RoleId",
                table: "ActorRoles",
                column: "RoleId");

            migrationBuilder.CreateIndex(
                name: "IX_AlertChannels_CreatedByActorId",
                table: "AlertChannels",
                column: "CreatedByActorId");

            migrationBuilder.CreateIndex(
                name: "IX_AlertEvents_AlertRuleId",
                table: "AlertEvents",
                column: "AlertRuleId");

            migrationBuilder.CreateIndex(
                name: "IX_AlertEvents_OpenIncidentKey",
                table: "AlertEvents",
                column: "OpenIncidentKey",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_AlertEvents_ResourceType",
                table: "AlertEvents",
                column: "ResourceType");

            migrationBuilder.CreateIndex(
                name: "IX_AlertEvents_Resource_CreatedAt",
                table: "AlertEvents",
                columns: new[] { "ResourceId", "CreatedAt" });

            migrationBuilder.CreateIndex(
                name: "IX_AlertEvents_Type",
                table: "AlertEvents",
                column: "Type");

            migrationBuilder.CreateIndex(
                name: "IX_AlertRuleChannels_AlertChannelId",
                table: "AlertRuleChannels",
                column: "AlertChannelId");

            migrationBuilder.CreateIndex(
                name: "IX_AlertRuleStates_CreatedByActorId",
                table: "AlertRuleStates",
                column: "CreatedByActorId");

            migrationBuilder.CreateIndex(
                name: "IX_AlertRules_CreatedByActorId",
                table: "AlertRules",
                column: "CreatedByActorId");

            migrationBuilder.CreateIndex(
                name: "IX_AlertRules_Type",
                table: "AlertRules",
                column: "Type");

            migrationBuilder.CreateIndex(
                name: "IX_ContainerStats_ContainerId_Created",
                table: "ContainerStats",
                columns: new[] { "ContainerId", "Created" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_Containers_ControlTriggeredBy",
                table: "Containers",
                column: "ControlTriggeredBy");

            migrationBuilder.CreateIndex(
                name: "IX_Containers_DeploymentId",
                table: "Containers",
                column: "DeploymentId");

            migrationBuilder.CreateIndex(
                name: "IX_Containers_DockerImageId",
                table: "Containers",
                column: "DockerImageId");

            migrationBuilder.CreateIndex(
                name: "IX_Containers_ImageId",
                table: "Containers",
                column: "ImageId");

            migrationBuilder.CreateIndex(
                name: "IX_Containers_PlatformId",
                table: "Containers",
                column: "PlatformId");

            migrationBuilder.CreateIndex(
                name: "IX__Containers_DockerContainerId_PlatformId",
                table: "Containers",
                columns: new[] { "DockerContainerId", "PlatformId" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_Deployments_ControlTriggeredBy",
                table: "Deployments",
                column: "ControlTriggeredBy");

            migrationBuilder.CreateIndex(
                name: "IX_Deployments_CreatedByActorId",
                table: "Deployments",
                column: "CreatedByActorId");

            migrationBuilder.CreateIndex(
                name: "IX_Deployments_Name_PlatformId",
                table: "Deployments",
                columns: new[] { "Name", "PlatformId" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_Deployments_PlatformId",
                table: "Deployments",
                column: "PlatformId");

            migrationBuilder.CreateIndex(
                name: "IX_GitAccounts_CreatedByActorId",
                table: "GitAccounts",
                column: "CreatedByActorId");

            migrationBuilder.CreateIndex(
                name: "IX_GitAccounts_Name",
                table: "GitAccounts",
                column: "Name",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_GitRepositories_ControlTriggeredBy",
                table: "GitRepositories",
                column: "ControlTriggeredBy");

            migrationBuilder.CreateIndex(
                name: "IX_GitRepositories_CreatedByActorId",
                table: "GitRepositories",
                column: "CreatedByActorId");

            migrationBuilder.CreateIndex(
                name: "IX_GitRepositories_GitAccountId",
                table: "GitRepositories",
                column: "GitAccountId");

            migrationBuilder.CreateIndex(
                name: "IX_GitRepositories_Name",
                table: "GitRepositories",
                column: "Name",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_Images_ControlTriggeredBy",
                table: "Images",
                column: "ControlTriggeredBy");

            migrationBuilder.CreateIndex(
                name: "IX_Images_DockerImageId_PlatformId",
                table: "Images",
                columns: new[] { "DockerImageId", "PlatformId" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_Images_PlatformId",
                table: "Images",
                column: "PlatformId");

            migrationBuilder.CreateIndex(
                name: "IX_Images_RegistryId",
                table: "Images",
                column: "RegistryId");

            migrationBuilder.CreateIndex(
                name: "IX_Permissions_RoleId",
                table: "Permissions",
                column: "RoleId");

            migrationBuilder.CreateIndex(
                name: "IX_PlatformStats_PlatformId_Created",
                table: "PlatformStats",
                columns: new[] { "PlatformId", "Created" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_Platforms_Address",
                table: "Platforms",
                column: "Address",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_RefreshTokens_UserId",
                table: "RefreshTokens",
                column: "UserId");

            migrationBuilder.CreateIndex(
                name: "IX_Registries_CreatedByActorId",
                table: "Registries",
                column: "CreatedByActorId");

            migrationBuilder.CreateIndex(
                name: "IX_Registries_Name",
                table: "Registries",
                column: "Name",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_ResourceAccesses_Actor",
                table: "ResourceAccesses",
                column: "ActorId");

            migrationBuilder.CreateIndex(
                name: "IX_ResourceAccesses_ResourceType_ResourceId_ActorId",
                table: "ResourceAccesses",
                columns: new[] { "ResourceType", "ResourceId", "ActorId" });

            migrationBuilder.CreateIndex(
                name: "IX_ResourceAccesses_ResourceType_ResourceId_ActorId_Action",
                table: "ResourceAccesses",
                columns: new[] { "ResourceType", "ResourceId", "ActorId", "Action" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_StackReleases_CreatedByActorId",
                table: "StackReleases",
                column: "CreatedByActorId");

            migrationBuilder.CreateIndex(
                name: "IX_StackReleases_PlatformId",
                table: "StackReleases",
                column: "PlatformId");

            migrationBuilder.CreateIndex(
                name: "IX_StackReleases_StackId",
                table: "StackReleases",
                column: "StackId");

            migrationBuilder.CreateIndex(
                name: "IX_Stacks_ControlTriggeredBy",
                table: "Stacks",
                column: "ControlTriggeredBy");

            migrationBuilder.CreateIndex(
                name: "IX_Stacks_CreatedByActorId",
                table: "Stacks",
                column: "CreatedByActorId");

            migrationBuilder.CreateIndex(
                name: "IX_Stacks_CurrentStackReleaseId",
                table: "Stacks",
                column: "CurrentStackReleaseId");

            migrationBuilder.CreateIndex(
                name: "IX_Teams_ActorId",
                table: "Teams",
                column: "ActorId",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_Users_ActorId",
                table: "Users",
                column: "ActorId",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_Users_CreatedByActorId",
                table: "Users",
                column: "CreatedByActorId");

            migrationBuilder.CreateIndex(
                name: "IX_Users_Email",
                table: "Users",
                column: "Email",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_UsersTeams_TeamId",
                table: "UsersTeams",
                column: "TeamId");

            migrationBuilder.CreateIndex(
                name: "IX_UsersTeams_UserId",
                table: "UsersTeams",
                column: "UserId");
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropTable(
                name: "ActivityEvents");

            migrationBuilder.DropTable(
                name: "ActorRoles");

            migrationBuilder.DropTable(
                name: "AlertEvents");

            migrationBuilder.DropTable(
                name: "AlertRuleChannels");

            migrationBuilder.DropTable(
                name: "AlertRuleStates");

            migrationBuilder.DropTable(
                name: "ContainerStats");

            migrationBuilder.DropTable(
                name: "GitRepositories");

            migrationBuilder.DropTable(
                name: "Permissions");

            migrationBuilder.DropTable(
                name: "PlatformStats");

            migrationBuilder.DropTable(
                name: "RefreshTokens");

            migrationBuilder.DropTable(
                name: "ResourceAccesses");

            migrationBuilder.DropTable(
                name: "StackReleases");

            migrationBuilder.DropTable(
                name: "UsersTeams");

            migrationBuilder.DropTable(
                name: "AlertChannels");

            migrationBuilder.DropTable(
                name: "AlertRules");

            migrationBuilder.DropTable(
                name: "Containers");

            migrationBuilder.DropTable(
                name: "GitAccounts");

            migrationBuilder.DropTable(
                name: "Roles");

            migrationBuilder.DropTable(
                name: "Stacks");

            migrationBuilder.DropTable(
                name: "Teams");

            migrationBuilder.DropTable(
                name: "Users");

            migrationBuilder.DropTable(
                name: "Deployments");

            migrationBuilder.DropTable(
                name: "Images");

            migrationBuilder.DropTable(
                name: "Platforms");

            migrationBuilder.DropTable(
                name: "Registries");

            migrationBuilder.DropTable(
                name: "Actors");
        }
    }
}
