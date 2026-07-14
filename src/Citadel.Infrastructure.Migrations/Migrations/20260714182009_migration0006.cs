using System;
using Microsoft.EntityFrameworkCore.Migrations;

#nullable disable

namespace Infrastructure.Migrations.Migrations
{
    /// <inheritdoc />
    public partial class migration0006 : Migration
    {
        /// <inheritdoc />
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.AddColumn<long>(
                name: "controlstartedat",
                table: "backuprepositories",
                type: "bigint",
                nullable: true);

            migrationBuilder.AddColumn<string>(
                name: "controlstate",
                table: "backuprepositories",
                type: "text",
                maxLength: 64,
                nullable: false,
                defaultValue: "Idle");

            migrationBuilder.AddColumn<Guid>(
                name: "currentrunid",
                table: "backuprepositories",
                type: "uuid",
                nullable: true);

            migrationBuilder.AddColumn<long>(
                name: "controlstartedat",
                table: "backuppolicies",
                type: "bigint",
                nullable: true);

            migrationBuilder.AddColumn<long>(
                name: "controlstartedat",
                table: "actions",
                type: "bigint",
                nullable: true);

            migrationBuilder.UpdateData(
                table: "actions",
                keyColumn: "id",
                keyValue: new Guid("41000000-0000-0000-0000-000000000001"),
                column: "controlstartedat",
                value: null);

            migrationBuilder.UpdateData(
                table: "actions",
                keyColumn: "id",
                keyValue: new Guid("41000000-0000-0000-0000-000000000002"),
                column: "controlstartedat",
                value: null);

            migrationBuilder.CreateIndex(
                name: "ix_backuprepositories_controlstate_controlstartedat",
                table: "backuprepositories",
                columns: new[] { "controlstate", "controlstartedat" });

            migrationBuilder.CreateIndex(
                name: "ix_backuppolicies_controlstate_controlstartedat",
                table: "backuppolicies",
                columns: new[] { "controlstate", "controlstartedat" });

            migrationBuilder.CreateIndex(
                name: "ix_actions_controlstate_controlstartedat",
                table: "actions",
                columns: new[] { "controlstate", "controlstartedat" });
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropIndex(
                name: "ix_backuprepositories_controlstate_controlstartedat",
                table: "backuprepositories");

            migrationBuilder.DropIndex(
                name: "ix_backuppolicies_controlstate_controlstartedat",
                table: "backuppolicies");

            migrationBuilder.DropIndex(
                name: "ix_actions_controlstate_controlstartedat",
                table: "actions");

            migrationBuilder.DropColumn(
                name: "controlstartedat",
                table: "backuprepositories");

            migrationBuilder.DropColumn(
                name: "controlstate",
                table: "backuprepositories");

            migrationBuilder.DropColumn(
                name: "currentrunid",
                table: "backuprepositories");

            migrationBuilder.DropColumn(
                name: "controlstartedat",
                table: "backuppolicies");

            migrationBuilder.DropColumn(
                name: "controlstartedat",
                table: "actions");
        }
    }
}
