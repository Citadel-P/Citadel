using System;
using Microsoft.EntityFrameworkCore.Migrations;
using Npgsql.EntityFrameworkCore.PostgreSQL.Metadata;

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
            migrationBuilder.AlterDatabase()
                .Annotation("Npgsql:PostgresExtension:pg_trgm", ",,");

            migrationBuilder.CreateTable(
                name: "actors",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    isenabled = table.Column<bool>(type: "boolean", nullable: false, defaultValue: true),
                    type = table.Column<string>(type: "text", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_actors", x => x.id);
                });

            migrationBuilder.CreateTable(
                name: "backupsourceleases",
                columns: table => new
                {
                    sourcekey = table.Column<string>(type: "text", maxLength: 512, nullable: false),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    expiresat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    operationtype = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    ownerrunid = table.Column<Guid>(type: "uuid", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_backupsourceleases", x => x.sourcekey);
                });

            migrationBuilder.CreateTable(
                name: "citadelinstanceidentity",
                columns: table => new
                {
                    id = table.Column<int>(type: "integer", nullable: false)
                        .Annotation("Npgsql:ValueGenerationStrategy", NpgsqlValueGenerationStrategy.IdentityByDefaultColumn),
                    createdat = table.Column<DateTimeOffset>(type: "timestamp with time zone", nullable: false),
                    instanceid = table.Column<Guid>(type: "uuid", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_citadelinstanceidentity", x => x.id);
                    table.CheckConstraint("CK_CitadelInstanceIdentity_Singleton", "\"id\" = 1");
                });

            migrationBuilder.CreateTable(
                name: "edgeagentbindings",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    agentfingerprint = table.Column<string>(type: "text", maxLength: 128, nullable: false),
                    agentid = table.Column<Guid>(type: "uuid", nullable: false),
                    agentpublickey = table.Column<string>(type: "text", nullable: false),
                    capabilitiesjson = table.Column<string>(type: "json", nullable: true),
                    connectionstatus = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    createdatutc = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    lastconnectedatutc = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    lastdisconnectedatutc = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    lastheartbeatatutc = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    lastseenhostname = table.Column<string>(type: "text", maxLength: 256, nullable: true),
                    lastseenversion = table.Column<string>(type: "text", maxLength: 128, nullable: true),
                    platformid = table.Column<Guid>(type: "uuid", nullable: false),
                    protocolversion = table.Column<int>(type: "integer", nullable: false, defaultValue: 1),
                    resourceid = table.Column<Guid>(type: "uuid", nullable: false),
                    resourcetype = table.Column<string>(type: "text", maxLength: 64, nullable: false, defaultValue: "Platform"),
                    revokedatutc = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    updatedatutc = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP")
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_edgeagentbindings", x => x.id);
                });

            migrationBuilder.CreateTable(
                name: "platforms",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    address = table.Column<string>(type: "text", nullable: false),
                    agentversion = table.Column<string>(type: "text", nullable: true),
                    connectortype = table.Column<string>(type: "text", nullable: false),
                    cpucount = table.Column<int>(type: "integer", nullable: false),
                    description = table.Column<string>(type: "text", maxLength: 600, nullable: true),
                    imagecount = table.Column<int>(type: "integer", nullable: false),
                    memtotal = table.Column<long>(type: "bigint", nullable: false),
                    name = table.Column<string>(type: "text", nullable: false),
                    networkcount = table.Column<int>(type: "integer", nullable: false),
                    platformdescriptor = table.Column<string>(type: "json", nullable: false),
                    serverversion = table.Column<string>(type: "text", nullable: true),
                    status = table.Column<string>(type: "text", nullable: false),
                    volumecount = table.Column<int>(type: "integer", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_platforms", x => x.id);
                });

            migrationBuilder.CreateTable(
                name: "resourceaccesses",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    actorid = table.Column<Guid>(type: "uuid", nullable: false),
                    permissionlevel = table.Column<int>(type: "integer", nullable: false),
                    resourceid = table.Column<Guid>(type: "uuid", nullable: false),
                    resourcetype = table.Column<int>(type: "integer", nullable: false),
                    specificpermissions = table.Column<int>(type: "integer", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_resourceaccesses", x => x.id);
                });

            migrationBuilder.CreateTable(
                name: "roles",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    name = table.Column<string>(type: "text", nullable: false),
                    roletype = table.Column<string>(type: "text", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_roles", x => x.id);
                });

            migrationBuilder.CreateTable(
                name: "secretproviders",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    configuration = table.Column<string>(type: "text", nullable: false),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    name = table.Column<string>(type: "text", nullable: false),
                    providertype = table.Column<string>(type: "text", nullable: false),
                    updatedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP")
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_secretproviders", x => x.id);
                });

            migrationBuilder.CreateTable(
                name: "actions",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    alertonfailure = table.Column<bool>(type: "boolean", nullable: false),
                    code = table.Column<string>(type: "text", nullable: false),
                    controlstartedat = table.Column<long>(type: "bigint", nullable: true),
                    controlstate = table.Column<string>(type: "text", maxLength: 64, nullable: false, defaultValue: "Idle"),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    createdbyactorid = table.Column<Guid>(type: "uuid", nullable: false),
                    currentrunid = table.Column<Guid>(type: "uuid", nullable: true),
                    defaultargsjson = table.Column<string>(type: "jsonb", nullable: false, defaultValueSql: "'{}'::jsonb"),
                    description = table.Column<string>(type: "text", maxLength: 600, nullable: true),
                    enabled = table.Column<bool>(type: "boolean", nullable: false),
                    lastscheduledrunat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    name = table.Column<string>(type: "text", maxLength: 128, nullable: false),
                    rowversion = table.Column<long>(type: "bigint", nullable: false, defaultValue: 0L),
                    runasactorid = table.Column<Guid>(type: "uuid", nullable: false),
                    schedulecron = table.Column<string>(type: "text", maxLength: 128, nullable: true),
                    scheduleenabled = table.Column<bool>(type: "boolean", nullable: false),
                    scheduletimezone = table.Column<string>(type: "text", maxLength: 128, nullable: false),
                    timeoutseconds = table.Column<int>(type: "integer", nullable: false),
                    updatedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    webhook = table.Column<string>(type: "jsonb", nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_actions", x => x.id);
                    table.ForeignKey(
                        name: "fk_actions_actors_createdbyactorid",
                        column: x => x.createdbyactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_actions_actors_runasactorid",
                        column: x => x.runasactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "alertchannels",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    alertdestination = table.Column<string>(type: "text", nullable: false),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    createdbyactorid = table.Column<Guid>(type: "uuid", nullable: false),
                    isactive = table.Column<bool>(type: "boolean", nullable: false, defaultValue: true),
                    name = table.Column<string>(type: "text", nullable: false),
                    url = table.Column<string>(type: "text", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_alertchannels", x => x.id);
                    table.ForeignKey(
                        name: "fk_alertchannels_actors_createdbyactorid",
                        column: x => x.createdbyactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "alertrules",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    cooldownseconds = table.Column<int>(type: "integer", nullable: true),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    createdbyactorid = table.Column<Guid>(type: "uuid", nullable: false),
                    description = table.Column<string>(type: "text", maxLength: 600, nullable: true),
                    limitedto = table.Column<string>(type: "json", nullable: false),
                    name = table.Column<string>(type: "text", maxLength: 120, nullable: false),
                    quiethours = table.Column<string>(type: "json", nullable: false),
                    requiredmatches = table.Column<int>(type: "integer", nullable: true),
                    severity = table.Column<string>(type: "text", nullable: false),
                    status = table.Column<string>(type: "text", nullable: false, defaultValue: "Enabled"),
                    threshold = table.Column<double>(type: "double precision", nullable: true),
                    type = table.Column<string>(type: "text", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_alertrules", x => x.id);
                    table.ForeignKey(
                        name: "fk_alertrules_actors_createdbyactorid",
                        column: x => x.createdbyactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "buildagentpools",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    archivedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    cleanuptimeoutseconds = table.Column<int>(type: "integer", nullable: false, defaultValue: 600),
                    controlstartedat = table.Column<long>(type: "bigint", nullable: true),
                    controlstate = table.Column<string>(type: "text", maxLength: 64, nullable: false, defaultValue: "Idle"),
                    controltriggeredby = table.Column<Guid>(type: "uuid", nullable: true),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    createdbyactorid = table.Column<Guid>(type: "uuid", nullable: false),
                    description = table.Column<string>(type: "text", maxLength: 600, nullable: true),
                    enabled = table.Column<bool>(type: "boolean", nullable: false, defaultValue: true),
                    failureretentionminutes = table.Column<int>(type: "integer", nullable: false, defaultValue: 0),
                    heartbeattimeoutseconds = table.Column<int>(type: "integer", nullable: false, defaultValue: 90),
                    lastvalidatedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    lastvalidationmessage = table.Column<string>(type: "text", maxLength: 1200, nullable: true),
                    lastvalidationstatus = table.Column<string>(type: "text", maxLength: 64, nullable: false, defaultValue: "NotTested"),
                    maxactivebuilders = table.Column<int>(type: "integer", nullable: false, defaultValue: 1),
                    maximuminstancelifetimeseconds = table.Column<int>(type: "integer", nullable: false, defaultValue: 7200),
                    name = table.Column<string>(type: "text", maxLength: 128, nullable: false),
                    normalizedname = table.Column<string>(type: "text", maxLength: 128, nullable: false),
                    provider = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    providerspec = table.Column<string>(type: "jsonb", nullable: false),
                    provisioningtimeoutseconds = table.Column<int>(type: "integer", nullable: false, defaultValue: 600),
                    queuetimeoutseconds = table.Column<int>(type: "integer", nullable: false, defaultValue: 3600),
                    registrationtimeoutseconds = table.Column<int>(type: "integer", nullable: false, defaultValue: 300),
                    rowversion = table.Column<long>(type: "bigint", nullable: false, defaultValue: 0L),
                    updatedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP")
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_buildagentpools", x => x.id);
                    table.ForeignKey(
                        name: "fk_buildagentpools_actors_controltriggeredby",
                        column: x => x.controltriggeredby,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_buildagentpools_actors_createdbyactorid",
                        column: x => x.createdbyactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "edgeagentenrollments",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    createdatutc = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    createdbyactorid = table.Column<Guid>(type: "uuid", nullable: false),
                    expiresatutc = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    platformid = table.Column<Guid>(type: "uuid", nullable: false),
                    resourceid = table.Column<Guid>(type: "uuid", nullable: false),
                    resourcetype = table.Column<string>(type: "text", maxLength: 64, nullable: false, defaultValue: "Platform"),
                    revokedatutc = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    tokenhash = table.Column<string>(type: "text", maxLength: 128, nullable: false),
                    usedatutc = table.Column<DateTime>(type: "timestamp with time zone", nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_edgeagentenrollments", x => x.id);
                    table.ForeignKey(
                        name: "fk_edgeagentenrollments_actors_createdbyactorid",
                        column: x => x.createdbyactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "gitaccounts",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    authtype = table.Column<string>(type: "text", nullable: false),
                    configuration = table.Column<string>(type: "json", nullable: false),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    createdbyactorid = table.Column<Guid>(type: "uuid", nullable: false),
                    domain = table.Column<string>(type: "text", nullable: false),
                    name = table.Column<string>(type: "text", nullable: false),
                    transport = table.Column<string>(type: "text", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_gitaccounts", x => x.id);
                    table.ForeignKey(
                        name: "fk_gitaccounts_actors_createdbyactorid",
                        column: x => x.createdbyactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "installedlicenses",
                columns: table => new
                {
                    id = table.Column<int>(type: "integer", nullable: false)
                        .Annotation("Npgsql:ValueGenerationStrategy", NpgsqlValueGenerationStrategy.IdentityByDefaultColumn),
                    fingerprint = table.Column<string>(type: "text", nullable: false),
                    installedat = table.Column<DateTimeOffset>(type: "timestamp with time zone", nullable: false),
                    installedbyactorid = table.Column<Guid>(type: "uuid", nullable: true),
                    lastvalidatedat = table.Column<DateTimeOffset>(type: "timestamp with time zone", nullable: true),
                    lastvalidationerrorcode = table.Column<string>(type: "text", maxLength: 128, nullable: true),
                    lastvalidationstatus = table.Column<string>(type: "text", maxLength: 64, nullable: true),
                    rawlicense = table.Column<string>(type: "text", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_installedlicenses", x => x.id);
                    table.CheckConstraint("CK_InstalledLicenses_Singleton", "\"id\" = 1");
                    table.ForeignKey(
                        name: "fk_installedlicenses_actors_installedbyactorid",
                        column: x => x.installedbyactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.SetNull);
                });

            migrationBuilder.CreateTable(
                name: "registries",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    configuration = table.Column<string>(type: "json", nullable: false),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    createdbyactorid = table.Column<Guid>(type: "uuid", nullable: false),
                    description = table.Column<string>(type: "text", maxLength: 600, nullable: true),
                    name = table.Column<string>(type: "text", nullable: false),
                    registryhost = table.Column<string>(type: "text", nullable: false),
                    status = table.Column<string>(type: "text", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_registries", x => x.id);
                    table.ForeignKey(
                        name: "fk_registries_actors_createdbyactorid",
                        column: x => x.createdbyactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "stacks",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    controlstartedat = table.Column<long>(type: "bigint", nullable: true),
                    controlstate = table.Column<string>(type: "text", maxLength: 64, nullable: true, defaultValue: "Idle"),
                    controltriggeredby = table.Column<Guid>(type: "uuid", nullable: true),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    createdbyactorid = table.Column<Guid>(type: "uuid", nullable: false),
                    currentstackreleaseid = table.Column<Guid>(type: "uuid", nullable: true),
                    description = table.Column<string>(type: "text", maxLength: 600, nullable: true),
                    driftpolicy = table.Column<string>(type: "json", nullable: false),
                    name = table.Column<string>(type: "text", nullable: false),
                    rowversion = table.Column<long>(type: "bigint", nullable: false, defaultValue: 0L),
                    stacksource = table.Column<string>(type: "text", nullable: false),
                    stackupdatestate = table.Column<string>(type: "json", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_stacks", x => x.id);
                    table.ForeignKey(
                        name: "fk_stacks_actors_controltriggeredby",
                        column: x => x.controltriggeredby,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_stacks_actors_createdbyactorid",
                        column: x => x.createdbyactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "tags",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    color = table.Column<string>(type: "text", maxLength: 7, nullable: false),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    createdbyactorid = table.Column<Guid>(type: "uuid", nullable: false),
                    name = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    normalizedname = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    updatedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP")
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_tags", x => x.id);
                    table.ForeignKey(
                        name: "fk_tags_actors_createdbyactorid",
                        column: x => x.createdbyactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "teams",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    actorid = table.Column<Guid>(type: "uuid", nullable: false),
                    name = table.Column<string>(type: "text", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_teams", x => x.id);
                    table.ForeignKey(
                        name: "fk_teams_actors_actorid",
                        column: x => x.actorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "users",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    actorid = table.Column<Guid>(type: "uuid", nullable: false),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    createdbyactorid = table.Column<Guid>(type: "uuid", nullable: false),
                    email = table.Column<string>(type: "text", nullable: true),
                    name = table.Column<string>(type: "text", maxLength: 100, nullable: false),
                    password = table.Column<string>(type: "text", nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_users", x => x.id);
                    table.ForeignKey(
                        name: "fk_users_actors_actorid",
                        column: x => x.actorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_users_actors_createdbyactorid",
                        column: x => x.createdbyactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "activityevents",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    createdbyactorid = table.Column<Guid>(type: "uuid", nullable: false),
                    eventtype = table.Column<string>(type: "text", nullable: false),
                    info = table.Column<string>(type: "text", nullable: false),
                    platformid = table.Column<Guid>(type: "uuid", nullable: true),
                    resourceid = table.Column<Guid>(type: "uuid", nullable: true),
                    resourcename = table.Column<string>(type: "text", nullable: false),
                    resourcetype = table.Column<string>(type: "text", nullable: false),
                    status = table.Column<string>(type: "text", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_activityevents", x => x.id);
                    table.ForeignKey(
                        name: "fk_activityevents_actors_createdbyactorid",
                        column: x => x.createdbyactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_activityevents_platforms_platformid",
                        column: x => x.platformid,
                        principalTable: "platforms",
                        principalColumn: "id",
                        onDelete: ReferentialAction.SetNull);
                });

            migrationBuilder.CreateTable(
                name: "deployments",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    autoupdatestate_currentdigest = table.Column<string>(type: "text", nullable: true),
                    autoupdatestate_lastcheckedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    autoupdatestate_lasterror = table.Column<string>(type: "text", maxLength: 2000, nullable: true),
                    autoupdatestate_remotedigest = table.Column<string>(type: "text", nullable: true),
                    autoupdatestate_status = table.Column<string>(type: "text", nullable: true),
                    controlstartedat = table.Column<long>(type: "bigint", nullable: true),
                    controlstate = table.Column<string>(type: "text", maxLength: 64, nullable: true, defaultValue: "Idle"),
                    controltriggeredby = table.Column<Guid>(type: "uuid", nullable: true),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    createdbyactorid = table.Column<Guid>(type: "uuid", nullable: false),
                    description = table.Column<string>(type: "text", maxLength: 600, nullable: true),
                    name = table.Column<string>(type: "text", nullable: false),
                    platformid = table.Column<Guid>(type: "uuid", nullable: false),
                    rowversion = table.Column<long>(type: "bigint", nullable: false, defaultValue: 0L),
                    spec = table.Column<string>(type: "jsonb", nullable: false),
                    status = table.Column<string>(type: "text", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_deployments", x => x.id);
                    table.ForeignKey(
                        name: "fk_deployments_actors_controltriggeredby",
                        column: x => x.controltriggeredby,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_deployments_actors_createdbyactorid",
                        column: x => x.createdbyactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_deployments_platforms_platformid",
                        column: x => x.platformid,
                        principalTable: "platforms",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "platformstats",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    cpuusage = table.Column<double>(type: "double precision", nullable: false),
                    created = table.Column<long>(type: "bigint", nullable: false),
                    memoryusage = table.Column<double>(type: "double precision", nullable: false),
                    platformid = table.Column<Guid>(type: "uuid", nullable: false),
                    rxbytes = table.Column<double>(type: "double precision", nullable: false),
                    txbytes = table.Column<double>(type: "double precision", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_platformstats", x => x.id);
                    table.ForeignKey(
                        name: "fk_platformstats_platforms_platformid",
                        column: x => x.platformid,
                        principalTable: "platforms",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "actorroles",
                columns: table => new
                {
                    actorid = table.Column<Guid>(type: "uuid", nullable: false),
                    roleid = table.Column<Guid>(type: "uuid", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_actorroles", x => new { x.actorid, x.roleid });
                    table.ForeignKey(
                        name: "fk_actorroles_actors_actorid",
                        column: x => x.actorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                    table.ForeignKey(
                        name: "fk_actorroles_roles_roleid",
                        column: x => x.roleid,
                        principalTable: "roles",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "oidcproviders",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    allowemailautolink = table.Column<bool>(type: "boolean", nullable: false, defaultValue: false),
                    allowedemaildomains = table.Column<string>(type: "text", maxLength: 1024, nullable: true),
                    autoprovisionusers = table.Column<bool>(type: "boolean", nullable: false, defaultValue: false),
                    clientid = table.Column<string>(type: "text", maxLength: 256, nullable: false),
                    clientsecretciphertext = table.Column<string>(type: "text", nullable: true),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    createdbyactorid = table.Column<Guid>(type: "uuid", nullable: false),
                    defaultroleid = table.Column<Guid>(type: "uuid", nullable: true),
                    description = table.Column<string>(type: "text", maxLength: 600, nullable: true),
                    displayname = table.Column<string>(type: "text", maxLength: 128, nullable: false),
                    enabled = table.Column<bool>(type: "boolean", nullable: false, defaultValue: true),
                    issuer = table.Column<string>(type: "text", maxLength: 512, nullable: false),
                    name = table.Column<string>(type: "text", maxLength: 128, nullable: false),
                    requireemailverified = table.Column<bool>(type: "boolean", nullable: false, defaultValue: true),
                    requiredclaimname = table.Column<string>(type: "text", maxLength: 256, nullable: true),
                    requiredclaimvalues = table.Column<string>(type: "text", maxLength: 1024, nullable: true),
                    scopes = table.Column<string>(type: "text", maxLength: 512, nullable: false),
                    updatedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP")
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_oidcproviders", x => x.id);
                    table.ForeignKey(
                        name: "fk_oidcproviders_actors_createdbyactorid",
                        column: x => x.createdbyactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_oidcproviders_roles_defaultroleid",
                        column: x => x.defaultroleid,
                        principalTable: "roles",
                        principalColumn: "id",
                        onDelete: ReferentialAction.SetNull);
                });

            migrationBuilder.CreateTable(
                name: "permissions",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    permissionlevel = table.Column<int>(type: "integer", nullable: false),
                    resourcetype = table.Column<int>(type: "integer", nullable: false),
                    roleid = table.Column<Guid>(type: "uuid", nullable: false),
                    specificpermissions = table.Column<int>(type: "integer", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_permissions", x => x.id);
                    table.ForeignKey(
                        name: "fk_permissions_roles_roleid",
                        column: x => x.roleid,
                        principalTable: "roles",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "secretdefinitions",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    externalkey = table.Column<string>(type: "text", nullable: true),
                    externalpath = table.Column<string>(type: "text", nullable: true),
                    externalversion = table.Column<int>(type: "integer", nullable: true),
                    name = table.Column<string>(type: "text", nullable: false),
                    providerid = table.Column<Guid>(type: "uuid", nullable: true),
                    providertype = table.Column<string>(type: "text", nullable: false),
                    updatedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP")
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_secretdefinitions", x => x.id);
                    table.ForeignKey(
                        name: "fk_secretdefinitions_secretproviders_providerid",
                        column: x => x.providerid,
                        principalTable: "secretproviders",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "actionruns",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    actionid = table.Column<Guid>(type: "uuid", nullable: false),
                    actionname = table.Column<string>(type: "text", maxLength: 128, nullable: false),
                    argsjson = table.Column<string>(type: "jsonb", nullable: false, defaultValueSql: "'{}'::jsonb"),
                    codehash = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    codesnapshot = table.Column<string>(type: "text", nullable: false),
                    durationms = table.Column<long>(type: "bigint", nullable: true),
                    errormessage = table.Column<string>(type: "text", nullable: true),
                    exitcode = table.Column<int>(type: "integer", nullable: true),
                    finishedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    logs = table.Column<string>(type: "text", nullable: true),
                    queuedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    runasactorid = table.Column<Guid>(type: "uuid", nullable: false),
                    startedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    status = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    timeoutseconds = table.Column<int>(type: "integer", nullable: false),
                    trigger = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    triggeredbyactorid = table.Column<Guid>(type: "uuid", nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_actionruns", x => x.id);
                    table.ForeignKey(
                        name: "fk_actionruns_actions_actionid",
                        column: x => x.actionid,
                        principalTable: "actions",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                    table.ForeignKey(
                        name: "fk_actionruns_actors_runasactorid",
                        column: x => x.runasactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_actionruns_actors_triggeredbyactorid",
                        column: x => x.triggeredbyactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.SetNull);
                });

            migrationBuilder.CreateTable(
                name: "alertevents",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    acknowledgedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    acknowledgedbyactorid = table.Column<Guid>(type: "uuid", nullable: true),
                    alertruleid = table.Column<Guid>(type: "uuid", nullable: false),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    deduplicationkey = table.Column<string>(type: "text", nullable: false),
                    info = table.Column<string>(type: "json", nullable: false),
                    openincidentkey = table.Column<string>(type: "text", nullable: true),
                    resolutionnote = table.Column<string>(type: "text", nullable: true),
                    resolvedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    resolvedbyactorid = table.Column<Guid>(type: "uuid", nullable: true),
                    resourceid = table.Column<Guid>(type: "uuid", nullable: true),
                    resourcename = table.Column<string>(type: "text", nullable: false),
                    resourcetype = table.Column<string>(type: "text", nullable: false),
                    severity = table.Column<string>(type: "text", nullable: false),
                    type = table.Column<string>(type: "text", nullable: false),
                    updatedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP")
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_alertevents", x => x.id);
                    table.ForeignKey(
                        name: "fk_alertevents_alertrules_alertruleid",
                        column: x => x.alertruleid,
                        principalTable: "alertrules",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "alertrulechannels",
                columns: table => new
                {
                    alertruleid = table.Column<Guid>(type: "uuid", nullable: false),
                    alertchannelid = table.Column<Guid>(type: "uuid", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_alertrulechannels", x => new { x.alertruleid, x.alertchannelid });
                    table.ForeignKey(
                        name: "fk_alertrulechannels_alertchannels_alertchannelid",
                        column: x => x.alertchannelid,
                        principalTable: "alertchannels",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                    table.ForeignKey(
                        name: "fk_alertrulechannels_alertrules_alertruleid",
                        column: x => x.alertruleid,
                        principalTable: "alertrules",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "alertrulestates",
                columns: table => new
                {
                    alertruleid = table.Column<Guid>(type: "uuid", nullable: false),
                    resourceid = table.Column<Guid>(type: "uuid", nullable: false),
                    consecutivematches = table.Column<int>(type: "integer", nullable: false, defaultValue: 3),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    createdbyactorid = table.Column<Guid>(type: "uuid", nullable: false),
                    lasttriggeredat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_alertrulestates", x => new { x.alertruleid, x.resourceid });
                    table.ForeignKey(
                        name: "fk_alertrulestates_actors_createdbyactorid",
                        column: x => x.createdbyactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_alertrulestates_alertrules_alertruleid",
                        column: x => x.alertruleid,
                        principalTable: "alertrules",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "gitrepositories",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    controlstartedat = table.Column<long>(type: "bigint", nullable: true),
                    controlstate = table.Column<string>(type: "text", maxLength: 64, nullable: true, defaultValue: "Idle"),
                    controltriggeredby = table.Column<Guid>(type: "uuid", nullable: true),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    createdbyactorid = table.Column<Guid>(type: "uuid", nullable: false),
                    defaultbranch = table.Column<string>(type: "text", nullable: false),
                    description = table.Column<string>(type: "text", maxLength: 600, nullable: true),
                    gitaccountid = table.Column<Guid>(type: "uuid", nullable: true),
                    name = table.Column<string>(type: "text", nullable: false),
                    onclone = table.Column<string>(type: "text", nullable: true),
                    onpull = table.Column<string>(type: "text", nullable: true),
                    rowversion = table.Column<long>(type: "bigint", nullable: false, defaultValue: 0L),
                    status = table.Column<string>(type: "text", nullable: false),
                    syncintervalminutes = table.Column<int>(type: "integer", nullable: true, defaultValue: 5),
                    syncmode = table.Column<string>(type: "text", nullable: false, defaultValue: "PullInterval"),
                    url = table.Column<string>(type: "text", nullable: false),
                    webhook = table.Column<string>(type: "jsonb", nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_gitrepositories", x => x.id);
                    table.ForeignKey(
                        name: "fk_gitrepositories_actors_controltriggeredby",
                        column: x => x.controltriggeredby,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_gitrepositories_actors_createdbyactorid",
                        column: x => x.createdbyactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_gitrepositories_gitaccounts_gitaccountid",
                        column: x => x.gitaccountid,
                        principalTable: "gitaccounts",
                        principalColumn: "id",
                        onDelete: ReferentialAction.SetNull);
                });

            migrationBuilder.CreateTable(
                name: "images",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    containers = table.Column<int>(type: "integer", nullable: false, defaultValue: 0),
                    controlstartedat = table.Column<long>(type: "bigint", nullable: true),
                    controlstate = table.Column<string>(type: "text", maxLength: 64, nullable: true, defaultValue: "Idle"),
                    controltriggeredby = table.Column<Guid>(type: "uuid", nullable: true),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    dockerimageid = table.Column<string>(type: "text", nullable: false),
                    name = table.Column<string>(type: "text", nullable: false),
                    platformid = table.Column<Guid>(type: "uuid", nullable: false),
                    registryid = table.Column<Guid>(type: "uuid", nullable: true),
                    rowversion = table.Column<long>(type: "bigint", nullable: false, defaultValue: 0L),
                    size = table.Column<double>(type: "double precision", nullable: false, defaultValue: 0.0),
                    tags = table.Column<string>(type: "json", nullable: false),
                    updatedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_images", x => x.id);
                    table.ForeignKey(
                        name: "fk_images_actors_controltriggeredby",
                        column: x => x.controltriggeredby,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_images_platforms_platformid",
                        column: x => x.platformid,
                        principalTable: "platforms",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                    table.ForeignKey(
                        name: "fk_images_registries_registryid",
                        column: x => x.registryid,
                        principalTable: "registries",
                        principalColumn: "id",
                        onDelete: ReferentialAction.SetNull);
                });

            migrationBuilder.CreateTable(
                name: "stackreleases",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    createdbyactorid = table.Column<Guid>(type: "uuid", nullable: false),
                    platformid = table.Column<Guid>(type: "uuid", nullable: false),
                    resourcebindings = table.Column<string>(type: "json", nullable: true),
                    source = table.Column<string>(type: "json", nullable: true),
                    spec = table.Column<string>(type: "jsonb", nullable: false),
                    stackid = table.Column<Guid>(type: "uuid", nullable: false),
                    status = table.Column<string>(type: "text", nullable: false),
                    version = table.Column<string>(type: "text", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_stackreleases", x => x.id);
                    table.ForeignKey(
                        name: "fk_stackreleases_actors_createdbyactorid",
                        column: x => x.createdbyactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_stackreleases_platforms_platformid",
                        column: x => x.platformid,
                        principalTable: "platforms",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_stackreleases_stacks_stackid",
                        column: x => x.stackid,
                        principalTable: "stacks",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "resourcetags",
                columns: table => new
                {
                    resourcetype = table.Column<string>(type: "text", nullable: false),
                    resourceid = table.Column<Guid>(type: "uuid", nullable: false),
                    tagid = table.Column<Guid>(type: "uuid", nullable: false),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    createdbyactorid = table.Column<Guid>(type: "uuid", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_resourcetags", x => new { x.resourcetype, x.resourceid, x.tagid });
                    table.ForeignKey(
                        name: "fk_resourcetags_actors_createdbyactorid",
                        column: x => x.createdbyactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_resourcetags_tags_tagid",
                        column: x => x.tagid,
                        principalTable: "tags",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "mfachallenges",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    consumedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    expiresat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    failedattempts = table.Column<int>(type: "integer", nullable: false, defaultValue: 0),
                    userid = table.Column<Guid>(type: "uuid", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_mfachallenges", x => x.id);
                    table.ForeignKey(
                        name: "fk_mfachallenges_users_userid",
                        column: x => x.userid,
                        principalTable: "users",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "mfasetupsessions",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    consumedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    expiresat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    protectedtotpsecret = table.Column<string>(type: "text", nullable: false),
                    userid = table.Column<Guid>(type: "uuid", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_mfasetupsessions", x => x.id);
                    table.ForeignKey(
                        name: "fk_mfasetupsessions_users_userid",
                        column: x => x.userid,
                        principalTable: "users",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "refreshtokens",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    expiresat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    ipaddress = table.Column<string>(type: "text", nullable: true),
                    lastseenat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    useragent = table.Column<string>(type: "text", nullable: true),
                    userid = table.Column<Guid>(type: "uuid", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_refreshtokens", x => x.id);
                    table.ForeignKey(
                        name: "fk_refreshtokens_users_userid",
                        column: x => x.userid,
                        principalTable: "users",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "usermfarecoverycodes",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    codehash = table.Column<string>(type: "text", nullable: false),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    usedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    userid = table.Column<Guid>(type: "uuid", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_usermfarecoverycodes", x => x.id);
                    table.ForeignKey(
                        name: "fk_usermfarecoverycodes_users_userid",
                        column: x => x.userid,
                        principalTable: "users",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "usermfasettings",
                columns: table => new
                {
                    userid = table.Column<Guid>(type: "uuid", nullable: false),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    enabledat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    lastacceptedtimestep = table.Column<long>(type: "bigint", nullable: true),
                    protectedtotpsecret = table.Column<string>(type: "text", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_usermfasettings", x => x.userid);
                    table.ForeignKey(
                        name: "fk_usermfasettings_users_userid",
                        column: x => x.userid,
                        principalTable: "users",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "userpreferences",
                columns: table => new
                {
                    userid = table.Column<Guid>(type: "uuid", nullable: false),
                    datetimeformat = table.Column<string>(type: "text", nullable: false),
                    theme = table.Column<string>(type: "text", nullable: false),
                    timezone = table.Column<string>(type: "text", nullable: false),
                    updatedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_userpreferences", x => x.userid);
                    table.ForeignKey(
                        name: "fk_userpreferences_users_userid",
                        column: x => x.userid,
                        principalTable: "users",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "usersteams",
                columns: table => new
                {
                    userid = table.Column<Guid>(type: "uuid", nullable: false),
                    teamid = table.Column<Guid>(type: "uuid", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_usersteams", x => new { x.userid, x.teamid });
                    table.ForeignKey(
                        name: "fk_usersteams_teams_teamid",
                        column: x => x.teamid,
                        principalTable: "teams",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                    table.ForeignKey(
                        name: "fk_usersteams_users_userid",
                        column: x => x.userid,
                        principalTable: "users",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "oidcexternallogins",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    email = table.Column<string>(type: "text", maxLength: 320, nullable: true),
                    providerid = table.Column<Guid>(type: "uuid", nullable: false),
                    subject = table.Column<string>(type: "text", maxLength: 512, nullable: false),
                    updatedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    userid = table.Column<Guid>(type: "uuid", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_oidcexternallogins", x => x.id);
                    table.ForeignKey(
                        name: "fk_oidcexternallogins_oidcproviders_providerid",
                        column: x => x.providerid,
                        principalTable: "oidcproviders",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                    table.ForeignKey(
                        name: "fk_oidcexternallogins_users_userid",
                        column: x => x.userid,
                        principalTable: "users",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "oidcloginstates",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    codeverifier = table.Column<string>(type: "text", maxLength: 256, nullable: false),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    expiresat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    nonce = table.Column<string>(type: "text", maxLength: 256, nullable: false),
                    providerid = table.Column<Guid>(type: "uuid", nullable: false),
                    returnurl = table.Column<string>(type: "text", maxLength: 2048, nullable: false),
                    statehash = table.Column<string>(type: "text", maxLength: 128, nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_oidcloginstates", x => x.id);
                    table.ForeignKey(
                        name: "fk_oidcloginstates_oidcproviders_providerid",
                        column: x => x.providerid,
                        principalTable: "oidcproviders",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "backuprepositories",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    archivedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    controlstartedat = table.Column<long>(type: "bigint", nullable: true),
                    controlstate = table.Column<string>(type: "text", maxLength: 64, nullable: false, defaultValue: "Idle"),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    createdbyactorid = table.Column<Guid>(type: "uuid", nullable: false),
                    currentrunid = table.Column<Guid>(type: "uuid", nullable: true),
                    description = table.Column<string>(type: "text", maxLength: 600, nullable: true),
                    lastcheckedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    lastprunedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    name = table.Column<string>(type: "text", maxLength: 128, nullable: false),
                    normalizedname = table.Column<string>(type: "text", maxLength: 128, nullable: false),
                    passwordsecretid = table.Column<Guid>(type: "uuid", nullable: false),
                    rowversion = table.Column<long>(type: "bigint", nullable: false, defaultValue: 0L),
                    spec = table.Column<string>(type: "jsonb", nullable: false),
                    status = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    type = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    updatedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP")
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_backuprepositories", x => x.id);
                    table.ForeignKey(
                        name: "fk_backuprepositories_actors_createdbyactorid",
                        column: x => x.createdbyactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_backuprepositories_secretdefinitions_passwordsecretid",
                        column: x => x.passwordsecretid,
                        principalTable: "secretdefinitions",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "internalsecretvalues",
                columns: table => new
                {
                    secretid = table.Column<Guid>(type: "uuid", nullable: false),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    encryptedvalue = table.Column<string>(type: "text", nullable: false),
                    updatedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP")
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_internalsecretvalues", x => x.secretid);
                    table.ForeignKey(
                        name: "fk_internalsecretvalues_secretdefinitions_secretid",
                        column: x => x.secretid,
                        principalTable: "secretdefinitions",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "resourcebindings",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    kind = table.Column<string>(type: "text", nullable: false),
                    name = table.Column<string>(type: "text", nullable: false),
                    resourceid = table.Column<Guid>(type: "uuid", nullable: true),
                    scope = table.Column<string>(type: "text", nullable: false),
                    secretdeliverymode = table.Column<string>(type: "text", nullable: true),
                    secretid = table.Column<Guid>(type: "uuid", nullable: true),
                    targetpath = table.Column<string>(type: "text", nullable: true),
                    updatedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    value = table.Column<string>(type: "text", nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_resourcebindings", x => x.id);
                    table.ForeignKey(
                        name: "fk_resourcebindings_secretdefinitions_secretid",
                        column: x => x.secretid,
                        principalTable: "secretdefinitions",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "buildprojects",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    archivedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    branch = table.Column<string>(type: "text", maxLength: 256, nullable: false),
                    buildagentpoolid = table.Column<Guid>(type: "uuid", nullable: true),
                    buildargs = table.Column<string>(type: "jsonb", nullable: false, defaultValueSql: "'[]'::jsonb"),
                    buildsecrets = table.Column<string>(type: "jsonb", nullable: false, defaultValueSql: "'[]'::jsonb"),
                    builderkind = table.Column<string>(type: "text", maxLength: 64, nullable: false, defaultValue: "Platform"),
                    contextpath = table.Column<string>(type: "text", maxLength: 512, nullable: false, defaultValue: "."),
                    controlstartedat = table.Column<long>(type: "bigint", nullable: true),
                    controlstate = table.Column<string>(type: "text", maxLength: 64, nullable: false, defaultValue: "Idle"),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    createdbyactorid = table.Column<Guid>(type: "uuid", nullable: false),
                    currentrunid = table.Column<Guid>(type: "uuid", nullable: true),
                    description = table.Column<string>(type: "text", maxLength: 600, nullable: true),
                    dockerfilepath = table.Column<string>(type: "text", maxLength: 512, nullable: false, defaultValue: "Dockerfile"),
                    enabled = table.Column<bool>(type: "boolean", nullable: false, defaultValue: true),
                    gitrepositoryid = table.Column<Guid>(type: "uuid", nullable: false),
                    imagerepository = table.Column<string>(type: "text", maxLength: 512, nullable: false),
                    name = table.Column<string>(type: "text", maxLength: 128, nullable: false),
                    normalizedname = table.Column<string>(type: "text", maxLength: 128, nullable: false),
                    platformid = table.Column<Guid>(type: "uuid", nullable: true),
                    registryid = table.Column<Guid>(type: "uuid", nullable: false),
                    retentionruncount = table.Column<int>(type: "integer", nullable: false, defaultValue: 20),
                    rowversion = table.Column<long>(type: "bigint", nullable: false, defaultValue: 0L),
                    tagtemplates = table.Column<string>(type: "jsonb", nullable: false, defaultValueSql: "'[\"{branch}-{shortSha}\"]'::jsonb"),
                    target = table.Column<string>(type: "text", maxLength: 128, nullable: true),
                    timeoutseconds = table.Column<int>(type: "integer", nullable: false, defaultValue: 1800),
                    updatedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    webhook = table.Column<string>(type: "jsonb", nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_buildprojects", x => x.id);
                    table.CheckConstraint("CK_BuildProjects_Builder_Target", "(\"builderkind\" = 'Platform' AND \"platformid\" IS NOT NULL AND \"buildagentpoolid\" IS NULL) OR (\"builderkind\" = 'BuildAgentPool' AND \"platformid\" IS NULL AND \"buildagentpoolid\" IS NOT NULL)");
                    table.ForeignKey(
                        name: "fk_buildprojects_actors_createdbyactorid",
                        column: x => x.createdbyactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_buildprojects_buildagentpools_buildagentpoolid",
                        column: x => x.buildagentpoolid,
                        principalTable: "buildagentpools",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_buildprojects_gitrepositories_gitrepositoryid",
                        column: x => x.gitrepositoryid,
                        principalTable: "gitrepositories",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_buildprojects_platforms_platformid",
                        column: x => x.platformid,
                        principalTable: "platforms",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_buildprojects_registries_registryid",
                        column: x => x.registryid,
                        principalTable: "registries",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "gitrepositoryrefs",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    branch = table.Column<string>(type: "text", nullable: false),
                    gitrepositoryid = table.Column<Guid>(type: "uuid", nullable: false),
                    lasterror = table.Column<string>(type: "text", nullable: true),
                    lastsyncedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    resolvedcommitsha = table.Column<string>(type: "text", nullable: true),
                    status = table.Column<string>(type: "text", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_gitrepositoryrefs", x => x.id);
                    table.ForeignKey(
                        name: "fk_gitrepositoryrefs_gitrepositories_gitrepositoryid",
                        column: x => x.gitrepositoryid,
                        principalTable: "gitrepositories",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "containers",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    controlstartedat = table.Column<long>(type: "bigint", nullable: true),
                    controlstate = table.Column<string>(type: "text", maxLength: 64, nullable: true, defaultValue: "Idle"),
                    controltriggeredby = table.Column<Guid>(type: "uuid", nullable: true),
                    created = table.Column<long>(type: "bigint", nullable: false),
                    deploymentid = table.Column<Guid>(type: "uuid", nullable: true),
                    dockercontainerid = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    dockerimageid = table.Column<string>(type: "text", nullable: false),
                    imageid = table.Column<Guid>(type: "uuid", nullable: true),
                    name = table.Column<string>(type: "text", nullable: false),
                    platformid = table.Column<Guid>(type: "uuid", nullable: false),
                    ports = table.Column<string>(type: "json", nullable: false),
                    rowversion = table.Column<long>(type: "bigint", nullable: false, defaultValue: 0L),
                    stack = table.Column<string>(type: "text", nullable: true),
                    stackid = table.Column<Guid>(type: "uuid", nullable: true),
                    state = table.Column<string>(type: "text", nullable: false),
                    updated = table.Column<long>(type: "bigint", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_containers", x => x.id);
                    table.ForeignKey(
                        name: "fk_containers_actors_controltriggeredby",
                        column: x => x.controltriggeredby,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_containers_deployments_deploymentid",
                        column: x => x.deploymentid,
                        principalTable: "deployments",
                        principalColumn: "id",
                        onDelete: ReferentialAction.SetNull);
                    table.ForeignKey(
                        name: "fk_containers_images_imageid",
                        column: x => x.imageid,
                        principalTable: "images",
                        principalColumn: "id",
                        onDelete: ReferentialAction.SetNull);
                    table.ForeignKey(
                        name: "fk_containers_platforms_platformid",
                        column: x => x.platformid,
                        principalTable: "platforms",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                    table.ForeignKey(
                        name: "fk_containers_stacks_stackid",
                        column: x => x.stackid,
                        principalTable: "stacks",
                        principalColumn: "id",
                        onDelete: ReferentialAction.SetNull);
                });

            migrationBuilder.CreateTable(
                name: "stackreleasevolumebindings",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    composevolumename = table.Column<string>(type: "text", maxLength: 255, nullable: true),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    isanonymous = table.Column<bool>(type: "boolean", nullable: false, defaultValue: false),
                    isexternal = table.Column<bool>(type: "boolean", nullable: false, defaultValue: false),
                    platformid = table.Column<Guid>(type: "uuid", nullable: false),
                    stackreleaseid = table.Column<Guid>(type: "uuid", nullable: false),
                    volumename = table.Column<string>(type: "text", maxLength: 255, nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_stackreleasevolumebindings", x => x.id);
                    table.ForeignKey(
                        name: "fk_stackreleasevolumebindings_platforms_platformid",
                        column: x => x.platformid,
                        principalTable: "platforms",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_stackreleasevolumebindings_stackreleases_stackreleaseid",
                        column: x => x.stackreleaseid,
                        principalTable: "stackreleases",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "backuppolicies",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    alertonfailure = table.Column<bool>(type: "boolean", nullable: false, defaultValue: true),
                    archivedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    backuprepositoryid = table.Column<Guid>(type: "uuid", nullable: false),
                    controlstartedat = table.Column<long>(type: "bigint", nullable: true),
                    controlstate = table.Column<string>(type: "text", maxLength: 64, nullable: false, defaultValue: "Idle"),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    createdbyactorid = table.Column<Guid>(type: "uuid", nullable: false),
                    cron = table.Column<string>(type: "text", maxLength: 128, nullable: true),
                    currentrunid = table.Column<Guid>(type: "uuid", nullable: true),
                    description = table.Column<string>(type: "text", maxLength: 600, nullable: true),
                    enabled = table.Column<bool>(type: "boolean", nullable: false, defaultValue: true),
                    firstsuccessfulrunat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    keeplastsuccessful = table.Column<int>(type: "integer", nullable: false, defaultValue: 14),
                    lastscheduledrunat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    name = table.Column<string>(type: "text", maxLength: 128, nullable: false),
                    normalizedname = table.Column<string>(type: "text", maxLength: 128, nullable: false),
                    rowversion = table.Column<long>(type: "bigint", nullable: false, defaultValue: 0L),
                    runasactorid = table.Column<Guid>(type: "uuid", nullable: false),
                    source = table.Column<string>(type: "jsonb", nullable: false),
                    timezone = table.Column<string>(type: "text", maxLength: 128, nullable: true),
                    timeoutseconds = table.Column<int>(type: "integer", nullable: false, defaultValue: 14400),
                    updatedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    webhook = table.Column<string>(type: "jsonb", nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_backuppolicies", x => x.id);
                    table.ForeignKey(
                        name: "fk_backuppolicies_actors_createdbyactorid",
                        column: x => x.createdbyactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_backuppolicies_actors_runasactorid",
                        column: x => x.runasactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_backuppolicies_backuprepositories_backuprepositoryid",
                        column: x => x.backuprepositoryid,
                        principalTable: "backuprepositories",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "backuprepositoryleases",
                columns: table => new
                {
                    backuprepositoryid = table.Column<Guid>(type: "uuid", nullable: false),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    expiresat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    operationtype = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    ownerrunid = table.Column<Guid>(type: "uuid", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_backuprepositoryleases", x => x.backuprepositoryid);
                    table.ForeignKey(
                        name: "fk_backuprepositoryleases_backuprepositories_backuprepositoryid",
                        column: x => x.backuprepositoryid,
                        principalTable: "backuprepositories",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "backuprepositoryvalidations",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    backuprepositoryid = table.Column<Guid>(type: "uuid", nullable: false),
                    lasterrorcode = table.Column<string>(type: "text", maxLength: 128, nullable: true),
                    lasterrormessage = table.Column<string>(type: "text", maxLength: 600, nullable: true),
                    lastvalidatedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    location = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    platformid = table.Column<Guid>(type: "uuid", nullable: true),
                    status = table.Column<string>(type: "text", maxLength: 64, nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_backuprepositoryvalidations", x => x.id);
                    table.ForeignKey(
                        name: "fk_backuprepositoryvalidations_backuprepositories_backupreposi~",
                        column: x => x.backuprepositoryid,
                        principalTable: "backuprepositories",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                    table.ForeignKey(
                        name: "fk_backuprepositoryvalidations_platforms_platformid",
                        column: x => x.platformid,
                        principalTable: "platforms",
                        principalColumn: "id",
                        onDelete: ReferentialAction.SetNull);
                });

            migrationBuilder.CreateTable(
                name: "buildruns",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    branch = table.Column<string>(type: "text", maxLength: 256, nullable: false),
                    buildargssnapshot = table.Column<string>(type: "jsonb", nullable: false, defaultValueSql: "'[]'::jsonb"),
                    buildprojectid = table.Column<Guid>(type: "uuid", nullable: false),
                    buildsecretidssnapshot = table.Column<string>(type: "jsonb", nullable: false, defaultValueSql: "'[]'::jsonb"),
                    completedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    contextpath = table.Column<string>(type: "text", maxLength: 512, nullable: false),
                    dockerfilepath = table.Column<string>(type: "text", maxLength: 512, nullable: false),
                    errorcode = table.Column<string>(type: "text", maxLength: 128, nullable: true),
                    errormessage = table.Column<string>(type: "text", maxLength: 1200, nullable: true),
                    exitcode = table.Column<int>(type: "integer", nullable: true),
                    gitrepositoryid = table.Column<Guid>(type: "uuid", nullable: false),
                    gitrepositorynamesnapshot = table.Column<string>(type: "text", maxLength: 128, nullable: false),
                    imagedigest = table.Column<string>(type: "text", maxLength: 256, nullable: true),
                    imagereferences = table.Column<string>(type: "jsonb", nullable: false, defaultValueSql: "'[]'::jsonb"),
                    imagerepository = table.Column<string>(type: "text", maxLength: 512, nullable: false),
                    platformsnapshot = table.Column<string>(type: "jsonb", nullable: false),
                    projectnamesnapshot = table.Column<string>(type: "text", maxLength: 128, nullable: false),
                    queuedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    registrysnapshot = table.Column<string>(type: "jsonb", nullable: false),
                    resolvedcommitsha = table.Column<string>(type: "text", maxLength: 128, nullable: true),
                    startedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    status = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    tagtemplatessnapshot = table.Column<string>(type: "jsonb", nullable: false, defaultValueSql: "'[]'::jsonb"),
                    target = table.Column<string>(type: "text", maxLength: 128, nullable: true),
                    timeoutseconds = table.Column<int>(type: "integer", nullable: false),
                    trigger = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    triggersourceid = table.Column<Guid>(type: "uuid", nullable: true),
                    triggeredbyactorid = table.Column<Guid>(type: "uuid", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_buildruns", x => x.id);
                    table.ForeignKey(
                        name: "fk_buildruns_actors_triggeredbyactorid",
                        column: x => x.triggeredbyactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_buildruns_buildprojects_buildprojectid",
                        column: x => x.buildprojectid,
                        principalTable: "buildprojects",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_buildruns_gitrepositories_gitrepositoryid",
                        column: x => x.gitrepositoryid,
                        principalTable: "gitrepositories",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "containerstats",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    containerid = table.Column<Guid>(type: "uuid", nullable: false),
                    cpuusage = table.Column<double>(type: "double precision", nullable: false),
                    created = table.Column<long>(type: "bigint", nullable: false),
                    memoryactive = table.Column<double>(type: "double precision", nullable: false),
                    memorycache = table.Column<double>(type: "double precision", nullable: false),
                    memorylimit = table.Column<double>(type: "double precision", nullable: false),
                    rxbytes = table.Column<double>(type: "double precision", nullable: false),
                    txbytes = table.Column<double>(type: "double precision", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_containerstats", x => x.id);
                    table.ForeignKey(
                        name: "fk_containerstats_containers_containerid",
                        column: x => x.containerid,
                        principalTable: "containers",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "backupruns",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    backuppolicyid = table.Column<Guid>(type: "uuid", nullable: false),
                    backuprepositoryid = table.Column<Guid>(type: "uuid", nullable: false),
                    bytesadded = table.Column<long>(type: "bigint", nullable: true),
                    bytesprocessed = table.Column<long>(type: "bigint", nullable: true),
                    completedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    errorcode = table.Column<string>(type: "text", maxLength: 128, nullable: true),
                    errormessage = table.Column<string>(type: "text", maxLength: 1200, nullable: true),
                    exitcode = table.Column<int>(type: "integer", nullable: true),
                    filesprocessed = table.Column<long>(type: "bigint", nullable: true),
                    parentsnapshotid = table.Column<string>(type: "text", maxLength: 128, nullable: true),
                    policynamesnapshot = table.Column<string>(type: "text", maxLength: 128, nullable: false),
                    queuedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    repositorytypesnapshot = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    resticsnapshotid = table.Column<string>(type: "text", maxLength: 128, nullable: true),
                    snapshotavailability = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    sourcesnapshot = table.Column<string>(type: "jsonb", nullable: false),
                    startedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    status = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    trigger = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    triggersourceid = table.Column<Guid>(type: "uuid", nullable: true),
                    triggeredbyactorid = table.Column<Guid>(type: "uuid", nullable: false),
                    warnings = table.Column<string>(type: "jsonb", nullable: false, defaultValueSql: "'[]'::jsonb")
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_backupruns", x => x.id);
                    table.ForeignKey(
                        name: "fk_backupruns_actors_triggeredbyactorid",
                        column: x => x.triggeredbyactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_backupruns_backuppolicies_backuppolicyid",
                        column: x => x.backuppolicyid,
                        principalTable: "backuppolicies",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_backupruns_backuprepositories_backuprepositoryid",
                        column: x => x.backuprepositoryid,
                        principalTable: "backuprepositories",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "buildrunlogs",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    buildrunid = table.Column<Guid>(type: "uuid", nullable: false),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    message = table.Column<string>(type: "text", nullable: false),
                    stream = table.Column<string>(type: "text", maxLength: 32, nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_buildrunlogs", x => x.id);
                    table.ForeignKey(
                        name: "fk_buildrunlogs_buildruns_buildrunid",
                        column: x => x.buildrunid,
                        principalTable: "buildruns",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "backuprestoreruns",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    affectedcontainers = table.Column<string>(type: "jsonb", nullable: false, defaultValueSql: "'[]'::jsonb"),
                    backuprepositoryid = table.Column<Guid>(type: "uuid", nullable: false),
                    backuprunid = table.Column<Guid>(type: "uuid", nullable: false),
                    completedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    errorcode = table.Column<string>(type: "text", maxLength: 128, nullable: true),
                    errormessage = table.Column<string>(type: "text", maxLength: 1200, nullable: true),
                    exitcode = table.Column<int>(type: "integer", nullable: true),
                    overwriteexisting = table.Column<bool>(type: "boolean", nullable: false),
                    queuedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    startedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    status = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    targetplatformid = table.Column<Guid>(type: "uuid", nullable: false),
                    targetvolumecreatedbycitadel = table.Column<bool>(type: "boolean", nullable: false),
                    targetvolumename = table.Column<string>(type: "text", maxLength: 255, nullable: false),
                    triggeredbyactorid = table.Column<Guid>(type: "uuid", nullable: false),
                    warnings = table.Column<string>(type: "jsonb", nullable: false, defaultValueSql: "'[]'::jsonb")
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_backuprestoreruns", x => x.id);
                    table.ForeignKey(
                        name: "fk_backuprestoreruns_actors_triggeredbyactorid",
                        column: x => x.triggeredbyactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_backuprestoreruns_backuprepositories_backuprepositoryid",
                        column: x => x.backuprepositoryid,
                        principalTable: "backuprepositories",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_backuprestoreruns_backupruns_backuprunid",
                        column: x => x.backuprunid,
                        principalTable: "backupruns",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_backuprestoreruns_platforms_targetplatformid",
                        column: x => x.targetplatformid,
                        principalTable: "platforms",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.CreateTable(
                name: "backuprunitems",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    backuprunid = table.Column<Guid>(type: "uuid", nullable: false),
                    bytesadded = table.Column<long>(type: "bigint", nullable: true),
                    bytesprocessed = table.Column<long>(type: "bigint", nullable: true),
                    completedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    errorcode = table.Column<string>(type: "text", maxLength: 128, nullable: true),
                    errormessage = table.Column<string>(type: "text", maxLength: 1200, nullable: true),
                    exitcode = table.Column<int>(type: "integer", nullable: true),
                    filesprocessed = table.Column<long>(type: "bigint", nullable: true),
                    parentsnapshotid = table.Column<string>(type: "text", maxLength: 128, nullable: true),
                    platformid = table.Column<Guid>(type: "uuid", nullable: false),
                    resticsnapshotid = table.Column<string>(type: "text", maxLength: 128, nullable: true),
                    startedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    status = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    updatedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    volumename = table.Column<string>(type: "text", maxLength: 255, nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_backuprunitems", x => x.id);
                    table.ForeignKey(
                        name: "fk_backuprunitems_backupruns_backuprunid",
                        column: x => x.backuprunid,
                        principalTable: "backupruns",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                    table.ForeignKey(
                        name: "fk_backuprunitems_platforms_platformid",
                        column: x => x.platformid,
                        principalTable: "platforms",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "backuprunlogs",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    backuprunid = table.Column<Guid>(type: "uuid", nullable: false),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    message = table.Column<string>(type: "text", nullable: false),
                    stream = table.Column<string>(type: "text", maxLength: 32, nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_backuprunlogs", x => x.id);
                    table.ForeignKey(
                        name: "fk_backuprunlogs_backupruns_backuprunid",
                        column: x => x.backuprunid,
                        principalTable: "backupruns",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "backuprestorerunlogs",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    backuprestorerunid = table.Column<Guid>(type: "uuid", nullable: false),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    message = table.Column<string>(type: "text", nullable: false),
                    stream = table.Column<string>(type: "text", maxLength: 32, nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_backuprestorerunlogs", x => x.id);
                    table.ForeignKey(
                        name: "fk_backuprestorerunlogs_backuprestoreruns_backuprestorerunid",
                        column: x => x.backuprestorerunid,
                        principalTable: "backuprestoreruns",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.InsertData(
                table: "actors",
                columns: new[] { "id", "isenabled", "type" },
                values: new object[,]
                {
                    { new Guid("00000000-0000-0000-0000-000000000001"), true, "System" },
                    { new Guid("00000000-0000-0000-0000-000000000002"), true, "User" },
                    { new Guid("00000000-0000-0000-0000-000000000003"), true, "Team" }
                });

            migrationBuilder.InsertData(
                table: "roles",
                columns: new[] { "id", "name", "roletype" },
                values: new object[,]
                {
                    { new Guid("30000000-0000-0000-0000-000000000001"), "Admin", "System" },
                    { new Guid("30000000-0000-0000-0000-000000000002"), "Operator", "System" },
                    { new Guid("30000000-0000-0000-0000-000000000003"), "Viewer", "System" }
                });

            migrationBuilder.InsertData(
                table: "actions",
                columns: new[] { "id", "alertonfailure", "code", "controlstartedat", "controlstate", "createdat", "createdbyactorid", "currentrunid", "defaultargsjson", "description", "enabled", "lastscheduledrunat", "name", "runasactorid", "schedulecron", "scheduleenabled", "scheduletimezone", "timeoutseconds", "updatedat", "webhook" },
                values: new object[,]
                {
                    { new Guid("41000000-0000-0000-0000-000000000001"), true, "const platformsResponse = await citadel.platforms.listPlatforms();\nconst platforms = platformsResponse?.platforms ?? [];\nlet pruned = 0;\nlet reclaimedBytes = 0;\n\nfor (const platform of platforms) {\n  if (platform.status === 'Offline') continue\n  const result = await citadel.platforms.prunePlatform(platform.id, { resource: \"Image\" });\n  const imagesDeleted = result?.imagesDeleted ?? [];\n  const reclaimed = Number(result?.spaceReclaimed ?? 0);\n  reclaimedBytes += reclaimed;\n  pruned += imagesDeleted.length;\n\n  if (imagesDeleted.length === 0) {\n    console.log(`No unused images on ${platform.name}.`);\n    continue;\n  }\n\n  console.log(`Pruned ${imagesDeleted.length} image item(s) on ${platform.name}; reclaimed ${reclaimed} bytes.`);\n}\n\nconsole.log(`Pruned ${pruned} image item(s); reclaimed ${reclaimedBytes} bytes.`);", null, "Idle", new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "{}", "Prunes unused Docker images on every platform.", false, null, "Prune images", new Guid("00000000-0000-0000-0000-000000000002"), "0 12 * * *", true, "UTC", 300, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), null },
                    { new Guid("41000000-0000-0000-0000-000000000002"), true, "const stacksResponse = await citadel.stacks.listStacks({ tags: [\"Prod\"] });\nconst stacks = stacksResponse?.stacks ?? [];\nconst unhealthyStacks = stacks.filter(\n  (stack) => stack.status !== \"Healthy\" && stack.controlState !== \"Processing\"\n);\n\nif (unhealthyStacks.length === 0) {\n  console.log(\"No unhealthy Prod stacks found.\");\n} else {\n  const stackIds = unhealthyStacks.map((stack) => stack.id);\n  await citadel.stacks.restartStacks(stackIds);\n  console.log(`Requested restart for ${stackIds.length} Prod stack(s).`);\n}", null, "Idle", new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "{}", "Restarts stacks tagged Prod when their current release is not healthy.", false, null, "Restart unhealthy stacks", new Guid("00000000-0000-0000-0000-000000000002"), "*/15 * * * *", true, "UTC", 300, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), null }
                });

            migrationBuilder.InsertData(
                table: "actorroles",
                columns: new[] { "actorid", "roleid" },
                values: new object[,]
                {
                    { new Guid("00000000-0000-0000-0000-000000000002"), new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("00000000-0000-0000-0000-000000000003"), new Guid("30000000-0000-0000-0000-000000000002") }
                });

            migrationBuilder.InsertData(
                table: "alertrules",
                columns: new[] { "id", "cooldownseconds", "createdat", "createdbyactorid", "description", "limitedto", "name", "quiethours", "requiredmatches", "severity", "threshold", "type" },
                values: new object[,]
                {
                    { new Guid("019d0000-0001-7000-8001-000000000001"), 300, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "CPU > 90% - Platform", "[]", 3, "Critical", 90.0, "PlatformCpuHigh" },
                    { new Guid("019d0000-0001-7000-8001-000000000002"), 300, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "RAM > 90% - Platform", "[]", 3, "Critical", 90.0, "PlatformRamHigh" },
                    { new Guid("019d0000-0001-7000-8001-000000000003"), 600, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Platform Unreachable", "[]", null, "Critical", null, "PlatformUnreachable" },
                    { new Guid("019d0000-0001-7000-8001-000000000004"), 3600, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Platform Version Mismatch", "[]", null, "Warning", null, "PlatformVersionMismatch" },
                    { new Guid("019d0000-0001-7000-8001-000000000005"), null, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Unmanaged Container Created", "[]", null, "Info", null, "UnmanagedContainerCreated" },
                    { new Guid("019d0000-0001-7000-8001-000000000006"), 86400, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Image Update Available - Deployment", "[]", null, "Info", null, "DeploymentImageUpdateAvailable" },
                    { new Guid("019d0000-0001-7000-8001-000000000007"), null, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Auto Deploy Failed - Deployment", "[]", null, "Critical", null, "DeploymentAutoDeployFailed" },
                    { new Guid("019d0000-0001-7000-8001-000000000008"), null, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Deployment Auto Updated", "[]", null, "Info", null, "DeploymentAutoUpdated" },
                    { new Guid("019d0000-0001-7000-8001-000000000009"), 86400, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Image Update Available - Stack", "[]", null, "Info", null, "StackImageUpdateAvailable" },
                    { new Guid("019d0000-0001-7000-8001-00000000000a"), null, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Auto Deploy Failed - Stack", "[]", null, "Critical", null, "StackAutoDeployFailed" },
                    { new Guid("019d0000-0001-7000-8001-00000000000b"), null, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Stack Auto Updated", "[]", null, "Info", null, "StackAutoUpdated" },
                    { new Guid("019d0000-0001-7000-8001-00000000000c"), 300, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Stack Drift Detected", "[]", null, "Warning", null, "StackDriftDetected" },
                    { new Guid("019d0000-0001-7000-8001-00000000000d"), null, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Stack Service Auto Updated", "[]", null, "Info", null, "StackServiceAutoUpdated" },
                    { new Guid("019d0000-0001-7000-8001-00000000000e"), null, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Auto Deploy Failed - Stack Service", "[]", null, "Critical", null, "StackServiceAutoDeployFailed" },
                    { new Guid("019d0000-0001-7000-8001-00000000000f"), 300, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Webhook Authentication Failed", "[]", null, "Warning", null, "WebhookAuthenticationFailed" },
                    { new Guid("019d0000-0001-7000-8001-000000000010"), 300, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Webhook Dispatch Failed", "[]", null, "Warning", null, "WebhookDispatchFailed" },
                    { new Guid("019d0000-0001-7000-8001-000000000011"), 300, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "CPU > 80% - Platform", "[]", 3, "Warning", 80.0, "PlatformCpuHigh" },
                    { new Guid("019d0000-0001-7000-8001-000000000012"), 300, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Webhook Sync Failed - Git Repository", "[]", null, "Warning", null, "WebhookGitRepoSyncFailed" },
                    { new Guid("019d0000-0001-7000-8001-000000000013"), 300, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Webhook Deploy Failed - Git Stack", "[]", null, "Critical", null, "WebhookStackGitDeployFailed" },
                    { new Guid("019d0000-0001-7000-8001-000000000014"), null, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Stack Drift Auto Reconciled", "[]", null, "Info", null, "StackDriftAutoReconciled" },
                    { new Guid("019d0000-0001-7000-8001-000000000015"), 86400, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Git Update Available - Stack", "[]", null, "Info", null, "StackGitUpdateAvailable" },
                    { new Guid("019d0000-0001-7000-8001-000000000016"), null, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Git Stack Auto Updated", "[]", null, "Info", null, "StackGitAutoUpdated" },
                    { new Guid("019d0000-0001-7000-8001-000000000017"), null, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Git Auto Deploy Failed - Stack", "[]", null, "Critical", null, "StackGitAutoDeployFailed" }
                });

            migrationBuilder.InsertData(
                table: "alertrules",
                columns: new[] { "id", "cooldownseconds", "createdat", "createdbyactorid", "description", "limitedto", "name", "quiethours", "requiredmatches", "severity", "status", "threshold", "type" },
                values: new object[,]
                {
                    { new Guid("019d0000-0001-7000-8001-000000000018"), null, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Automation Action Run Failed", "[]", null, "Critical", "Enabled", null, "AutomationActionRunFailed" },
                    { new Guid("019d0000-0001-7000-8001-000000000019"), null, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "License Entered Grace Period", "[]", null, "Warning", "Enabled", null, "LicenseEnteredGracePeriod" },
                    { new Guid("019d0000-0001-7000-8001-00000000001a"), null, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "License Expired", "[]", null, "Critical", "Enabled", null, "LicenseExpired" },
                    { new Guid("019d0000-0001-7000-8001-00000000001b"), null, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Build Run Failed", "[]", null, "Critical", "Enabled", null, "BuildRunFailed" }
                });

            migrationBuilder.InsertData(
                table: "alertrules",
                columns: new[] { "id", "cooldownseconds", "createdat", "createdbyactorid", "description", "limitedto", "name", "quiethours", "requiredmatches", "severity", "threshold", "type" },
                values: new object[] { new Guid("019d0000-0001-7000-8001-000000000022"), 300, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "RAM > 80% - Platform", "[]", 3, "Warning", 80.0, "PlatformRamHigh" });

            migrationBuilder.InsertData(
                table: "permissions",
                columns: new[] { "id", "permissionlevel", "resourcetype", "roleid", "specificpermissions" },
                values: new object[,]
                {
                    { new Guid("030c8f34-4447-d6b0-bc28-62b9626999c7"), 1, 1, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("077b64cb-9dc8-4ac2-be0a-81550b977042"), 1, 19, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("07b143ad-6b02-c7ff-3d7d-48af137b2bbc"), 2, 0, new Guid("30000000-0000-0000-0000-000000000002"), 27 },
                    { new Guid("102eae03-0582-a53c-2287-ce55c2f222a8"), 2, 16, new Guid("30000000-0000-0000-0000-000000000002"), 128 },
                    { new Guid("197f429f-cbe9-0e6c-239b-2f24196ec817"), 2, 2, new Guid("30000000-0000-0000-0000-000000000002"), 103 },
                    { new Guid("2d9c5d81-bce2-e0a6-004b-138d3ac0a4a9"), 4, 3, new Guid("30000000-0000-0000-0000-000000000001"), 0 },
                    { new Guid("361e1bf8-ef0f-1409-9137-6fa885696a19"), 4, 7, new Guid("30000000-0000-0000-0000-000000000001"), 0 },
                    { new Guid("378efbb8-bac7-1928-e93a-7d152a50b3b6"), 1, 15, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("440deb9d-ace8-5e15-ef80-3a42f11a0c42"), 4, 10, new Guid("30000000-0000-0000-0000-000000000001"), 0 },
                    { new Guid("51ac9abd-9f17-8530-e1a4-8fe69e44ac1d"), 4, 1, new Guid("30000000-0000-0000-0000-000000000001"), 55 },
                    { new Guid("611400ed-eed0-2a88-49d3-02354e25f43c"), 2, 19, new Guid("30000000-0000-0000-0000-000000000002"), 4 },
                    { new Guid("6128787e-901d-f15b-c094-054be267a6c8"), 1, 16, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("645b4c54-7937-2180-7186-be24ac6bf330"), 4, 5, new Guid("30000000-0000-0000-0000-000000000001"), 0 },
                    { new Guid("656ccf45-65a9-33b3-de65-d18d5988151a"), 4, 12, new Guid("30000000-0000-0000-0000-000000000001"), 0 },
                    { new Guid("662623f3-aa3b-220d-546b-971d2947f7cd"), 2, 13, new Guid("30000000-0000-0000-0000-000000000002"), 0 },
                    { new Guid("677df0f0-2ce5-4b25-76ad-eb4e21f0748d"), 4, 17, new Guid("30000000-0000-0000-0000-000000000001"), 768 },
                    { new Guid("6a2b1742-029b-d0df-1d2a-1d0aa1ed9a3f"), 1, 18, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("7125b1ec-d593-d356-f559-c4655a392c31"), 2, 1, new Guid("30000000-0000-0000-0000-000000000002"), 55 },
                    { new Guid("7cb0a723-f754-2be2-38fa-8d151c2e1c7f"), 2, 15, new Guid("30000000-0000-0000-0000-000000000002"), 0 },
                    { new Guid("86dadd60-fced-3dcd-cdbe-8d262bec7d22"), 2, 5, new Guid("30000000-0000-0000-0000-000000000002"), 0 },
                    { new Guid("8b91dc54-cf72-44f6-0505-3c426552df35"), 4, 14, new Guid("30000000-0000-0000-0000-000000000001"), 0 },
                    { new Guid("9042fcd7-44f2-8a16-dca9-2fabdba5a0bc"), 1, 17, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("909763b4-50a0-e1c7-6df1-61add076910c"), 1, 2, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("936632a5-4e74-0a17-fb8e-497c960c3005"), 1, 0, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("94717f37-cc1a-de60-9bca-dc6379444bfb"), 1, 5, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("97597a3e-c415-667b-039a-a7a287daefea"), 1, 4, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("987e89d0-2c8f-87d8-830f-7461a7db392e"), 4, 4, new Guid("30000000-0000-0000-0000-000000000001"), 0 },
                    { new Guid("9b075e03-6326-7b95-ae78-2b296990ce26"), 2, 4, new Guid("30000000-0000-0000-0000-000000000002"), 0 },
                    { new Guid("9e0c1481-c640-3182-c9a0-4687ef91bd6a"), 4, 13, new Guid("30000000-0000-0000-0000-000000000001"), 0 },
                    { new Guid("a3cd7182-baa1-324f-79f0-3a04d7647032"), 4, 11, new Guid("30000000-0000-0000-0000-000000000001"), 0 },
                    { new Guid("a4b222e3-7452-b5f4-377b-c05652533f67"), 4, 0, new Guid("30000000-0000-0000-0000-000000000001"), 27 },
                    { new Guid("b2298835-c351-7367-ad8d-e5884be235f3"), 2, 12, new Guid("30000000-0000-0000-0000-000000000002"), 0 },
                    { new Guid("b3abb382-80da-8170-b011-05af044e7908"), 2, 3, new Guid("30000000-0000-0000-0000-000000000002"), 0 },
                    { new Guid("ba1393e4-1090-990e-aee3-a3e36ede0fee"), 4, 16, new Guid("30000000-0000-0000-0000-000000000001"), 128 },
                    { new Guid("c472d905-c03c-a9a8-0527-c2b540274078"), 2, 18, new Guid("30000000-0000-0000-0000-000000000002"), 4 },
                    { new Guid("d1af8dbf-ef7d-d81d-33f9-e3be5ee72243"), 4, 18, new Guid("30000000-0000-0000-0000-000000000001"), 4 },
                    { new Guid("d5fa8563-b0a2-4f11-7e16-7c1877e43dda"), 4, 6, new Guid("30000000-0000-0000-0000-000000000001"), 0 },
                    { new Guid("dbb104e4-d7e2-5173-b0b2-6d1519c2f682"), 4, 8, new Guid("30000000-0000-0000-0000-000000000001"), 0 },
                    { new Guid("e04cd0d3-47bf-2d28-e099-c7a9b61e3875"), 1, 3, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("e89ccf24-0132-149c-c8bc-33265af98ed8"), 1, 12, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("e9b46175-cc60-8d02-1dbd-ddf7f907eb16"), 4, 15, new Guid("30000000-0000-0000-0000-000000000001"), 0 },
                    { new Guid("f1633935-71e7-32f3-4264-d7120dcf22f1"), 1, 13, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("f2552404-ef60-5f22-0eaf-fd7db21f2579"), 2, 17, new Guid("30000000-0000-0000-0000-000000000002"), 768 },
                    { new Guid("f5794228-f6b6-84fa-4fa3-7324decd3402"), 4, 19, new Guid("30000000-0000-0000-0000-000000000001"), 4 },
                    { new Guid("fbb8ef70-2ec3-134f-0c18-1533173d5849"), 4, 9, new Guid("30000000-0000-0000-0000-000000000001"), 0 },
                    { new Guid("fd0c028a-8225-0f65-8a7b-cb058f29c740"), 4, 2, new Guid("30000000-0000-0000-0000-000000000001"), 103 }
                });

            migrationBuilder.InsertData(
                table: "registries",
                columns: new[] { "id", "configuration", "createdat", "createdbyactorid", "description", "name", "registryhost", "status" },
                values: new object[] { new Guid("00000000-0000-0000-0000-000000000100"), "{\r\n    \"$type\": \"DockerHub\"\r\n}", new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), "Public Docker Hub Registry", "Docker Hub", "hub.docker.com", "Active" });

            migrationBuilder.InsertData(
                table: "tags",
                columns: new[] { "id", "color", "createdat", "createdbyactorid", "name", "normalizedname", "updatedat" },
                values: new object[,]
                {
                    { new Guid("40000000-0000-0000-0000-000000000001"), "#6b21a8", new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), "System", "system", new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc) },
                    { new Guid("40000000-0000-0000-0000-000000000002"), "#f87171", new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), "Prod", "prod", new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc) }
                });

            migrationBuilder.InsertData(
                table: "teams",
                columns: new[] { "id", "actorid", "name" },
                values: new object[] { new Guid("20000000-0000-0000-0000-000000000001"), new Guid("00000000-0000-0000-0000-000000000003"), "Operators" });

            migrationBuilder.InsertData(
                table: "users",
                columns: new[] { "id", "actorid", "createdat", "createdbyactorid", "email", "name", "password" },
                values: new object[] { new Guid("10000000-0000-0000-0000-000000000001"), new Guid("00000000-0000-0000-0000-000000000002"), new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), "admin@citadel.local", "admin", "o6hWzZ+DIuSZoHNjf5D1t6101vfm4w2kmPRiAZ3Xq53JMMl1" });

            migrationBuilder.InsertData(
                table: "resourcetags",
                columns: new[] { "resourceid", "resourcetype", "tagid", "createdat", "createdbyactorid" },
                values: new object[,]
                {
                    { new Guid("41000000-0000-0000-0000-000000000001"), "AutomationAction", new Guid("40000000-0000-0000-0000-000000000001"), new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001") },
                    { new Guid("41000000-0000-0000-0000-000000000002"), "AutomationAction", new Guid("40000000-0000-0000-0000-000000000001"), new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001") },
                    { new Guid("41000000-0000-0000-0000-000000000002"), "AutomationAction", new Guid("40000000-0000-0000-0000-000000000002"), new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001") }
                });

            migrationBuilder.CreateIndex(
                name: "ix_actionruns_actionid_queuedat",
                table: "actionruns",
                columns: new[] { "actionid", "queuedat", "id" },
                descending: new[] { false, true, true })
                .Annotation("Npgsql:IndexInclude", new[] { "status" });

            migrationBuilder.CreateIndex(
                name: "ix_actionruns_runasactorid",
                table: "actionruns",
                column: "runasactorid");

            migrationBuilder.CreateIndex(
                name: "ix_actionruns_status_queuedat",
                table: "actionruns",
                columns: new[] { "status", "queuedat" });

            migrationBuilder.CreateIndex(
                name: "ix_actionruns_triggeredbyactorid",
                table: "actionruns",
                column: "triggeredbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_actions_controlstate_controlstartedat",
                table: "actions",
                columns: new[] { "controlstate", "controlstartedat" });

            migrationBuilder.CreateIndex(
                name: "ix_actions_createdbyactorid",
                table: "actions",
                column: "createdbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_actions_globalsearch_name_trgm",
                table: "actions",
                column: "name")
                .Annotation("Npgsql:IndexMethod", "gin")
                .Annotation("Npgsql:IndexOperators", new[] { "gin_trgm_ops" });

            migrationBuilder.CreateIndex(
                name: "ix_actions_name",
                table: "actions",
                column: "name",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_actions_runasactorid",
                table: "actions",
                column: "runasactorid");

            migrationBuilder.CreateIndex(
                name: "ix_actions_schedule",
                table: "actions",
                columns: new[] { "enabled", "scheduleenabled", "schedulecron" });

            migrationBuilder.CreateIndex(
                name: "ix_activityevents_createdbyactorid",
                table: "activityevents",
                column: "createdbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_activityevents_eventtype",
                table: "activityevents",
                column: "eventtype");

            migrationBuilder.CreateIndex(
                name: "ix_activityevents_platform_createdat",
                table: "activityevents",
                columns: new[] { "platformid", "createdat" });

            migrationBuilder.CreateIndex(
                name: "ix_activityevents_resource_createdat",
                table: "activityevents",
                columns: new[] { "resourceid", "createdat" });

            migrationBuilder.CreateIndex(
                name: "ix_activityevents_status",
                table: "activityevents",
                column: "status");

            migrationBuilder.CreateIndex(
                name: "ix_actorroles_actorid",
                table: "actorroles",
                column: "actorid");

            migrationBuilder.CreateIndex(
                name: "ix_actorroles_roleid",
                table: "actorroles",
                column: "roleid");

            migrationBuilder.CreateIndex(
                name: "ix_alertchannels_createdbyactorid",
                table: "alertchannels",
                column: "createdbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_alertevents_alertruleid",
                table: "alertevents",
                column: "alertruleid");

            migrationBuilder.CreateIndex(
                name: "ix_alertevents_openincidentkey",
                table: "alertevents",
                column: "openincidentkey",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_alertevents_resource_createdat",
                table: "alertevents",
                columns: new[] { "resourceid", "createdat" });

            migrationBuilder.CreateIndex(
                name: "ix_alertevents_resourcetype",
                table: "alertevents",
                column: "resourcetype");

            migrationBuilder.CreateIndex(
                name: "ix_alertevents_type",
                table: "alertevents",
                column: "type");

            migrationBuilder.CreateIndex(
                name: "ix_alertrulechannels_alertchannelid",
                table: "alertrulechannels",
                column: "alertchannelid");

            migrationBuilder.CreateIndex(
                name: "ix_alertrules_createdbyactorid",
                table: "alertrules",
                column: "createdbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_alertrules_type",
                table: "alertrules",
                column: "type");

            migrationBuilder.CreateIndex(
                name: "ix_alertrulestates_createdbyactorid",
                table: "alertrulestates",
                column: "createdbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_backuppolicies_archivedat",
                table: "backuppolicies",
                column: "archivedat");

            migrationBuilder.CreateIndex(
                name: "ix_backuppolicies_backuprepositoryid",
                table: "backuppolicies",
                column: "backuprepositoryid");

            migrationBuilder.CreateIndex(
                name: "ix_backuppolicies_controlstate_controlstartedat",
                table: "backuppolicies",
                columns: new[] { "controlstate", "controlstartedat" });

            migrationBuilder.CreateIndex(
                name: "ix_backuppolicies_createdbyactorid",
                table: "backuppolicies",
                column: "createdbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_backuppolicies_globalsearch_name_trgm",
                table: "backuppolicies",
                column: "name",
                filter: "archivedat IS NULL")
                .Annotation("Npgsql:IndexMethod", "gin")
                .Annotation("Npgsql:IndexOperators", new[] { "gin_trgm_ops" });

            migrationBuilder.CreateIndex(
                name: "ix_backuppolicies_normalizedname",
                table: "backuppolicies",
                column: "normalizedname",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_backuppolicies_runasactorid",
                table: "backuppolicies",
                column: "runasactorid");

            migrationBuilder.CreateIndex(
                name: "ix_backuppolicies_schedule",
                table: "backuppolicies",
                columns: new[] { "enabled", "cron" });

            migrationBuilder.CreateIndex(
                name: "ix_backuppolicies_source_gin",
                table: "backuppolicies",
                column: "source")
                .Annotation("Npgsql:IndexMethod", "gin");

            migrationBuilder.CreateIndex(
                name: "ix_backuprepositories_archivedat",
                table: "backuprepositories",
                column: "archivedat");

            migrationBuilder.CreateIndex(
                name: "ix_backuprepositories_controlstate_controlstartedat",
                table: "backuprepositories",
                columns: new[] { "controlstate", "controlstartedat" });

            migrationBuilder.CreateIndex(
                name: "ix_backuprepositories_createdbyactorid",
                table: "backuprepositories",
                column: "createdbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_backuprepositories_globalsearch_name_trgm",
                table: "backuprepositories",
                column: "name",
                filter: "archivedat IS NULL")
                .Annotation("Npgsql:IndexMethod", "gin")
                .Annotation("Npgsql:IndexOperators", new[] { "gin_trgm_ops" });

            migrationBuilder.CreateIndex(
                name: "ix_backuprepositories_globalsearch_type_trgm",
                table: "backuprepositories",
                column: "type",
                filter: "archivedat IS NULL")
                .Annotation("Npgsql:IndexMethod", "gin")
                .Annotation("Npgsql:IndexOperators", new[] { "gin_trgm_ops" });

            migrationBuilder.CreateIndex(
                name: "ix_backuprepositories_normalizedname",
                table: "backuprepositories",
                column: "normalizedname",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_backuprepositories_passwordsecretid",
                table: "backuprepositories",
                column: "passwordsecretid");

            migrationBuilder.CreateIndex(
                name: "ix_backuprepositories_status",
                table: "backuprepositories",
                column: "status");

            migrationBuilder.CreateIndex(
                name: "ix_backuprepositoryleases_expiresat",
                table: "backuprepositoryleases",
                column: "expiresat");

            migrationBuilder.CreateIndex(
                name: "ix_backuprepositoryvalidations_platformid",
                table: "backuprepositoryvalidations",
                column: "platformid");

            migrationBuilder.CreateIndex(
                name: "ix_backuprepositoryvalidations_repository_location_platform",
                table: "backuprepositoryvalidations",
                columns: new[] { "backuprepositoryid", "location", "platformid" });

            migrationBuilder.CreateIndex(
                name: "ix_backuprestorerunlogs_restorerun_createdat",
                table: "backuprestorerunlogs",
                columns: new[] { "backuprestorerunid", "createdat" });

            migrationBuilder.CreateIndex(
                name: "ix_backuprestoreruns_backuprun_queuedat",
                table: "backuprestoreruns",
                columns: new[] { "backuprunid", "queuedat" });

            migrationBuilder.CreateIndex(
                name: "ix_backuprestoreruns_queuedat",
                table: "backuprestoreruns",
                column: "queuedat");

            migrationBuilder.CreateIndex(
                name: "ix_backuprestoreruns_repository_status",
                table: "backuprestoreruns",
                columns: new[] { "backuprepositoryid", "status" });

            migrationBuilder.CreateIndex(
                name: "ix_backuprestoreruns_status_queuedat",
                table: "backuprestoreruns",
                columns: new[] { "status", "queuedat" });

            migrationBuilder.CreateIndex(
                name: "ix_backuprestoreruns_targetvolume",
                table: "backuprestoreruns",
                columns: new[] { "targetplatformid", "targetvolumename" });

            migrationBuilder.CreateIndex(
                name: "ix_backuprestoreruns_triggeredbyactorid",
                table: "backuprestoreruns",
                column: "triggeredbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_backuprunitems_platform_volumename",
                table: "backuprunitems",
                columns: new[] { "platformid", "volumename" });

            migrationBuilder.CreateIndex(
                name: "ix_backuprunitems_resticsnapshotid",
                table: "backuprunitems",
                column: "resticsnapshotid");

            migrationBuilder.CreateIndex(
                name: "ix_backuprunitems_run_status",
                table: "backuprunitems",
                columns: new[] { "backuprunid", "status" });

            migrationBuilder.CreateIndex(
                name: "ix_backuprunitems_run_volumename",
                table: "backuprunitems",
                columns: new[] { "backuprunid", "volumename" });

            migrationBuilder.CreateIndex(
                name: "ix_backuprunlogs_run_createdat",
                table: "backuprunlogs",
                columns: new[] { "backuprunid", "createdat" });

            migrationBuilder.CreateIndex(
                name: "ix_backupruns_active_policy",
                table: "backupruns",
                column: "backuppolicyid",
                unique: true,
                filter: "status IN ('Queued', 'Preparing', 'Running', 'ApplyingRetention')");

            migrationBuilder.CreateIndex(
                name: "ix_backupruns_policy_queuedat",
                table: "backupruns",
                columns: new[] { "backuppolicyid", "queuedat", "id" },
                descending: new[] { false, true, true })
                .Annotation("Npgsql:IndexInclude", new[] { "status" });

            migrationBuilder.CreateIndex(
                name: "ix_backupruns_queuedat",
                table: "backupruns",
                column: "queuedat");

            migrationBuilder.CreateIndex(
                name: "ix_backupruns_repository_status",
                table: "backupruns",
                columns: new[] { "backuprepositoryid", "status" });

            migrationBuilder.CreateIndex(
                name: "ix_backupruns_snapshotavailability",
                table: "backupruns",
                column: "snapshotavailability");

            migrationBuilder.CreateIndex(
                name: "ix_backupruns_status_queuedat",
                table: "backupruns",
                columns: new[] { "status", "queuedat" });

            migrationBuilder.CreateIndex(
                name: "ix_backupruns_triggeredbyactorid",
                table: "backupruns",
                column: "triggeredbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_backupsourceleases_expiresat",
                table: "backupsourceleases",
                column: "expiresat");

            migrationBuilder.CreateIndex(
                name: "ix_buildagentpools_archivedat",
                table: "buildagentpools",
                column: "archivedat");

            migrationBuilder.CreateIndex(
                name: "ix_buildagentpools_controltriggeredby",
                table: "buildagentpools",
                column: "controltriggeredby");

            migrationBuilder.CreateIndex(
                name: "ix_buildagentpools_createdbyactorid",
                table: "buildagentpools",
                column: "createdbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_buildagentpools_enabled",
                table: "buildagentpools",
                column: "enabled");

            migrationBuilder.CreateIndex(
                name: "ix_buildagentpools_globalsearch_name_trgm",
                table: "buildagentpools",
                column: "name",
                filter: "archivedat IS NULL")
                .Annotation("Npgsql:IndexMethod", "gin")
                .Annotation("Npgsql:IndexOperators", new[] { "gin_trgm_ops" });

            migrationBuilder.CreateIndex(
                name: "ix_buildagentpools_globalsearch_provider_trgm",
                table: "buildagentpools",
                column: "provider",
                filter: "archivedat IS NULL")
                .Annotation("Npgsql:IndexMethod", "gin")
                .Annotation("Npgsql:IndexOperators", new[] { "gin_trgm_ops" });

            migrationBuilder.CreateIndex(
                name: "ix_buildagentpools_normalizedname",
                table: "buildagentpools",
                column: "normalizedname",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_buildagentpools_provider",
                table: "buildagentpools",
                column: "provider");

            migrationBuilder.CreateIndex(
                name: "ix_buildprojects_archivedat",
                table: "buildprojects",
                column: "archivedat");

            migrationBuilder.CreateIndex(
                name: "ix_buildprojects_buildagentpoolid",
                table: "buildprojects",
                column: "buildagentpoolid");

            migrationBuilder.CreateIndex(
                name: "ix_buildprojects_builderkind",
                table: "buildprojects",
                column: "builderkind");

            migrationBuilder.CreateIndex(
                name: "ix_buildprojects_controlstate_controlstartedat",
                table: "buildprojects",
                columns: new[] { "controlstate", "controlstartedat" });

            migrationBuilder.CreateIndex(
                name: "ix_buildprojects_createdbyactorid",
                table: "buildprojects",
                column: "createdbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_buildprojects_gitrepositoryid",
                table: "buildprojects",
                column: "gitrepositoryid");

            migrationBuilder.CreateIndex(
                name: "ix_buildprojects_globalsearch_branch_trgm",
                table: "buildprojects",
                column: "branch",
                filter: "archivedat IS NULL")
                .Annotation("Npgsql:IndexMethod", "gin")
                .Annotation("Npgsql:IndexOperators", new[] { "gin_trgm_ops" });

            migrationBuilder.CreateIndex(
                name: "ix_buildprojects_globalsearch_name_trgm",
                table: "buildprojects",
                column: "name",
                filter: "archivedat IS NULL")
                .Annotation("Npgsql:IndexMethod", "gin")
                .Annotation("Npgsql:IndexOperators", new[] { "gin_trgm_ops" });

            migrationBuilder.CreateIndex(
                name: "ix_buildprojects_normalizedname",
                table: "buildprojects",
                column: "normalizedname",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_buildprojects_platformid",
                table: "buildprojects",
                column: "platformid");

            migrationBuilder.CreateIndex(
                name: "ix_buildprojects_registryid",
                table: "buildprojects",
                column: "registryid");

            migrationBuilder.CreateIndex(
                name: "ix_buildrunlogs_run_createdat",
                table: "buildrunlogs",
                columns: new[] { "buildrunid", "createdat" });

            migrationBuilder.CreateIndex(
                name: "ix_buildruns_active_project",
                table: "buildruns",
                column: "buildprojectid",
                unique: true,
                filter: "status IN ('Queued', 'Preparing', 'Running')");

            migrationBuilder.CreateIndex(
                name: "ix_buildruns_gitrepositoryid",
                table: "buildruns",
                column: "gitrepositoryid");

            migrationBuilder.CreateIndex(
                name: "ix_buildruns_project_queuedat",
                table: "buildruns",
                columns: new[] { "buildprojectid", "queuedat", "id" },
                descending: new[] { false, true, true })
                .Annotation("Npgsql:IndexInclude", new[] { "status" });

            migrationBuilder.CreateIndex(
                name: "ix_buildruns_queuedat",
                table: "buildruns",
                column: "queuedat");

            migrationBuilder.CreateIndex(
                name: "ix_buildruns_status_queuedat",
                table: "buildruns",
                columns: new[] { "status", "queuedat" });

            migrationBuilder.CreateIndex(
                name: "ix_buildruns_triggeredbyactorid",
                table: "buildruns",
                column: "triggeredbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_citadelinstanceidentity_instanceid",
                table: "citadelinstanceidentity",
                column: "instanceid",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix__containers_dockercontainerid_platformid",
                table: "containers",
                columns: new[] { "dockercontainerid", "platformid" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_containers_controltriggeredby",
                table: "containers",
                column: "controltriggeredby");

            migrationBuilder.CreateIndex(
                name: "ix_containers_deploymentid",
                table: "containers",
                column: "deploymentid");

            migrationBuilder.CreateIndex(
                name: "ix_containers_dockerimageid",
                table: "containers",
                column: "dockerimageid");

            migrationBuilder.CreateIndex(
                name: "ix_containers_imageid",
                table: "containers",
                column: "imageid");

            migrationBuilder.CreateIndex(
                name: "ix_containers_platformid",
                table: "containers",
                column: "platformid");

            migrationBuilder.CreateIndex(
                name: "ix_containers_stackid",
                table: "containers",
                column: "stackid");

            migrationBuilder.CreateIndex(
                name: "ix_containerstats_containerid_created",
                table: "containerstats",
                columns: new[] { "containerid", "created" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_deployments_controltriggeredby",
                table: "deployments",
                column: "controltriggeredby");

            migrationBuilder.CreateIndex(
                name: "ix_deployments_createdbyactorid",
                table: "deployments",
                column: "createdbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_deployments_globalsearch_name_trgm",
                table: "deployments",
                column: "name")
                .Annotation("Npgsql:IndexMethod", "gin")
                .Annotation("Npgsql:IndexOperators", new[] { "gin_trgm_ops" });

            migrationBuilder.CreateIndex(
                name: "ix_deployments_name_platformid",
                table: "deployments",
                columns: new[] { "name", "platformid" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_deployments_platformid",
                table: "deployments",
                column: "platformid");

            migrationBuilder.CreateIndex(
                name: "ix_edgeagentbindings_agentfingerprint",
                table: "edgeagentbindings",
                column: "agentfingerprint");

            migrationBuilder.CreateIndex(
                name: "ix_edgeagentbindings_agentid",
                table: "edgeagentbindings",
                column: "agentid");

            migrationBuilder.CreateIndex(
                name: "ix_edgeagentbindings_resource",
                table: "edgeagentbindings",
                columns: new[] { "resourcetype", "resourceid" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_edgeagentenrollments_createdbyactorid",
                table: "edgeagentenrollments",
                column: "createdbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_edgeagentenrollments_platformid_expiresatutc",
                table: "edgeagentenrollments",
                columns: new[] { "platformid", "expiresatutc" });

            migrationBuilder.CreateIndex(
                name: "ix_edgeagentenrollments_resource_expiresatutc",
                table: "edgeagentenrollments",
                columns: new[] { "resourcetype", "resourceid", "expiresatutc" });

            migrationBuilder.CreateIndex(
                name: "ix_edgeagentenrollments_tokenhash",
                table: "edgeagentenrollments",
                column: "tokenhash",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_gitaccounts_createdbyactorid",
                table: "gitaccounts",
                column: "createdbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_gitaccounts_name",
                table: "gitaccounts",
                column: "name",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_gitrepositories_controltriggeredby",
                table: "gitrepositories",
                column: "controltriggeredby");

            migrationBuilder.CreateIndex(
                name: "ix_gitrepositories_createdbyactorid",
                table: "gitrepositories",
                column: "createdbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_gitrepositories_gitaccountid",
                table: "gitrepositories",
                column: "gitaccountid");

            migrationBuilder.CreateIndex(
                name: "ix_gitrepositories_globalsearch_name_trgm",
                table: "gitrepositories",
                column: "name")
                .Annotation("Npgsql:IndexMethod", "gin")
                .Annotation("Npgsql:IndexOperators", new[] { "gin_trgm_ops" });

            migrationBuilder.CreateIndex(
                name: "ix_gitrepositories_globalsearch_url_trgm",
                table: "gitrepositories",
                column: "url")
                .Annotation("Npgsql:IndexMethod", "gin")
                .Annotation("Npgsql:IndexOperators", new[] { "gin_trgm_ops" });

            migrationBuilder.CreateIndex(
                name: "ix_gitrepositories_name",
                table: "gitrepositories",
                column: "name",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_gitrepositoryrefs_gitrepositoryid_branch",
                table: "gitrepositoryrefs",
                columns: new[] { "gitrepositoryid", "branch" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_images_controltriggeredby",
                table: "images",
                column: "controltriggeredby");

            migrationBuilder.CreateIndex(
                name: "ix_images_dockerimageid_platformid",
                table: "images",
                columns: new[] { "dockerimageid", "platformid" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_images_platformid",
                table: "images",
                column: "platformid");

            migrationBuilder.CreateIndex(
                name: "ix_images_registryid",
                table: "images",
                column: "registryid");

            migrationBuilder.CreateIndex(
                name: "ix_installedlicenses_installedbyactorid",
                table: "installedlicenses",
                column: "installedbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_mfachallenges_expiresat",
                table: "mfachallenges",
                column: "expiresat");

            migrationBuilder.CreateIndex(
                name: "ix_mfachallenges_userid",
                table: "mfachallenges",
                column: "userid");

            migrationBuilder.CreateIndex(
                name: "ix_mfasetupsessions_expiresat",
                table: "mfasetupsessions",
                column: "expiresat");

            migrationBuilder.CreateIndex(
                name: "ix_mfasetupsessions_userid",
                table: "mfasetupsessions",
                column: "userid");

            migrationBuilder.CreateIndex(
                name: "ix_oidcexternallogins_providerid_subject",
                table: "oidcexternallogins",
                columns: new[] { "providerid", "subject" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_oidcexternallogins_userid",
                table: "oidcexternallogins",
                column: "userid");

            migrationBuilder.CreateIndex(
                name: "ix_oidcloginstates_expiresat",
                table: "oidcloginstates",
                column: "expiresat");

            migrationBuilder.CreateIndex(
                name: "ix_oidcloginstates_providerid",
                table: "oidcloginstates",
                column: "providerid");

            migrationBuilder.CreateIndex(
                name: "ix_oidcloginstates_statehash",
                table: "oidcloginstates",
                column: "statehash",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_oidcproviders_createdbyactorid",
                table: "oidcproviders",
                column: "createdbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_oidcproviders_defaultroleid",
                table: "oidcproviders",
                column: "defaultroleid");

            migrationBuilder.CreateIndex(
                name: "ix_oidcproviders_enabled",
                table: "oidcproviders",
                column: "enabled");

            migrationBuilder.CreateIndex(
                name: "ix_oidcproviders_name",
                table: "oidcproviders",
                column: "name",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_permissions_roleid",
                table: "permissions",
                column: "roleid");

            migrationBuilder.CreateIndex(
                name: "ix_permissions_roleid_resourcetype",
                table: "permissions",
                columns: new[] { "roleid", "resourcetype" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_platforms_address",
                table: "platforms",
                column: "address",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_platforms_globalsearch_address_trgm",
                table: "platforms",
                column: "address")
                .Annotation("Npgsql:IndexMethod", "gin")
                .Annotation("Npgsql:IndexOperators", new[] { "gin_trgm_ops" });

            migrationBuilder.CreateIndex(
                name: "ix_platforms_globalsearch_name_trgm",
                table: "platforms",
                column: "name")
                .Annotation("Npgsql:IndexMethod", "gin")
                .Annotation("Npgsql:IndexOperators", new[] { "gin_trgm_ops" });

            migrationBuilder.CreateIndex(
                name: "ix_platformstats_platformid_created",
                table: "platformstats",
                columns: new[] { "platformid", "created" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_refreshtokens_expiresat",
                table: "refreshtokens",
                column: "expiresat");

            migrationBuilder.CreateIndex(
                name: "ix_refreshtokens_userid",
                table: "refreshtokens",
                column: "userid");

            migrationBuilder.CreateIndex(
                name: "ix_registries_createdbyactorid",
                table: "registries",
                column: "createdbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_registries_globalsearch_name_trgm",
                table: "registries",
                column: "name")
                .Annotation("Npgsql:IndexMethod", "gin")
                .Annotation("Npgsql:IndexOperators", new[] { "gin_trgm_ops" });

            migrationBuilder.CreateIndex(
                name: "ix_registries_globalsearch_registryhost_trgm",
                table: "registries",
                column: "registryhost")
                .Annotation("Npgsql:IndexMethod", "gin")
                .Annotation("Npgsql:IndexOperators", new[] { "gin_trgm_ops" });

            migrationBuilder.CreateIndex(
                name: "ix_registries_name",
                table: "registries",
                column: "name",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_resourceaccesses_actor_resourcetype",
                table: "resourceaccesses",
                columns: new[] { "actorid", "resourcetype" });

            migrationBuilder.CreateIndex(
                name: "ix_resourceaccesses_permissionlookup",
                table: "resourceaccesses",
                columns: new[] { "resourcetype", "resourceid", "actorid", "permissionlevel" });

            migrationBuilder.CreateIndex(
                name: "ix_resourceaccesses_resourcetype_resourceid_actorid",
                table: "resourceaccesses",
                columns: new[] { "resourcetype", "resourceid", "actorid" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_resourcebindings_scope_name_global",
                table: "resourcebindings",
                columns: new[] { "scope", "name" },
                unique: true,
                filter: "resourceid IS NULL");

            migrationBuilder.CreateIndex(
                name: "ix_resourcebindings_scope_resourceid_name",
                table: "resourcebindings",
                columns: new[] { "scope", "resourceid", "name" },
                unique: true,
                filter: "resourceid IS NOT NULL");

            migrationBuilder.CreateIndex(
                name: "ix_resourcebindings_secretid",
                table: "resourcebindings",
                column: "secretid");

            migrationBuilder.CreateIndex(
                name: "ix_resourcetags_createdbyactorid",
                table: "resourcetags",
                column: "createdbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_resourcetags_filter",
                table: "resourcetags",
                columns: new[] { "resourcetype", "tagid", "resourceid" });

            migrationBuilder.CreateIndex(
                name: "ix_resourcetags_resource",
                table: "resourcetags",
                columns: new[] { "resourcetype", "resourceid" });

            migrationBuilder.CreateIndex(
                name: "ix_resourcetags_tagid",
                table: "resourcetags",
                column: "tagid");

            migrationBuilder.CreateIndex(
                name: "ix_secretdefinitions_name",
                table: "secretdefinitions",
                column: "name");

            migrationBuilder.CreateIndex(
                name: "ix_secretdefinitions_providerid",
                table: "secretdefinitions",
                column: "providerid");

            migrationBuilder.CreateIndex(
                name: "ix_secretproviders_name",
                table: "secretproviders",
                column: "name",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_stackreleases_createdbyactorid",
                table: "stackreleases",
                column: "createdbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_stackreleases_platformid",
                table: "stackreleases",
                column: "platformid");

            migrationBuilder.CreateIndex(
                name: "ix_stackreleases_stackid",
                table: "stackreleases",
                column: "stackid");

            migrationBuilder.CreateIndex(
                name: "ix_stackreleasevolumebindings_platform_volumename",
                table: "stackreleasevolumebindings",
                columns: new[] { "platformid", "volumename" });

            migrationBuilder.CreateIndex(
                name: "ix_stackreleasevolumebindings_release_volumename",
                table: "stackreleasevolumebindings",
                columns: new[] { "stackreleaseid", "volumename" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_stacks_controltriggeredby",
                table: "stacks",
                column: "controltriggeredby");

            migrationBuilder.CreateIndex(
                name: "ix_stacks_createdbyactorid",
                table: "stacks",
                column: "createdbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_stacks_currentstackreleaseid",
                table: "stacks",
                column: "currentstackreleaseid");

            migrationBuilder.CreateIndex(
                name: "ix_stacks_globalsearch_name_trgm",
                table: "stacks",
                column: "name")
                .Annotation("Npgsql:IndexMethod", "gin")
                .Annotation("Npgsql:IndexOperators", new[] { "gin_trgm_ops" });

            migrationBuilder.CreateIndex(
                name: "ix_tags_createdbyactorid",
                table: "tags",
                column: "createdbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_tags_normalizedname",
                table: "tags",
                column: "normalizedname",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_teams_actorid",
                table: "teams",
                column: "actorid",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_usermfarecoverycodes_userid",
                table: "usermfarecoverycodes",
                column: "userid");

            migrationBuilder.CreateIndex(
                name: "ix_usermfarecoverycodes_userid_codehash",
                table: "usermfarecoverycodes",
                columns: new[] { "userid", "codehash" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_users_actorid",
                table: "users",
                column: "actorid",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_users_createdbyactorid",
                table: "users",
                column: "createdbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_users_email",
                table: "users",
                column: "email",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_usersteams_teamid",
                table: "usersteams",
                column: "teamid");

            migrationBuilder.CreateIndex(
                name: "ix_usersteams_userid",
                table: "usersteams",
                column: "userid");
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropTable(
                name: "actionruns");

            migrationBuilder.DropTable(
                name: "activityevents");

            migrationBuilder.DropTable(
                name: "actorroles");

            migrationBuilder.DropTable(
                name: "alertevents");

            migrationBuilder.DropTable(
                name: "alertrulechannels");

            migrationBuilder.DropTable(
                name: "alertrulestates");

            migrationBuilder.DropTable(
                name: "backuprepositoryleases");

            migrationBuilder.DropTable(
                name: "backuprepositoryvalidations");

            migrationBuilder.DropTable(
                name: "backuprestorerunlogs");

            migrationBuilder.DropTable(
                name: "backuprunitems");

            migrationBuilder.DropTable(
                name: "backuprunlogs");

            migrationBuilder.DropTable(
                name: "backupsourceleases");

            migrationBuilder.DropTable(
                name: "buildrunlogs");

            migrationBuilder.DropTable(
                name: "citadelinstanceidentity");

            migrationBuilder.DropTable(
                name: "containerstats");

            migrationBuilder.DropTable(
                name: "edgeagentbindings");

            migrationBuilder.DropTable(
                name: "edgeagentenrollments");

            migrationBuilder.DropTable(
                name: "gitrepositoryrefs");

            migrationBuilder.DropTable(
                name: "installedlicenses");

            migrationBuilder.DropTable(
                name: "internalsecretvalues");

            migrationBuilder.DropTable(
                name: "mfachallenges");

            migrationBuilder.DropTable(
                name: "mfasetupsessions");

            migrationBuilder.DropTable(
                name: "oidcexternallogins");

            migrationBuilder.DropTable(
                name: "oidcloginstates");

            migrationBuilder.DropTable(
                name: "permissions");

            migrationBuilder.DropTable(
                name: "platformstats");

            migrationBuilder.DropTable(
                name: "refreshtokens");

            migrationBuilder.DropTable(
                name: "resourceaccesses");

            migrationBuilder.DropTable(
                name: "resourcebindings");

            migrationBuilder.DropTable(
                name: "resourcetags");

            migrationBuilder.DropTable(
                name: "stackreleasevolumebindings");

            migrationBuilder.DropTable(
                name: "usermfarecoverycodes");

            migrationBuilder.DropTable(
                name: "usermfasettings");

            migrationBuilder.DropTable(
                name: "userpreferences");

            migrationBuilder.DropTable(
                name: "usersteams");

            migrationBuilder.DropTable(
                name: "actions");

            migrationBuilder.DropTable(
                name: "alertchannels");

            migrationBuilder.DropTable(
                name: "alertrules");

            migrationBuilder.DropTable(
                name: "backuprestoreruns");

            migrationBuilder.DropTable(
                name: "buildruns");

            migrationBuilder.DropTable(
                name: "containers");

            migrationBuilder.DropTable(
                name: "oidcproviders");

            migrationBuilder.DropTable(
                name: "tags");

            migrationBuilder.DropTable(
                name: "stackreleases");

            migrationBuilder.DropTable(
                name: "teams");

            migrationBuilder.DropTable(
                name: "users");

            migrationBuilder.DropTable(
                name: "backupruns");

            migrationBuilder.DropTable(
                name: "buildprojects");

            migrationBuilder.DropTable(
                name: "deployments");

            migrationBuilder.DropTable(
                name: "images");

            migrationBuilder.DropTable(
                name: "roles");

            migrationBuilder.DropTable(
                name: "stacks");

            migrationBuilder.DropTable(
                name: "backuppolicies");

            migrationBuilder.DropTable(
                name: "buildagentpools");

            migrationBuilder.DropTable(
                name: "gitrepositories");

            migrationBuilder.DropTable(
                name: "platforms");

            migrationBuilder.DropTable(
                name: "registries");

            migrationBuilder.DropTable(
                name: "backuprepositories");

            migrationBuilder.DropTable(
                name: "gitaccounts");

            migrationBuilder.DropTable(
                name: "secretdefinitions");

            migrationBuilder.DropTable(
                name: "actors");

            migrationBuilder.DropTable(
                name: "secretproviders");
        }
    }
}
