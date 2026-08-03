using DbUp;
using Npgsql;
using System.Net.Http.Json;
using System.Text.Json;
using Tests.Acceptance.Infrastructure;

namespace Tests.Acceptance.Upgrades;

[Collection("AcceptancePostgres")]
public sealed class PreReleaseUpgradeTests(AcceptancePostgresFixture postgres)
{
    private static readonly Guid PreservedInstanceId =
        Guid.Parse("019f0000-0000-7000-8000-000000000001");

    [Fact]
    public async Task Candidate_ShouldUpgradeBaselinePreserveStateAndOperateAfterRestart()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var connectionString = await postgres.CreateDatabaseAsync(cancellationToken);
        await PreReleaseBaseline.RestoreAsync(connectionString, cancellationToken);
        var workingDirectory = Path.Combine(
            Path.GetTempPath(),
            $"citadel-upgrade-acceptance-{Guid.NewGuid():N}");
        Directory.CreateDirectory(workingDirectory);

        try
        {
            await AssertCandidateStateAndCreateTagAsync(
                connectionString,
                workingDirectory,
                expectedTags: ["Pre-release preserved"],
                createTag: "Created after upgrade",
                cancellationToken);

            await AssertCandidateStateAndCreateTagAsync(
                connectionString,
                workingDirectory,
                expectedTags: ["Pre-release preserved", "Created after upgrade"],
                createTag: "Created after restart",
                cancellationToken);
        }
        finally
        {
            if (Directory.Exists(workingDirectory))
                Directory.Delete(workingDirectory, recursive: true);
        }

        await using var connection = new NpgsqlConnection(connectionString);
        await connection.OpenAsync(cancellationToken);

