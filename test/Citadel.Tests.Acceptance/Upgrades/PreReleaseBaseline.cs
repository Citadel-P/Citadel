using DbUp;
using Infrastructure;
using Npgsql;
using System.Reflection;
using System.Security.Cryptography;
using System.Text;

namespace Tests.Acceptance.Upgrades;

internal static class PreReleaseBaseline
{
    private const string BaselineScriptName = "Infrastructure.Scripts.script0001.sql";
    private const string BaselineSchemaHash = "9503f358ddd6121550e8f0758f639f4d9cbb081e3dffa4b72a1f6ffb8bf9e860";
    private const string FixtureDirectory = "Fixtures/Upgrades";

    public static async Task RestoreAsync(
        string connectionString,
        CancellationToken cancellationToken)
    {
        var schema = await ReadFixtureAsync("pre-release-1.0.0.sql", cancellationToken);
        Assert.Equal(BaselineSchemaHash, ComputeNormalizedHash(schema));

        var baseline = DeployChanges.To
            .PostgresqlDatabase(connectionString)
            .WithScript(BaselineScriptName, schema)
            .Build()
            .PerformUpgrade();
        Assert.True(baseline.Successful, baseline.Error?.ToString());

        var seed = await ReadFixtureAsync("pre-release-1.0.0-seed.sql", cancellationToken);
        await using var connection = new NpgsqlConnection(connectionString);
        await connection.OpenAsync(cancellationToken);
        await using var command = new NpgsqlCommand(seed, connection);
        await command.ExecuteNonQueryAsync(cancellationToken);
    }

    public static void ApplyCandidateMigrations(string connectionString)
    {
        var candidate = DeployChanges.To
            .PostgresqlDatabase(connectionString)
            .WithScriptsAndCodeEmbeddedInAssembly(typeof(InfrastructureModule).Assembly)
            .Build()
            .PerformUpgrade();

        Assert.True(candidate.Successful, candidate.Error?.ToString());
    }

    private static async Task<string> ReadFixtureAsync(
        string fileName,
        CancellationToken cancellationToken)
    {
        var path = Path.Combine(
            AppContext.BaseDirectory,
            FixtureDirectory,
            fileName);
        return await File.ReadAllTextAsync(path, cancellationToken);
    }

    private static string ComputeNormalizedHash(string content)
    {
        var normalized = content.Replace("\r\n", "\n", StringComparison.Ordinal);
        return Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(normalized)))
            .ToLowerInvariant();
    }
}
