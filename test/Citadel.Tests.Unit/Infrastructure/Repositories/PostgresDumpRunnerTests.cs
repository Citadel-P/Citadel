using Domain.Contracts.Interfaces;
using Infrastructure.Repositories;
using Npgsql;

namespace Tests.Unit.Infrastructure.Repositories;

public sealed class PostgresDumpRunnerTests
{
    [Fact]
    public void BuildProcessError_ShouldExplainMissingPostgresClient()
    {
        const string rawError =
            "An error occurred trying to start process 'pg_dump' with working directory '/app/data/backups/work'. No such file or directory";

        var error = PostgresDumpRunner.BuildProcessError("pg_dump", -1, rawError);

        Assert.Equal(
            "Citadel could not start the PostgreSQL dump executable \"pg_dump\". "
            + "Install PostgreSQL client tools or configure Backups:PostgresDumpPath.",
            error);
    }

    [Fact]
    public void BuildProcessCommand_ShouldKeepCredentialsOutOfArguments()
    {
        const string password = "do-not-put-this-in-arguments";
        var definition = new PostgresDumpCommand(
            "pg_dump",
            $"Host=db.internal;Port=6432;Database=citadel;Username=backup;Password={password};SSL Mode=VerifyFull;Root Certificate=/certs/ca.pem",
            "/work/database/citadel.dump",
            "/work",
            TimeSpan.FromMinutes(2),
            8192);
        var connection = new NpgsqlConnectionStringBuilder(definition.ConnectionString);

        var command = PostgresDumpRunner.BuildProcessCommand(definition, connection);

        Assert.Equal("pg_dump", command.FileName);
        Assert.Contains("--format=custom", command.Arguments);
        Assert.Contains("--no-owner", command.Arguments);
        Assert.Contains("--no-privileges", command.Arguments);
        Assert.Contains("--file=/work/database/citadel.dump", command.Arguments);
        Assert.DoesNotContain(command.Arguments, argument => argument.Contains(password, StringComparison.Ordinal));
        Assert.Equal("db.internal", command.Environment["PGHOST"]);
        Assert.Equal("6432", command.Environment["PGPORT"]);
        Assert.Equal("citadel", command.Environment["PGDATABASE"]);
        Assert.Equal("backup", command.Environment["PGUSER"]);
        Assert.Equal(password, command.Environment["PGPASSWORD"]);
        Assert.Equal("verify-full", command.Environment["PGSSLMODE"]);
        Assert.Equal("/certs/ca.pem", command.Environment["PGSSLROOTCERT"]);
        Assert.Contains(password, command.RedactionValues);
        Assert.Equal("PostgreSQL dump", command.OperationName);
    }
}
