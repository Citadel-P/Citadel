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
            migrationBuilder.AddColumn<string>(
                name: "dockerstacknamespace",
                table: "swarmserviceprojections",
                type: "text",
                maxLength: 255,
                nullable: true);

            migrationBuilder.AddColumn<string>(
                name: "ownership",
                table: "swarmserviceprojections",
                type: "text",
                maxLength: 32,
                nullable: false,
                defaultValue: "Unmanaged");

            migrationBuilder.AddColumn<string>(
                name: "ownershipdiagnostic",
                table: "swarmserviceprojections",
                type: "text",
                maxLength: 255,
                nullable: true);
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropColumn(
                name: "dockerstacknamespace",
                table: "swarmserviceprojections");

            migrationBuilder.DropColumn(
                name: "ownership",
                table: "swarmserviceprojections");

            migrationBuilder.DropColumn(
                name: "ownershipdiagnostic",
                table: "swarmserviceprojections");
        }
    }
}
