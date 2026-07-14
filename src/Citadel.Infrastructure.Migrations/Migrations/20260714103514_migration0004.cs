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
                        onDelete: ReferentialAction.Restrict);
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

            migrationBuilder.InsertData(
                table: "permissions",
                columns: new[] { "id", "permissionlevel", "resourcetype", "roleid", "specificpermissions" },
                values: new object[,]
                {
                    { new Guid("677df0f0-2ce5-4b25-76ad-eb4e21f0748d"), 4, 17, new Guid("30000000-0000-0000-0000-000000000001"), 768 },
                    { new Guid("9042fcd7-44f2-8a16-dca9-2fabdba5a0bc"), 1, 17, new Guid("30000000-0000-0000-0000-000000000003"), 0 },
                    { new Guid("f2552404-ef60-5f22-0eaf-fd7db21f2579"), 2, 17, new Guid("30000000-0000-0000-0000-000000000002"), 768 }
                });

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
                name: "ix_stackreleasevolumebindings_platform_volumename",
                table: "stackreleasevolumebindings",
                columns: new[] { "platformid", "volumename" });

            migrationBuilder.CreateIndex(
                name: "ix_stackreleasevolumebindings_release_volumename",
                table: "stackreleasevolumebindings",
                columns: new[] { "stackreleaseid", "volumename" },
                unique: true);
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropTable(
                name: "backuprunitems");

            migrationBuilder.DropTable(
                name: "stackreleasevolumebindings");

            migrationBuilder.DeleteData(
                table: "permissions",
                keyColumn: "id",
                keyValue: new Guid("677df0f0-2ce5-4b25-76ad-eb4e21f0748d"));

            migrationBuilder.DeleteData(
                table: "permissions",
                keyColumn: "id",
                keyValue: new Guid("9042fcd7-44f2-8a16-dca9-2fabdba5a0bc"));

            migrationBuilder.DeleteData(
                table: "permissions",
                keyColumn: "id",
                keyValue: new Guid("f2552404-ef60-5f22-0eaf-fd7db21f2579"));
        }
    }
}
