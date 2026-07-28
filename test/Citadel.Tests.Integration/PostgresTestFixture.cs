using DbUp;
using Infrastructure;
using Microsoft.AspNetCore.Mvc.Testing;
using Npgsql;
using System.Collections.Concurrent;
using System.Diagnostics;
using Testcontainers.PostgreSql;

namespace Tests.Integration;

public sealed class PostgresTestFixture : IAsyncLifetime
{
    private readonly ConcurrentDictionary<Type, Lazy<Task<ReusableIntegrationTestHost>>> reusableHosts = [];
    private PostgreSqlContainer _container = default!;
    private const string TemplateDbName = "citadel_template";
    private const string PostgreSqlImage = "postgres:17.5-alpine";

    public string MasterConnectionString => _container.GetConnectionString();

    public async ValueTask InitializeAsync()
    {
        _container = new PostgreSqlBuilder(PostgreSqlImage).Build();

        await _container.StartAsync();

        await CreateTemplateDbAsync();
        RunMigrations(GetConnectionStringForDb(TemplateDbName, pooling: false));
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

        return GetConnectionStringForDb(newDbName, pooling: true);
    }

    internal async Task<ReusableIntegrationTestHost> GetOrCreateReusableHostAsync(
        Type testClass,
        Func<string, WebApplicationFactory<Program>> factoryFactory)
    {
        var lazyHost = reusableHosts.GetOrAdd(
            testClass,
            _ => new Lazy<Task<ReusableIntegrationTestHost>>(
                () => CreateReusableHostAsync(factoryFactory),
                LazyThreadSafetyMode.ExecutionAndPublication));

        return await lazyHost.Value;
    }

    private async Task<ReusableIntegrationTestHost> CreateReusableHostAsync(
        Func<string, WebApplicationFactory<Program>> factoryFactory)
    {
        var connectionString = await CreateDatabaseFromTemplateAsync();
        WebApplicationFactory<Program>? factory = null;

        try
        {
            factory = factoryFactory(connectionString);
            _ = factory.Services;
            return new ReusableIntegrationTestHost(connectionString, factory);
        }
        catch
        {
            if (factory is not null)
            {
                await factory.DisposeAsync();
            }

            await DropDatabaseAsync(connectionString);
            throw;
        }
    }

    public async Task ResetDatabaseFromTemplateAsync(string connectionString)
    {
        var databaseName = GetTestDatabaseName(connectionString);

        await using var conn = new NpgsqlConnection(MasterConnectionString);
        await conn.OpenAsync();

        await DropDatabaseAsync(conn, databaseName);

        var quotedDatabaseName = QuoteIdentifier(databaseName);
        await using var create = new NpgsqlCommand(
            $"CREATE DATABASE {quotedDatabaseName} TEMPLATE {TemplateDbName}",
            conn);
        await create.ExecuteNonQueryAsync();
    }

    public async Task DropDatabaseAsync(string connectionString)
    {
        var databaseName = GetTestDatabaseName(connectionString);

        await using var conn = new NpgsqlConnection(MasterConnectionString);
        await conn.OpenAsync();

        await DropDatabaseAsync(conn, databaseName);
    }

    private static async Task DropDatabaseAsync(
        NpgsqlConnection connection,
        string databaseName)
    {
        await WaitForConnectionsToDrainAsync(connection, databaseName);

        var quotedDatabaseName = QuoteIdentifier(databaseName);
        await using var cmd = new NpgsqlCommand(
            $"DROP DATABASE IF EXISTS {quotedDatabaseName} WITH (FORCE)",
            connection);
        await cmd.ExecuteNonQueryAsync();
    }

    private static string GetTestDatabaseName(string connectionString)
    {
        var databaseName = new NpgsqlConnectionStringBuilder(connectionString).Database;
        if (string.IsNullOrWhiteSpace(databaseName)
            || !databaseName.StartsWith("db_", StringComparison.Ordinal))
        {
            throw new InvalidOperationException(
                $"Refusing to modify non-test database '{databaseName}'.");
        }

        return databaseName;
    }

    private static string QuoteIdentifier(string identifier)
        => $"\"{identifier.Replace("\"", "\"\"")}\"";

    private static async Task WaitForConnectionsToDrainAsync(
        NpgsqlConnection connection,
        string databaseName)
    {
        var startedAt = Stopwatch.GetTimestamp();
        while (Stopwatch.GetElapsedTime(startedAt) < TimeSpan.FromSeconds(2))
        {
            await using var cmd = new NpgsqlCommand(
                "SELECT NOT EXISTS (SELECT 1 FROM pg_stat_activity WHERE datname = @databaseName)",
                connection);
            cmd.Parameters.AddWithValue("databaseName", databaseName);

            if (await cmd.ExecuteScalarAsync() is true)
            {
                return;
            }

            await Task.Delay(25);
        }
    }

    private string GetConnectionStringForDb(string dbName, bool pooling)
    {
        var builder = new NpgsqlConnectionStringBuilder(MasterConnectionString)
        {
            Database = dbName,
            Pooling = pooling
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

    public async ValueTask DisposeAsync()
    {
        var exceptions = new List<Exception>();

        foreach (var lazyHost in reusableHosts.Values)
        {
            if (!lazyHost.IsValueCreated)
            {
                continue;
            }

            try
            {
                var host = await lazyHost.Value;
                await host.DisposeAsync();
            }
            catch (Exception exception)
            {
                exceptions.Add(exception);
            }
        }

        try
        {
            await _container.DisposeAsync();
        }
        catch (Exception exception)
        {
            exceptions.Add(exception);
        }

        if (exceptions.Count > 0)
        {
            throw new AggregateException(
                "One or more reusable integration test hosts failed to dispose.",
                exceptions);
        }
    }
}
