using System.Net;
using System.Net.Http.Json;
using System.Runtime.CompilerServices;
using System.Text;
using System.Text.Json;
using Application.Services.Backups;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Volumes;
using Domain.Entities.Backups;
using Domain.Entities.Platforms;
using Domain.Entities.SwarmServices;
using Domain.Entities.Tags;
using Hosting.Common;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Moq;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Backups;

public sealed class BackupEndpointTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<IBackupRepositoryDestinationService> destinationService = new();
    private readonly Mock<ISwarmNodeRuntimeConnector> swarmNodeRuntimeConnector = new(MockBehavior.Strict);
    private Guid platformId;
    private Guid swarmServiceId;
    private Guid tagId;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        destinationService
            .Setup(service => service.ValidateAsync(
                It.IsAny<Guid>(),
                It.IsAny<BackupExecutionContext>(),
                It.IsAny<CancellationToken>()))
            .Returns((Guid repositoryId, BackupExecutionContext context, CancellationToken _) =>
                ValueTask.FromResult(Result.Success(new BackupRepositoryValidation(
                    repositoryId,
                    context.Location,
                    context.PlatformId,
                    BackupRepositoryValidationStatus.Ready,
                    DateTimeOffset.UtcNow,
                    null,
                    null))));
        destinationService
            .Setup(service => service.InitializeAsync(
                It.IsAny<Guid>(),
                It.IsAny<BackupExecutionContext>(),
                It.IsAny<CancellationToken>()))
            .Returns(ValueTask.FromResult(Result.Success()));
        destinationService
            .Setup(service => service.CheckAsync(
                It.IsAny<Guid>(),
                It.IsAny<BackupExecutionContext>(),
                It.IsAny<CancellationToken>()))
            .Returns(ValueTask.FromResult(Result.Success()));
        destinationService
            .Setup(service => service.PruneAsync(
                It.IsAny<Guid>(),
                It.IsAny<BackupExecutionContext>(),
                It.IsAny<CancellationToken>()))
            .Returns(ValueTask.FromResult(Result.Success()));

        services.ReplaceService<IBackupRepositoryDestinationService>(destinationService.Object);
        services.ReplaceService<IBackupRunExecutionService>(new FakeBackupRunExecutionService());
        services.ReplaceService<IBackupRestoreRunExecutionService>(new FakeBackupRestoreRunExecutionService());
        services.ReplaceService<ISwarmNodeRuntimeConnector>(swarmNodeRuntimeConnector.Object);

        swarmNodeRuntimeConnector
            .Setup(connector => connector.InspectContainerAsync(
                It.IsAny<Platform>(),
                "worker-1",
                "container-backup-task",
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(CreateContainerInspection()));
        swarmNodeRuntimeConnector
            .Setup(connector => connector.InspectVolumeAsync(
                It.IsAny<Platform>(),
                "worker-1",
                "service-data",
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(CreateDockerVolume("service-data")));
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var platform = new Platform(
            "backup-endpoint-platform",
            "unix:///var/run/docker.sock",
            networkCount: 0,
            volumeCount: 1,
            imageCount: 0,
            cpuCount: 2,
            memTotal: 2048,
            serverVersion: "test",
            agentVersion: null,
            PlatformStatus.Online,
            PlatformConnectorType.Local,
            new DockerPlatformDescriptor("backup-endpoint-daemon", 0, 0, 0, 0));
        var tag = Tag.Create("backup-endpoint-tag", "#2A66AA", Constants.SystemId);
        var swarmPlatform = CreateSwarmPlatform();
        var swarmService = SwarmService.AdoptExisting(
            "backup-service",
            null,
            swarmPlatform.Id,
            Constants.SystemId,
            "backup-service",
            "docker-backup-service",
            1,
            "runtime-hash",
            new SwarmServiceSpec
            {
                Image = new SwarmExternalImage(Guid.CreateVersion7(), "postgres:17"),
                Replicas = 1,
                Mounts = [new SwarmServiceMount(SwarmServiceMountKind.Volume, "service-data", "/var/lib/postgresql/data")]
            });

        await uow.Platforms.AddAsync(platform, cancellationToken);
        await uow.Platforms.AddAsync(swarmPlatform, cancellationToken);
        await uow.SwarmServices.AddAsync(swarmService, cancellationToken);
        await uow.Tags.AddAsync(tag, cancellationToken);
        await uow.CommitAsync(cancellationToken);

        var observedAt = DateTimeOffset.UtcNow;
        await uow.Swarm.ReplaceAsync(
            swarmPlatform.Id,
            new SwarmProjectionSnapshot(
                [CreateSwarmNode(swarmPlatform.Id, observedAt)],
                [new SwarmServiceProjection(
                    swarmPlatform.Id,
                    "docker-backup-service",
                    1,
                    "backup-service",
                    "Replicated",
                    "postgres:17",
                    1,
                    1,
                    "Completed",
                    null,
                    [],
                    [],
                    [],
                    [],
                    new Dictionary<string, string>(),
                    observedAt,
                    observedAt,
                    observedAt,
                    false,
                    SwarmServiceOwnership.CitadelService,
                    SwarmServiceId: swarmService.Id)],
                [new SwarmTaskProjection(
                    swarmPlatform.Id,
                    "task-backup-service",
                    1,
                    "backup-service.1",
                    "docker-backup-service",
                    "backup-service",
                    1,
                    "worker-1",
                    "worker-one",
                    "running",
                    "running",
                    null,
                    null,
                    "postgres:17",
                    [],
                    observedAt,
                    observedAt,
                    observedAt,
                    observedAt,
                    false,
                    "container-backup-task")],
                [],
                [],
                []),
            cancellationToken);
        await uow.CommitAsync(cancellationToken);

        platformId = platform.Id;
        swarmServiceId = swarmService.Id;
        tagId = tag.Id;
    }

    [Fact]
    public async Task BackupEndpoints_ShouldPersistRepositoryPolicyRunAndRestoreLifecycles()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var passwordSecretId = await CreateSecretAsync(
            "BACKUP_ENDPOINT_PASSWORD",
            "integration-password",
            cancellationToken);

        using var createRepositoryResponse = await Client.PostAsJsonAsync(
            "/api/v1/backupRepositories",
            new
            {
                name = "backup-endpoint-repository",
                description = "created through the API",
                spec = new Dictionary<string, object?>
                {
                    ["$type"] = "FileSystem",
                    ["location"] = "Core",
                    ["platformId"] = null,
                    ["path"] = "/tmp/citadel-backup-endpoint"
                },
                passwordSecretId
            },
            cancellationToken);
        createRepositoryResponse.EnsureSuccessStatusCode();
        var repositoryId = await ReadIdAsync(createRepositoryResponse, cancellationToken);

        using var listRepositoriesResponse = await Client.GetAsync("/api/v1/backupRepositories", cancellationToken);
        listRepositoriesResponse.EnsureSuccessStatusCode();
        using var getRepositoryResponse = await Client.GetAsync(
            $"/api/v1/backupRepositories/{repositoryId:D}",
            cancellationToken);
        getRepositoryResponse.EnsureSuccessStatusCode();
        using var updateRepositoryResponse = await SendMergePatchAsync(
            $"/api/v1/backupRepositories/{repositoryId:D}",
            """{"description":"updated repository"}""",
            cancellationToken);
        updateRepositoryResponse.EnsureSuccessStatusCode();

        var executionContext = new { location = "Core", platformId = (Guid?)null };
        using var validateResponse = await Client.PostAsJsonAsync(
            $"/api/v1/backupRepositories/{repositoryId:D}/validate",
            executionContext,
            cancellationToken);
        validateResponse.EnsureSuccessStatusCode();
        using var initializeResponse = await Client.PostAsJsonAsync(
            $"/api/v1/backupRepositories/{repositoryId:D}/initialize",
            executionContext,
            cancellationToken);
        Assert.Equal(HttpStatusCode.NoContent, initializeResponse.StatusCode);
        using var checkResponse = await Client.PostAsJsonAsync(
            $"/api/v1/backupRepositories/{repositoryId:D}/check",
            executionContext,
            cancellationToken);
        Assert.Equal(HttpStatusCode.NoContent, checkResponse.StatusCode);
        using var pruneResponse = await Client.PostAsJsonAsync(
            $"/api/v1/backupRepositories/{repositoryId:D}/prune",
            executionContext,
            cancellationToken);
        Assert.Equal(HttpStatusCode.NoContent, pruneResponse.StatusCode);

        using var createPolicyResponse = await Client.PostAsJsonAsync(
            "/api/v1/backupPolicies",
            new
            {
                name = "backup-endpoint-policy",
                description = "created through the API",
                source = new Dictionary<string, object?>
                {
                    ["$type"] = "DockerVolume",
                    ["platformId"] = platformId,
                    ["volumeName"] = "source-data",
                    ["consistency"] = "Live"
                },
                backupRepositoryId = repositoryId,
                enabled = true,
                cron = (string?)null,
                timeZone = (string?)null,
                webhook = (object?)null,
                keepLastSuccessful = 5,
                timeoutSeconds = 300,
                alertOnFailure = true,
                runAsActorId = Constants.DefaultAdminId,
                tagIds = Array.Empty<Guid>()
            },
            cancellationToken);
        createPolicyResponse.EnsureSuccessStatusCode();
        var policyId = await ReadIdAsync(createPolicyResponse, cancellationToken);

        using var listPoliciesResponse = await Client.GetAsync("/api/v1/backupPolicies", cancellationToken);
        listPoliciesResponse.EnsureSuccessStatusCode();
        using var getPolicyResponse = await Client.GetAsync(
            $"/api/v1/backupPolicies/{policyId:D}",
            cancellationToken);
        getPolicyResponse.EnsureSuccessStatusCode();
        using var summariesResponse = await Client.GetAsync(
            $"/api/v1/backupPolicies/platform-summaries?platformIds={platformId:D}",
            cancellationToken);
        summariesResponse.EnsureSuccessStatusCode();

        using var servicePreviewResponse = await Client.GetAsync(
            $"/api/v1/swarmServices/{swarmServiceId:D}/backup-source-preview",
            cancellationToken);
        servicePreviewResponse.EnsureSuccessStatusCode();
        using (var servicePreview = await JsonDocument.ParseAsync(
            await servicePreviewResponse.Content.ReadAsStreamAsync(cancellationToken),
            cancellationToken: cancellationToken))
        {
            var volume = Assert.Single(servicePreview.RootElement.GetProperty("volumes").EnumerateArray());
            Assert.Equal("service-data", volume.GetProperty("name").GetString());
            Assert.Equal("worker-1", volume.GetProperty("dockerNodeId").GetString());
            Assert.Equal("worker-one", volume.GetProperty("nodeHostname").GetString());
        }

        using var replaceTagsResponse = await Client.PutAsJsonAsync(
            $"/api/v1/backupPolicies/{policyId:D}/tags",
            new { tagIds = new[] { tagId } },
            cancellationToken);
        replaceTagsResponse.EnsureSuccessStatusCode();
        using var getTagsResponse = await Client.GetAsync(
            $"/api/v1/backupPolicies/{policyId:D}/tags",
            cancellationToken);
        getTagsResponse.EnsureSuccessStatusCode();

        using var updatePolicyResponse = await SendMergePatchAsync(
            $"/api/v1/backupPolicies/{policyId:D}",
            """{"description":"updated policy","keepLastSuccessful":7}""",
            cancellationToken);
        updatePolicyResponse.EnsureSuccessStatusCode();
        using var renamePolicyResponse = await Client.PostAsJsonAsync(
            "/api/v1/backupPolicies/rename",
            new { id = policyId, name = "backup-endpoint-renamed" },
            cancellationToken);
        renamePolicyResponse.EnsureSuccessStatusCode();
        using var metadataPolicyResponse = await SendMergePatchAsync(
            $"/api/v1/backupPolicies/{policyId:D}/_metadata",
            """{"description":"metadata policy"}""",
            cancellationToken);
        metadataPolicyResponse.EnsureSuccessStatusCode();

        var cancelledRunId = await QueueBackupRunAsync(policyId, cancellationToken);
        await AssertBackupRunReadEndpointsAsync(policyId, cancelledRunId, cancellationToken);
        using var cancelRunResponse = await Client.PostAsync(
            $"/api/v1/backupRuns/{cancelledRunId:D}/cancel",
            content: null,
            cancellationToken);
        Assert.Equal(HttpStatusCode.NoContent, cancelRunResponse.StatusCode);

        var successfulRunId = await QueueBackupRunAsync(policyId, cancellationToken);
        await CompleteBackupRunAsync(policyId, successfulRunId, cancellationToken);

        using var restoreResponse = await Client.PostAsJsonAsync(
            $"/api/v1/backupRuns/{successfulRunId:D}/restoreVolume",
            new
            {
                targetPlatformId = platformId,
                targetVolumeName = "restored-data",
                overwriteExisting = false
            },
            cancellationToken);
        restoreResponse.EnsureSuccessStatusCode();
        var restoreRunId = await ReadIdAsync(restoreResponse, cancellationToken);
        await AssertRestoreRunReadEndpointsAsync(policyId, successfulRunId, restoreRunId, cancellationToken);
        await using (var scope = Services.CreateAsyncScope())
        {
            var persistedRestore = await scope.ServiceProvider.GetRequiredService<IUnitOfWork>()
                .BackupRestoreRuns.GetAsync(restoreRunId, cancellationToken);
            Assert.NotNull(persistedRestore);
            Assert.Null(persistedRestore.SourceBackupRunItemId);
        }
        using var cancelRestoreResponse = await Client.PostAsync(
            $"/api/v1/backupRestoreRuns/{restoreRunId:D}/cancel",
            content: null,
            cancellationToken);
        Assert.Equal(HttpStatusCode.NoContent, cancelRestoreResponse.StatusCode);

        using var streamedBackupResponse = await Client.PostAsJsonAsync(
            $"/api/v1/backupPolicies/{policyId:D}/run",
            new { trigger = "Manual", triggerSourceId = (Guid?)null },
            cancellationToken);
        streamedBackupResponse.EnsureSuccessStatusCode();
        _ = await streamedBackupResponse.Content.ReadAsStringAsync(cancellationToken);
        var streamedBackupRunId = await GetOnlyActiveBackupRunIdAsync(policyId, cancellationToken);
        using var cancelStreamedBackupResponse = await Client.PostAsync(
            $"/api/v1/backupRuns/{streamedBackupRunId:D}/cancel",
            content: null,
            cancellationToken);
        Assert.Equal(HttpStatusCode.NoContent, cancelStreamedBackupResponse.StatusCode);

        using var streamedRestoreResponse = await Client.PostAsJsonAsync(
            $"/api/v1/backupRuns/{successfulRunId:D}/restoreVolume/run",
            new
            {
                targetPlatformId = platformId,
                targetVolumeName = "stream-restored-data",
                overwriteExisting = false
            },
            cancellationToken);
        streamedRestoreResponse.EnsureSuccessStatusCode();
        _ = await streamedRestoreResponse.Content.ReadAsStringAsync(cancellationToken);
        var streamedRestoreRunId = await GetOnlyActiveRestoreRunIdAsync(successfulRunId, cancellationToken);
        using var cancelStreamedRestoreResponse = await Client.PostAsync(
            $"/api/v1/backupRestoreRuns/{streamedRestoreRunId:D}/cancel",
            content: null,
            cancellationToken);
        Assert.Equal(HttpStatusCode.NoContent, cancelStreamedRestoreResponse.StatusCode);

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var repository = await uow.BackupRepositories.GetAsync(repositoryId, cancellationToken);
            var policy = await uow.BackupPolicies.GetAsync(policyId, cancellationToken);
            var cancelledRun = await uow.BackupRuns.GetAsync(cancelledRunId, cancellationToken);
            var successfulRun = await uow.BackupRuns.GetAsync(successfulRunId, cancellationToken);
            var cancelledRestore = await uow.BackupRestoreRuns.GetAsync(restoreRunId, cancellationToken);

            Assert.NotNull(repository);
            Assert.Equal("updated repository", repository.Description);
            Assert.NotNull(policy);
            Assert.Equal("backup-endpoint-renamed", policy.Name);
            Assert.Equal("metadata policy", policy.Description);
            Assert.Equal(7, policy.KeepLastSuccessful);
            Assert.Contains(policy.Tags, tag => tag.Id == tagId);
            Assert.Equal(BackupRunStatus.Cancelled, cancelledRun?.Status);
            Assert.Equal(BackupRunStatus.Succeeded, successfulRun?.Status);
            Assert.Equal(BackupSnapshotAvailability.Available, successfulRun?.SnapshotAvailability);
            Assert.Equal(BackupRestoreStatus.Cancelled, cancelledRestore?.Status);
        }

        using var archivePolicyResponse = await Client.DeleteAsync(
            $"/api/v1/backupPolicies/{policyId:D}",
            cancellationToken);
        Assert.Equal(HttpStatusCode.NoContent, archivePolicyResponse.StatusCode);
        using var archiveRepositoryResponse = await Client.DeleteAsync(
            $"/api/v1/backupRepositories/{repositoryId:D}",
            cancellationToken);
        Assert.Equal(HttpStatusCode.NoContent, archiveRepositoryResponse.StatusCode);

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var archivedPolicy = await uow.BackupPolicies.GetAsync(policyId, cancellationToken, includeArchived: true);
            var archivedRepository = await uow.BackupRepositories.GetAsync(repositoryId, cancellationToken, includeArchived: true);
            Assert.NotNull(archivedPolicy?.ArchivedAt);
            Assert.NotNull(archivedRepository?.ArchivedAt);
        }

        destinationService.Verify(
            service => service.ValidateAsync(
                repositoryId,
                It.Is<BackupExecutionContext>(context =>
                    context.Location == BackupExecutionLocation.Core && context.PlatformId == null),
                It.IsAny<CancellationToken>()),
            Times.Once);
        destinationService.Verify(service => service.InitializeAsync(repositoryId, It.IsAny<BackupExecutionContext>(), It.IsAny<CancellationToken>()), Times.Once);
        destinationService.Verify(service => service.CheckAsync(repositoryId, It.IsAny<BackupExecutionContext>(), It.IsAny<CancellationToken>()), Times.Once);
        destinationService.Verify(service => service.PruneAsync(repositoryId, It.IsAny<BackupExecutionContext>(), It.IsAny<CancellationToken>()), Times.Once);
    }

    private async Task<Guid> QueueBackupRunAsync(Guid policyId, CancellationToken cancellationToken)
    {
        using var response = await Client.PostAsJsonAsync(
            $"/api/v1/backupPolicies/{policyId:D}/runs",
            new { trigger = "Manual", triggerSourceId = (Guid?)null },
            cancellationToken);
        response.EnsureSuccessStatusCode();
        return await ReadIdAsync(response, cancellationToken);
    }

    private async Task AssertBackupRunReadEndpointsAsync(
        Guid policyId,
        Guid runId,
        CancellationToken cancellationToken)
    {
        using var listResponse = await Client.GetAsync(
            $"/api/v1/backupRuns?policyId={policyId:D}&limit=10",
            cancellationToken);
        listResponse.EnsureSuccessStatusCode();
        using var getResponse = await Client.GetAsync($"/api/v1/backupRuns/{runId:D}", cancellationToken);
        getResponse.EnsureSuccessStatusCode();
        using var logsResponse = await Client.GetAsync($"/api/v1/backupRuns/{runId:D}/logs", cancellationToken);
        logsResponse.EnsureSuccessStatusCode();
        using var eventsResponse = await Client.GetAsync($"/api/v1/backupRuns/{runId:D}/events", cancellationToken);
        eventsResponse.EnsureSuccessStatusCode();
    }

    private async Task AssertRestoreRunReadEndpointsAsync(
        Guid policyId,
        Guid backupRunId,
        Guid restoreRunId,
        CancellationToken cancellationToken)
    {
        using var listResponse = await Client.GetAsync(
            $"/api/v1/backupRestoreRuns?backupRunId={backupRunId:D}&policyId={policyId:D}&limit=10",
            cancellationToken);
        listResponse.EnsureSuccessStatusCode();
        using var getResponse = await Client.GetAsync(
            $"/api/v1/backupRestoreRuns/{restoreRunId:D}",
            cancellationToken);
        getResponse.EnsureSuccessStatusCode();
        using var logsResponse = await Client.GetAsync(
            $"/api/v1/backupRestoreRuns/{restoreRunId:D}/logs",
            cancellationToken);
        logsResponse.EnsureSuccessStatusCode();
        using var eventsResponse = await Client.GetAsync(
            $"/api/v1/backupRestoreRuns/{restoreRunId:D}/events",
            cancellationToken);
        eventsResponse.EnsureSuccessStatusCode();
    }

    private async Task CompleteBackupRunAsync(
        Guid policyId,
        Guid runId,
        CancellationToken cancellationToken)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var run = await uow.BackupRuns.GetAsync(runId, cancellationToken)
            ?? throw new InvalidOperationException("Queued backup run was not persisted.");
        var now = DateTimeOffset.UtcNow;
        run.MarkRunning(now);
        var item = new BackupRunItem(run.Id, platformId, "source-data");
        item.MarkRunning(now);
        item.CompleteSucceeded("snapshot-endpoint", null, 1, 128, 64, now.AddSeconds(1));
        await uow.BackupRunItems.AddRangeAsync([item], cancellationToken);
        run.CompleteSucceeded("snapshot-endpoint", null, 1, 128, 64, [], now.AddSeconds(1));
        await uow.BackupRuns.FinishRunAndMarkPolicyIdleAsync(
            run,
            policyId,
            successful: true,
            run.CompletedAt ?? now,
            cancellationToken);
        await uow.CommitAsync(cancellationToken);
    }

    private async Task<Guid> GetOnlyActiveBackupRunIdAsync(
        Guid policyId,
        CancellationToken cancellationToken)
    {
        await using var scope = Services.CreateAsyncScope();
        var runs = await scope.ServiceProvider.GetRequiredService<IUnitOfWork>()
            .BackupRuns.GetByPolicyAsync(policyId, 20, cancellationToken);
        return Assert.Single(runs, run => run.Status is BackupRunStatus.Queued or BackupRunStatus.Preparing or BackupRunStatus.Running).Id;
    }

    private async Task<Guid> GetOnlyActiveRestoreRunIdAsync(
        Guid backupRunId,
        CancellationToken cancellationToken)
    {
        await using var scope = Services.CreateAsyncScope();
        var runs = await scope.ServiceProvider.GetRequiredService<IUnitOfWork>()
            .BackupRestoreRuns.GetByBackupRunAsync(backupRunId, 20, cancellationToken);
        return Assert.Single(runs, run => run.Status is BackupRestoreStatus.Queued or BackupRestoreStatus.Preparing or BackupRestoreStatus.Running).Id;
    }

    private async Task<Guid> CreateSecretAsync(
        string name,
        string value,
        CancellationToken cancellationToken)
    {
        using var response = await Client.PostAsJsonAsync(
            "/api/v1/resourceBindings/secrets",
            new { name, value },
            cancellationToken);
        response.EnsureSuccessStatusCode();
        return await ReadIdAsync(response, cancellationToken);
    }

    private async Task<HttpResponseMessage> SendMergePatchAsync(
        string path,
        string json,
        CancellationToken cancellationToken)
    {
        using var request = new HttpRequestMessage(HttpMethod.Patch, path)
        {
            Content = new StringContent(json, Encoding.UTF8, "application/merge-patch+json")
        };
        return await Client.SendAsync(request, cancellationToken);
    }

    private static async Task<Guid> ReadIdAsync(
        HttpResponseMessage response,
        CancellationToken cancellationToken)
    {
        using var document = JsonDocument.Parse(await response.Content.ReadAsStreamAsync(cancellationToken));
        return document.RootElement.GetProperty("id").GetGuid();
    }

    private sealed class FakeBackupRunExecutionService : IBackupRunExecutionService
    {
        public IAsyncEnumerable<BackupRunStreamItem> ExecuteQueuedAsync(
            Guid runId,
            CancellationToken cancellationToken)
            => ExecuteAsync(runId, cancellationToken);

        public async IAsyncEnumerable<BackupRunStreamItem> ExecuteAsync(
            Guid runId,
            [EnumeratorCancellation] CancellationToken cancellationToken)
        {
            await Task.Yield();
            yield return new BackupRunStreamItem(runId, BackupRunStatus.Running, "Execution delegated by endpoint.");
        }
    }

    private static Platform CreateSwarmPlatform() => new(
        "backup-endpoint-swarm",
        "edge://backup-endpoint-swarm",
        0,
        1,
        0,
        2,
        2_048,
        "test",
        "test",
        PlatformStatus.Online,
        PlatformConnectorType.EdgeAgent,
        new DockerSwarmPlatformDescriptor(
            "manager-1",
            "10.0.0.1",
            "Active",
            true,
            2,
            1,
            "backup-endpoint-swarm-daemon",
            1,
            1,
            0,
            0));

    private static SwarmNodeProjection CreateSwarmNode(Guid platformId, DateTimeOffset observedAt) => new(
        platformId,
        "worker-1",
        1,
        "worker-one",
        "Worker",
        false,
        "Reachable",
        "Ready",
        null,
        "Active",
        "test",
        "linux",
        "amd64",
        "10.0.0.2",
        new Dictionary<string, string>(),
        1,
        1,
        observedAt,
        observedAt,
        observedAt,
        false);

    private static ContainerInspectionInfo CreateContainerInspection() => new(
        "container-backup-task",
        string.Empty,
        null,
        [],
        null,
        null,
        null,
        null,
        null,
        null,
        null,
        0,
        null,
        null,
        null,
        null,
        null,
        [],
        null,
        null,
        null,
        null,
        [new MountPointInfo("volume", "service-data", "service-data", "/var/lib/postgresql/data", "local", "rw", true, string.Empty)],
        null,
        null);

    private static DockerVolumeResult CreateDockerVolume(string name) => new(
        name,
        name,
        true,
        "local",
        "local",
        $"/var/lib/docker/volumes/{name}/_data",
        string.Empty,
        null,
        null,
        [],
        new Dictionary<string, string>(),
        new Dictionary<string, string>(),
        new Dictionary<string, string>());

    private sealed class FakeBackupRestoreRunExecutionService : IBackupRestoreRunExecutionService
    {
        public IAsyncEnumerable<BackupRestoreRunStreamItem> ExecuteQueuedAsync(
            Guid restoreRunId,
            CancellationToken cancellationToken)
            => ExecuteAsync(restoreRunId, cancellationToken);

        public async IAsyncEnumerable<BackupRestoreRunStreamItem> ExecuteAsync(
            Guid restoreRunId,
            [EnumeratorCancellation] CancellationToken cancellationToken)
        {
            await Task.Yield();
            yield return new BackupRestoreRunStreamItem(
                restoreRunId,
                BackupRestoreStatus.Running,
                "Restore execution delegated by endpoint.");
        }
    }
}
