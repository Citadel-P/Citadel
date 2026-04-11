using DbUp;
using Infrastructure;
using Npgsql;
using Testcontainers.PostgreSql;

namespace Tests.Integration;

public sealed class PostgresTestFixture : IAsyncLifetime
{
    private PostgreSqlContainer _container = default!;
    private const string TemplateDbName = "citadel_template";

    public string MasterConnectionString => _container.GetConnectionString();

    public async ValueTask InitializeAsync()
    {
        _container = new PostgreSqlBuilder("postgres:alpine").Build();

        await _container.StartAsync();

        // Create the Template Database
        await CreateTemplateDbAsync();

        // Run migrations once on the template
        RunMigrations(GetConnectionStringForDb(TemplateDbName));
    }

    private async Task CreateTemplateDbAsync()
    {
        using var conn = new NpgsqlConnection(MasterConnectionString);
        await conn.OpenAsync();
        using var cmd = new NpgsqlCommand($"CREATE DATABASE {TemplateDbName}", conn);
        await cmd.ExecuteNonQueryAsync();
    }

    public async Task<string> CreateDatabaseFromTemplateAsync()
    {
        var newDbName = $"db_{Guid.NewGuid():N}";

        using var conn = new NpgsqlConnection(MasterConnectionString);
        await conn.OpenAsync();

        using var cmd = new NpgsqlCommand($@"CREATE DATABASE ""{newDbName}"" TEMPLATE {TemplateDbName}", conn);
        await cmd.ExecuteNonQueryAsync();

        return GetConnectionStringForDb(newDbName);
    }

    private string GetConnectionStringForDb(string dbName)
    {
        var builder = new NpgsqlConnectionStringBuilder(MasterConnectionString)
        {
            Database = dbName,
            Pooling = false // Disable pooling for the setup/teardown to avoid locks
        };
        return builder.ConnectionString;
    }

    private static void RunMigrations(string connectionString)
    {
        var upgrader = DeployChanges.To
            .PostgresqlDatabase(connectionString)
            .WithScriptsAndCodeEmbeddedInAssembly(typeof(InfrastructureModule).Assembly)
            .Build();

        var result = upgrader.PerformUpgrade();
        if (!result.Successful) throw result.Error!;
    }

    public async ValueTask DisposeAsync() => await _container.DisposeAsync();
}

[CollectionDefinition("Postgres")]
public class PostgresCollection : ICollectionFixture<PostgresTestFixture>
{
    // This class has no code, and is never created. Its purpose is simply
    // to be the place to apply [CollectionDefinition] and all the
    // ICollectionFixture<> interfaces.
}
