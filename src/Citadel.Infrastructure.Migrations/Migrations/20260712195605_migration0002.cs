using System;
using Microsoft.EntityFrameworkCore.Migrations;

#nullable disable

#pragma warning disable CA1814 // Prefer jagged arrays over multidimensional

namespace Infrastructure.Migrations.Migrations
{
    /// <inheritdoc />
    public partial class migration0002 : Migration
    {
        /// <inheritdoc />
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.CreateTable(
                name: "backuprepositories",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    archivedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP"),
                    createdbyactorid = table.Column<Guid>(type: "uuid", nullable: false),
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
                name: "backuppolicies",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    alertonfailure = table.Column<bool>(type: "boolean", nullable: false, defaultValue: true),
                    archivedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    backuprepositoryid = table.Column<Guid>(type: "uuid", nullable: false),
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
                    updatedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP")
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
                table: "permissions",
                columns: new[] { "id", "permissionlevel", "resourcetype", "roleid", "specificpermissions" },
                values: new object[,]
                {
                    { new Guid("102eae03-0582-a53c-2287-ce55c2f222a8"), 2, 16, new Guid("30000000-0000-0000-0000-000000000002"), 128 },
                    { new Guid("378efbb8-bac7-1928-e93a-7d152a50b3b6"), 1, 15, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("6128787e-901d-f15b-c094-054be267a6c8"), 1, 16, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("7cb0a723-f754-2be2-38fa-8d151c2e1c7f"), 2, 15, new Guid("30000000-0000-0000-0000-000000000002"), 0 },
                    { new Guid("ba1393e4-1090-990e-aee3-a3e36ede0fee"), 4, 16, new Guid("30000000-0000-0000-0000-000000000001"), 128 },
                    { new Guid("e9b46175-cc60-8d02-1dbd-ddf7f907eb16"), 4, 15, new Guid("30000000-0000-0000-0000-000000000001"), 0 }
                });

            migrationBuilder.CreateIndex(
                name: "ix_backuppolicies_archivedat",
                table: "backuppolicies",
                column: "archivedat");

            migrationBuilder.CreateIndex(
                name: "ix_backuppolicies_backuprepositoryid",
                table: "backuppolicies",
                column: "backuprepositoryid");

            migrationBuilder.CreateIndex(
                name: "ix_backuppolicies_createdbyactorid",
                table: "backuppolicies",
                column: "createdbyactorid");

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
                name: "ix_backuprepositories_archivedat",
                table: "backuprepositories",
                column: "archivedat");

            migrationBuilder.CreateIndex(
                name: "ix_backuprepositories_createdbyactorid",
                table: "backuprepositories",
                column: "createdbyactorid");

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
                columns: new[] { "backuppolicyid", "queuedat" });

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
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropTable(
                name: "backuprepositoryleases");

            migrationBuilder.DropTable(
                name: "backuprepositoryvalidations");

            migrationBuilder.DropTable(
                name: "backuprestorerunlogs");

            migrationBuilder.DropTable(
                name: "backuprunlogs");

            migrationBuilder.DropTable(
                name: "backupsourceleases");

            migrationBuilder.DropTable(
                name: "backuprestoreruns");

            migrationBuilder.DropTable(
                name: "backupruns");

            migrationBuilder.DropTable(
                name: "backuppolicies");

            migrationBuilder.DropTable(
                name: "backuprepositories");

            migrationBuilder.DeleteData(
                table: "permissions",
                keyColumn: "id",
                keyValue: new Guid("102eae03-0582-a53c-2287-ce55c2f222a8"));

            migrationBuilder.DeleteData(
                table: "permissions",
                keyColumn: "id",
                keyValue: new Guid("378efbb8-bac7-1928-e93a-7d152a50b3b6"));

            migrationBuilder.DeleteData(
                table: "permissions",
                keyColumn: "id",
                keyValue: new Guid("6128787e-901d-f15b-c094-054be267a6c8"));

            migrationBuilder.DeleteData(
                table: "permissions",
                keyColumn: "id",
                keyValue: new Guid("7cb0a723-f754-2be2-38fa-8d151c2e1c7f"));

            migrationBuilder.DeleteData(
                table: "permissions",
                keyColumn: "id",
                keyValue: new Guid("ba1393e4-1090-990e-aee3-a3e36ede0fee"));

            migrationBuilder.DeleteData(
                table: "permissions",
                keyColumn: "id",
                keyValue: new Guid("e9b46175-cc60-8d02-1dbd-ddf7f907eb16"));
        }
    }
}
