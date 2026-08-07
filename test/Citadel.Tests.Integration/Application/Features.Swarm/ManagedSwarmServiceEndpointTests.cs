using System.Net;
using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Text;
using System.Text.Json;
using Application.Services;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Swarm;
using Domain.Entities.Activities;
using Domain.Entities.Platforms;
using Domain.Entities.Registries;
using Domain.Entities.ResourceBindings;
using Domain.Entities.SwarmServices;
using Domain.Entities.Tags;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Grpc.Core;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Moq;
using LightResults;

namespace Tests.Integration.Application.Features.Swarm;

public sealed class ManagedSwarmServiceEndpointTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private const string SecretId = "secret-1";
    private const string ConfigId = "config-1";
    private readonly Mock<IConnectorFactory<ISwarmConnector>> connectorFactory = new();
    private readonly Mock<ISwarmConnector> connector = new();
    private readonly Mock<IImageDigestScanner> imageDigestScanner = new();
    private readonly Mock<ISwarmReconciliationCoordinator> reconciliationCoordinator = new();
    private Guid platformId;
    private Guid standalonePlatformId;
    private Guid registryId;
    private Guid tagId;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<IConnectorFactory<ISwarmConnector>>();
        services.RemoveAll<IImageDigestScanner>();
        services.RemoveAll<ISwarmReconciliationCoordinator>();
        services.AddSingleton(connectorFactory.Object);
        services.AddSingleton(imageDigestScanner.Object);
        services.AddSingleton(reconciliationCoordinator.Object);

        connectorFactory.Setup(factory => factory.GetConnector(It.IsAny<PlatformConnectorType>()))
            .Returns(connector.Object);
        imageDigestScanner.Setup(scanner => scanner.ScanAsync(
                It.IsAny<ImageScanTask>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success("sha256:current"));
        reconciliationCoordinator.Setup(coordinator => coordinator.RefreshAsync(
                It.IsAny<Guid>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var platform = CreateSwarmPlatform();
        var standalone = CreateStandalonePlatform();
        var registry = new Registry(
            "managed-service-registry",
            "registry.example.test",
            RegistryStatus.Active,
            Constants.SystemId,
            new CustomRegistry());
        var tag = Tag.Create("managed-service", "#2563EB", Constants.SystemId);

        await uow.Platforms.AddAsync(platform, cancellationToken);
        await uow.Platforms.AddAsync(standalone, cancellationToken);
        await uow.Registries.AddAsync(registry, cancellationToken);
        await uow.Tags.AddAsync(tag, cancellationToken);
        await uow.CommitAsync(cancellationToken);

        var observedAt = DateTimeOffset.UtcNow;
        await uow.Swarm.ReplaceAsync(
            platform.Id,
            new SwarmProjectionSnapshot(
                [],
                [],
                [],
                [
                    new SwarmNetworkProjection(
                        platform.Id,
                        "overlay-network",
                        "application-overlay",
                        "Swarm",
                        "overlay",
                        true,
                        false,
                        false,
                        true,
                        false,
                        ["10.10.0.0/24"],
                        [],
                        new Dictionary<string, string>(),
                        observedAt,
                        observedAt,
                        false),
                    new SwarmNetworkProjection(
                        platform.Id,
                        "ingress-network",
                        "ingress",
                        "Swarm",
                        "overlay",
                        false,
                        false,
                        true,
                        false,
                        false,
                        ["10.20.0.0/24"],
                        [],
                        new Dictionary<string, string>(),
                        observedAt,
                        observedAt,
                        false),
                ],
                [new SwarmSecretProjection(
                    platform.Id, SecretId, 1, "api-key", null, [],
                    new Dictionary<string, string>(), observedAt, observedAt, observedAt, false)],
                [new SwarmConfigProjection(
                    platform.Id, ConfigId, 1, "nginx-config", null, [],
                    new Dictionary<string, string>(), observedAt, observedAt, observedAt, false)]),
            cancellationToken);
        await uow.CommitAsync(cancellationToken);

        platformId = platform.Id;
        standalonePlatformId = standalone.Id;
        registryId = registry.Id;
        tagId = tag.Id;
    }

    [Fact]
    public async Task CrudEndpoints_ShouldPersistCompleteTypedServiceThroughDapperAot()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        using var createResponse = await Client.PostAsJsonAsync(
            "/api/v1/swarmServices",
            new
            {
                name = "managed-web",
                platformId,
                description = "Managed through Citadel",
                spec = CreateSpec(replicas: 2),
                tagIds = new[] { tagId }
            },
            cancellationToken);
        createResponse.EnsureSuccessStatusCode();

        using var created = await JsonDocument.ParseAsync(
            await createResponse.Content.ReadAsStreamAsync(cancellationToken),
            cancellationToken: cancellationToken);
        var id = created.RootElement.GetProperty("id").GetGuid();
        var rowVersion = created.RootElement.GetProperty("rowVersion").GetInt64();
        Assert.Equal("Created", created.RootElement.GetProperty("health").GetString());
        Assert.Equal("NeverApplied", created.RootElement.GetProperty("synchronizationState").GetString());
        Assert.Equal(2, created.RootElement.GetProperty("spec").GetProperty("replicas").GetInt32());
        Assert.Contains(
            created.RootElement.GetProperty("tags").EnumerateArray(),
            value => value.GetProperty("id").GetGuid() == tagId);

        using var getResponse = await Client.GetAsync($"/api/v1/swarmServices/{id:D}", cancellationToken);
        getResponse.EnsureSuccessStatusCode();
        using (var fetched = await JsonDocument.ParseAsync(
            await getResponse.Content.ReadAsStreamAsync(cancellationToken),
            cancellationToken: cancellationToken))
        {
            Assert.Equal(id, fetched.RootElement.GetProperty("id").GetGuid());
            Assert.Equal("managed-web", fetched.RootElement.GetProperty("name").GetString());
            Assert.Equal("Created", fetched.RootElement.GetProperty("health").GetString());
            Assert.Equal("NeverApplied", fetched.RootElement.GetProperty("synchronizationState").GetString());
        }

        using var listResponse = await Client.GetAsync(
            $"/api/v1/swarmServices?platformId={platformId:D}&tags=managed-service",
            cancellationToken);
        listResponse.EnsureSuccessStatusCode();
        using (var listed = await JsonDocument.ParseAsync(
            await listResponse.Content.ReadAsStreamAsync(cancellationToken),
            cancellationToken: cancellationToken))
        {
            Assert.Equal(id, Assert.Single(listed.RootElement.GetProperty("swarmServices").EnumerateArray()).GetProperty("id").GetGuid());
        }

        using var updateResponse = await Client.PatchAsJsonAsync(
            $"/api/v1/swarmServices/{id:D}",
            new { spec = CreateSpec(replicas: 4), rowVersion },
            cancellationToken);
        updateResponse.EnsureSuccessStatusCode();

        using var renamedResponse = await Client.PostAsJsonAsync(
            "/api/v1/swarmServices/rename",
            new { id, name = "managed-web-renamed" },
            cancellationToken);
        renamedResponse.EnsureSuccessStatusCode();

        using var metadataRequest = new HttpRequestMessage(
            HttpMethod.Patch,
            $"/api/v1/swarmServices/{id:D}/_metadata")
        {
            Content = JsonContent.Create(new { description = "Updated metadata" })
        };
        metadataRequest.Content.Headers.ContentType = new MediaTypeHeaderValue("application/merge-patch+json");
        using var metadataResponse = await Client.SendAsync(metadataRequest, cancellationToken);
        metadataResponse.EnsureSuccessStatusCode();

        await using (var scope = Services.CreateAsyncScope())
        {
            var service = await scope.ServiceProvider.GetRequiredService<IUnitOfWork>()
                .SwarmServices.GetAsync(id, cancellationToken);
            Assert.NotNull(service);
            Assert.Equal("managed-web-renamed", service.Name);
            Assert.Equal("Updated metadata", service.Description);
            Assert.Equal(4, service.Spec.Replicas);
            Assert.Equal("/srv/app", service.Spec.WorkingDirectory);
            Assert.Equal("overlay-network", Assert.Single(service.Spec.NetworkIds));
            Assert.Equal("/data", Assert.Single(service.Spec.Mounts).Target);
            Assert.Equal(2_000_000_000, service.Spec.Resources?.LimitNanoCpus);
            Assert.Equal(2, service.Spec.UpdatePolicy?.Parallelism);
            Assert.Contains(service.Tags, tag => tag.Id == tagId);
        }

        using var deleteRequest = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/swarmServices")
        {
            Content = JsonContent.Create(new[] { id })
        };
        using var deleteResponse = await Client.SendAsync(deleteRequest, cancellationToken);
        Assert.Equal(HttpStatusCode.NoContent, deleteResponse.StatusCode);

        await using var verificationScope = Services.CreateAsyncScope();
        Assert.Null(await verificationScope.ServiceProvider.GetRequiredService<IUnitOfWork>()
            .SwarmServices.GetAsync(id, cancellationToken));
    }

    [Fact]
    public async Task CreateAndGet_ShouldPersistGenericWebhookConfigurationThroughDapperAot()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var webhook = new
        {
            enabled = true,
            provider = "Generic",
            authScheme = "BearerToken",
            secret = "integration-shared-secret",
            branchFilter = (string?)null,
        };
        using var createResponse = await Client.PostAsJsonAsync(
            "/api/v1/swarmServices",
            new
            {
                name = "managed-webhook-service",
                platformId,
                description = (string?)null,
                spec = CreateSpec(replicas: 1, webhook: webhook),
                tagIds = Array.Empty<Guid>(),
            },
            cancellationToken);
        createResponse.EnsureSuccessStatusCode();
        using var created = await JsonDocument.ParseAsync(
            await createResponse.Content.ReadAsStreamAsync(cancellationToken),
            cancellationToken: cancellationToken);
        var id = created.RootElement.GetProperty("id").GetGuid();

        await using var scope = Services.CreateAsyncScope();
        var persisted = await scope.ServiceProvider.GetRequiredService<IUnitOfWork>()
            .SwarmServices.GetAsync(id, cancellationToken);

        Assert.NotNull(persisted);
        Assert.True(persisted.Spec.Webhook?.Enabled);
        Assert.Equal(WebhookProvider.Generic, persisted.Spec.Webhook?.Provider);
        Assert.Equal(WebhookAuthScheme.BearerToken, persisted.Spec.Webhook?.AuthScheme);
        Assert.Equal("integration-shared-secret", persisted.Spec.Webhook?.Secret);
    }

    [Fact]
    public async Task Create_WithEnabledWebhookAndDisabledUpdates_ShouldRejectWithoutPersisting()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        using var response = await Client.PostAsJsonAsync(
            "/api/v1/swarmServices",
            new
            {
                name = "invalid-disabled-webhook-service",
                platformId,
                description = (string?)null,
                spec = CreateSpec(
                    replicas: 1,
                    webhook: CreateGenericWebhook(),
                    updateBehavior: "Disabled"),
                tagIds = Array.Empty<Guid>(),
            },
            cancellationToken);

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
        await using var scope = Services.CreateAsyncScope();
        Assert.False(await scope.ServiceProvider.GetRequiredService<IUnitOfWork>()
            .SwarmServices.ExistsAsync(platformId, "invalid-disabled-webhook-service", cancellationToken));
    }

    [Fact]
    public async Task Create_WithOversizedWebhookSecret_ShouldRejectWithoutPersisting()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        using var response = await Client.PostAsJsonAsync(
            "/api/v1/swarmServices",
            new
            {
                name = "invalid-webhook-secret-service",
                platformId,
                description = (string?)null,
                spec = CreateSpec(
                    replicas: 1,
                    webhook: CreateGenericWebhook(new string('s', 257))),
                tagIds = Array.Empty<Guid>(),
            },
            cancellationToken);

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
        await using var scope = Services.CreateAsyncScope();
        Assert.False(await scope.ServiceProvider.GetRequiredService<IUnitOfWork>()
            .SwarmServices.ExistsAsync(platformId, "invalid-webhook-secret-service", cancellationToken));
    }

    [Fact]
    public async Task DuplicateDraft_ShouldCreateServiceWithoutImageProvenance_AndRecordDuplicateActivity()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        using var sourceResponse = await Client.PostAsJsonAsync(
            "/api/v1/swarmServices",
            new
            {
                name = "duplicate-source-service",
                platformId,
                description = "Source description",
                spec = CreateSpec(
                    3,
                    resolvedDigest: "sha256:source",
                    webhook: CreateGenericWebhook("source-only-secret")),
                tagIds = new[] { tagId },
            },
            cancellationToken);
        sourceResponse.EnsureSuccessStatusCode();
        using var sourceDocument = await JsonDocument.ParseAsync(
            await sourceResponse.Content.ReadAsStreamAsync(cancellationToken),
            cancellationToken: cancellationToken);
        var sourceId = sourceDocument.RootElement.GetProperty("id").GetGuid();

        await using (var bindingScope = Services.CreateAsyncScope())
        {
            var bindingUow = bindingScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await bindingUow.ResourceBindings.AddAsync(
                new ResourceBinding(
                    Name: "service_mode",
                    Kind: ResourceBindingKind.Variable,
                    Scope: ResourceBindingScope.SwarmService,
                    ResourceId: sourceId,
                    Value: "production",
                    SecretId: null),
                cancellationToken);
            await bindingUow.CommitAsync(cancellationToken);
        }

        using var draftResponse = await Client.GetAsync(
            $"/api/v1/swarmServices/{sourceId:D}/duplicate-draft",
            cancellationToken);
        draftResponse.EnsureSuccessStatusCode();
        using var draftDocument = await JsonDocument.ParseAsync(
            await draftResponse.Content.ReadAsStreamAsync(cancellationToken),
            cancellationToken: cancellationToken);
        var draft = draftDocument.RootElement.GetProperty("draft").Clone();
        var image = draft.GetProperty("spec").GetProperty("image");

        Assert.Equal("duplicate-source-service-copy", draft.GetProperty("name").GetString());
        Assert.Equal("nginx:1.27", image.GetProperty("imageTag").GetString());
        Assert.True(
            !image.TryGetProperty("resolvedDigest", out var resolvedDigest)
            || resolvedDigest.ValueKind == JsonValueKind.Null);
        Assert.True(
            !draft.GetProperty("spec").TryGetProperty("webhook", out var duplicateWebhook)
            || duplicateWebhook.ValueKind == JsonValueKind.Null);
        Assert.Equal(tagId, Assert.Single(draft.GetProperty("tagIds").EnumerateArray()).GetGuid());

        using var duplicateResponse = await Client.PostAsJsonAsync(
            "/api/v1/swarmServices",
            draft,
            cancellationToken);
        duplicateResponse.EnsureSuccessStatusCode();
        using var duplicateDocument = await JsonDocument.ParseAsync(
            await duplicateResponse.Content.ReadAsStreamAsync(cancellationToken),
            cancellationToken: cancellationToken);
        var duplicateId = duplicateDocument.RootElement.GetProperty("id").GetGuid();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var duplicate = await uow.SwarmServices.GetAsync(duplicateId, cancellationToken);
        Assert.NotNull(duplicate);
        Assert.Equal("duplicate-source-service-copy", duplicate.Name);
        Assert.Equal("Source description", duplicate.Description);
        Assert.Null(Assert.IsType<SwarmExternalImage>(duplicate.Spec.Image).ResolvedDigest);
        Assert.Null(duplicate.Spec.Webhook);
        Assert.Equal(tagId, Assert.Single(duplicate.Tags).Id);

        var copiedBinding = Assert.Single(await uow.ResourceBindings.GetEntriesAsync(
            ResourceBindingScope.SwarmService,
            duplicateId,
            cancellationToken));
        Assert.Equal("service_mode", copiedBinding.Name);
        Assert.Equal("production", copiedBinding.Value);
        Assert.Equal(duplicateId, copiedBinding.ResourceId);
        var sourceBinding = Assert.Single(await uow.ResourceBindings.GetEntriesAsync(
            ResourceBindingScope.SwarmService,
            sourceId,
            cancellationToken));
        Assert.NotEqual(sourceBinding.Id, copiedBinding.Id);

        var activities = await uow.ActivityEventRepository.GetPagedAsync(
            duplicateId,
            ActivityResourceType.SwarmService,
            ActivityEventType.SwarmServiceDuplicated,
            1,
            10,
            cancellationToken);
        var activity = await uow.ActivityEventRepository.GetByIdAsync(
            Assert.Single(activities.Items).Id,
            cancellationToken);
        var info = Assert.IsType<SwarmServiceDuplicated>(activity?.Info);
        Assert.Equal(sourceId, info.Source.ResourceId);
        Assert.Equal("duplicate-source-service", info.Source.ResourceName);
        Assert.Equal("duplicate-source-service-copy", info.Service.Name);
    }

    [Fact]
    public async Task Create_WithInvalidDuplicateSourceType_ShouldRejectTheRequest()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var sourceId = await CreateServiceAsync(cancellationToken);
        using var response = await Client.PostAsJsonAsync(
            "/api/v1/swarmServices",
            new
            {
                name = "invalid-duplicate-source",
                platformId,
                description = (string?)null,
                spec = CreateSpec(1),
                tagIds = Array.Empty<Guid>(),
                duplicateSource = new
                {
                    resourceType = "Deployment",
                    resourceId = sourceId,
                    resourceName = "forged-source",
                },
            },
            cancellationToken);

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
        await using var scope = Services.CreateAsyncScope();
        Assert.False(await scope.ServiceProvider.GetRequiredService<IUnitOfWork>()
            .SwarmServices.ExistsAsync(platformId, "invalid-duplicate-source", cancellationToken));
    }

    [Fact]
    public async Task List_ShouldReturnPersistedTasksForEachAuthorizedManagedService()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var id = await CreateServiceAsync(cancellationToken);
        var observedAt = DateTimeOffset.UtcNow;

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var service = await uow.SwarmServices.GetAsync(id, cancellationToken);
            Assert.NotNull(service);
            Assert.True(service.TryPrepareOperation(
                SwarmServiceOperationKind.Apply,
                Guid.CreateVersion7(),
                Constants.SystemId,
                targetRuntimeHash: "runtime-hash",
                clusterId: "managed-service-cluster"));
            service.MarkOperationAttempted();
            service.MarkOperationAccepted("docker-service-list", 7);
            service.CompleteOperation(SwarmServiceOperationState.Completed, "runtime-hash");
            Assert.Equal(1, await uow.SwarmServices.UpdateAsync(service, cancellationToken));
            await uow.CommitAsync(cancellationToken);

            await uow.Swarm.ReplaceAsync(
                platformId,
                new SwarmProjectionSnapshot(
                    [],
                    [new SwarmServiceProjection(
                        platformId, "docker-service-list", 7, service.DockerName, "Replicated",
                        "nginx:latest", 1, 1, "completed", null, [], [], [], [],
                        new Dictionary<string, string>(), observedAt, observedAt, observedAt, false)],
                    [new SwarmTaskProjection(
                        platformId, "task-list-1", 3, "web.1", "docker-service-list", service.DockerName,
                        1, "node-list-1", "manager-1", "running", "running", null, null,
                        "nginx:latest", [], observedAt, observedAt, observedAt, observedAt, false)],
                    [],
                    [],
                    []),
                cancellationToken);
            await uow.CommitAsync(cancellationToken);
        }

        using var response = await Client.GetAsync(
            $"/api/v1/swarmServices?platformId={platformId:D}",
            cancellationToken);
        response.EnsureSuccessStatusCode();
        using var payload = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(cancellationToken),
            cancellationToken: cancellationToken);
        var listed = Assert.Single(payload.RootElement.GetProperty("swarmServices").EnumerateArray());
        var task = Assert.Single(listed.GetProperty("tasks").EnumerateArray());
        Assert.Equal("task-list-1", task.GetProperty("id").GetString());
        Assert.Equal("manager-1", task.GetProperty("nodeHostname").GetString());
        Assert.Equal("running", task.GetProperty("state").GetString());
    }

    [Fact]
    public async Task Create_ShouldRejectDockerStandalonePlatform()
    {
        using var response = await Client.PostAsJsonAsync(
            "/api/v1/swarmServices",
            new
            {
                name = "standalone-service",
                platformId = standalonePlatformId,
                description = (string?)null,
                spec = CreateSpec(1),
                tagIds = Array.Empty<Guid>()
            },
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task Create_ShouldRejectExplicitIngressNetworkAttachment()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        using var response = await Client.PostAsJsonAsync(
            "/api/v1/swarmServices",
            new
            {
                name = "invalid-ingress-service",
                platformId,
                description = (string?)null,
                spec = CreateSpec(1, networkIds: ["ingress-network"]),
                tagIds = Array.Empty<Guid>()
            },
            cancellationToken);

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
        Assert.Contains(
            "ingress network is managed by Docker",
            await response.Content.ReadAsStringAsync(cancellationToken),
            StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public async Task Create_WithNullNestedImage_ShouldReturnBadRequest()
    {
        var payload = $$"""
            {
              "name": "invalid-null-image",
              "platformId": "{{platformId}}",
              "description": null,
              "spec": {
                "image": null,
                "updateBehavior": "Disabled",
                "schedulingMode": "Replicated",
                "replicas": 1,
                "command": [],
                "arguments": [],
                "environment": [],
                "ports": [],
                "networkIds": [],
                "mounts": [],
                "secrets": [],
                "configs": [],
                "placementConstraints": []
              },
              "tagIds": []
            }
            """;
        using var response = await Client.PostAsync(
            "/api/v1/swarmServices",
            new StringContent(payload, Encoding.UTF8, "application/json"),
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task ConcurrentCreate_WithSameName_ShouldPersistOnlyOneService()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var name = $"create-race-{Guid.NewGuid():N}"[..40];
        var request = new
        {
            name,
            platformId,
            description = (string?)null,
            spec = CreateSpec(2),
            tagIds = Array.Empty<Guid>()
        };

        var responses = await Task.WhenAll(
            Client.PostAsJsonAsync("/api/v1/swarmServices", request, cancellationToken),
            Client.PostAsJsonAsync("/api/v1/swarmServices", request, cancellationToken));
        try
        {
            Assert.Single(responses, response => response.IsSuccessStatusCode);
            Assert.Single(responses, response => response.StatusCode == HttpStatusCode.Conflict);
        }
        finally
        {
            foreach (var response in responses)
                response.Dispose();
        }

        await using var scope = Services.CreateAsyncScope();
        var services = await scope.ServiceProvider.GetRequiredService<IUnitOfWork>()
            .SwarmServices.GetByPlatformAsync(platformId, cancellationToken);
        Assert.Single(services, service => service.Name == name);
    }

    [Fact]
    public async Task Create_ShouldRejectStaleSecretOrConfigNames()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        using var secretResponse = await Client.PostAsJsonAsync(
            "/api/v1/swarmServices",
            new
            {
                name = "stale-secret-name",
                platformId,
                description = (string?)null,
                spec = CreateSpec(1, secrets: [new { secretId = SecretId, secretName = "renamed-secret", targetName = "api-key" }]),
                tagIds = Array.Empty<Guid>()
            },
            cancellationToken);
        Assert.Equal(HttpStatusCode.BadRequest, secretResponse.StatusCode);

        using var configResponse = await Client.PostAsJsonAsync(
            "/api/v1/swarmServices",
            new
            {
                name = "stale-config-name",
                platformId,
                description = (string?)null,
                spec = CreateSpec(1, configs: [new { configId = ConfigId, configName = "renamed-config", targetName = "nginx.conf" }]),
                tagIds = Array.Empty<Guid>()
            },
            cancellationToken);
        Assert.Equal(HttpStatusCode.BadRequest, configResponse.StatusCode);
    }

    [Fact]
    public async Task ResourceGrantWithoutPlatformVisibility_ShouldNotExposeManagedService()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var id = await CreateServiceAsync(cancellationToken);
        var serviceOnly = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(
                    ResourceType.SwarmService,
                    id,
                    PermissionLevel.Write,
                    SpecificPermission.Apply
                    | SpecificPermission.Logs
                    | SpecificPermission.Inspect
                    | SpecificPermission.ResourceBindings)
            ]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(serviceOnly.UserId, serviceOnly.ActorId));

        using var hidden = await Client.GetAsync($"/api/v1/swarmServices/{id:D}", cancellationToken);
        Assert.Equal(HttpStatusCode.NotFound, hidden.StatusCode);
        using var hiddenList = await Client.GetAsync("/api/v1/swarmServices", cancellationToken);
        hiddenList.EnsureSuccessStatusCode();
        using (var listed = await JsonDocument.ParseAsync(
            await hiddenList.Content.ReadAsStreamAsync(cancellationToken),
            cancellationToken: cancellationToken))
            Assert.Empty(listed.RootElement.GetProperty("swarmServices").EnumerateArray());
        using var hiddenLookup = await Client.GetAsync(
            "/api/v1/lookup?targetResourceType=SwarmService",
            cancellationToken);
        hiddenLookup.EnsureSuccessStatusCode();
        using (var lookup = await JsonDocument.ParseAsync(
            await hiddenLookup.Content.ReadAsStreamAsync(cancellationToken),
            cancellationToken: cancellationToken))
            Assert.DoesNotContain(lookup.RootElement.EnumerateArray(), value => value.GetProperty("id").GetGuid() == id);
        using var hiddenSearch = await Client.GetAsync(
            "/api/v1/search?q=managed&types=SwarmService",
            cancellationToken);
        hiddenSearch.EnsureSuccessStatusCode();
        using (var search = await JsonDocument.ParseAsync(
            await hiddenSearch.Content.ReadAsStreamAsync(cancellationToken),
            cancellationToken: cancellationToken))
            Assert.DoesNotContain(
                search.RootElement.GetProperty("groups").EnumerateArray()
                    .SelectMany(group => group.GetProperty("items").EnumerateArray()),
                value => value.GetProperty("id").GetGuid() == id);
        using var hiddenTags = await Client.GetAsync($"/api/v1/swarmServices/{id:D}/tags", cancellationToken);
        Assert.Equal(HttpStatusCode.NotFound, hiddenTags.StatusCode);
        using var hiddenBindings = await Client.GetAsync(
            $"/api/v1/resourceBindings/SwarmService/{id:D}",
            cancellationToken);
        Assert.Equal(HttpStatusCode.NotFound, hiddenBindings.StatusCode);

        var visible = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.Platform, platformId, PermissionLevel.Read),
                new ResourceGrant(
                    ResourceType.SwarmService,
                    id,
                    PermissionLevel.Read,
                    SpecificPermission.ResourceBindings)
            ]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(visible.UserId, visible.ActorId));

        using var allowed = await Client.GetAsync($"/api/v1/swarmServices/{id:D}", cancellationToken);
        allowed.EnsureSuccessStatusCode();
        using var allowedLookup = await Client.GetAsync(
            "/api/v1/lookup?targetResourceType=SwarmService",
            cancellationToken);
        allowedLookup.EnsureSuccessStatusCode();
        using (var lookup = await JsonDocument.ParseAsync(
            await allowedLookup.Content.ReadAsStreamAsync(cancellationToken),
            cancellationToken: cancellationToken))
            Assert.Contains(lookup.RootElement.EnumerateArray(), value => value.GetProperty("id").GetGuid() == id);
        using var allowedBindings = await Client.GetAsync(
            $"/api/v1/resourceBindings/SwarmService/{id:D}",
            cancellationToken);
        allowedBindings.EnsureSuccessStatusCode();
    }

    [Fact]
    public async Task Update_WithStaleRowVersion_ShouldReturnConflictAndKeepPersistedSpec()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var id = await CreateServiceAsync(cancellationToken);

        using var response = await Client.PatchAsJsonAsync(
            $"/api/v1/swarmServices/{id:D}",
            new { spec = CreateSpec(9), rowVersion = 99 },
            cancellationToken);

        Assert.Equal(HttpStatusCode.Conflict, response.StatusCode);
        await using var scope = Services.CreateAsyncScope();
        var persisted = await scope.ServiceProvider.GetRequiredService<IUnitOfWork>()
            .SwarmServices.GetAsync(id, cancellationToken);
        Assert.Equal(2, persisted?.Spec.Replicas);
    }

    [Fact]
    public async Task Apply_ShouldResolveCurrentTagDigestAndAllowSystemActor()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var id = await CreateServiceAsync(cancellationToken);
        CreateManagedSwarmServiceCommand? submitted = null;
        connector.Setup(value => value.CreateServiceAsync(
                It.IsAny<CreateManagedSwarmServiceCommand>(),
                It.IsAny<CancellationToken>()))
            .Callback<CreateManagedSwarmServiceCommand, CancellationToken>((command, _) => submitted = command)
            .ReturnsAsync(Result.Success(new ManagedSwarmServiceMutationResult("docker-service", [])));

        await using var scope = Services.CreateAsyncScope();
        var result = await scope.ServiceProvider.GetRequiredService<ISwarmServiceMutationService>()
            .ApplyAsync(id, Constants.SystemId, cancellationToken);

        Assert.True(result.IsSuccess());
        Assert.NotNull(submitted);
        Assert.Equal("nginx@sha256:current", submitted.ResolvedImage);
        Assert.NotEqual(Guid.Empty, submitted.OperationId);
        Assert.Equal(
            submitted.OperationId.ToString(),
            submitted.Labels["com.citadel.operation-id"]);
        connector.Verify(value => value.CreateServiceAsync(
            It.IsAny<CreateManagedSwarmServiceCommand>(),
            It.IsAny<CancellationToken>()), Times.Once);
        var prematureSuccesses = await scope.ServiceProvider.GetRequiredService<IUnitOfWork>()
            .ActivityEventRepository.GetPagedAsync(
                id,
                ActivityResourceType.SwarmService,
                ActivityEventType.SwarmServiceApplied,
                1,
                10,
                cancellationToken);
        Assert.Empty(prematureSuccesses.Items);
    }

    [Fact]
    public async Task CheckUpdates_ShouldPersistAvailableDigestAndReleaseProcessingState()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var id = await CreateServiceAsync(cancellationToken);
        await MarkServiceAppliedAsync(id, cancellationToken);

        using var response = await Client.PostAsync(
            $"/api/v1/swarmServices/{id:D}/check-updates",
            content: null,
            cancellationToken);

        response.EnsureSuccessStatusCode();
        await using var verificationScope = Services.CreateAsyncScope();
        var persisted = await verificationScope.ServiceProvider.GetRequiredService<IUnitOfWork>()
            .SwarmServices.GetAsync(id, cancellationToken);
        Assert.Equal(ResourceControlState.Idle, persisted?.ControlState);
        Assert.Equal(AutoUpdateStatus.UpdateAvailable, persisted?.AutoUpdateState.Status);
        Assert.Equal("sha256:old", persisted?.AutoUpdateState.CurrentDigest);
        Assert.Equal("sha256:current", persisted?.AutoUpdateState.RemoteDigest);
    }

    [Fact]
    public async Task CheckUpdates_WhenScanFails_ShouldReleaseServiceWithCompletedPriorOperation()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        imageDigestScanner
            .Setup(scanner => scanner.ScanAsync(
                It.IsAny<ImageScanTask>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Failure<string>(new ConflictError("The platform is unavailable.")));
        var id = await CreateServiceAsync(cancellationToken);
        await MarkServiceAppliedAsync(id, cancellationToken);

        using var response = await Client.PostAsync(
            $"/api/v1/swarmServices/{id:D}/check-updates",
            content: null,
            cancellationToken);

        Assert.Equal(HttpStatusCode.Conflict, response.StatusCode);
        await using var verificationScope = Services.CreateAsyncScope();
        var persisted = await verificationScope.ServiceProvider.GetRequiredService<IUnitOfWork>()
            .SwarmServices.GetAsync(id, cancellationToken);
        Assert.Equal(ResourceControlState.Idle, persisted?.ControlState);
        Assert.Equal(SwarmServiceOperationState.Completed, persisted?.CurrentOperation?.State);
    }

    [Fact]
    public async Task CheckUpdates_WhenRegistryScanFails_ShouldPersistSafeFailureAndReleaseProcessingState()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        imageDigestScanner
            .Setup(scanner => scanner.ScanAsync(
                It.IsAny<ImageScanTask>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Failure<string>(new BadGatewayError("Registry is unavailable.")));
        var id = await CreateServiceAsync(cancellationToken);
        await MarkServiceAppliedAsync(id, cancellationToken);

        using var response = await Client.PostAsync(
            $"/api/v1/swarmServices/{id:D}/check-updates",
            content: null,
            cancellationToken);

        Assert.Equal(HttpStatusCode.BadGateway, response.StatusCode);
        await using var verificationScope = Services.CreateAsyncScope();
        var persisted = await verificationScope.ServiceProvider.GetRequiredService<IUnitOfWork>()
            .SwarmServices.GetAsync(id, cancellationToken);
        Assert.Equal(ResourceControlState.Idle, persisted?.ControlState);
        Assert.Equal(AutoUpdateStatus.Failed, persisted?.AutoUpdateState.Status);
        Assert.Equal("sha256:old", persisted?.AutoUpdateState.CurrentDigest);
        Assert.Equal("Registry is unavailable.", persisted?.AutoUpdateState.LastError);
    }

    [Fact]
    public async Task Apply_ShouldInterpolateReferencedServiceBindingsOnly()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var id = await CreateServiceAsync(
            cancellationToken,
            ["LOG_LEVEL=${LOG_LEVELS}", "STATIC=value"]);
        await using (var bindingScope = Services.CreateAsyncScope())
        {
            var uow = bindingScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.ResourceBindings.AddAsync(new ResourceBinding(
                "LOG_LEVELS",
                ResourceBindingKind.Variable,
                ResourceBindingScope.SwarmService,
                id,
                "debug",
                null), cancellationToken);
            await uow.ResourceBindings.AddAsync(new ResourceBinding(
                "UNUSED",
                ResourceBindingKind.Variable,
                ResourceBindingScope.SwarmService,
                id,
                "must-not-be-injected",
                null), cancellationToken);
            await uow.CommitAsync(cancellationToken);
        }

        CreateManagedSwarmServiceCommand? submitted = null;
        connector.Setup(value => value.CreateServiceAsync(
                It.IsAny<CreateManagedSwarmServiceCommand>(),
                It.IsAny<CancellationToken>()))
            .Callback<CreateManagedSwarmServiceCommand, CancellationToken>((command, _) => submitted = command)
            .ReturnsAsync(Result.Success(new ManagedSwarmServiceMutationResult("docker-service", [])));

        await using var scope = Services.CreateAsyncScope();
        string? progressMessage = null;
        var result = await scope.ServiceProvider.GetRequiredService<ISwarmServiceMutationService>()
            .ApplyAsync(id, Constants.SystemId, cancellationToken, message => progressMessage = message);

        Assert.True(result.IsSuccess());
        Assert.Equal(["LOG_LEVEL=debug", "STATIC=value"], submitted?.Spec.Environment);
        Assert.Equal("Injected 1 variable LOG_LEVELS=debug into the Service environment.", progressMessage);
    }

    [Fact]
    public async Task Apply_ShouldRejectAnUndefinedServiceBindingBeforeDockerMutation()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var id = await CreateServiceAsync(cancellationToken, ["LOG_LEVEL=${LOG_LEVELS}"]);

        await using var scope = Services.CreateAsyncScope();
        var result = await scope.ServiceProvider.GetRequiredService<ISwarmServiceMutationService>()
            .ApplyAsync(id, Constants.SystemId, cancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Contains("LOG_LEVELS", error.Message, StringComparison.Ordinal);
        connector.Verify(value => value.CreateServiceAsync(
            It.IsAny<CreateManagedSwarmServiceCommand>(),
            It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task ApplyEndpoint_ShouldCloseAfterWritingTheTerminalProgressItem()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var id = await CreateServiceAsync(cancellationToken);
        connector.Setup(value => value.CreateServiceAsync(
                It.IsAny<CreateManagedSwarmServiceCommand>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new ManagedSwarmServiceMutationResult("docker-service", [])));
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeout.CancelAfter(TimeSpan.FromSeconds(10));

        using var response = await Client.PostAsync(
            $"/api/v1/swarmServices/{id:D}/apply",
            new StringContent("{}", Encoding.UTF8, "application/json"),
            timeout.Token);

        response.EnsureSuccessStatusCode();
        using var payload = JsonDocument.Parse(await response.Content.ReadAsStringAsync(timeout.Token));
        var items = payload.RootElement.EnumerateArray().ToArray();
        Assert.True(items.Length >= 4);
        Assert.Contains(items, item =>
            item.GetProperty("message").GetString() == "Resolving Service variables and secrets.");
        Assert.Contains(items, item =>
            item.GetProperty("message").GetString() == "No Citadel variables or secrets were referenced by this Service.");
        var terminal = items[^1];
        Assert.Equal("accepted", terminal.GetProperty("stage").GetString());
        Assert.True(terminal.GetProperty("isCompleted").GetBoolean());
    }

    [Fact]
    public async Task Apply_WhenReconciliationCommitsFirst_ShouldReturnTheCommittedAggregate()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var id = await CreateServiceAsync(cancellationToken);
        connector.Setup(value => value.CreateServiceAsync(
                It.IsAny<CreateManagedSwarmServiceCommand>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new ManagedSwarmServiceMutationResult("docker-reconciled", [])));
        reconciliationCoordinator.Setup(coordinator => coordinator.RefreshAsync(
                platformId,
                It.IsAny<CancellationToken>()))
            .Returns<Guid, CancellationToken>(async (_, ct) =>
            {
                await using var reconciliationScope = Services.CreateAsyncScope();
                var reconciliationUow = reconciliationScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
                var current = await reconciliationUow.SwarmServices.GetAsync(id, ct);
                Assert.NotNull(current);
                current.CompleteOperation(
                    SwarmServiceOperationState.Completed,
                    current.CurrentOperation?.TargetRuntimeHash,
                    "sha256:current");
                Assert.Equal(1, await reconciliationUow.SwarmServices.UpdateAsync(current, ct));
                await reconciliationUow.CommitAsync(ct);
                return Result.Success();
            });

        await using var scope = Services.CreateAsyncScope();
        var result = await scope.ServiceProvider.GetRequiredService<ISwarmServiceMutationService>()
            .ApplyAsync(id, Constants.SystemId, cancellationToken);

        Assert.True(result.IsSuccess(out var service));
        Assert.Equal(SwarmServiceOperationState.Completed, service.CurrentOperation?.State);
        Assert.Equal("sha256:current", service.AppliedImageDigest);
    }

    [Fact]
    public async Task Reconciliation_WhenRolloutPauses_ShouldPersistTaskFailureWithoutApplySuccess()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var id = await CreateServiceAsync(cancellationToken);
        const string taskError = "starting container failed: strconv.Atoi: parsing \"\": invalid syntax";

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var service = await uow.SwarmServices.GetAsync(id, cancellationToken);
        Assert.NotNull(service);
        var operationId = Guid.CreateVersion7();
        Assert.True(service.TryPrepareOperation(
            SwarmServiceOperationKind.Apply,
            operationId,
            Constants.SystemId,
            targetRuntimeHash: "target-runtime",
            clusterId: "managed-service-cluster"));
        service.MarkOperationAttempted();
        service.MarkOperationAccepted("docker-service-paused", dockerVersion: null);
        Assert.Equal(1, await uow.SwarmServices.UpdateAsync(service, cancellationToken));
        await uow.CommitAsync(cancellationToken);

        var observedAt = DateTimeOffset.UtcNow;
        var live = new SwarmServiceProjection(
            platformId,
            "docker-service-paused",
            1,
            service.DockerName,
            "Replicated",
            "nginx@sha256:current",
            1,
            2,
            "Paused",
            "update paused after a Task failed",
            [],
            [],
            [],
            [],
            new Dictionary<string, string>
            {
                ["com.citadel.managed"] = "true",
                ["com.citadel.service-id"] = id.ToString(),
                ["com.citadel.operation-id"] = operationId.ToString()
            },
            observedAt,
            observedAt,
            observedAt,
            false,
            SwarmServiceOwnership.CitadelService,
            SwarmServiceId: id,
            LiveRuntimeHash: "target-runtime");
        var failedTask = new SwarmTaskProjection(
            platformId,
            "task-failed",
            1,
            "managed-service.2",
            live.DockerServiceId,
            service.Name,
            2,
            "worker-node",
            "worker-1",
            "Running",
            "Failed",
            "starting container failed",
            taskError,
            live.Image,
            [],
            observedAt,
            observedAt,
            observedAt,
            observedAt,
            false);

        await new PersistSwarmSnapshotWorkItem(
                platformId,
                new SwarmProjectionSnapshot([], [live], [failedTask], [], [], []))
            .ExecuteAsync(uow, cancellationToken);

        var persisted = await uow.SwarmServices.GetAsync(id, cancellationToken);
        Assert.Equal(SwarmServiceHealth.Failed, persisted?.Health);
        Assert.Equal(SwarmServiceOperationState.Rejected, persisted?.CurrentOperation?.State);
        Assert.Equal("RolloutPaused", persisted?.CurrentOperation?.ResultCode);
        Assert.Equal(taskError, persisted?.CurrentOperation?.ResultMessage);
        var failures = await uow.ActivityEventRepository.GetPagedAsync(
            id,
            ActivityResourceType.SwarmService,
            ActivityEventType.SwarmServiceOperationFailed,
            1,
            10,
            cancellationToken);
        var failure = Assert.Single(failures.Items);
        Assert.Equal(taskError, Assert.IsType<SwarmServiceOperationFailed>(failure.Info).Reason);
        var successes = await uow.ActivityEventRepository.GetPagedAsync(
            id,
            ActivityResourceType.SwarmService,
            ActivityEventType.SwarmServiceApplied,
            1,
            10,
            cancellationToken);
        Assert.Empty(successes.Items);
    }

    [Fact]
    public async Task Apply_WhenTransportOutcomeIsAmbiguous_ShouldRemainRecoverable()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var id = await CreateServiceAsync(cancellationToken);
        connector.Setup(value => value.CreateServiceAsync(
                It.IsAny<CreateManagedSwarmServiceCommand>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Failure<ManagedSwarmServiceMutationResult>(
                new InternalServerError("Agent connection was interrupted.")));

        await using var scope = Services.CreateAsyncScope();
        var result = await scope.ServiceProvider.GetRequiredService<ISwarmServiceMutationService>()
            .ApplyAsync(id, Constants.SystemId, cancellationToken);

        Assert.True(result.IsFailure());
        var persisted = await scope.ServiceProvider.GetRequiredService<IUnitOfWork>()
            .SwarmServices.GetAsync(id, cancellationToken);
        Assert.Equal(SwarmServiceOperationState.OutcomeUnknown, persisted?.CurrentOperation?.State);
        Assert.Equal(ResourceControlState.Idle, persisted?.ControlState);
    }

    [Fact]
    public async Task Apply_WhenDockerDefinitelyRejectsRequest_ShouldRecordRejection()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var id = await CreateServiceAsync(cancellationToken);
        connector.Setup(value => value.CreateServiceAsync(
                It.IsAny<CreateManagedSwarmServiceCommand>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Failure<ManagedSwarmServiceMutationResult>(
                new BadRequestError("Invalid Service specification.")));

        await using var scope = Services.CreateAsyncScope();
        var result = await scope.ServiceProvider.GetRequiredService<ISwarmServiceMutationService>()
            .ApplyAsync(id, Constants.SystemId, cancellationToken);

        Assert.True(result.IsFailure());
        var persisted = await scope.ServiceProvider.GetRequiredService<IUnitOfWork>()
            .SwarmServices.GetAsync(id, cancellationToken);
        Assert.Equal(SwarmServiceOperationState.Rejected, persisted?.CurrentOperation?.State);
        Assert.Equal(ResourceControlState.Idle, persisted?.ControlState);
    }

    [Theory]
    [InlineData(StatusCode.InvalidArgument, SwarmServiceOperationState.Rejected)]
    [InlineData(StatusCode.Unavailable, SwarmServiceOperationState.OutcomeUnknown)]
    public async Task Apply_ShouldDistinguishRpcRejectionFromAmbiguousTransportFailure(
        StatusCode statusCode,
        SwarmServiceOperationState expectedState)
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var id = await CreateServiceAsync(cancellationToken);
        connector.Setup(value => value.CreateServiceAsync(
                It.IsAny<CreateManagedSwarmServiceCommand>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Failure<ManagedSwarmServiceMutationResult>(
                new ClientRpcException("Agent request failed.", statusCode)));

        await using var scope = Services.CreateAsyncScope();
        var result = await scope.ServiceProvider.GetRequiredService<ISwarmServiceMutationService>()
            .ApplyAsync(id, Constants.SystemId, cancellationToken);

        Assert.True(result.IsFailure());
        var persisted = await scope.ServiceProvider.GetRequiredService<IUnitOfWork>()
            .SwarmServices.GetAsync(id, cancellationToken);
        Assert.Equal(expectedState, persisted?.CurrentOperation?.State);
        Assert.Equal(ResourceControlState.Idle, persisted?.ControlState);
    }

    private async Task<Guid> CreateServiceAsync(
        CancellationToken cancellationToken,
        string[]? environment = null)
    {
        using var response = await Client.PostAsJsonAsync(
            "/api/v1/swarmServices",
            new
            {
                name = $"managed-{Guid.NewGuid():N}"[..40],
                platformId,
                description = (string?)null,
                spec = CreateSpec(2, environment: environment),
                tagIds = Array.Empty<Guid>()
            },
            cancellationToken);
        response.EnsureSuccessStatusCode();
        using var document = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(cancellationToken),
            cancellationToken: cancellationToken);
        return document.RootElement.GetProperty("id").GetGuid();
    }

    private async Task MarkServiceAppliedAsync(Guid id, CancellationToken cancellationToken)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var service = await uow.SwarmServices.GetAsync(id, cancellationToken);
        Assert.NotNull(service);
        Assert.True(service.TryPrepareOperation(
            SwarmServiceOperationKind.Apply,
            Guid.CreateVersion7(),
            Constants.SystemId,
            targetRuntimeHash: "runtime-hash",
            clusterId: "managed-service-cluster"));
        service.MarkOperationAttempted();
        service.MarkOperationAccepted("docker-service", 1);
        service.CompleteOperation(
            SwarmServiceOperationState.Completed,
            "runtime-hash",
            "sha256:old");
        Assert.Equal(1, await uow.SwarmServices.UpdateAsync(service, cancellationToken));
        await uow.CommitAsync(cancellationToken);
    }

    private object CreateSpec(
        int replicas,
        object[]? secrets = null,
        object[]? configs = null,
        string? resolvedDigest = null,
        string[]? networkIds = null,
        string[]? environment = null,
        object? webhook = null,
        string updateBehavior = "Notify") => new
    {
        image = new Dictionary<string, object?>
        {
            ["$type"] = "External",
            ["registryId"] = registryId,
            ["imageTag"] = "nginx:1.27",
            ["resolvedDigest"] = resolvedDigest
        },
        updateBehavior,
        webhook,
        schedulingMode = "Replicated",
        replicas,
        command = new[] { "/docker-entrypoint.sh" },
        arguments = new[] { "nginx", "-g", "daemon off;" },
        environment = environment ?? ["APP_ENV=integration"],
        user = "101",
        workingDirectory = "/srv/app",
        healthCheck = new
        {
            test = new[] { "CMD", "curl", "-f", "http://localhost" },
            intervalNanoseconds = 30_000_000_000L,
            timeoutNanoseconds = 5_000_000_000L,
            retries = 3,
            startPeriodNanoseconds = 10_000_000_000L
        },
        stopGracePeriodNanoseconds = 20_000_000_000L,
        ports = new[] { new { targetPort = 80, publishedPort = 8080, protocol = "tcp", publishMode = "Ingress" } },
        networkIds = networkIds ?? ["overlay-network"],
        mounts = new[] { new { kind = "Volume", source = "web-data", target = "/data", readOnly = false } },
        secrets = secrets ?? [],
        configs = configs ?? [],
        resources = new
        {
            limitNanoCpus = 2_000_000_000L,
            limitMemoryBytes = 536_870_912L,
            reservationNanoCpus = 500_000_000L,
            reservationMemoryBytes = 134_217_728L
        },
        placementConstraints = new[] { "node.role==worker" },
        restartPolicy = new
        {
            condition = "OnFailure",
            delayNanoseconds = 5_000_000_000L,
            maximumAttempts = 3,
            windowNanoseconds = 60_000_000_000L
        },
        updatePolicy = new
        {
            parallelism = 2,
            delayNanoseconds = 2_000_000_000L,
            order = "StartFirst",
            failureAction = "Pause"
        }
    };

    private static object CreateGenericWebhook(string secret = "integration-shared-secret") => new
    {
        enabled = true,
        provider = "Generic",
        authScheme = "BearerToken",
        secret,
        branchFilter = (string?)null,
    };

    private static Platform CreateSwarmPlatform() => new(
        "managed-service-swarm",
        "https://managed-service-swarm.test",
        networkCount: 1,
        volumeCount: 1,
        imageCount: 1,
        cpuCount: 4,
        memTotal: 4096,
        serverVersion: "28.0",
        agentVersion: "test",
        status: PlatformStatus.Online,
        connectorType: PlatformConnectorType.Agent,
        platformDescriptor: new DockerSwarmPlatformDescriptor(
            "manager-1", "10.0.0.1", "Active", true, 3, 1,
            "managed-service-daemon", 0, 0, 0, 0, "managed-service-cluster"),
        clusterId: "managed-service-cluster");

    private static Platform CreateStandalonePlatform() => new(
        "managed-service-standalone",
        "unix:///var/run/docker.sock",
        0, 0, 0, 2, 2048, "28.0", null,
        PlatformStatus.Online,
        PlatformConnectorType.Local,
        new DockerPlatformDescriptor("managed-service-standalone-daemon", 0, 0, 0, 0));
}
