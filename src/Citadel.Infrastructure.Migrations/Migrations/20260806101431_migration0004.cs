using System;
using Microsoft.EntityFrameworkCore.Migrations;

#nullable disable

#pragma warning disable CA1814 // Prefer jagged arrays over multidimensional

namespace Infrastructure.Migrations.Migrations
{
    /// <inheritdoc />
    public partial class migration0004 : Migration
    {
        /// <inheritdoc />
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.AddColumn<long>(
                name: "forceupdate",
                table: "swarmserviceprojections",
                type: "bigint",
                nullable: false,
                defaultValue: 0L);

            migrationBuilder.AddColumn<string>(
                name: "liveruntimehash",
                table: "swarmserviceprojections",
                type: "text",
                maxLength: 64,
                nullable: true);

            migrationBuilder.AddColumn<Guid>(
                name: "swarmserviceid",
                table: "swarmserviceprojections",
                type: "uuid",
                nullable: true);

            migrationBuilder.CreateTable(
                name: "swarmservices",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    appliedimagedigest = table.Column<string>(type: "text", maxLength: 1000, nullable: true),
                    attemptedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    autoupdatestate_currentdigest = table.Column<string>(type: "text", nullable: true),
                    autoupdatestate_lastcheckedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    autoupdatestate_lasterror = table.Column<string>(type: "text", maxLength: 2000, nullable: true),
                    autoupdatestate_remotedigest = table.Column<string>(type: "text", nullable: true),
                    autoupdatestate_status = table.Column<string>(type: "text", maxLength: 64, nullable: true),
                    basedockerversion = table.Column<long>(type: "bigint", nullable: true),
                    completedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    controlstartedat = table.Column<long>(type: "bigint", nullable: true),
                    controlstate = table.Column<string>(type: "text", maxLength: 64, nullable: false, defaultValue: "Idle"),
                    controltriggeredby = table.Column<Guid>(type: "uuid", nullable: true),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    createdbyactorid = table.Column<Guid>(type: "uuid", nullable: false),
                    description = table.Column<string>(type: "text", maxLength: 600, nullable: true),
                    desiredspechash = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    dockername = table.Column<string>(type: "text", maxLength: 63, nullable: false),
                    dockerserviceid = table.Column<string>(type: "text", maxLength: 64, nullable: true),
                    dockerversionindex = table.Column<long>(type: "bigint", nullable: true),
                    expectedforceupdate = table.Column<long>(type: "bigint", nullable: true),
                    health = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    lastapplieddesiredspechash = table.Column<string>(type: "text", maxLength: 64, nullable: true),
                    lastappliedruntimehash = table.Column<string>(type: "text", maxLength: 64, nullable: true),
                    name = table.Column<string>(type: "text", maxLength: 255, nullable: false),
                    observeddockerversion = table.Column<long>(type: "bigint", nullable: true),
                    operationactorid = table.Column<Guid>(type: "uuid", nullable: true),
                    operationclusterid = table.Column<string>(type: "text", maxLength: 255, nullable: true),
                    operationid = table.Column<Guid>(type: "uuid", nullable: true),
                    operationkind = table.Column<string>(type: "text", maxLength: 64, nullable: true),
                    operationstate = table.Column<string>(type: "text", maxLength: 64, nullable: true),
                    platformid = table.Column<Guid>(type: "uuid", nullable: false),
                    preparedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    resultcode = table.Column<string>(type: "text", maxLength: 128, nullable: true),
                    resultmessage = table.Column<string>(type: "text", maxLength: 2000, nullable: true),
                    rowversion = table.Column<long>(type: "bigint", nullable: false, defaultValue: 0L),
                    spec = table.Column<string>(type: "jsonb", nullable: false),
                    synchronizationstate = table.Column<string>(type: "text", maxLength: 64, nullable: false),
                    targetdesiredspechash = table.Column<string>(type: "text", maxLength: 64, nullable: true),
                    targetrowversion = table.Column<long>(type: "bigint", nullable: true),
                    targetruntimehash = table.Column<string>(type: "text", maxLength: 64, nullable: true),
                    updatedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    warnings = table.Column<string>(type: "jsonb", nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_swarmservices", x => x.id);
                    table.CheckConstraint("CK_SwarmServices_CanceledOperation", "operationstate <> 'Canceled' OR (attemptedat IS NULL AND completedat IS NOT NULL)");
                    table.CheckConstraint("CK_SwarmServices_OperationFields", "(operationid IS NULL AND operationkind IS NULL AND operationstate IS NULL AND basedockerversion IS NULL AND targetdesiredspechash IS NULL AND targetruntimehash IS NULL AND targetrowversion IS NULL AND expectedforceupdate IS NULL AND preparedat IS NULL AND attemptedat IS NULL AND completedat IS NULL AND observeddockerversion IS NULL AND resultcode IS NULL AND warnings IS NULL AND resultmessage IS NULL AND operationclusterid IS NULL AND operationactorid IS NULL) OR (operationid IS NOT NULL AND operationkind IS NOT NULL AND operationstate IS NOT NULL AND targetdesiredspechash IS NOT NULL AND targetrowversion IS NOT NULL AND preparedat IS NOT NULL AND operationclusterid IS NOT NULL AND operationactorid IS NOT NULL)");
                    table.ForeignKey(
                        name: "fk_swarmservices_actors_controltriggeredby",
                        column: x => x.controltriggeredby,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_swarmservices_actors_createdbyactorid",
                        column: x => x.createdbyactorid,
                        principalTable: "actors",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                    table.ForeignKey(
                        name: "fk_swarmservices_platforms_platformid",
                        column: x => x.platformid,
                        principalTable: "platforms",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Restrict);
                });

            migrationBuilder.InsertData(
                table: "permissions",
                columns: new[] { "id", "permissionlevel", "resourcetype", "roleid", "specificpermissions" },
                values: new object[,]
                {
                    { new Guid("00389706-8a88-9cfa-583a-1d4b7d2ce63a"), 4, 20, new Guid("30000000-0000-0000-0000-000000000001"), 39 },
                    { new Guid("0d39d935-1b2b-bf74-bfb3-51d40c4cfc56"), 1, 20, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("f2c76082-b7aa-7bea-bb5f-d22de2632a62"), 2, 20, new Guid("30000000-0000-0000-0000-000000000002"), 39 }
                });

            migrationBuilder.CreateIndex(
                name: "ix_swarmserviceprojections_swarmserviceid",
                table: "swarmserviceprojections",
                column: "swarmserviceid");

            migrationBuilder.CreateIndex(
                name: "ix_swarmservices_controltriggeredby",
                table: "swarmservices",
                column: "controltriggeredby");

            migrationBuilder.CreateIndex(
                name: "ix_swarmservices_createdbyactorid",
                table: "swarmservices",
                column: "createdbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_swarmservices_dockername_platformid",
                table: "swarmservices",
                columns: new[] { "dockername", "platformid" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_swarmservices_globalsearch_name_trgm",
                table: "swarmservices",
                column: "name")
                .Annotation("Npgsql:IndexMethod", "gin")
                .Annotation("Npgsql:IndexOperators", new[] { "gin_trgm_ops" });

            migrationBuilder.CreateIndex(
                name: "ix_swarmservices_name_platformid",
                table: "swarmservices",
                columns: new[] { "name", "platformid" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_swarmservices_platformid_dockerserviceid",
                table: "swarmservices",
                columns: new[] { "platformid", "dockerserviceid" },
                unique: true,
                filter: "\"dockerserviceid\" IS NOT NULL");

            migrationBuilder.CreateIndex(
                name: "ix_swarmservices_recoverableoperation",
                table: "swarmservices",
                columns: new[] { "operationstate", "preparedat" });
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropTable(
                name: "swarmservices");

            migrationBuilder.DropIndex(
                name: "ix_swarmserviceprojections_swarmserviceid",
                table: "swarmserviceprojections");

            migrationBuilder.DeleteData(
                table: "permissions",
                keyColumn: "id",
                keyValue: new Guid("00389706-8a88-9cfa-583a-1d4b7d2ce63a"));

            migrationBuilder.DeleteData(
                table: "permissions",
                keyColumn: "id",
                keyValue: new Guid("0d39d935-1b2b-bf74-bfb3-51d40c4cfc56"));

            migrationBuilder.DeleteData(
                table: "permissions",
                keyColumn: "id",
                keyValue: new Guid("f2c76082-b7aa-7bea-bb5f-d22de2632a62"));

            migrationBuilder.DropColumn(
                name: "forceupdate",
                table: "swarmserviceprojections");

            migrationBuilder.DropColumn(
                name: "liveruntimehash",
                table: "swarmserviceprojections");

            migrationBuilder.DropColumn(
                name: "swarmserviceid",
                table: "swarmserviceprojections");
        }
    }
}
