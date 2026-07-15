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
            migrationBuilder.DropForeignKey(
                name: "fk_backuprunitems_platforms_platformid",
                table: "backuprunitems");

            migrationBuilder.AddForeignKey(
                name: "fk_backuprunitems_platforms_platformid",
                table: "backuprunitems",
                column: "platformid",
                principalTable: "platforms",
                principalColumn: "id",
                onDelete: ReferentialAction.Cascade);
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropForeignKey(
                name: "fk_backuprunitems_platforms_platformid",
                table: "backuprunitems");

            migrationBuilder.AddForeignKey(
                name: "fk_backuprunitems_platforms_platformid",
                table: "backuprunitems",
                column: "platformid",
                principalTable: "platforms",
                principalColumn: "id",
                onDelete: ReferentialAction.Restrict);
        }
    }
}
