using Microsoft.EntityFrameworkCore.Migrations;

#nullable disable

namespace Infrastructure.Migrations
{
    /// <inheritdoc />
    public partial class migration0002 : Migration
    {
        /// <inheritdoc />
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.UpdateData(
                table: "Users",
                keyColumn: "Id",
                keyValue: new byte[] { 209, 222, 150, 1, 241, 19, 206, 119, 136, 78, 60, 182, 54, 236, 9, 168 },
                column: "Password",
                value: "S4buHR9wq9tSuwh1SjsexfxBwUm2rWBHLEbT1aMVKVIxV2Ay");

            migrationBuilder.UpdateData(
                table: "Users",
                keyColumn: "Id",
                keyValue: new byte[] { 209, 222, 150, 1, 241, 19, 251, 115, 172, 240, 24, 129, 21, 192, 28, 14 },
                column: "Password",
                value: "6HpzHnYaGNOH4EYpbXzdUytceFF3KOedbhWOYxUCoSWJdUQC");

            migrationBuilder.UpdateData(
                table: "Users",
                keyColumn: "Id",
                keyValue: new byte[] { 209, 222, 150, 1, 241, 19, 58, 116, 138, 27, 94, 36, 48, 72, 199, 126 },
                column: "Password",
                value: "H2YmIHnpinUJ3R7m+Km+2cr+7yt1SaFjuSmaSPgyz3V+9/4X");
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.UpdateData(
                table: "Users",
                keyColumn: "Id",
                keyValue: new byte[] { 209, 222, 150, 1, 241, 19, 206, 119, 136, 78, 60, 182, 54, 236, 9, 168 },
                column: "Password",
                value: "WV/F/nmHBA85V3bz35Vk2odzbfaKz/WVL6FOYHZq01w5e2vg");

            migrationBuilder.UpdateData(
                table: "Users",
                keyColumn: "Id",
                keyValue: new byte[] { 209, 222, 150, 1, 241, 19, 251, 115, 172, 240, 24, 129, 21, 192, 28, 14 },
                column: "Password",
                value: "hQJrv8yx/Rxu+Q6z7RuW5JvbWBgK155lAr/vPTYFRKhHKmKv");

            migrationBuilder.UpdateData(
                table: "Users",
                keyColumn: "Id",
                keyValue: new byte[] { 209, 222, 150, 1, 241, 19, 58, 116, 138, 27, 94, 36, 48, 72, 199, 126 },
                column: "Password",
                value: "Z74g22pM7mC8LASea/79XZ0uVhs1fYmMMr2NzU1u6JA9F3IW");
        }
    }
}