        Assert.Equal(
            PreservedInstanceId,
            await ScalarAsync<Guid>(
                connection,
                "SELECT instanceid FROM citadelinstanceidentity WHERE id = 1;",
                cancellationToken));
        Assert.Equal(
            1L,
            await ScalarAsync<long>(
                connection,
                "SELECT COUNT(*) FROM users WHERE email = 'admin@citadel.local';",
                cancellationToken));
        Assert.Equal(
            1L,
            await ScalarAsync<long>(
                connection,
                "SELECT COUNT(*) FROM teams WHERE name = 'Operators';",
                cancellationToken));
        Assert.Equal(
            2L,
            await ScalarAsync<long>(
                connection,
                """
                SELECT COUNT(*)
                FROM actions
                WHERE name IN ('Prune images', 'Restart unhealthy stacks')
                  AND enabled = FALSE
                  AND scheduleenabled = FALSE;
                """,
                cancellationToken));
        Assert.Equal(
            3L,
            await ScalarAsync<long>(
                connection,
                """
                SELECT COUNT(*)
                FROM information_schema.columns
                WHERE table_schema = 'public'
                  AND (table_name, column_name) IN (
                    ('platforms', 'clusterid'),
                    ('deployments', 'lockedplatformtype'),
                    ('stacks', 'lockedplatformtype'));
                """,
                cancellationToken));
        Assert.Equal(
            1L,
            await ScalarAsync<long>(
                connection,
                """
                SELECT COUNT(*)
                FROM pg_indexes
                WHERE schemaname = 'public'
                  AND indexname = 'ix_platforms_clusterid';
                """,
                cancellationToken));
        Assert.Equal(
            3L,
            await ScalarAsync<long>(
                connection,
                """
                SELECT COUNT(*)
                FROM tags
                WHERE name IN (
                    'Pre-release preserved',
                    'Created after upgrade',
                    'Created after restart');
                """,
                cancellationToken));
        Assert.Equal(
            2L,
            await ScalarAsync<long>(
                connection,
                """
                SELECT COUNT(*)
                FROM pg_indexes
                WHERE schemaname = 'public'
                  AND indexname IN (
                    'ix_containerstats_created',
                    'ix_platformstats_created');
                """,
                cancellationToken));
    }

    [Fact]
    public async Task FailingMigration_ShouldRollbackSchemaAndRemainUnjournaled()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var connectionString = await postgres.CreateDatabaseAsync(cancellationToken);
        await PreReleaseBaseline.RestoreAsync(connectionString, cancellationToken);

        const string scriptName = "Infrastructure.Scripts.script9999_failure_probe.sql";
        const string failingScript = """
            START TRANSACTION;
            CREATE TABLE migration_failure_probe (id integer PRIMARY KEY);
            INSERT INTO migration_failure_probe (id) VALUES (1);
            SELECT 1 / 0;
            COMMIT;
            """;

        var result = DeployChanges.To
            .PostgresqlDatabase(connectionString)
            .WithScript(scriptName, failingScript)
            .Build()
            .PerformUpgrade();

        Assert.False(result.Successful, result.Error?.ToString());

        await using var connection = new NpgsqlConnection(connectionString);
        await connection.OpenAsync(cancellationToken);
        Assert.Null(
            await ScalarAsync<string?>(
                connection,
                "SELECT to_regclass('public.migration_failure_probe')::text;",
                cancellationToken));
        Assert.Equal(
            0L,
            await ScalarAsync<long>(
                connection,
                "SELECT COUNT(*) FROM schemaversions WHERE scriptname = @scriptName;",
                cancellationToken,
                new NpgsqlParameter("scriptName", scriptName)));

        PreReleaseBaseline.ApplyCandidateMigrations(connectionString);
    }

    [Fact]
    public async Task CandidateMigration_ShouldEnforceSingleActiveActionRunPerAction()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var connectionString = await postgres.CreateDatabaseAsync(cancellationToken);
        await PreReleaseBaseline.RestoreAsync(connectionString, cancellationToken);

        await using var connection = new NpgsqlConnection(connectionString);
        await connection.OpenAsync(cancellationToken);
        await using (var seed = new NpgsqlCommand(
            """
            INSERT INTO actions (
                id, alertonfailure, code, createdbyactorid, enabled, name,
                runasactorid, scheduleenabled, scheduletimezone, timeoutseconds)
            VALUES (
                '019f0000-0000-7000-8000-000000000100', FALSE, '',
                '00000000-0000-0000-0000-000000000001', TRUE, 'Active run constraint probe',
                '00000000-0000-0000-0000-000000000001', FALSE, 'UTC', 60);

            INSERT INTO actionruns (
                id, actionid, actionname, codehash, codesnapshot, queuedat,
                runasactorid, startedat, status, timeoutseconds, trigger)
            VALUES (
                '019f0000-0000-7000-8000-000000000101',
                '019f0000-0000-7000-8000-000000000100',
                'Active run constraint probe', '', '', TIMESTAMPTZ '2026-07-31T10:00:00Z',
                '00000000-0000-0000-0000-000000000001',
                TIMESTAMPTZ '2026-07-31T10:00:01Z', 'Running', 60, 'Manual');
            """,
            connection))
        {
            await seed.ExecuteNonQueryAsync(cancellationToken);
        }

        await using var duplicate = new NpgsqlCommand(
            """
            INSERT INTO actionruns (
                id, actionid, actionname, codehash, codesnapshot, queuedat,
                runasactorid, status, timeoutseconds, trigger)
            VALUES (
                '019f0000-0000-7000-8000-000000000102',
                '019f0000-0000-7000-8000-000000000100',
                'Active run constraint probe', '', '', TIMESTAMPTZ '2026-07-31T10:01:00Z',
                '00000000-0000-0000-0000-000000000001',
                'Queued', 60, 'Manual');
            """,
            connection);
        var exception = await Assert.ThrowsAsync<PostgresException>(
            () => duplicate.ExecuteNonQueryAsync(cancellationToken));

        Assert.Equal(PostgresErrorCodes.UniqueViolation, exception.SqlState);
        Assert.Equal("ix_actionruns_active_action", exception.ConstraintName);
    }

    private static async Task AssertCandidateStateAndCreateTagAsync(
        string connectionString,
        string workingDirectory,
        IReadOnlyCollection<string> expectedTags,
        string createTag,
        CancellationToken cancellationToken)
    {
        await using var candidate = await CandidateApplicationProcess.StartAsync(
            connectionString,
            workingDirectory,
            cancellationToken);
        var client = candidate.Client;
        await candidate.AuthenticateAsAdminAsync(cancellationToken);

        var tags = await client.GetFromJsonAsync<JsonElement>(
            "/api/v1/tags/",
            cancellationToken);
        var tagNames = tags
            .GetProperty("tags")
            .EnumerateArray()
            .Select(tag => tag.GetProperty("name").GetString())
            .ToArray();
        foreach (var expectedTag in expectedTags)
            Assert.Contains(expectedTag, tagNames);

        var create = await client.PostAsJsonAsync(
            "/api/v1/tags/",
            new
            {
                name = createTag,
                color = "#16a34a"
            },
            cancellationToken);
        Assert.True(
            create.IsSuccessStatusCode,
            $"""
            Candidate tag creation failed with HTTP {(int)create.StatusCode}.
            {await create.Content.ReadAsStringAsync(cancellationToken)}
            {candidate.Output}
            """);
    }

    private static async Task<T> ScalarAsync<T>(
        NpgsqlConnection connection,
        string sql,
        CancellationToken cancellationToken,
        params NpgsqlParameter[] parameters)
    {
        await using var command = new NpgsqlCommand(sql, connection);
        command.Parameters.AddRange(parameters);
        var value = await command.ExecuteScalarAsync(cancellationToken);
        if (value is null or DBNull)
            return default!;

        return (T)value!;
    }
}
