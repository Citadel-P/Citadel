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
            migrationBuilder.AddColumn<long>(
                name: "disktotalbytes",
                table: "platformstats",
                type: "bigint",
                nullable: true);

            migrationBuilder.AddColumn<double>(
                name: "diskusage",
                table: "platformstats",
                type: "double precision",
                nullable: true);

            migrationBuilder.AddColumn<long>(
                name: "diskusedbytes",
                table: "platformstats",
                type: "bigint",
                nullable: true);

            migrationBuilder.InsertData(
                table: "alertrules",
                columns: new[] { "id", "cooldownseconds", "createdat", "createdbyactorid", "description", "limitedto", "name", "quiethours", "requiredmatches", "severity", "threshold", "type" },
                values: new object[,]
                {
                    { new Guid("019d0000-0001-7000-8001-000000000023"), 300, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Disk > 70% - Platform", "[]", 3, "Warning", 70.0, "PlatformDiskHigh" },
                    { new Guid("019d0000-0001-7000-8001-000000000024"), 300, new DateTime(2026, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc), new Guid("00000000-0000-0000-0000-000000000001"), null, "[]", "Disk > 90% - Platform", "[]", 3, "Critical", 90.0, "PlatformDiskHigh" }
                });
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DeleteData(
                table: "alertrules",
                keyColumn: "id",
                keyValue: new Guid("019d0000-0001-7000-8001-000000000023"));

            migrationBuilder.DeleteData(
                table: "alertrules",
                keyColumn: "id",
                keyValue: new Guid("019d0000-0001-7000-8001-000000000024"));

            migrationBuilder.DropColumn(
                name: "disktotalbytes",
                table: "platformstats");

            migrationBuilder.DropColumn(
                name: "diskusage",
                table: "platformstats");

            migrationBuilder.DropColumn(
                name: "diskusedbytes",
                table: "platformstats");
        }
    }
}
