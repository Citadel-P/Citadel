using Microsoft.EntityFrameworkCore.Migrations;

#nullable disable

namespace Infrastructure.Migrations.Migrations
{
    /// <inheritdoc />
    public partial class build_deployment_consumers : Migration
    {
        /// <inheritdoc />
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.AddColumn<string>(
                name: "webhook",
                table: "buildprojects",
                type: "jsonb",
                nullable: true);

            migrationBuilder.Sql("""
                CREATE INDEX IF NOT EXISTS ix_deployments_spec_image_build
                ON deployments
                USING gin ((spec -> 'Image'));
                """);

            migrationBuilder.Sql("""
                CREATE INDEX IF NOT EXISTS ix_stackreleases_spec_buildimagebindings
                ON stackreleases
                USING gin ((spec -> 'BuildImageBindings'));
                """);
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.Sql("DROP INDEX IF EXISTS ix_stackreleases_spec_buildimagebindings;");
            migrationBuilder.Sql("DROP INDEX IF EXISTS ix_deployments_spec_image_build;");

            migrationBuilder.DropColumn(
                name: "webhook",
                table: "buildprojects");
        }
    }
}
