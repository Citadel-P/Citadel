using System;
using Microsoft.EntityFrameworkCore.Migrations;

#nullable disable

#pragma warning disable CA1814 // Prefer jagged arrays over multidimensional

namespace Infrastructure.Migrations.Migrations
{
    /// <inheritdoc />
    public partial class migration0002_builds : Migration
    {
        /// <inheritdoc />
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.CreateTable(
                name: "buildprojects",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    archivedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    branch = table.Column<string>(type: "text", maxLength: 256, nullable: false),
                    buildargs = table.Column<string>(type: "jsonb", nullable: false, defaultValueSql: "'[]'::jsonb"),
                    buildsecrets = table.Column<string>(type: "jsonb", nullable: false, defaultValueSql: "'[]'::jsonb"),
                    contextpath = table.Column<string>(type: "text", maxLength: 512, nullable: false, defaultValue: "."),
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
                    platformid = table.Column<Guid>(type: "uuid", nullable: false),
                    registryid = table.Column<Guid>(type: "uuid", nullable: false),
                    retentionruncount = table.Column<int>(type: "integer", nullable: false, defaultValue: 20),
                    rowversion = table.Column<long>(type: "bigint", nullable: false, defaultValue: 0L),
                    tagtemplates = table.Column<string>(type: "jsonb", nullable: false, defaultValueSql: "'[\"{branch}-{shortSha}\"]'::jsonb"),
                    target = table.Column<string>(type: "text", maxLength: 128, nullable: true),
                    timeoutseconds = table.Column<int>(type: "integer", nullable: false, defaultValue: 1800),
                    updatedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false, defaultValueSql: "CURRENT_TIMESTAMP")
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_buildprojects", x => x.id);
                    table.ForeignKey(
                        name: "fk_buildprojects_actors_createdbyactorid",
                        column: x => x.createdbyactorid,
                        principalTable: "actors",
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

            migrationBuilder.UpdateData(
                table: "actions",
                keyColumn: "id",
                keyValue: new Guid("41000000-0000-0000-0000-000000000001"),
                column: "code",
                value: "const platformsResponse = await citadel.platforms.listPlatforms();\nconst platforms = platformsResponse?.platforms ?? [];\nlet pruned = 0;\nlet reclaimedBytes = 0;\n\nfor (const platform of platforms) {\n  if (platform.status === 'Offline') continue\n  const result = await citadel.platforms.prunePlatform(platform.id, { resource: \"Image\" });\n  const imagesDeleted = result?.imagesDeleted ?? [];\n  const reclaimed = Number(result?.spaceReclaimed ?? 0);\n  reclaimedBytes += reclaimed;\n  pruned += imagesDeleted.length;\n\n  if (imagesDeleted.length === 0) {\n    console.log(`No unused images on ${platform.name}.`);\n    continue;\n  }\n\n  console.log(`Pruned ${imagesDeleted.length} image item(s) on ${platform.name}; reclaimed ${reclaimed} bytes.`);\n}\n\nconsole.log(`Pruned ${pruned} image item(s); reclaimed ${reclaimedBytes} bytes.`);");

            migrationBuilder.InsertData(
                table: "permissions",
                columns: new[] { "id", "permissionlevel", "resourcetype", "roleid", "specificpermissions" },
                values: new object[,]
                {
                    { new Guid("6a2b1742-029b-d0df-1d2a-1d0aa1ed9a3f"), 1, 18, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("c472d905-c03c-a9a8-0527-c2b540274078"), 2, 18, new Guid("30000000-0000-0000-0000-000000000002"), 4 },
                    { new Guid("d1af8dbf-ef7d-d81d-33f9-e3be5ee72243"), 4, 18, new Guid("30000000-0000-0000-0000-000000000001"), 4 }
                });

            migrationBuilder.CreateIndex(
                name: "ix_buildprojects_archivedat",
                table: "buildprojects",
                column: "archivedat");

            migrationBuilder.CreateIndex(
                name: "ix_buildprojects_createdbyactorid",
                table: "buildprojects",
                column: "createdbyactorid");

            migrationBuilder.CreateIndex(
                name: "ix_buildprojects_gitrepositoryid",
                table: "buildprojects",
                column: "gitrepositoryid");

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
                columns: new[] { "buildprojectid", "queuedat" });

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
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropTable(
                name: "buildrunlogs");

            migrationBuilder.DropTable(
                name: "buildruns");

            migrationBuilder.DropTable(
                name: "buildprojects");

            migrationBuilder.DeleteData(
                table: "permissions",
                keyColumn: "id",
                keyValue: new Guid("6a2b1742-029b-d0df-1d2a-1d0aa1ed9a3f"));

            migrationBuilder.DeleteData(
                table: "permissions",
                keyColumn: "id",
                keyValue: new Guid("c472d905-c03c-a9a8-0527-c2b540274078"));

            migrationBuilder.DeleteData(
                table: "permissions",
                keyColumn: "id",
                keyValue: new Guid("d1af8dbf-ef7d-d81d-33f9-e3be5ee72243"));

            migrationBuilder.UpdateData(
                table: "actions",
                keyColumn: "id",
                keyValue: new Guid("41000000-0000-0000-0000-000000000001"),
                column: "code",
                value: "const platformsResponse = await citadel.platforms.listPlatforms();\nconst platforms = platformsResponse?.platforms ?? [];\nlet pruned = 0;\nlet reclaimedBytes = 0;\n\nfor (const platform of platforms) {\n  const result = await citadel.platforms.prunePlatform(platform.id, { resource: \"Image\" });\n  const imagesDeleted = result?.imagesDeleted ?? [];\n  const reclaimed = Number(result?.spaceReclaimed ?? 0);\n  reclaimedBytes += reclaimed;\n  pruned += imagesDeleted.length;\n\n  if (imagesDeleted.length === 0) {\n    console.log(`No unused images on ${platform.name}.`);\n    continue;\n  }\n\n  console.log(`Pruned ${imagesDeleted.length} image item(s) on ${platform.name}; reclaimed ${reclaimed} bytes.`);\n}\n\nconsole.log(`Pruned ${pruned} image item(s); reclaimed ${reclaimedBytes} bytes.`);");
        }
    }
}
