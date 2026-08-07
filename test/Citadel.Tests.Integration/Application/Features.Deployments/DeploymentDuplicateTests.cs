using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Domain.Entities.Deployments;
using Domain.Entities.ResourceBindings;
using Domain.Entities.Tags;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Net;
using System.Text;
using System.Text.Json.Nodes;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Deployments;

public class DeploymentDuplicateTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private Guid _platformId;
    private Guid _sourceDeploymentId;
    private Guid _tagId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();
        var tag = Tag.Create("duplicate-deployment", "#3366FF", Constants.SystemId);
        var deployment = new Deployment(
            name: "source-deployment",
            createdByActorId: Constants.SystemId,
            platformId: platform.Id,
            spec: new DeploymentSpec(
                Image: new ExternalImage(Constants.DefaultRegistryId, "nginx:latest", "sha256:original"),
                UpdateBehavior: UpdateBehavior.Notify,
                Ports: ["8080:80"],
                Volumes: ["/srv/source-data:/data"],
                Networks: ["frontend"],
                EnvironmentVariables: ["API_KEY=${api_key}"]),
            description: "source description");

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Tags.AddAsync(tag, TestContext.Current.CancellationToken);
        await uow.Deployments.AddAsync(
            deployment,
            TestContext.Current.CancellationToken,
            [tag.Id],
            Constants.SystemId);
        await uow.ResourceBindings.AddAsync(
            new ResourceBinding(
                Name: "api_key",
                Kind: ResourceBindingKind.Variable,
                Scope: ResourceBindingScope.Deployment,
                ResourceId: deployment.Id,
                Value: "secret-value",
                SecretId: null),
            TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        _sourceDeploymentId = deployment.Id;
        _platformId = platform.Id;
        _tagId = tag.Id;
    }

    [Fact]
    public async Task DuplicateDraft_Should_CreateDeployment_And_RecordDuplicateActivity()
    {
        var draftResponse = await Client.GetAsync(
            $"/api/v1/deployments/{_sourceDeploymentId}/duplicate-draft",
            TestContext.Current.CancellationToken);
        draftResponse.EnsureSuccessStatusCode();

        var draftDocument = await ReadJsonObjectAsync(draftResponse);
        var draft = draftDocument["draft"]!.AsObject();
        var warnings = draftDocument["warnings"]!.AsArray();
        var image = draft["spec"]!["image"]!.AsObject();

        Assert.Equal("source-deployment-copy", draft["name"]!.GetValue<string>());
        Assert.Equal(_tagId, draft["tagIds"]!.AsArray()[0]!.GetValue<Guid>());
        Assert.Equal("External", image["$type"]!.GetValue<string>());
        Assert.True(!image.TryGetPropertyValue("resolvedDigest", out var resolvedDigest) || resolvedDigest is null);
        Assert.DoesNotContain(warnings, warning =>
            warning?["code"]?.GetValue<string>() == "RESOURCE_BINDINGS_NOT_COPIED");
        AssertHasWarning(warnings, "HOST_BIND_MOUNT");

        var createResponse = await Client.PostAsync(
            "/api/v1/deployments",
            JsonContent(draft.ToJsonString()),
            TestContext.Current.CancellationToken);
        createResponse.EnsureSuccessStatusCode();

        var createDocument = await ReadJsonObjectAsync(createResponse);
        var createdDeploymentId = createDocument["id"]!.GetValue<Guid>();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var created = await uow.Deployments.GetAsync(createdDeploymentId, TestContext.Current.CancellationToken);

        Assert.NotNull(created);
        Assert.Equal("source-deployment-copy", created.Name);
        Assert.Equal("source description", created.Description);
        var externalImage = Assert.IsType<ExternalImage>(created.Spec!.Image);
        Assert.Null(externalImage.ResolvedDigest);
        Assert.Equal(_tagId, Assert.Single(created.Tags).Id);

        var copiedBinding = Assert.Single(await uow.ResourceBindings.GetEntriesAsync(
            ResourceBindingScope.Deployment,
            createdDeploymentId,
            TestContext.Current.CancellationToken));
        Assert.Equal("api_key", copiedBinding.Name);
        Assert.Equal("secret-value", copiedBinding.Value);
        Assert.Equal(createdDeploymentId, copiedBinding.ResourceId);
        var sourceBinding = Assert.Single(await uow.ResourceBindings.GetEntriesAsync(
            ResourceBindingScope.Deployment,
            _sourceDeploymentId,
            TestContext.Current.CancellationToken));
        Assert.NotEqual(sourceBinding.Id, copiedBinding.Id);

        var activities = await uow.ActivityEventRepository.GetPagedAsync(
            createdDeploymentId,
            ActivityResourceType.Deployment,
            ActivityEventType.DeploymentDuplicated,
            1,
            10,
            TestContext.Current.CancellationToken);
        var activity = await uow.ActivityEventRepository.GetByIdAsync(
            Assert.Single(activities.Items).Id,
            TestContext.Current.CancellationToken);
        var info = Assert.IsType<DeploymentDuplicated>(activity?.Info);

        Assert.Equal(_sourceDeploymentId, info.Source.ResourceId);
        Assert.Equal("source-deployment", info.Source.ResourceName);
        Assert.Equal("source-deployment-copy", info.Deployment.Name);
    }

    [Fact]
    public async Task DuplicateDraft_Should_UseNextAvailableName_WhenDefaultCopyNameExists()
    {
        await AddDeploymentAsync("source-deployment-copy");

        var draftResponse = await Client.GetAsync(
            $"/api/v1/deployments/{_sourceDeploymentId}/duplicate-draft",
            TestContext.Current.CancellationToken);
        draftResponse.EnsureSuccessStatusCode();

        var draftDocument = await ReadJsonObjectAsync(draftResponse);
        var draft = draftDocument["draft"]!.AsObject();

        Assert.Equal("source-deployment-copy-2", draft["name"]!.GetValue<string>());
    }

    [Fact]
    public async Task CreateDeployment_WithInvalidDuplicateSourceType_Should_ReturnBadRequest_And_NotCreateDeployment()
    {
        var createJson = $$"""
        {
          "name": "invalid-source-type-deployment",
          "platformId": "{{_platformId}}",
          "description": "invalid duplicate source type",
          "spec": {
            "image": {
              "$type": "External",
              "registryId": "{{Constants.DefaultRegistryId}}",
              "imageTag": "nginx:latest"
            },
            "updateBehavior": "Disabled"
          },
          "duplicateSource": {
            "resourceType": "Stack",
            "resourceId": "{{_sourceDeploymentId}}",
            "resourceName": "forged-source"
          }
        }
        """;

        var response = await Client.PostAsync(
            "/api/v1/deployments",
            JsonContent(createJson),
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        Assert.False(await uow.Deployments.ExistsAsync(
            "invalid-source-type-deployment",
            _platformId,
            TestContext.Current.CancellationToken));
    }

    [Fact]
    public async Task CreateDeployment_WithMissingDuplicateSource_Should_ReturnNotFound_And_NotCreateDeployment()
    {
        var createJson = $$"""
        {
          "name": "missing-source-deployment",
          "platformId": "{{_platformId}}",
          "description": "missing duplicate source",
          "spec": {
            "image": {
              "$type": "External",
              "registryId": "{{Constants.DefaultRegistryId}}",
              "imageTag": "nginx:latest"
            },
            "updateBehavior": "Disabled"
          },
          "duplicateSource": {
            "resourceType": "Deployment",
            "resourceId": "{{Guid.CreateVersion7()}}",
            "resourceName": "missing-source"
          }
        }
        """;

        var response = await Client.PostAsync(
            "/api/v1/deployments",
            JsonContent(createJson),
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        Assert.False(await uow.Deployments.ExistsAsync(
            "missing-source-deployment",
            _platformId,
            TestContext.Current.CancellationToken));
    }

    private async Task AddDeploymentAsync(string name)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var deployment = new Deployment(
            name: name,
            createdByActorId: Constants.SystemId,
            platformId: _platformId,
            spec: new DeploymentSpec(
                Image: new ExternalImage(Constants.DefaultRegistryId, "nginx:latest"),
                UpdateBehavior: UpdateBehavior.Disabled));

        await uow.Deployments.AddAsync(deployment, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    private static async Task<JsonObject> ReadJsonObjectAsync(HttpResponseMessage response)
    {
        var json = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        return JsonNode.Parse(json)!.AsObject();
    }

    private static StringContent JsonContent(string json)
        => new(json, Encoding.UTF8, "application/json");

    private static void AssertHasWarning(JsonArray warnings, string code)
        => Assert.Contains(warnings, warning => warning?["code"]?.GetValue<string>() == code);
}
