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
                name: "swarmservicestats",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    cpuusage = table.Column<double>(type: "double precision", nullable: false),
                    created = table.Column<long>(type: "bigint", nullable: false),
                    dockerserviceid = table.Column<string>(type: "text", maxLength: 255, nullable: false),
                    dockertaskid = table.Column<string>(type: "text", maxLength: 255, nullable: false),
                    memoryactive = table.Column<double>(type: "double precision", nullable: false),
                    memorycache = table.Column<double>(type: "double precision", nullable: false),
                    memorylimit = table.Column<double>(type: "double precision", nullable: false),
                    platformid = table.Column<Guid>(type: "uuid", nullable: false),
                    rxbytes = table.Column<double>(type: "double precision", nullable: false),
                    servicename = table.Column<string>(type: "text", maxLength: 255, nullable: false),
                    stackid = table.Column<Guid>(type: "uuid", nullable: true),
                    swarmserviceid = table.Column<Guid>(type: "uuid", nullable: true),
                    taskkey = table.Column<string>(type: "text", maxLength: 512, nullable: false),
                    txbytes = table.Column<double>(type: "double precision", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_swarmservicestats", x => x.id);
                    table.ForeignKey(
                        name: "fk_swarmservicestats_platforms_platformid",
                        column: x => x.platformid,
                        principalTable: "platforms",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateIndex(
                name: "ix_swarmservicestats_created",
                table: "swarmservicestats",
                column: "created");

            migrationBuilder.CreateIndex(
                name: "ix_swarmservicestats_managedservicecreated",
                table: "swarmservicestats",
                columns: new[] { "swarmserviceid", "created" },
                filter: "swarmserviceid IS NOT NULL");

            migrationBuilder.CreateIndex(
                name: "ix_swarmservicestats_platformservicecreated",
                table: "swarmservicestats",
                columns: new[] { "platformid", "dockerserviceid", "created" });

            migrationBuilder.CreateIndex(
                name: "ix_swarmservicestats_platformtaskcreated",
                table: "swarmservicestats",
                columns: new[] { "platformid", "dockertaskid", "created" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_swarmservicestats_stackservicecreated",
                table: "swarmservicestats",
                columns: new[] { "stackid", "servicename", "created" },
                filter: "stackid IS NOT NULL");
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropTable(
                name: "swarmservicestats");
        }
    }
}
