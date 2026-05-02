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
                name: "platforms",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    address = table.Column<string>(type: "text", nullable: false),
                    agentversion = table.Column<string>(type: "text", nullable: true),
                    connectortype = table.Column<string>(type: "text", nullable: false),
                    cpucount = table.Column<int>(type: "integer", nullable: false),
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
                    action = table.Column<string>(type: "text", nullable: false),
                    actorid = table.Column<Guid>(type: "uuid", nullable: false),
                    resourceid = table.Column<Guid>(type: "uuid", nullable: false),
                    resourcetype = table.Column<string>(type: "text", nullable: false)
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
                        onDelete: ReferentialAction.Cascade);
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
                    spec = table.Column<string>(type: "json", nullable: false),
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
                name: "permissions",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    resourceaction = table.Column<string>(type: "text", nullable: false),
                    resourcetype = table.Column<string>(type: "text", nullable: false),
                    roleid = table.Column<Guid>(type: "uuid", nullable: false)
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
                    url = table.Column<string>(type: "text", nullable: false),
                    webhookenabled = table.Column<bool>(type: "boolean", nullable: false, defaultValue: false),
                    webhooksecret = table.Column<string>(type: "text", nullable: true)
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
                    spec = table.Column<string>(type: "json", nullable: false),
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
                name: "refreshtokens",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
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
                columns: new[] { "id", "name" },
                values: new object[,]
                {
                    { new Guid("30000000-0000-0000-0000-000000000001"), "Admin" },
                    { new Guid("30000000-0000-0000-0000-000000000002"), "Operator" },
                    { new Guid("30000000-0000-0000-0000-000000000003"), "Viewer" }
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
                    { new Guid("019d0000-0001-7000-8001-000000000011"), 300, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "CPU > 80% - Platform", "[]", 3, "Warning", 80.0, "PlatformCpuHigh" },
                    { new Guid("019d0000-0001-7000-8001-000000000022"), 300, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "RAM > 80% - Platform", "[]", 3, "Warning", 80.0, "PlatformRamHigh" }
                });

            migrationBuilder.InsertData(
                table: "permissions",
                columns: new[] { "id", "resourceaction", "resourcetype", "roleid" },
                values: new object[,]
                {
                    { new Guid("017b5a65-e312-8a67-b7fd-6124f6ddeeee"), "View", "GitRepository", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("0292fdeb-9a33-5a4c-60e9-395eac821cdc"), "Delete", "User", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("0309bcb2-05ec-623d-e45b-ec10cfddee24"), "Create", "Role", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("0430e179-4cf1-193b-c54d-5d014203921b"), "Exec", "Team", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("052e45fb-cb15-9380-6a33-c233fde703a6"), "Update", "GitAccount", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("0854c122-21bc-147b-506b-6caf72ac48ca"), "View", "Team", new Guid("30000000-0000-0000-0000-000000000003") },
                    { new Guid("0dd6ded1-5ef7-c1b5-a36a-d0de73459d8a"), "Apply", "Registry", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("0eda225f-cb5b-1bb0-9525-be92b14fc322"), "Apply", "GitRepository", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("0edd69d5-bb37-653c-9b25-5ff32a2b8243"), "Apply", "Role", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("0f030749-53d1-bade-8ef3-30112991786d"), "Create", "Stack", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("11689210-d56f-4df8-9887-3338512e781d"), "Log", "Team", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("117176b6-ca23-e53d-d996-83affab7ed48"), "View", "Stack", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("12bbcf07-7237-3afb-65f7-1fc8e2de4939"), "Apply", "Registry", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("153c4670-ec3e-4037-6fd6-dd83bf29d3af"), "Log", "Role", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("15923c89-875e-7d0b-b80b-96af1f0cd1f2"), "Apply", "Role", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("17aefc62-767d-6e40-0a30-82d8cf360dec"), "Exec", "Alert", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("18255099-e963-802b-21a3-d115440e9322"), "Pull", "Deployment", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("18b8b740-528c-6366-9902-ebf6a025d063"), "Apply", "Alert", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("19357866-b0f7-4c0b-fb01-c3a6556d1e5f"), "Apply", "AlertChannel", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("1a0b6504-d508-0f6d-4513-12b80c3ab4d8"), "Update", "Platform", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("1a84bf5e-dcf2-8a0b-8c4f-066f98ed2498"), "Exec", "Platform", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("1f00afd4-a4d1-94cd-3a94-44d420eef066"), "Apply", "GitRepository", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("21d8c7f7-5480-5509-e2ab-e3c8fdbb5ab8"), "Update", "AlertChannel", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("2548763c-c9b7-5359-a80a-5ce706c5c42c"), "Update", "Alert", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("258c5870-adb0-f28f-bed2-5c993e5d11da"), "Pull", "GitRepository", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("26bb8bc9-e526-dfd5-c3cc-2ebc0fb9837b"), "Apply", "Alert", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("2941a68e-0eb8-2ba5-0d80-8ecf0dd21df8"), "Log", "GitAccount", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("29b878e1-64b3-eee4-d8d9-7a7fb015c14c"), "Log", "Deployment", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("2ab50947-b044-4b44-6683-6fa76d3dd8e5"), "Exec", "GitAccount", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("2e0582c8-9569-22cd-c777-69f03893b8ec"), "Delete", "GitRepository", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("30f18293-2d41-2525-3210-e0f83bafa13d"), "Update", "Stack", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("36667c46-31d9-3c62-1375-459c4daba3ed"), "Log", "Stack", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("37023893-2218-6946-8caa-bbc8a7ad77a1"), "Log", "Platform", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("3a14f868-33fb-3a2e-92e1-5579da6962ce"), "Pull", "Role", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("3a8c08b1-d033-1580-65f9-a1cb3ed3fc6a"), "Create", "Registry", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("3b12173b-62c2-740b-72bd-9727e893e58b"), "Exec", "Team", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("3c380043-2b56-a61a-6855-8a8d1a50d9f1"), "Log", "Alert", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("3d84b4f0-2433-c34e-45e1-84d18b6c155d"), "Pull", "Platform", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("3e1abbe9-b2f2-21b8-bf02-38d4c10cd79d"), "Create", "Role", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("3e5365c6-7759-a0ad-e7fa-32263d00682c"), "View", "GitAccount", new Guid("30000000-0000-0000-0000-000000000003") },
                    { new Guid("40aadb71-124b-7c1b-44b9-f507a69ade11"), "Pull", "Stack", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("41da5515-1e2d-bb3a-dd26-f13eb17fb81d"), "Exec", "Alert", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("4711987b-af34-12f7-4cf4-795f51049571"), "Pull", "Platform", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("47a25f07-9bb1-361d-788e-4d99fd0e50ee"), "Update", "User", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("47c763f4-71e9-2992-3223-8ab97876b727"), "Apply", "Platform", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("49ce8531-88f1-5ed2-a1c7-95d40cc72c47"), "Create", "User", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("4aedeb0a-de43-b841-5970-9743bcde952b"), "Create", "Stack", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("4b68a9cf-0af5-b0d6-8c05-e8b7e98b3919"), "View", "Stack", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("4cfe0dcb-ce92-0500-981f-7d79b3782877"), "Create", "Alert", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("4fa196f3-a9e5-7716-b1dd-aa574061e1f7"), "Pull", "GitAccount", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("510688a3-f3e5-851a-29fb-8c8ae66a06d5"), "View", "User", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("529f7f68-9ab0-a2f2-15af-a284b4d71ab6"), "Exec", "AlertChannel", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("55228106-7ae9-6748-5a33-73e253ad940d"), "Apply", "AlertChannel", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("57de5bf0-3ba6-3067-d4d0-c8c6a889507b"), "View", "Role", new Guid("30000000-0000-0000-0000-000000000003") },
                    { new Guid("58e018b1-ba4c-56bb-c56c-b9473688127b"), "Pull", "GitRepository", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("5b860b42-6a54-df7f-b375-2e1d1f47a563"), "Log", "Registry", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("5cd694be-92f2-0857-3c43-3fde801e6173"), "Log", "AlertChannel", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("5e169a67-b789-1d24-db33-ea48a69f362e"), "Create", "Team", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("5e8afc50-270c-4413-c5f1-bd25dac435d9"), "Apply", "Stack", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("5ede29a8-8e33-2d7c-48c3-1e5d733edd73"), "View", "AlertChannel", new Guid("30000000-0000-0000-0000-000000000003") },
                    { new Guid("5ffa0070-42f0-4b37-efe5-4b0a394a1782"), "Log", "Alert", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("613e9000-da2c-b4e7-9e02-b4bef349f0f7"), "Pull", "AlertChannel", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("62d97329-3b51-37c0-abe7-aba92734e97e"), "Pull", "Team", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("62e8e1fe-c911-e1b9-cc70-7efff6e08327"), "Pull", "Deployment", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("6395043d-510b-dc85-f19d-2a57463f4e8f"), "Pull", "GitAccount", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("65540197-9169-5e94-9ad1-112ffe006a6d"), "Exec", "AlertChannel", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("65cb87df-133d-b577-21f6-2f20190ce45f"), "View", "Registry", new Guid("30000000-0000-0000-0000-000000000003") },
                    { new Guid("6deaa9b4-66e6-22bb-3f49-62584ccd9e1f"), "View", "Platform", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("6e332b06-35e2-fbbd-e12c-bb7f02ab2474"), "Create", "Alert", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("6ed1c4e3-9d28-23d8-2939-6a414aa0f53d"), "Apply", "Platform", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("6f3a7126-e96a-d249-5e58-108bd0bd1f0f"), "View", "AlertChannel", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("7238045c-c070-0ba5-b133-6083ea208d1b"), "Apply", "Stack", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("723bb5cb-0c68-80e7-d890-7c4f6e3ce23a"), "Delete", "Team", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("73174250-b459-3def-d4e6-82d09d07ece9"), "Update", "GitRepository", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("75575fd4-2d45-4302-12ba-1ebf9e9ea17f"), "Create", "GitRepository", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("780d5066-5b19-668e-9f2f-0103f6cb23be"), "View", "Deployment", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("783ca30a-d153-8f1b-27db-c3e722f7f34d"), "Apply", "GitAccount", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("7862a71c-6493-24a1-84ca-dd2a630cc55a"), "Log", "Role", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("7ad3c461-3f87-fe3c-a1e7-4a490906600e"), "Delete", "Platform", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("7ba78b50-a388-1606-e334-30c69758e60f"), "View", "Deployment", new Guid("30000000-0000-0000-0000-000000000003") },
                    { new Guid("7ccb3b9e-a09a-d3c9-82ed-3f71646d2576"), "View", "GitAccount", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("7d64c2fa-810b-9866-5d31-be639383d279"), "Exec", "Deployment", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("7defaa75-0f73-24b3-e32e-842bc1ec9885"), "Log", "Registry", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("7fddc3c2-6d6e-cb92-a7a3-59a7a18b7c08"), "View", "GitRepository", new Guid("30000000-0000-0000-0000-000000000003") },
                    { new Guid("83ac5b37-8bd4-e093-3cdd-7e9f61300108"), "View", "Alert", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("8469f325-f132-73f4-0fa4-42131875a5ed"), "Update", "Role", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("8487254d-0383-5b91-fa5f-816cfdc29054"), "View", "Team", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("8658afec-8be0-2f4b-7b1e-478ac44341ec"), "Create", "Registry", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("8722b0da-7d07-7c14-0f9c-161e0c39a751"), "Pull", "User", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("87d03608-e55e-6aed-7bb4-2a505eea1474"), "Exec", "User", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("88a68bf5-8366-4d1a-f0fe-48225bc865bd"), "Exec", "GitRepository", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("88dc9733-349d-635c-7ef6-829065f4f87b"), "Create", "AlertChannel", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("8a0a4849-d9de-a63a-4e34-d25e431f6324"), "Log", "GitRepository", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("8b1633bc-d6ba-a419-38ea-e4d7e4b48cbe"), "Create", "Deployment", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("8ce09606-e435-8a9b-2dab-8d1dfc91a198"), "Delete", "Alert", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("8de4cd72-b2ce-4e18-f148-b93892845825"), "View", "Alert", new Guid("30000000-0000-0000-0000-000000000003") },
                    { new Guid("92694036-978b-d38d-81ed-8d28aeed9bd2"), "Log", "User", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("92d404fe-a143-fba2-03cf-16479135fa81"), "Exec", "User", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("9308eb9b-7faa-e3d3-ffa8-86fc7946fae0"), "View", "Team", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("9365df99-cab8-01da-f3c7-dc17a54e8801"), "Delete", "GitAccount", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("961c1641-93aa-54ca-9b00-6608da3ae4c8"), "Pull", "Registry", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("969a37a2-af1e-fa24-35b8-4857f802e001"), "Apply", "Deployment", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("9982a665-f493-ce5f-9fd7-cd355f1ce257"), "Exec", "Platform", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("99a0ba24-900d-448e-70f0-6e5eda10f8fc"), "Apply", "Deployment", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("99b23eff-a5c8-0cbe-6383-b99e43a43b23"), "Create", "AlertChannel", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("9a7b1e84-3fd5-f750-2948-4a824aa66c82"), "Exec", "Role", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("9aa863b1-84ad-e6c5-738f-41425290cbb8"), "Create", "User", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("9d3d40da-3f88-d22a-e596-adcb5e101a71"), "Log", "Stack", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("9e087c4f-e933-37c9-3941-b1803f83ede3"), "Create", "Platform", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("9f3d1be4-d19e-9453-86aa-2ae06eae6d18"), "Delete", "AlertChannel", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("9fbad9ed-4795-61d8-e05b-748d2ee8aa30"), "Exec", "Deployment", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("9fd296b8-cb43-5a62-9672-cf562aa6efd1"), "Update", "GitRepository", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("9feb50a6-6f53-b270-5e7c-f679e7d85ed5"), "Update", "Role", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("a178007d-0c14-258e-bb6a-8828a5c28db7"), "Update", "Deployment", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("a1a791a5-9c37-88ad-c0e2-9a6717191298"), "Update", "Stack", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("a7ac62e2-2a4d-50c6-6700-af7a5a345bf7"), "Update", "Team", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("a826cc2c-86ce-d61e-def2-e1ffa9bd5e89"), "Log", "GitAccount", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("a9d333db-b006-8e96-192d-2c8444e2837d"), "Log", "Platform", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("ab32d40c-3859-6377-1634-a84b67e820dc"), "Apply", "Team", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("ab3dfa51-423f-716d-a623-760c9f72f791"), "Pull", "Role", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("acab0152-cf67-d16c-fe1e-c579972ad2df"), "Pull", "Registry", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("acece9ff-20d1-f3d5-c304-3a07adb9a03b"), "Update", "Alert", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("b19a2bbe-e59f-b092-4f40-20282533b1db"), "Exec", "GitAccount", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("b4101c57-b3f6-724d-8f9e-20b96ff470d3"), "Exec", "Stack", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("b503cc9e-5d58-8682-b509-5b5275d9ae5f"), "Exec", "Role", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("b60ccb85-3aaa-0077-b0e8-7adbb9f4a596"), "View", "User", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("b678aa42-8c01-b706-1332-b85adb4e3096"), "Delete", "Registry", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("b70ec2d0-198c-893d-9b83-f51cd8d4e5fd"), "Exec", "Registry", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("bc2f22f1-0c5b-29f7-00e6-61f5fed77470"), "Log", "GitRepository", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("bf01fa5c-a0a9-749e-b6c9-2d04af953b2b"), "Create", "GitRepository", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("c1184544-a092-3f80-c4d6-6f778db56b26"), "Update", "GitAccount", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("c12c9075-9343-73ec-322a-cc41a23230db"), "Delete", "Deployment", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("c3e7230a-af2b-ba29-57ae-3c4043b66d61"), "View", "Deployment", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("c4d01170-4f17-919d-9e91-210805644c7e"), "Log", "Team", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("c71643e0-9549-edeb-c7ff-496efa58260c"), "Create", "Platform", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("c717528a-5a83-69a2-6893-4ab5fbc14d1b"), "View", "Platform", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("c75f0cb6-117e-8929-d133-c45f363c1610"), "Delete", "Role", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("c9ef58ef-d39e-eec5-7e23-dba79f2a1823"), "Log", "User", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("cbcc1ffb-e622-9159-df1e-d0d05e50385c"), "Pull", "Team", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("cdd4e750-840f-2a08-7a9a-3bd65a4360e9"), "Delete", "Stack", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("ce566660-f041-32b6-0942-2f8abb92b17d"), "View", "Registry", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("d23893f2-7f44-a593-83de-22ca4a8c80a1"), "Update", "Registry", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("d41f9d7b-5371-c322-3049-cb166f19e83c"), "Exec", "Stack", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("d5ba4edc-5278-a21e-613d-91350e52bce7"), "Apply", "GitAccount", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("d868c269-b60f-2be0-2451-9b81b3c95674"), "View", "AlertChannel", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("d990b800-123d-a0ec-9b6a-239915235880"), "Create", "GitAccount", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("ddcb3cb1-e44f-0ab8-9e1e-698ed0352dc6"), "Apply", "User", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("deb33289-b4e7-0111-9e93-079c08f09cf3"), "Create", "Team", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("df77eb4e-7860-5319-431e-481bfe08baeb"), "Pull", "AlertChannel", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("e0494cd9-b3ec-b088-0532-089d029accca"), "Update", "Team", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("e08e5c0a-2e94-f112-22dd-e06d44dd2d9b"), "View", "Stack", new Guid("30000000-0000-0000-0000-000000000003") },
                    { new Guid("e0e6d40a-91ae-d947-56a3-6e71df38581a"), "Update", "User", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("e13e141d-8322-f5f5-0488-cf961e919143"), "Update", "Deployment", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("e3b44e1b-f776-b5ce-509d-2b529768a528"), "View", "Registry", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("e6a62042-68c0-d60f-2888-f685176a8f4f"), "Create", "Deployment", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("e92ef59e-f1ab-51fc-abc8-4d90c98e5bf9"), "Update", "Platform", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("ea5f48d2-2719-0a78-dcb4-2efd447e674f"), "Pull", "Alert", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("eaf65f71-81b1-b50f-041e-6f1471125bea"), "Exec", "GitRepository", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("ec427e29-a8ed-8604-59cc-7eda3268fc30"), "View", "Alert", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("ee63f887-67f1-a9f7-8028-31394316e680"), "Log", "AlertChannel", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("eef93be9-d327-6cfe-3bc9-5c5290f4b686"), "Apply", "User", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("ef3b1688-1b7b-f6c2-c4a2-169538d71970"), "Exec", "Registry", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("f12e902b-29c0-d404-41ef-6c7731211a70"), "Pull", "Stack", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("f1782e79-808e-d628-a83a-10b4639b9a68"), "View", "GitRepository", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("f30ff44f-86d5-8215-2c0f-60ed6ebd6234"), "Update", "Registry", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("f3e95d8a-2b63-3cb3-51ca-c5329b5b823e"), "Log", "Deployment", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("f43aa780-94d7-54b7-ceef-0cee3a2922df"), "View", "Role", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("f4595acc-f991-34d8-834c-4a1136010e17"), "View", "Platform", new Guid("30000000-0000-0000-0000-000000000003") },
                    { new Guid("f482aa00-8a5a-30da-4d1b-f0dfe770bcb3"), "Pull", "User", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("f7be80a4-dffa-1049-994a-5314ee03efaf"), "Create", "GitAccount", new Guid("30000000-0000-0000-0000-000000000001") },
                    { new Guid("f8833b18-d702-70b1-f75a-33f732e5ac29"), "Apply", "Team", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("f893a35b-7eac-bd31-921e-ec48bd5335e8"), "View", "Role", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("f9af8f42-cf9c-21a8-43ba-793b4dd1bd3f"), "Pull", "Alert", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("fc6bc1bc-bb69-098b-b09e-07a96444f57e"), "Update", "AlertChannel", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("ff6e9bea-dbf2-e811-d4ba-6702a55c3f92"), "View", "GitAccount", new Guid("30000000-0000-0000-0000-000000000002") },
                    { new Guid("ffc7419f-9c54-80fa-cac0-9e52ebeda6d3"), "View", "User", new Guid("30000000-0000-0000-0000-000000000003") }
                });

            migrationBuilder.InsertData(
                table: "registries",
                columns: new[] { "id", "configuration", "createdat", "createdbyactorid", "description", "name", "registryhost", "status" },
                values: new object[] { new Guid("00000000-0000-0000-0000-000000000100"), "{\r\n    \"$type\": \"DockerHub\"\r\n}", new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), "Public Docker Hub Registry", "Docker Hub", "hub.docker.com", "Active" });

            migrationBuilder.InsertData(
                table: "teams",
                columns: new[] { "id", "actorid", "name" },
                values: new object[] { new Guid("20000000-0000-0000-0000-000000000001"), new Guid("00000000-0000-0000-0000-000000000003"), "Default Team" });

            migrationBuilder.InsertData(
                table: "users",
                columns: new[] { "id", "actorid", "createdat", "createdbyactorid", "email", "name", "password" },
                values: new object[] { new Guid("10000000-0000-0000-0000-000000000001"), new Guid("00000000-0000-0000-0000-000000000002"), new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), "admin@citadel.local", "admin", "o6hWzZ+DIuSZoHNjf5D1t6101vfm4w2kmPRiAZ3Xq53JMMl1" });

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
                name: "ix_deployments_name_platformid",
                table: "deployments",
                columns: new[] { "name", "platformid" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_deployments_platformid",
                table: "deployments",
                column: "platformid");

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
                name: "ix_gitrepositories_name",
                table: "gitrepositories",
                column: "name",
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
                name: "ix_permissions_roleid",
                table: "permissions",
                column: "roleid");

            migrationBuilder.CreateIndex(
                name: "ix_platforms_address",
                table: "platforms",
                column: "address",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_platformstats_platformid_created",
                table: "platformstats",
                columns: new[] { "platformid", "created" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_refreshtokens_userid",
                table: "refreshtokens",
                column: "userid");

            migrationBuilder.CreateIndex(
                name: "ix_registries_createdbyactorid",
                table: "registries",
                column: "createdbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_registries_name",
                table: "registries",
                column: "name",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_resourceaccesses_actor",
                table: "resourceaccesses",
                column: "actorid");

            migrationBuilder.CreateIndex(
                name: "ix_resourceaccesses_resourcetype_resourceid_actorid",
                table: "resourceaccesses",
                columns: new[] { "resourcetype", "resourceid", "actorid" });

            migrationBuilder.CreateIndex(
                name: "ix_resourceaccesses_resourcetype_resourceid_actorid_action",
                table: "resourceaccesses",
                columns: new[] { "resourcetype", "resourceid", "actorid", "action" },
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
                name: "ix_teams_actorid",
                table: "teams",
                column: "actorid",
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
                name: "containerstats");

            migrationBuilder.DropTable(
                name: "gitrepositories");

            migrationBuilder.DropTable(
                name: "permissions");

            migrationBuilder.DropTable(
                name: "platformstats");

            migrationBuilder.DropTable(
                name: "refreshtokens");

            migrationBuilder.DropTable(
                name: "resourceaccesses");

            migrationBuilder.DropTable(
                name: "stackreleases");

            migrationBuilder.DropTable(
                name: "usersteams");

            migrationBuilder.DropTable(
                name: "alertchannels");

            migrationBuilder.DropTable(
                name: "alertrules");

            migrationBuilder.DropTable(
                name: "containers");

            migrationBuilder.DropTable(
                name: "gitaccounts");

            migrationBuilder.DropTable(
                name: "roles");

            migrationBuilder.DropTable(
                name: "stacks");

            migrationBuilder.DropTable(
                name: "teams");

            migrationBuilder.DropTable(
                name: "users");

            migrationBuilder.DropTable(
                name: "deployments");

            migrationBuilder.DropTable(
                name: "images");

            migrationBuilder.DropTable(
                name: "platforms");

            migrationBuilder.DropTable(
                name: "registries");

            migrationBuilder.DropTable(
                name: "actors");
        }
    }
}
