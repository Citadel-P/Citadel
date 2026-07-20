using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Builds;
using Domain.Entities.Git;
using Domain.Entities.Platforms;
using Domain.Entities.Registries;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Npgsql;

namespace Tests.Integration.Infrastructure.Persistence;

public sealed class BuildRepositoryTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task BuildProjectRepository_ShouldPersistJsonBackedConfiguration()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var actorId = Constants.SystemId;
        var suffix = Guid.NewGuid().ToString("N")[..8];

        Guid projectId;
        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var repository = CreateRepository($"build-repo-{suffix}", actorId);
            var platform = CreatePlatform($"build-platform-{suffix}");
            var registry = CreateRegistry($"build-registry-{suffix}", actorId);

            await uow.GitRepositories.AddAsync(repository, cancellationToken);
            await uow.Platforms.AddAsync(platform, cancellationToken);
            await uow.Registries.AddAsync(registry, cancellationToken);

            var secretId = Guid.CreateVersion7();
            var project = new BuildProject(
                $"build-project-{suffix}",
                "Build persistence integration test",
                enabled: true,
                repository.Id,
                branch: "feature/test",
                contextPath: "services/api",
                dockerfilePath: "services/api/Dockerfile",
                target: "runtime",
                buildArgs:
                [
                    new BuildArgSpec("APP_ENV", "production"),
                    new BuildArgSpec("VERSION", "1.2.3")
                ],
                buildSecrets: [new BuildSecretSpec("npmrc", secretId)],
                platform.Id,
                registry.Id,
                imageRepository: "team/api",
                tagTemplates: ["{branch}-{shortSha}", "latest"],
                webhook: new BuildWebhookConfig(
                    Enabled: true,
                    Provider: WebhookProvider.GitHub,
                    AuthScheme: WebhookAuthScheme.GitHubHmacSha256,
                    Secret: "webhook-secret",
                    BranchFilter: "feature/*"),
                timeoutSeconds: 900,
                retentionRunCount: 7,
                createdByActorId: actorId);

            await uow.BuildProjects.AddAsync(project, cancellationToken);
            await uow.CommitAsync(cancellationToken);
            projectId = project.Id;
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var stored = await uow.BuildProjects.GetAsync(projectId, cancellationToken);

            Assert.NotNull(stored);
            Assert.Equal("feature/test", stored.Branch);
            Assert.Equal("services/api", stored.ContextPath);
            Assert.Equal("services/api/Dockerfile", stored.DockerfilePath);
            Assert.Equal("runtime", stored.Target);
            Assert.Equal(900, stored.TimeoutSeconds);
            Assert.Equal(7, stored.RetentionRunCount);
            Assert.Equal(["{branch}-{shortSha}", "latest"], stored.TagTemplates);
            Assert.Equal("APP_ENV", stored.BuildArgs[0].Name);
            Assert.Equal("production", stored.BuildArgs[0].Value);
            var buildSecret = Assert.Single(stored.BuildSecrets);
            Assert.Equal("npmrc", buildSecret.Id);
            Assert.True(buildSecret.SecretId != Guid.Empty);
            Assert.NotNull(stored.Webhook);
            Assert.True(stored.Webhook.Enabled);
            Assert.Equal("webhook-secret", stored.Webhook.Secret);
            Assert.Equal("feature/*", stored.Webhook.BranchFilter);
        }

        var persisted = await ReadProjectJsonColumnsAsync(projectId, cancellationToken);
        Assert.Contains("\"Name\"", persisted.BuildArgs, StringComparison.Ordinal);
        Assert.Contains("\"APP_ENV\"", persisted.BuildArgs, StringComparison.Ordinal);
        Assert.Contains("\"SecretId\"", persisted.BuildSecrets, StringComparison.Ordinal);
        Assert.Contains("\"{branch}-{shortSha}\"", persisted.TagTemplates, StringComparison.Ordinal);
        Assert.Contains("\"Secret\"", persisted.Webhook, StringComparison.Ordinal);
        Assert.Contains("webhook-secret", persisted.Webhook, StringComparison.Ordinal);
    }

    [Fact]
    public async Task BuildRunRepository_ShouldPruneTerminalRunsAndCascadeLogs()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var actorId = Constants.SystemId;
        var suffix = Guid.NewGuid().ToString("N")[..8];
        Guid projectId;
        Guid newestRunId;
        Guid activeRunId;
        Guid oldestRunId;
        Guid middleRunId;

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var repository = CreateRepository($"retention-repo-{suffix}", actorId);
            var platform = CreatePlatform($"retention-platform-{suffix}");
            var registry = CreateRegistry($"retention-registry-{suffix}", actorId);
            await uow.GitRepositories.AddAsync(repository, cancellationToken);
            await uow.Platforms.AddAsync(platform, cancellationToken);
            await uow.Registries.AddAsync(registry, cancellationToken);

            var project = new BuildProject(
                $"retention-build-{suffix}",
                null,
                enabled: true,
                repository.Id,
                "main",
                ".",
                "Dockerfile",
                null,
                [],
                [],
                platform.Id,
                registry.Id,
                "team/api",
                ["{branch}-{shortSha}"],
                webhook: null,
                timeoutSeconds: BuildProject.DefaultTimeoutSeconds,
                retentionRunCount: 1,
                createdByActorId: actorId);
            await uow.BuildProjects.AddAsync(project, cancellationToken);
            projectId = project.Id;

            var oldestRun = CreateSucceededRun(project, repository, platform, registry, actorId, DateTimeOffset.UtcNow.AddHours(-3), "oldest");
            var middleRun = CreateSucceededRun(project, repository, platform, registry, actorId, DateTimeOffset.UtcNow.AddHours(-2), "middle");
            var newestRun = CreateSucceededRun(project, repository, platform, registry, actorId, DateTimeOffset.UtcNow.AddHours(-1), "newest");
            var activeRun = CreateQueuedRun(project, repository, platform, registry, actorId, "active");
            oldestRunId = oldestRun.Id;
            middleRunId = middleRun.Id;
            newestRunId = newestRun.Id;
            activeRunId = activeRun.Id;

            await uow.BuildRuns.AddAsync(oldestRun, cancellationToken);
            await uow.BuildRuns.AddAsync(middleRun, cancellationToken);
            await uow.BuildRuns.AddAsync(newestRun, cancellationToken);
            await uow.BuildRuns.AddAsync(activeRun, cancellationToken);
            await uow.BuildRunLogs.AddRangeAsync(
                [
                    new BuildRunLogEntry(Guid.CreateVersion7(), oldestRun.Id, DateTimeOffset.UtcNow, "stdout", "oldest"),
                    new BuildRunLogEntry(Guid.CreateVersion7(), middleRun.Id, DateTimeOffset.UtcNow, "stdout", "middle"),
                    new BuildRunLogEntry(Guid.CreateVersion7(), newestRun.Id, DateTimeOffset.UtcNow, "stdout", "newest"),
                    new BuildRunLogEntry(Guid.CreateVersion7(), activeRun.Id, DateTimeOffset.UtcNow, "stdout", "active")
                ],
                cancellationToken);
            await uow.CommitAsync(cancellationToken);
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var deleted = await uow.BuildRuns.DeleteTerminalRunsBeyondRetentionAsync(projectId, keepRunCount: 1, cancellationToken);
            await uow.CommitAsync(cancellationToken);

            Assert.Equal([oldestRunId, middleRunId], deleted.Select(static run => run.Id).ToArray());
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var remainingRuns = (await uow.BuildRuns.GetByProjectAsync(projectId, 10, cancellationToken)).ToArray();
            Assert.Contains(remainingRuns, run => run.Id == newestRunId);
            Assert.Contains(remainingRuns, run => run.Id == activeRunId);
            Assert.DoesNotContain(remainingRuns, run => run.Id == oldestRunId);
            Assert.DoesNotContain(remainingRuns, run => run.Id == middleRunId);

            Assert.Empty(await uow.BuildRunLogs.GetByRunAsync(oldestRunId, cancellationToken));
            Assert.Empty(await uow.BuildRunLogs.GetByRunAsync(middleRunId, cancellationToken));
            Assert.Single(await uow.BuildRunLogs.GetByRunAsync(newestRunId, cancellationToken));
            Assert.Single(await uow.BuildRunLogs.GetByRunAsync(activeRunId, cancellationToken));
        }
    }

    private async Task<ProjectJsonColumns> ReadProjectJsonColumnsAsync(Guid projectId, CancellationToken cancellationToken)
    {
        await using var connection = new NpgsqlConnection(ConnectionString);
        await connection.OpenAsync(cancellationToken);
        await using var command = connection.CreateCommand();
        command.CommandText = """
            SELECT BuildArgs::text, BuildSecrets::text, TagTemplates::text, Webhook::text
            FROM BuildProjects
            WHERE Id = @Id
            LIMIT 1
            """;
        command.Parameters.AddWithValue("Id", projectId);

        await using var reader = await command.ExecuteReaderAsync(cancellationToken);
        Assert.True(await reader.ReadAsync(cancellationToken));
        return new ProjectJsonColumns(
            reader.GetString(0),
            reader.GetString(1),
            reader.GetString(2),
            reader.GetString(3));
    }

    private static GitRepository CreateRepository(string name, Guid actorId)
        => new(
            name,
            null,
            $"https://example.test/{name}.git",
            "main",
            gitAccountId: null,
            createdByActorId: actorId);

    private static Platform CreatePlatform(string name)
        => new(
            name,
            "unix:///var/run/docker.sock",
            networkCount: 0,
            volumeCount: 0,
            imageCount: 0,
            cpuCount: 1,
            memTotal: 1024,
            serverVersion: "test",
            agentVersion: null,
            PlatformStatus.Online,
            PlatformConnectorType.Local,
            new DockerPlatformDescriptor("daemon", 0, 0, 0, 0));

    private static Registry CreateRegistry(string name, Guid actorId)
        => new(
            name,
            "registry.example.test",
            RegistryStatus.Active,
            actorId,
            new CustomRegistry());

    private static BuildRun CreateQueuedRun(
        BuildProject project,
        GitRepository repository,
        Platform platform,
        Registry registry,
        Guid actorId,
        string tag)
        => new(
            project.Id,
            project.Name,
            repository.Id,
            repository.Name,
            project.Branch,
            null,
            project.ContextPath,
            project.DockerfilePath,
            project.Target,
            project.BuildArgs,
            [.. project.BuildSecrets.Select(static secret => secret.Id)],
            new BuildPlatformSnapshot(platform.Id, platform.Name, platform.Address, platform.ConnectorType),
            new BuildRegistrySnapshot(registry.Id, registry.Name, registry.RegistryHost),
            project.ImageRepository,
            project.TagTemplates,
            [$"{registry.RegistryHost}/{project.ImageRepository}:{tag}"],
            BuildRunTrigger.Manual,
            triggerSourceId: null,
            actorId,
            project.TimeoutSeconds);

    private static BuildRun CreateSucceededRun(
        BuildProject project,
        GitRepository repository,
        Platform platform,
        Registry registry,
        Guid actorId,
        DateTimeOffset timestamp,
        string tag)
    {
        var run = CreateQueuedRun(project, repository, platform, registry, actorId, tag);
        run.MarkPreparing(timestamp);
        run.MarkRunning(timestamp.AddSeconds(1));
        run.ResolveCommit($"{tag}-commit", [$"{registry.RegistryHost}/{project.ImageRepository}:{tag}"]);
        run.CompleteSucceeded($"sha256:{tag}", run.ImageReferences, 0, timestamp.AddSeconds(2));
        return run;
    }

    private sealed record ProjectJsonColumns(
        string BuildArgs,
        string BuildSecrets,
        string TagTemplates,
        string Webhook);
}
