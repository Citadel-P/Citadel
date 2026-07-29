using System.Net.Http.Json;
using Npgsql;
using Tests.Acceptance.Infrastructure;

namespace Tests.Acceptance.Setup;

[Collection("AcceptancePostgres")]
public sealed class FirstRunSetupAcceptanceTests(
    AcceptancePostgresFixture postgres)
{
    [Fact]
    public async Task PasswordFileBootstrap_ShouldInitializeOnceAndIgnoreOptionsAfterRestart()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var connectionString = await postgres.CreateDatabaseAsync(cancellationToken);
        var workingDirectory = Path.Combine(
            Path.GetTempPath(),
            $"citadel-bootstrap-{Guid.NewGuid():N}");
        var passwordFile = Path.Combine(workingDirectory, "admin-password");
        Directory.CreateDirectory(workingDirectory);
        await File.WriteAllTextAsync(
            passwordFile,
            "citadel-acceptance-admin-password\n",
            cancellationToken);

        var options = new CandidateApplicationOptions(
            BootstrapAdminName: "admin",
            BootstrapAdminEmail: "admin@citadel.local",
            BootstrapAdminPasswordFile: passwordFile);

        try
        {
            await using (var candidate = await CandidateApplicationProcess.StartAsync(
                connectionString,
                workingDirectory,
                cancellationToken,
                options))
            {
                var status = await candidate.Client.GetFromJsonAsync<SetupStatus>(
                    "/api/v1/setup/status",
                    cancellationToken);
                Assert.False(status?.RequiresSetup);
                await candidate.AuthenticateAsAdminAsync(cancellationToken);
            }

            File.Delete(passwordFile);

            await using (var restarted = await CandidateApplicationProcess.StartAsync(
                connectionString,
                workingDirectory,
                cancellationToken,
                options))
            {
                await restarted.AuthenticateAsAdminAsync(cancellationToken);
            }

            await using var connection = new NpgsqlConnection(connectionString);
            await connection.OpenAsync(cancellationToken);
            await using var command = new NpgsqlCommand(
                """
                SELECT
                    (
                        SELECT COUNT(*)
                        FROM users
                        WHERE name = 'admin'
                          AND email = 'admin@citadel.local'
                    ),
                    (
                        SELECT COUNT(*)
                        FROM actions
                        WHERE name IN ('Prune images', 'Restart unhealthy stacks')
                    );
                """,
                connection);
            await using var reader = await command.ExecuteReaderAsync(cancellationToken);
            Assert.True(await reader.ReadAsync(cancellationToken));
            Assert.Equal(1L, reader.GetInt64(0));
            Assert.Equal(2L, reader.GetInt64(1));
        }
        finally
        {
            await DeleteDirectoryAsync(workingDirectory);
        }
    }

    [Fact]
    public async Task PartialBootstrapConfiguration_ShouldFailAndLeaveSetupPending()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var connectionString = await postgres.CreateDatabaseAsync(cancellationToken);
        var workingDirectory = Path.Combine(
            Path.GetTempPath(),
            $"citadel-bootstrap-partial-{Guid.NewGuid():N}");
        Directory.CreateDirectory(workingDirectory);

        try
        {
            CandidateApplicationProcess? candidate = null;
            string failureOutput;
            try
            {
                candidate = await CandidateApplicationProcess.StartAsync(
                    connectionString,
                    workingDirectory,
                    cancellationToken,
                    new CandidateApplicationOptions(
                        BootstrapAdminName: "admin"));
                var exitCode = await candidate.WaitForExitAsync(
                    TimeSpan.FromSeconds(10),
                    cancellationToken);
                Assert.NotEqual(0, exitCode);
                failureOutput = candidate.Output;
            }
            catch (InvalidOperationException exception)
            {
                failureOutput = exception.Message;
            }
            finally
            {
                if (candidate is not null)
                    await candidate.DisposeAsync();
            }

            Assert.Contains(
                "Incomplete bootstrap administrator configuration",
                failureOutput);

            await using (var connection = new NpgsqlConnection(connectionString))
            {
                await connection.OpenAsync(cancellationToken);
                await using var command = new NpgsqlCommand(
                    """
                    SELECT
                        (SELECT COUNT(*) FROM users),
                        (SELECT initializedat IS NULL FROM instancesetupstates WHERE id = 1);
                    """,
                    connection);
                await using var reader = await command.ExecuteReaderAsync(cancellationToken);
                Assert.True(await reader.ReadAsync(cancellationToken));
                Assert.Equal(0L, reader.GetInt64(0));
                Assert.True(reader.GetBoolean(1));
            }

            await using (var restarted = await CandidateApplicationProcess.StartAsync(
                connectionString,
                workingDirectory,
                cancellationToken))
            {
                var status = await restarted.Client.GetFromJsonAsync<SetupStatus>(
                    "/api/v1/setup/status",
                    cancellationToken);
                Assert.True(status?.RequiresSetup);
            }
        }
        finally
        {
            await DeleteDirectoryAsync(workingDirectory);
        }
    }

    private static async Task DeleteDirectoryAsync(string path)
    {
        for (var attempt = 0; attempt < 5; attempt++)
        {
            if (!Directory.Exists(path))
                return;

            try
            {
                Directory.Delete(path, recursive: true);
                return;
            }
            catch (IOException)
            {
                if (attempt == 4)
                    return;
            }

            await Task.Delay(TimeSpan.FromMilliseconds(200));
        }
    }

    private sealed record SetupStatus(bool RequiresSetup);
}
