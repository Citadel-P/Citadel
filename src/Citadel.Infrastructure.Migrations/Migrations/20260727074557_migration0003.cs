using Microsoft.EntityFrameworkCore.Migrations;

#nullable disable

namespace Infrastructure.Migrations.Migrations
{
    /// <inheritdoc />
    public partial class migration0003 : Migration
    {
        /// <inheritdoc />
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.CreateIndex(
                name: "ix_platformstats_created",
                table: "platformstats",
                column: "created");

            migrationBuilder.CreateIndex(
                name: "ix_containerstats_created",
                table: "containerstats",
                column: "created");
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropIndex(
                name: "ix_platformstats_created",
                table: "platformstats");

            migrationBuilder.DropIndex(
                name: "ix_containerstats_created",
                table: "containerstats");
        }
    }
}
