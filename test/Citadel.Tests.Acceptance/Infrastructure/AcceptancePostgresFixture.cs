using Npgsql;
using Testcontainers.PostgreSql;

namespace Tests.Acceptance.Infrastructure;

public sealed class AcceptancePostgresFixture : IAsyncLifetime
{
    private const string Username = "citadel";
    private const string Password = "citadel-acceptance";
    private PostgreSqlContainer container = default!;

    public string ContainerId => container.Id;

    public async ValueTask InitializeAsync()
    {
        container = new PostgreSqlBuilder("postgres:17.5-alpine")
            .WithDatabase("postgres")
            .WithUsername(Username)
            .WithPassword(Password)
            .Build();

        await container.StartAsync();
    }

    public async Task<string> CreateDatabaseAsync(CancellationToken cancellationToken)
    {
        var databaseName = $"acceptance_{Guid.NewGuid():N}";
        await using var connection = new NpgsqlConnection(container.GetConnectionString());
        await connection.OpenAsync(cancellationToken);

        await using var command = new NpgsqlCommand(
            $"""CREATE DATABASE "{databaseName}";""",
            connection);
        await command.ExecuteNonQueryAsync(cancellationToken);

        return new NpgsqlConnectionStringBuilder(container.GetConnectionString())
        {
            Database = databaseName,
            Pooling = false
        }.ConnectionString;
    }

    public async Task<byte[]> DumpDatabaseAsync(
        string connectionString,
        CancellationToken cancellationToken)
    {
        var databaseName = GetDatabaseName(connectionString);
        var dumpPath = $"/tmp/{databaseName}-{Guid.NewGuid():N}.dump";
        try
        {
            var result = await container.ExecAsync(
                [
                    "sh",
                    "-c",
                    $"""
                    PGPASSWORD="$POSTGRES_PASSWORD" pg_dump \
                      --host=127.0.0.1 \
                      --port=5432 \
                      --username="$POSTGRES_USER" \
                      --format=custom \
                      --no-owner \
                      --no-privileges \
                      --file="{dumpPath}" \
                      "{databaseName}"
                    """
                ],
                cancellationToken);

            Assert.Equal(
                0L,
                result.ExitCode ?? -1L);
            Assert.True(
                string.IsNullOrWhiteSpace(result.Stderr),
                $"pg_dump reported an error: {result.Stderr}");

            return await container.ReadFileAsync(dumpPath, cancellationToken);
        }
        finally
        {
            await container.ExecAsync(
                ["rm", "-f", dumpPath],
                CancellationToken.None);
        }
    }

    public async Task RestoreDatabaseAsync(
        string connectionString,
        byte[] dump,
        CancellationToken cancellationToken)
    {
        Assert.True(
            await IsDatabaseEmptyAsync(connectionString, cancellationToken),
            "Control-plane recovery must target an empty PostgreSQL database.");

        var databaseName = GetDatabaseName(connectionString);
        var dumpPath = $"/tmp/{databaseName}-{Guid.NewGuid():N}.dump";
        try
        {
            await container.CopyAsync(dump, dumpPath, ct: cancellationToken);
            var result = await container.ExecAsync(
                [
                    "sh",
                    "-c",
                    $"""
                    PGPASSWORD="$POSTGRES_PASSWORD" pg_restore \
                      --host=127.0.0.1 \
                      --port=5432 \
                      --username="$POSTGRES_USER" \
                      --dbname="{databaseName}" \
                      --no-owner \
                      --no-privileges \
                      --exit-on-error \
                      --single-transaction \
                      "{dumpPath}"
                    """
                ],
                cancellationToken);

            Assert.Equal(
                0L,
                result.ExitCode ?? -1L);
            Assert.True(
                string.IsNullOrWhiteSpace(result.Stderr),
                $"pg_restore reported an error: {result.Stderr}");
        }
        finally
        {
            await container.ExecAsync(
                ["rm", "-f", dumpPath],
                CancellationToken.None);
        }
    }

    public async Task DropDatabaseAsync(
        string connectionString,
        CancellationToken cancellationToken)
    {
        var databaseName = GetDatabaseName(connectionString);
        await using var connection = new NpgsqlConnection(container.GetConnectionString());
        await connection.OpenAsync(cancellationToken);

        await using var command = new NpgsqlCommand(
            $"""DROP DATABASE "{databaseName}" WITH (FORCE);""",
            connection);
        await command.ExecuteNonQueryAsync(cancellationToken);
    }

    public static async Task<bool> IsDatabaseEmptyAsync(
        string connectionString,
        CancellationToken cancellationToken)
    {
        await using var connection = new NpgsqlConnection(connectionString);
        await connection.OpenAsync(cancellationToken);
        await using var command = new NpgsqlCommand(
            """
            SELECT NOT EXISTS (
                SELECT 1
                FROM pg_class AS c
                JOIN pg_namespace AS n ON n.oid = c.relnamespace
                WHERE n.nspname = 'public'
                  AND c.relkind IN ('r', 'p', 'v', 'm', 'S')
            );
            """,
            connection);

        return (bool)(await command.ExecuteScalarAsync(cancellationToken))!;
    }

    public async ValueTask DisposeAsync()
    {
        await container.DisposeAsync();
    }

    private static string GetDatabaseName(string connectionString)
    {
        var databaseName = new NpgsqlConnectionStringBuilder(connectionString).Database;
        Assert.False(string.IsNullOrWhiteSpace(databaseName));
        Assert.Matches("^acceptance_[a-f0-9]{32}$", databaseName);
        return databaseName!;
    }
}

[CollectionDefinition("AcceptancePostgres")]
public sealed class AcceptancePostgresCollection : ICollectionFixture<AcceptancePostgresFixture>;
