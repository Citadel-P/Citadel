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
                columns: new[] { "id", "name", "roletype" },
                values: new object[,]
                {
                    { new Guid("30000000-0000-0000-0000-000000000001"), "Admin", "System" },
                    { new Guid("30000000-0000-0000-0000-000000000002"), "Operator", "System" },
                    { new Guid("30000000-0000-0000-0000-000000000003"), "Viewer", "System" }
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
                    { new Guid("019d0000-0001-7000-8001-000000000011"), 300, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "CPU > 80% - Platform", "[]", 3, "Warning", 80.0, "PlatformCpuHigh" },
                    { new Guid("019d0000-0001-7000-8001-000000000022"), 300, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "RAM > 80% - Platform", "[]", 3, "Warning", 80.0, "PlatformRamHigh" }
                });

            migrationBuilder.InsertData(
                table: "permissions",
                columns: new[] { "id", "permissionlevel", "resourcetype", "roleid", "specificpermissions" },
                values: new object[,]
                {
                    { new Guid("030c8f34-4447-d6b0-bc28-62b9626999c7"), 1, 1, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("07b143ad-6b02-c7ff-3d7d-48af137b2bbc"), 2, 0, new Guid("30000000-0000-0000-0000-000000000002"), 27 },
                    { new Guid("1879288f-3bbb-20f0-ab2c-1eefcc32262a"), 2, 1, new Guid("30000000-0000-0000-0000-000000000002"), 23 },
                    { new Guid("2533e6f2-53e3-1281-01f7-cc28045cbc4f"), 2, 10, new Guid("30000000-0000-0000-0000-000000000002"), 0 },
                    { new Guid("2d9c5d81-bce2-e0a6-004b-138d3ac0a4a9"), 4, 3, new Guid("30000000-0000-0000-0000-000000000001"), 0 },
                    { new Guid("361e1bf8-ef0f-1409-9137-6fa885696a19"), 4, 7, new Guid("30000000-0000-0000-0000-000000000001"), 0 },
                    { new Guid("3bf8e221-07a0-052c-d831-2c82d22f7660"), 2, 9, new Guid("30000000-0000-0000-0000-000000000002"), 0 },
                    { new Guid("4032d1e2-fe5e-16ef-f554-69bd2c2ac19d"), 1, 10, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("440deb9d-ace8-5e15-ef80-3a42f11a0c42"), 4, 10, new Guid("30000000-0000-0000-0000-000000000001"), 0 },
                    { new Guid("44debca1-5d97-b691-196c-8e421143e307"), 2, 8, new Guid("30000000-0000-0000-0000-000000000002"), 0 },
                    { new Guid("5f36ed62-49a9-8610-dc34-2bc89165e4de"), 4, 1, new Guid("30000000-0000-0000-0000-000000000001"), 23 },
                    { new Guid("645b4c54-7937-2180-7186-be24ac6bf330"), 4, 5, new Guid("30000000-0000-0000-0000-000000000001"), 0 },
                    { new Guid("672ebf04-40e5-547b-29f2-6daf5c3c3856"), 1, 8, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("80aa1c34-79dd-6587-52db-52605326fe77"), 2, 7, new Guid("30000000-0000-0000-0000-000000000002"), 0 },
                    { new Guid("86dadd60-fced-3dcd-cdbe-8d262bec7d22"), 2, 5, new Guid("30000000-0000-0000-0000-000000000002"), 0 },
                    { new Guid("909763b4-50a0-e1c7-6df1-61add076910c"), 1, 2, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("936632a5-4e74-0a17-fb8e-497c960c3005"), 1, 0, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("94717f37-cc1a-de60-9bca-dc6379444bfb"), 1, 5, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("97597a3e-c415-667b-039a-a7a287daefea"), 1, 4, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("987e89d0-2c8f-87d8-830f-7461a7db392e"), 4, 4, new Guid("30000000-0000-0000-0000-000000000001"), 0 },
                    { new Guid("9b075e03-6326-7b95-ae78-2b296990ce26"), 2, 4, new Guid("30000000-0000-0000-0000-000000000002"), 0 },
                    { new Guid("a4b222e3-7452-b5f4-377b-c05652533f67"), 4, 0, new Guid("30000000-0000-0000-0000-000000000001"), 27 },
                    { new Guid("a60ba8de-ff46-b387-85ed-913d96170a2a"), 1, 7, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("b104e60e-87ef-a58f-f04a-ba2fc634f037"), 1, 6, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("b3abb382-80da-8170-b011-05af044e7908"), 2, 3, new Guid("30000000-0000-0000-0000-000000000002"), 0 },
                    { new Guid("c5e4df97-9c4a-cdf4-6568-b709276db612"), 4, 2, new Guid("30000000-0000-0000-0000-000000000001"), 7 },
                    { new Guid("d0d48de1-86a5-c43e-309c-1adce8776662"), 2, 2, new Guid("30000000-0000-0000-0000-000000000002"), 7 },
                    { new Guid("d5fa8563-b0a2-4f11-7e16-7c1877e43dda"), 4, 6, new Guid("30000000-0000-0000-0000-000000000001"), 0 },
                    { new Guid("dbb104e4-d7e2-5173-b0b2-6d1519c2f682"), 4, 8, new Guid("30000000-0000-0000-0000-000000000001"), 0 },
                    { new Guid("e04cd0d3-47bf-2d28-e099-c7a9b61e3875"), 1, 3, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("ee9254c3-9b59-15a0-aa85-898f5974a603"), 2, 6, new Guid("30000000-0000-0000-0000-000000000002"), 0 },
                    { new Guid("fb710f21-c146-e381-00f0-820f58ecb69a"), 1, 9, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("fbb8ef70-2ec3-134f-0c18-1533173d5849"), 4, 9, new Guid("30000000-0000-0000-0000-000000000001"), 0 }
                });

            migrationBuilder.InsertData(
                table: "registries",
                columns: new[] { "id", "configuration", "createdat", "createdbyactorid", "description", "name", "registryhost", "status" },
                values: new object[] { new Guid("00000000-0000-0000-0000-000000000100"), "{\r\n    \"$type\": \"DockerHub\"\r\n}", new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), "Public Docker Hub Registry", "Docker Hub", "hub.docker.com", "Active" });

            migrationBuilder.InsertData(
                table: "teams",
                columns: new[] { "id", "actorid", "name" },
                values: new object[] { new Guid("20000000-0000-0000-0000-000000000001"), new Guid("00000000-0000-0000-0000-000000000003"), "Operators" });

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
                name: "teams");

            migrationBuilder.DropTable(
                name: "users");

            migrationBuilder.DropTable(
                name: "deployments");

            migrationBuilder.DropTable(
                name: "images");

            migrationBuilder.DropTable(
                name: "stacks");

            migrationBuilder.DropTable(
                name: "platforms");

            migrationBuilder.DropTable(
                name: "registries");

            migrationBuilder.DropTable(
                name: "actors");
        }
    }
}
