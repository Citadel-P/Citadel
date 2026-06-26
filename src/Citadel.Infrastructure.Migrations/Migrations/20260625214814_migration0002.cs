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
            migrationBuilder.InsertData(
                table: "alertrules",
                columns: new[] { "id", "cooldownseconds", "createdat", "createdbyactorid", "description", "limitedto", "name", "quiethours", "requiredmatches", "severity", "threshold", "type" },
                values: new object[,]
                {
                    { new Guid("019d0000-0001-7000-8001-00000000000f"), 300, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Webhook Authentication Failed", "[]", null, "Warning", null, "WebhookAuthenticationFailed" },
                    { new Guid("019d0000-0001-7000-8001-000000000010"), 300, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Webhook Dispatch Failed", "[]", null, "Warning", null, "WebhookDispatchFailed" },
                    { new Guid("019d0000-0001-7000-8001-000000000012"), 300, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Webhook Sync Failed - Git Repository", "[]", null, "Warning", null, "WebhookGitRepoSyncFailed" },
                    { new Guid("019d0000-0001-7000-8001-000000000013"), 300, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Webhook Deploy Failed - Git Stack", "[]", null, "Critical", null, "WebhookStackGitDeployFailed" }
                });
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DeleteData(
                table: "alertrules",
                keyColumn: "id",
                keyValue: new Guid("019d0000-0001-7000-8001-00000000000f"));

            migrationBuilder.DeleteData(
                table: "alertrules",
                keyColumn: "id",
                keyValue: new Guid("019d0000-0001-7000-8001-000000000010"));

            migrationBuilder.DeleteData(
                table: "alertrules",
                keyColumn: "id",
                keyValue: new Guid("019d0000-0001-7000-8001-000000000012"));

            migrationBuilder.DeleteData(
                table: "alertrules",
                keyColumn: "id",
                keyValue: new Guid("019d0000-0001-7000-8001-000000000013"));
        }
    }
}
