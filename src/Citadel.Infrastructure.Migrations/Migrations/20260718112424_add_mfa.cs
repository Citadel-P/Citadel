using System;
using Microsoft.EntityFrameworkCore.Migrations;

#nullable disable

namespace Infrastructure.Migrations.Migrations
{
    /// <inheritdoc />
    public partial class add_mfa : Migration
    {
        /// <inheritdoc />
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.CreateTable(
                name: "mfachallenges",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    consumedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    expiresat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    failedattempts = table.Column<int>(type: "integer", nullable: false, defaultValue: 0),
                    userid = table.Column<Guid>(type: "uuid", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_mfachallenges", x => x.id);
                    table.ForeignKey(
                        name: "fk_mfachallenges_users_userid",
                        column: x => x.userid,
                        principalTable: "users",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "mfasetupsessions",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    consumedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    expiresat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    protectedtotpsecret = table.Column<string>(type: "text", nullable: false),
                    userid = table.Column<Guid>(type: "uuid", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_mfasetupsessions", x => x.id);
                    table.ForeignKey(
                        name: "fk_mfasetupsessions_users_userid",
                        column: x => x.userid,
                        principalTable: "users",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "usermfarecoverycodes",
                columns: table => new
                {
                    id = table.Column<Guid>(type: "uuid", nullable: false),
                    codehash = table.Column<string>(type: "text", nullable: false),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    usedat = table.Column<DateTime>(type: "timestamp with time zone", nullable: true),
                    userid = table.Column<Guid>(type: "uuid", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_usermfarecoverycodes", x => x.id);
                    table.ForeignKey(
                        name: "fk_usermfarecoverycodes_users_userid",
                        column: x => x.userid,
                        principalTable: "users",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "usermfasettings",
                columns: table => new
                {
                    userid = table.Column<Guid>(type: "uuid", nullable: false),
                    createdat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    enabledat = table.Column<DateTime>(type: "timestamp with time zone", nullable: false),
                    lastacceptedtimestep = table.Column<long>(type: "bigint", nullable: true),
                    protectedtotpsecret = table.Column<string>(type: "text", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_usermfasettings", x => x.userid);
                    table.ForeignKey(
                        name: "fk_usermfasettings_users_userid",
                        column: x => x.userid,
                        principalTable: "users",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateIndex(
                name: "ix_mfachallenges_expiresat",
                table: "mfachallenges",
                column: "expiresat");

            migrationBuilder.CreateIndex(
                name: "ix_mfachallenges_userid",
                table: "mfachallenges",
                column: "userid");

            migrationBuilder.CreateIndex(
                name: "ix_mfasetupsessions_expiresat",
                table: "mfasetupsessions",
                column: "expiresat");

            migrationBuilder.CreateIndex(
                name: "ix_mfasetupsessions_userid",
                table: "mfasetupsessions",
                column: "userid");

            migrationBuilder.CreateIndex(
                name: "ix_usermfarecoverycodes_userid",
                table: "usermfarecoverycodes",
                column: "userid");

            migrationBuilder.CreateIndex(
                name: "ix_usermfarecoverycodes_userid_codehash",
                table: "usermfarecoverycodes",
                columns: new[] { "userid", "codehash" },
                unique: true);
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropTable(
                name: "mfachallenges");

            migrationBuilder.DropTable(
                name: "mfasetupsessions");

            migrationBuilder.DropTable(
                name: "usermfarecoverycodes");

            migrationBuilder.DropTable(
                name: "usermfasettings");
        }
    }
}
