using System.Net;
using System.Net.Http.Json;
using System.Text;
using System.Text.Json;
using Application.Services.Builds;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Builds;
using Domain.Entities.Git;
using Domain.Entities.Platforms;
using Domain.Entities.Registries;
using Domain.Entities.Tags;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Moq;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Builds;

public sealed class BuildEndpointTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<IBuildAgentPoolValidationService> poolValidationService = new();
    private Guid gitRepositoryId;
    private Guid platformId;
    private Guid registryId;
    private Guid tagId;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        poolValidationService
            .Setup(service => service.ValidateAsync(It.IsAny<BuildAgentPool>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((BuildAgentPoolValidationStatus.Ready, "Build agent is ready."));
        services.ReplaceService<IBuildAgentPoolValidationService>(poolValidationService.Object);
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var repository = new GitRepository(
            "build-endpoint-repository",
            null,
            "https://example.test/citadel/build-endpoint-repository.git",
            "main",
            gitAccountId: null,
            createdByActorId: Constants.SystemId);
        var platform = new Platform(
            "build-endpoint-platform",
            "unix:///var/run/docker.sock",
            networkCount: 0,
            volumeCount: 0,
            imageCount: 0,
            cpuCount: 2,
            memTotal: 2048,
            serverVersion: "test",
            agentVersion: null,
            PlatformStatus.Online,
            PlatformConnectorType.Local,
            new DockerPlatformDescriptor("build-endpoint-daemon", 0, 0, 0, 0));
        var registry = new Registry(
            "build-endpoint-registry",
            "registry.example.test",
            RegistryStatus.Active,
            Constants.SystemId,
            new CustomRegistry());
        var tag = Tag.Create("build-endpoint-tag", "#2455AA", Constants.SystemId);

        await uow.GitRepositories.AddAsync(repository, cancellationToken);
        await uow.Platforms.AddAsync(platform, cancellationToken);
        await uow.Registries.AddAsync(registry, cancellationToken);
        await uow.Tags.AddAsync(tag, cancellationToken);
        await uow.CommitAsync(cancellationToken);

        gitRepositoryId = repository.Id;
        platformId = platform.Id;
        registryId = registry.Id;
        tagId = tag.Id;
    }

    [Fact]
    public async Task BuildProjectAndRunEndpoints_ShouldPersistLifecycle()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        using var createResponse = await Client.PostAsJsonAsync(
            "/api/v1/buildProjects",
            new
            {
                name = "build-endpoint-project",
                description = "created through the API",
                enabled = true,
                gitRepositoryId,
                branch = "main",
                contextPath = ".",
                dockerfilePath = "Dockerfile",
                target = (string?)null,
                buildArgs = Array.Empty<object>(),
                buildSecrets = Array.Empty<object>(),
                builderKind = "Platform",
                platformId,
                buildAgentPoolId = (Guid?)null,
                registryId,
                imageRepository = "citadel/api",
                tagTemplates = new[] { "latest", "{branch}-{shortSha}" },
                webhook = (object?)null,
                timeoutSeconds = 300,
                retentionRunCount = 5,
                tagIds = Array.Empty<Guid>()
            },
            cancellationToken);
        createResponse.EnsureSuccessStatusCode();
        var projectId = await ReadIdAsync(createResponse, cancellationToken);

        using var listResponse = await Client.GetAsync("/api/v1/buildProjects", cancellationToken);
        listResponse.EnsureSuccessStatusCode();
        using var getResponse = await Client.GetAsync($"/api/v1/buildProjects/{projectId:D}", cancellationToken);
        getResponse.EnsureSuccessStatusCode();

        using var replaceTagsResponse = await Client.PutAsJsonAsync(
            $"/api/v1/buildProjects/{projectId:D}/tags",
            new { tagIds = new[] { tagId } },
            cancellationToken);
        replaceTagsResponse.EnsureSuccessStatusCode();
        using var getTagsResponse = await Client.GetAsync(
            $"/api/v1/buildProjects/{projectId:D}/tags",
            cancellationToken);
        getTagsResponse.EnsureSuccessStatusCode();

        using var updateResponse = await SendMergePatchAsync(
            $"/api/v1/buildProjects/{projectId:D}",
            """{"description":"updated project","dockerfilePath":"src/Dockerfile"}""",
            cancellationToken);
        updateResponse.EnsureSuccessStatusCode();
        using var renameResponse = await Client.PostAsJsonAsync(
            "/api/v1/buildProjects/rename",
            new { id = projectId, name = "build-endpoint-renamed" },
            cancellationToken);
        renameResponse.EnsureSuccessStatusCode();
        using var metadataResponse = await SendMergePatchAsync(
            $"/api/v1/buildProjects/{projectId:D}/_metadata",
            """{"description":"metadata project"}""",
            cancellationToken);
        metadataResponse.EnsureSuccessStatusCode();

        using var queueResponse = await Client.PostAsJsonAsync(
            $"/api/v1/buildProjects/{projectId:D}/runs",
            new { trigger = "Manual", triggerSourceId = (Guid?)null },
            cancellationToken);
        queueResponse.EnsureSuccessStatusCode();
        var runId = await ReadIdAsync(queueResponse, cancellationToken);

        using var listRunsResponse = await Client.GetAsync(
            $"/api/v1/buildRuns?projectId={projectId:D}&limit=10",
            cancellationToken);
        listRunsResponse.EnsureSuccessStatusCode();
        using var getRunResponse = await Client.GetAsync($"/api/v1/buildRuns/{runId:D}", cancellationToken);
        getRunResponse.EnsureSuccessStatusCode();
        using var logsResponse = await Client.GetAsync($"/api/v1/buildRuns/{runId:D}/logs", cancellationToken);
        logsResponse.EnsureSuccessStatusCode();

        using var cancelResponse = await Client.PostAsync(
            $"/api/v1/buildRuns/{runId:D}/cancel",
            content: null,
            cancellationToken);
        Assert.Equal(HttpStatusCode.NoContent, cancelResponse.StatusCode);

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var project = await uow.BuildProjects.GetAsync(projectId, cancellationToken);
            var run = await uow.BuildRuns.GetAsync(runId, cancellationToken);
            Assert.NotNull(project);
            Assert.Equal("build-endpoint-renamed", project.Name);
            Assert.Equal("metadata project", project.Description);
            Assert.Equal("src/Dockerfile", project.DockerfilePath);
            Assert.Null(project.CurrentRunId);
            Assert.Contains(project.Tags, tag => tag.Id == tagId);
            Assert.NotNull(run);
            Assert.Equal(BuildRunStatus.Cancelled, run.Status);
        }

        using var archiveResponse = await Client.DeleteAsync(
            $"/api/v1/buildProjects/{projectId:D}",
            cancellationToken);
        Assert.Equal(HttpStatusCode.NoContent, archiveResponse.StatusCode);

        await using (var scope = Services.CreateAsyncScope())
        {
            var archived = await scope.ServiceProvider.GetRequiredService<IUnitOfWork>()
                .BuildProjects.GetAsync(projectId, cancellationToken, includeArchived: true);
            Assert.NotNull(archived);
            Assert.NotNull(archived.ArchivedAt);
        }
    }

    [Fact]
    public async Task BuildAgentPoolEndpoints_ShouldPersistLifecycleAndEdgeEnrollment()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        using var createResponse = await Client.PostAsJsonAsync(
            "/api/v1/buildAgentPools",
            new
            {
                name = "build-endpoint-pool",
                description = "created through the API",
                enabled = true,
                providerSpec = new Dictionary<string, object?>
                {
                    ["$type"] = "SelfManagedVm",
                    ["endpoint"] = null,
                    ["architecture"] = "Amd64",
                    ["maxWorkers"] = 2,
                    ["registrationSecretId"] = null,
                    ["labels"] = Array.Empty<string>(),
                    ["connectionMode"] = "EdgeAgent"
                },
                maxActiveBuilders = 2,
                queueTimeoutSeconds = 300,
                provisioningTimeoutSeconds = 300,
                registrationTimeoutSeconds = 300,
                heartbeatTimeoutSeconds = 120,
                cleanupTimeoutSeconds = 300,
                maximumInstanceLifetimeSeconds = 3600,
                failureRetentionMinutes = 60,
                tagIds = Array.Empty<Guid>()
            },
            cancellationToken);
        createResponse.EnsureSuccessStatusCode();
        var poolId = await ReadIdAsync(createResponse, cancellationToken);

        using var listResponse = await Client.GetAsync("/api/v1/buildAgentPools", cancellationToken);
        listResponse.EnsureSuccessStatusCode();
        using var getResponse = await Client.GetAsync($"/api/v1/buildAgentPools/{poolId:D}", cancellationToken);
        getResponse.EnsureSuccessStatusCode();

        using var replaceTagsResponse = await Client.PutAsJsonAsync(
            $"/api/v1/buildAgentPools/{poolId:D}/tags",
            new { tagIds = new[] { tagId } },
            cancellationToken);
        replaceTagsResponse.EnsureSuccessStatusCode();
        using var getTagsResponse = await Client.GetAsync(
            $"/api/v1/buildAgentPools/{poolId:D}/tags",
            cancellationToken);
        getTagsResponse.EnsureSuccessStatusCode();

        using var updateResponse = await SendMergePatchAsync(
            $"/api/v1/buildAgentPools/{poolId:D}",
            """{"description":"updated pool","maxActiveBuilders":3}""",
            cancellationToken);
        updateResponse.EnsureSuccessStatusCode();
        using var renameResponse = await Client.PostAsJsonAsync(
            "/api/v1/buildAgentPools/rename",
            new { id = poolId, name = "build-endpoint-pool-renamed" },
            cancellationToken);
        renameResponse.EnsureSuccessStatusCode();
        using var metadataResponse = await SendMergePatchAsync(
            $"/api/v1/buildAgentPools/{poolId:D}/_metadata",
            """{"description":"metadata pool"}""",
            cancellationToken);
        metadataResponse.EnsureSuccessStatusCode();

        using var testResponse = await Client.PostAsync(
            $"/api/v1/buildAgentPools/{poolId:D}/test",
            content: null,
            cancellationToken);
        testResponse.EnsureSuccessStatusCode();

        using var enrollmentRequest = new HttpRequestMessage(
            HttpMethod.Post,
            $"/api/v1/buildAgentPools/{poolId:D}/edge/enrollments");
        enrollmentRequest.Headers.Host = "localhost:8000";
        using var enrollmentResponse = await Client.SendAsync(enrollmentRequest, cancellationToken);
        enrollmentResponse.EnsureSuccessStatusCode();
        using var enrollmentDocument = JsonDocument.Parse(
            await enrollmentResponse.Content.ReadAsStreamAsync(cancellationToken));
        Assert.False(string.IsNullOrWhiteSpace(enrollmentDocument.RootElement.GetProperty("token").GetString()));

        using var statusResponse = await Client.GetAsync(
            $"/api/v1/buildAgentPools/{poolId:D}/edge/status",
            cancellationToken);
        statusResponse.EnsureSuccessStatusCode();
        using var revokeResponse = await Client.PostAsync(
            $"/api/v1/buildAgentPools/{poolId:D}/edge/revoke",
            content: null,
            cancellationToken);
        Assert.Equal(HttpStatusCode.NoContent, revokeResponse.StatusCode);

        await using (var scope = Services.CreateAsyncScope())
        {
            var pool = await scope.ServiceProvider.GetRequiredService<IUnitOfWork>()
                .BuildAgentPools.GetAsync(poolId, cancellationToken);
            Assert.NotNull(pool);
            Assert.Equal("build-endpoint-pool-renamed", pool.Name);
            Assert.Equal("metadata pool", pool.Description);
            Assert.Equal(3, pool.MaxActiveBuilders);
            Assert.Equal(BuildAgentPoolValidationStatus.Ready, pool.LastValidationStatus);
            Assert.Contains(pool.Tags, tag => tag.Id == tagId);
        }

        using var archiveResponse = await Client.DeleteAsync(
            $"/api/v1/buildAgentPools/{poolId:D}",
            cancellationToken);
        Assert.Equal(HttpStatusCode.NoContent, archiveResponse.StatusCode);

        await using (var scope = Services.CreateAsyncScope())
        {
            var archived = await scope.ServiceProvider.GetRequiredService<IUnitOfWork>()
                .BuildAgentPools.GetAsync(poolId, cancellationToken, includeArchived: true);
            Assert.NotNull(archived);
            Assert.NotNull(archived.ArchivedAt);
        }
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
}
