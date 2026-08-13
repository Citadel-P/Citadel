using DbUp;
using Hosting.Common;
using Microsoft.Extensions.Configuration;
using Npgsql;
using System.Reflection;

namespace Infrastructure.Persistence;

internal static class DbUpgrader
{
    public static async Task Upgrade(IConfiguration config)
    {
        var connectionString = config.GetConnectionString("Postgres")
            ?? throw new InvalidOperationException("Missing Postgres connection string");

        if (string.IsNullOrWhiteSpace(connectionString))
        {
            Console.Error.WriteLine("Connection string not found.");
            throw new ArgumentNullException();
        }

        await EnsureDatabaseCreated(connectionString);

        using var conn = new NpgsqlConnection(connectionString);
        await conn.OpenAsync();

        await using (var cmd = new NpgsqlCommand("SELECT pg_advisory_lock(12345)", conn))
            await cmd.ExecuteNonQueryAsync();

        try
        {
            var upgrader = DeployChanges.To
                .PostgresqlDatabase(connectionString)
                .WithScriptsEmbeddedInAssembly(Assembly.GetExecutingAssembly())
                .WithVariablesDisabled()
                .LogToConsole()
                .Build();

            var result = upgrader.PerformUpgrade();

            if (!result.Successful)
            {
                Console.Error.WriteLine(result.Error);
                throw new Exception(result.Error.Message, result.Error);
            }
        }
        finally
        {
            await using var unlock = new NpgsqlCommand("SELECT pg_advisory_unlock(12345)", conn);
            await unlock.ExecuteNonQueryAsync();
        }

        static async Task EnsureDatabaseCreated(string connectionString)
        {
            if (string.IsNullOrWhiteSpace(connectionString)) return;
            var builder = new NpgsqlConnectionStringBuilder(connectionString);
            var targetDb = builder.Database;
            builder.Database = "postgres";

            using var conn = new NpgsqlConnection(builder.ConnectionString);
            await conn.OpenAsync();

            using var cmd = new NpgsqlCommand("SELECT 1 FROM pg_database WHERE datname = @dbName", conn);
            cmd.Parameters.AddWithValue("dbName", targetDb);

            var exists = await cmd.ExecuteScalarAsync() != null;

            if (!exists)
            {
                using var createCmd = new NpgsqlCommand($@"CREATE DATABASE ""{targetDb}""", conn);
                await createCmd.ExecuteNonQueryAsync();
            }
        }
    }
}
