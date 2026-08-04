using System;
using Microsoft.EntityFrameworkCore.Migrations;

#nullable disable

namespace Infrastructure.Migrations.Migrations
{
    /// <inheritdoc />
    public partial class migration0002 : Migration
    {
        /// <inheritdoc />
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.CreateTable(
                name: "swarmconfigprojections",
                columns: table => new
                {
                    platformid = table.Column<Guid>(type: "uuid", nullable: false),
                    dockerconfigid = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    dockercreatedat = table.Column<DateTimeOffset>(type: "timestamp with time zone", nullable: true),
                    dockerupdatedat = table.Column<DateTimeOffset>(type: "timestamp with time zone", nullable: true),
                    isstale = table.Column<bool>(type: "boolean", nullable: false, defaultValue: false),
                    labels = table.Column<string>(type: "jsonb", nullable: false),
                    name = table.Column<string>(type: "text", maxLength: 255, nullable: false),
                    observedat = table.Column<DateTimeOffset>(type: "timestamp with time zone", nullable: false),
                    servicenames = table.Column<string>(type: "jsonb", nullable: false),
                    templatingdriver = table.Column<string>(type: "text", maxLength: 255, nullable: true),
                    versionindex = table.Column<long>(type: "bigint", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_swarmconfigprojections", x => new { x.platformid, x.dockerconfigid });
                    table.ForeignKey(
                        name: "fk_swarmconfigprojections_platforms_platformid",
                        column: x => x.platformid,
                        principalTable: "platforms",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "swarmnetworkprojections",
                columns: table => new
                {
                    platformid = table.Column<Guid>(type: "uuid", nullable: false),
                    dockernetworkid = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    dockercreatedat = table.Column<DateTimeOffset>(type: "timestamp with time zone", nullable: true),
                    driver = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    enableipv6 = table.Column<bool>(type: "boolean", nullable: false),
                    isattachable = table.Column<bool>(type: "boolean", nullable: false),
                    isencrypted = table.Column<bool>(type: "boolean", nullable: false),
                    isingress = table.Column<bool>(type: "boolean", nullable: false),
                    isinternal = table.Column<bool>(type: "boolean", nullable: false),
                    isstale = table.Column<bool>(type: "boolean", nullable: false, defaultValue: false),
                    labels = table.Column<string>(type: "jsonb", nullable: false),
                    name = table.Column<string>(type: "text", maxLength: 255, nullable: false),
                    observedat = table.Column<DateTimeOffset>(type: "timestamp with time zone", nullable: false),
                    scope = table.Column<string>(type: "text", maxLength: 32, nullable: false),
                    servicenames = table.Column<string>(type: "jsonb", nullable: false),
                    subnets = table.Column<string>(type: "jsonb", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_swarmnetworkprojections", x => new { x.platformid, x.dockernetworkid });
                    table.ForeignKey(
                        name: "fk_swarmnetworkprojections_platforms_platformid",
                        column: x => x.platformid,
                        principalTable: "platforms",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "swarmnodeprojections",
                columns: table => new
                {
                    platformid = table.Column<Guid>(type: "uuid", nullable: false),
                    dockernodeid = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    address = table.Column<string>(type: "text", maxLength: 255, nullable: false),
                    architecture = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    availability = table.Column<string>(type: "text", maxLength: 32, nullable: false),
                    desiredtaskcount = table.Column<int>(type: "integer", nullable: false),
                    dockercreatedat = table.Column<DateTimeOffset>(type: "timestamp with time zone", nullable: true),
                    dockerupdatedat = table.Column<DateTimeOffset>(type: "timestamp with time zone", nullable: true),
                    engineversion = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    hostname = table.Column<string>(type: "text", maxLength: 255, nullable: false),
                    isleader = table.Column<bool>(type: "boolean", nullable: false),
                    isstale = table.Column<bool>(type: "boolean", nullable: false, defaultValue: false),
                    labels = table.Column<string>(type: "jsonb", nullable: false),
                    observedat = table.Column<DateTimeOffset>(type: "timestamp with time zone", nullable: false),
                    operatingsystem = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    reachability = table.Column<string>(type: "text", maxLength: 32, nullable: false),
                    role = table.Column<string>(type: "text", maxLength: 32, nullable: false),
                    runningtaskcount = table.Column<int>(type: "integer", nullable: false),
                    status = table.Column<string>(type: "text", maxLength: 32, nullable: false),
                    statusmessage = table.Column<string>(type: "text", maxLength: 1000, nullable: true),
                    versionindex = table.Column<long>(type: "bigint", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_swarmnodeprojections", x => new { x.platformid, x.dockernodeid });
                    table.ForeignKey(
                        name: "fk_swarmnodeprojections_platforms_platformid",
                        column: x => x.platformid,
                        principalTable: "platforms",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "swarmsecretprojections",
                columns: table => new
                {
                    platformid = table.Column<Guid>(type: "uuid", nullable: false),
                    dockersecretid = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    dockercreatedat = table.Column<DateTimeOffset>(type: "timestamp with time zone", nullable: true),
                    dockerupdatedat = table.Column<DateTimeOffset>(type: "timestamp with time zone", nullable: true),
                    driver = table.Column<string>(type: "text", maxLength: 255, nullable: true),
                    isstale = table.Column<bool>(type: "boolean", nullable: false, defaultValue: false),
                    labels = table.Column<string>(type: "jsonb", nullable: false),
                    name = table.Column<string>(type: "text", maxLength: 255, nullable: false),
                    observedat = table.Column<DateTimeOffset>(type: "timestamp with time zone", nullable: false),
                    servicenames = table.Column<string>(type: "jsonb", nullable: false),
                    versionindex = table.Column<long>(type: "bigint", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_swarmsecretprojections", x => new { x.platformid, x.dockersecretid });
                    table.ForeignKey(
                        name: "fk_swarmsecretprojections_platforms_platformid",
                        column: x => x.platformid,
                        principalTable: "platforms",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "swarmserviceprojections",
                columns: table => new
                {
                    platformid = table.Column<Guid>(type: "uuid", nullable: false),
                    dockerserviceid = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    configids = table.Column<string>(type: "jsonb", nullable: false),
                    desiredtaskcount = table.Column<int>(type: "integer", nullable: false),
                    dockercreatedat = table.Column<DateTimeOffset>(type: "timestamp with time zone", nullable: true),
                    dockerupdatedat = table.Column<DateTimeOffset>(type: "timestamp with time zone", nullable: true),
                    image = table.Column<string>(type: "text", maxLength: 1000, nullable: false),
                    isstale = table.Column<bool>(type: "boolean", nullable: false, defaultValue: false),
                    labels = table.Column<string>(type: "jsonb", nullable: false),
                    mode = table.Column<string>(type: "text", maxLength: 32, nullable: false),
                    name = table.Column<string>(type: "text", maxLength: 255, nullable: false),
                    networkids = table.Column<string>(type: "jsonb", nullable: false),
                    observedat = table.Column<DateTimeOffset>(type: "timestamp with time zone", nullable: false),
                    ports = table.Column<string>(type: "jsonb", nullable: false),
                    runningtaskcount = table.Column<int>(type: "integer", nullable: false),
                    secretids = table.Column<string>(type: "jsonb", nullable: false),
                    updatemessage = table.Column<string>(type: "text", maxLength: 1000, nullable: true),
                    updatestate = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    versionindex = table.Column<long>(type: "bigint", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_swarmserviceprojections", x => new { x.platformid, x.dockerserviceid });
                    table.ForeignKey(
                        name: "fk_swarmserviceprojections_platforms_platformid",
                        column: x => x.platformid,
                        principalTable: "platforms",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "swarmtaskprojections",
                columns: table => new
                {
                    platformid = table.Column<Guid>(type: "uuid", nullable: false),
                    dockertaskid = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    desiredstate = table.Column<string>(type: "text", maxLength: 32, nullable: false),
                    dockercreatedat = table.Column<DateTimeOffset>(type: "timestamp with time zone", nullable: true),
                    dockernodeid = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    dockerserviceid = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    dockerupdatedat = table.Column<DateTimeOffset>(type: "timestamp with time zone", nullable: true),
                    error = table.Column<string>(type: "text", maxLength: 1000, nullable: true),
                    image = table.Column<string>(type: "text", maxLength: 1000, nullable: false),
                    isstale = table.Column<bool>(type: "boolean", nullable: false, defaultValue: false),
                    name = table.Column<string>(type: "text", maxLength: 255, nullable: false),
                    nodehostname = table.Column<string>(type: "text", maxLength: 255, nullable: false),
                    observedat = table.Column<DateTimeOffset>(type: "timestamp with time zone", nullable: false),
                    ports = table.Column<string>(type: "jsonb", nullable: false),
                    servicename = table.Column<string>(type: "text", maxLength: 255, nullable: false),
                    slot = table.Column<int>(type: "integer", nullable: true),
                    state = table.Column<string>(type: "text", maxLength: 32, nullable: false),
                    statusmessage = table.Column<string>(type: "text", maxLength: 1000, nullable: true),
                    statustimestamp = table.Column<DateTimeOffset>(type: "timestamp with time zone", nullable: true),
                    versionindex = table.Column<long>(type: "bigint", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_swarmtaskprojections", x => new { x.platformid, x.dockertaskid });
                    table.ForeignKey(
                        name: "fk_swarmtaskprojections_platforms_platformid",
                        column: x => x.platformid,
                        principalTable: "platforms",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropTable(
                name: "swarmconfigprojections");

            migrationBuilder.DropTable(
                name: "swarmnetworkprojections");

            migrationBuilder.DropTable(
                name: "swarmnodeprojections");

            migrationBuilder.DropTable(
                name: "swarmsecretprojections");

            migrationBuilder.DropTable(
                name: "swarmserviceprojections");

            migrationBuilder.DropTable(
                name: "swarmtaskprojections");
        }
    }
}
