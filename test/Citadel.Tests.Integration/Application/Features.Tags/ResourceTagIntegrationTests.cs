using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Domain.Entities.Git;
using Domain.Entities.Platforms;
using Domain.Entities.Registries;
using Domain.Entities.Stacks;
using Domain.Entities.Tags;
using Hosting.Common;
using Infrastructure.Persistence;
using Microsoft.Extensions.DependencyInjection;
using System.Net;
using System.Text;
using System.Text.Json;
using StackEntity = Domain.Entities.Stacks.Stack;

namespace Tests.Integration.Application.Features.Tags;

public class ResourceTagIntegrationTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task Deployment_Api_Should_Create_Filter_Hydrate_And_Replace_Tags()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        Guid platformId;
        Tag blueTag;
        Tag greenTag;

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            blueTag = await CreateTagAsync(uow, "api-blue", "#3366FF");
            greenTag = await CreateTagAsync(uow, "api-green", "#33AA66");

            var platform = CreatePlatform("api-tag-platform");
            await uow.Platforms.AddAsync(platform, cancellationToken);
            await uow.CommitAsync(cancellationToken);
            platformId = platform.Id;
        }

        var taggedDeploymentId = await CreateDeploymentAsync("api-tagged-deployment", platformId, [blueTag.Id]);
        await CreateDeploymentAsync("api-untagged-deployment", platformId, []);

        var listResponse = await Client.GetAsync($"/api/v1/deployments?tagIds={blueTag.Id}", cancellationToken);
        listResponse.EnsureSuccessStatusCode();

        using (var document = await ReadJsonAsync(listResponse))
        {
            var deployments = document.RootElement.GetProperty("deployments");
            var deployment = Assert.Single(deployments.EnumerateArray());

            Assert.Equal(taggedDeploymentId, deployment.GetProperty("id").GetGuid());
            AssertContainsTag(deployment.GetProperty("tags"), blueTag.Id);
        }

        var getResponse = await Client.GetAsync($"/api/v1/deployments/{taggedDeploymentId}", cancellationToken);
        getResponse.EnsureSuccessStatusCode();

        using (var document = await ReadJsonAsync(getResponse))
        {
            AssertContainsTag(document.RootElement.GetProperty("tags"), blueTag.Id);
        }

        var replaceResponse = await Client.PutAsync(
            $"/api/v1/deployments/{taggedDeploymentId}/tags",
            JsonContent($$"""{ "tagIds": ["{{greenTag.Id}}"] }"""),
            cancellationToken);

        replaceResponse.EnsureSuccessStatusCode();

        var oldTagResponse = await Client.GetAsync($"/api/v1/deployments?tagIds={blueTag.Id}", cancellationToken);
        oldTagResponse.EnsureSuccessStatusCode();

        using (var document = await ReadJsonAsync(oldTagResponse))
        {
            Assert.Empty(document.RootElement.GetProperty("deployments").EnumerateArray());
        }

        var newTagResponse = await Client.GetAsync($"/api/v1/deployments?tagIds={greenTag.Id}", cancellationToken);
        newTagResponse.EnsureSuccessStatusCode();

        using (var document = await ReadJsonAsync(newTagResponse))
        {
            var deployment = Assert.Single(document.RootElement.GetProperty("deployments").EnumerateArray());
            Assert.Equal(taggedDeploymentId, deployment.GetProperty("id").GetGuid());
            AssertContainsTag(deployment.GetProperty("tags"), greenTag.Id);
        }
    }

    [Fact]
    public async Task Deployment_Api_Should_Reject_Create_When_Tag_Does_Not_Exist()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        Guid platformId;

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var platform = CreatePlatform("api-missing-tag-platform");
            await uow.Platforms.AddAsync(platform, cancellationToken);
            await uow.CommitAsync(cancellationToken);
            platformId = platform.Id;
        }

        var response = await Client.PostAsync(
            "/api/v1/deployments",
            JsonContent(DeploymentJson("api-missing-tag-deployment", platformId, [Guid.NewGuid()])),
            cancellationToken);

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);

        await using var verifyScope = Services.CreateAsyncScope();
        var verifyUow = verifyScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var deployments = await verifyUow.Deployments.GetAllAsync(cancellationToken);

        Assert.DoesNotContain(deployments, deployment => deployment.Name == "api-missing-tag-deployment");
    }

    [Fact]
    public async Task Deployment_Repository_Should_Create_Filter_And_Hydrate_Tags()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        Tag matchTag;
        Tag otherTag;
        Deployment taggedDeployment;

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            (matchTag, otherTag) = await CreateTagPairAsync(uow, "deployment");

            var platform = CreatePlatform("repo-deployment-platform");
            await uow.Platforms.AddAsync(platform, cancellationToken);

            taggedDeployment = new Deployment("repo-tagged-deployment", Constants.SystemId, platform.Id);
            var otherDeployment = new Deployment("repo-other-deployment", Constants.SystemId, platform.Id);

            Assert.True(await uow.Deployments.AddAsync(taggedDeployment, cancellationToken, [matchTag.Id], Constants.SystemId) > 0);
            Assert.True(await uow.Deployments.AddAsync(otherDeployment, cancellationToken, [otherTag.Id], Constants.SystemId) > 0);
            await uow.CommitAsync(cancellationToken);
        }

        await using var verifyScope = Services.CreateAsyncScope();
        var verifyUow = verifyScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var filtered = (await verifyUow.Deployments.GetInfoAsync(cancellationToken, [matchTag.Id])).ToArray();
        var result = Assert.Single(filtered);

        Assert.Equal(taggedDeployment.Id, result.Id);
        Assert.Contains(result.Tags, tag => tag.Id == matchTag.Id);

        var detail = await verifyUow.Deployments.GetInfoAsync(taggedDeployment.Id, cancellationToken);
        Assert.NotNull(detail);
        Assert.Contains(detail.Tags, tag => tag.Id == matchTag.Id);
    }

    [Fact]
    public async Task Stack_Repository_Should_Create_Filter_And_Hydrate_Tags()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        Tag matchTag;
        Tag otherTag;
        StackEntity taggedStack;

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            (matchTag, otherTag) = await CreateTagPairAsync(uow, "stack");

            var platform = CreatePlatform("repo-stack-platform");
            await uow.Platforms.AddAsync(platform, cancellationToken);

            taggedStack = CreateStack("repo-tagged-stack", platform.Id);
            var otherStack = CreateStack("repo-other-stack", platform.Id);

            Assert.True(await uow.Stacks.AddAsync(taggedStack, cancellationToken, [matchTag.Id], Constants.SystemId) > 0);
            Assert.True(await uow.Stacks.AddAsync(otherStack, cancellationToken, [otherTag.Id], Constants.SystemId) > 0);
            await uow.CommitAsync(cancellationToken);
        }

        await using var verifyScope = Services.CreateAsyncScope();
        var verifyUow = verifyScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var filtered = (await verifyUow.Stacks.GetInfoAsync(cancellationToken, [matchTag.Id])).ToArray();
        var result = Assert.Single(filtered);

        Assert.Equal(taggedStack.Id, result.Id);
        Assert.Contains(result.Tags, tag => tag.Id == matchTag.Id);

        var detail = await verifyUow.Stacks.GetInfoAsync(taggedStack.Id, cancellationToken);
        Assert.NotNull(detail);
        Assert.Contains(detail.Tags, tag => tag.Id == matchTag.Id);
    }

    [Fact]
    public async Task Platform_Repository_Should_Create_Filter_And_Hydrate_Tags()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        Tag matchTag;
        Tag otherTag;
        Platform taggedPlatform;

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            (matchTag, otherTag) = await CreateTagPairAsync(uow, "platform");

            taggedPlatform = CreatePlatform("repo-tagged-platform");
            var otherPlatform = CreatePlatform("repo-other-platform");

            Assert.True(await uow.Platforms.AddAsync(taggedPlatform, cancellationToken, [matchTag.Id], Constants.SystemId) > 0);
            Assert.True(await uow.Platforms.AddAsync(otherPlatform, cancellationToken, [otherTag.Id], Constants.SystemId) > 0);
            await uow.CommitAsync(cancellationToken);
        }

        await using var verifyScope = Services.CreateAsyncScope();
        var verifyUow = verifyScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var filtered = (await verifyUow.Platforms.GetPlatformsWithLatestStatAsync(cancellationToken, [matchTag.Id]))!.ToArray();
        var result = Assert.Single(filtered);

        Assert.Equal(taggedPlatform.Id, result.Id);
        Assert.Contains(result.Tags, tag => tag.Id == matchTag.Id);
    }

    [Fact]
    public async Task GitRepository_Repository_Should_Create_Filter_And_Hydrate_Tags()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        Tag matchTag;
        Tag otherTag;
        GitRepository taggedRepository;

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            (matchTag, otherTag) = await CreateTagPairAsync(uow, "git");

            taggedRepository = CreateGitRepository("repo-tagged-git");
            var otherRepository = CreateGitRepository("repo-other-git");

            Assert.True(await uow.GitRepositories.AddAsync(taggedRepository, cancellationToken, [matchTag.Id], Constants.SystemId) > 0);
            Assert.True(await uow.GitRepositories.AddAsync(otherRepository, cancellationToken, [otherTag.Id], Constants.SystemId) > 0);
            await uow.CommitAsync(cancellationToken);
        }

        await using var verifyScope = Services.CreateAsyncScope();
        var verifyUow = verifyScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var filtered = (await verifyUow.GitRepositories.GetAllAsync(cancellationToken, [matchTag.Id])).ToArray();
        var result = Assert.Single(filtered);

        Assert.Equal(taggedRepository.Id, result.Id);
        Assert.Contains(result.Tags, tag => tag.Id == matchTag.Id);
    }

    [Fact]
    public async Task Registry_Api_Should_Filter_Hydrate_And_Replace_Tags()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        Tag blueTag;
        Tag greenTag;
        Registry taggedRegistry;

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            blueTag = await CreateTagAsync(uow, "api-registry-blue", "#3366FF");
            greenTag = await CreateTagAsync(uow, "api-registry-green", "#33AA66");

            taggedRegistry = CreateRegistry("api-tagged-registry");
            var otherRegistry = CreateRegistry("api-other-registry");

            Assert.True(await uow.Registries.AddAsync(taggedRegistry, cancellationToken, [blueTag.Id], Constants.SystemId) > 0);
            Assert.True(await uow.Registries.AddAsync(otherRegistry, cancellationToken, [greenTag.Id], Constants.SystemId) > 0);
            await uow.CommitAsync(cancellationToken);
        }

        var listResponse = await Client.GetAsync($"/api/v1/registries?includeDisabled=true&tagIds={blueTag.Id}", cancellationToken);
        listResponse.EnsureSuccessStatusCode();

        using (var document = await ReadJsonAsync(listResponse))
        {
            var registry = Assert.Single(document.RootElement.GetProperty("registries").EnumerateArray());

            Assert.Equal(taggedRegistry.Id, registry.GetProperty("id").GetGuid());
            AssertContainsTag(registry.GetProperty("tags"), blueTag.Id);
        }

        var configResponse = await Client.GetAsync($"/api/v1/registries/{taggedRegistry.Id}/_cfg", cancellationToken);
        configResponse.EnsureSuccessStatusCode();

        using (var document = await ReadJsonAsync(configResponse))
        {
            AssertContainsTag(document.RootElement.GetProperty("tags"), blueTag.Id);
        }

        var replaceResponse = await Client.PutAsync(
            $"/api/v1/registries/{taggedRegistry.Id}/tags",
            JsonContent($$"""{ "tagIds": ["{{greenTag.Id}}"] }"""),
            cancellationToken);

        replaceResponse.EnsureSuccessStatusCode();

        var oldTagResponse = await Client.GetAsync($"/api/v1/registries?includeDisabled=true&tagIds={blueTag.Id}", cancellationToken);
        oldTagResponse.EnsureSuccessStatusCode();

        using (var document = await ReadJsonAsync(oldTagResponse))
        {
            Assert.Empty(document.RootElement.GetProperty("registries").EnumerateArray());
        }

        var newTagResponse = await Client.GetAsync($"/api/v1/registries?includeDisabled=true&tagIds={greenTag.Id}", cancellationToken);
        newTagResponse.EnsureSuccessStatusCode();

        using (var document = await ReadJsonAsync(newTagResponse))
        {
            var registries = document.RootElement.GetProperty("registries").EnumerateArray().ToArray();

            Assert.Contains(registries, registry => registry.GetProperty("id").GetGuid() == taggedRegistry.Id);
        }
    }

    [Fact]
    public async Task Registry_Repository_Should_Create_Filter_And_Hydrate_Tags()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        Tag matchTag;
        Tag otherTag;
        Registry taggedRegistry;

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            (matchTag, otherTag) = await CreateTagPairAsync(uow, "registry");

            taggedRegistry = CreateRegistry("repo-tagged-registry");
            var otherRegistry = CreateRegistry("repo-other-registry");

            Assert.True(await uow.Registries.AddAsync(taggedRegistry, cancellationToken, [matchTag.Id], Constants.SystemId) > 0);
            Assert.True(await uow.Registries.AddAsync(otherRegistry, cancellationToken, [otherTag.Id], Constants.SystemId) > 0);
            await uow.CommitAsync(cancellationToken);
        }

        await using var verifyScope = Services.CreateAsyncScope();
        var verifyUow = verifyScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var filtered = (await verifyUow.Registries.GetAllAsync(cancellationToken, [matchTag.Id])).ToArray();
        var result = Assert.Single(filtered);

        Assert.Equal(taggedRegistry.Id, result.Id);
        Assert.Contains(result.Tags, tag => tag.Id == matchTag.Id);

        var detail = await verifyUow.Registries.GetAsync(taggedRegistry.Id, cancellationToken);
        Assert.NotNull(detail);
        Assert.Contains(detail.Tags, tag => tag.Id == matchTag.Id);
    }

    [Fact]
    public async Task ResourceTag_Migration_Should_Keep_Query_Supporting_Indexes()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        await using var scope = Services.CreateAsyncScope();
        var connectionFactory = scope.ServiceProvider.GetRequiredService<IDbConnectionFactory>();
        await using var connection = connectionFactory.Create();
        await connection.OpenAsync(cancellationToken);

        await using var command = connection.CreateCommand();
        command.CommandText = """
            SELECT indexname
            FROM pg_indexes
            WHERE schemaname = 'public'
              AND tablename = 'resourcetags'
        """;

        var indexes = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        await using var reader = await command.ExecuteReaderAsync(cancellationToken);

        while (await reader.ReadAsync(cancellationToken))
        {
            indexes.Add(reader.GetString(0));
        }

        Assert.Contains("ix_resourcetags_resource", indexes);
        Assert.Contains("ix_resourcetags_filter", indexes);
    }

    private static async Task<Tag> CreateTagAsync(IUnitOfWork uow, string name, string color)
    {
        var tag = Tag.Create(name, color, Constants.SystemId);
        await uow.Tags.AddAsync(tag, TestContext.Current.CancellationToken);
        return tag;
    }

    private static async Task<(Tag MatchTag, Tag OtherTag)> CreateTagPairAsync(IUnitOfWork uow, string prefix)
        => (
            await CreateTagAsync(uow, $"{prefix}-match", "#1144AA"),
            await CreateTagAsync(uow, $"{prefix}-other", "#AA4411"));

    private async Task<Guid> CreateDeploymentAsync(string name, Guid platformId, IReadOnlyCollection<Guid> tagIds)
    {
        var response = await Client.PostAsync(
            "/api/v1/deployments",
            JsonContent(DeploymentJson(name, platformId, tagIds)),
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        using var document = await ReadJsonAsync(response);
        return document.RootElement.GetProperty("id").GetGuid();
    }

    private static string DeploymentJson(string name, Guid platformId, IReadOnlyCollection<Guid> tagIds)
    {
        var tagIdsJson = string.Join(",", tagIds.Select(id => $"\"{id}\""));

        return $$"""
            {
                "name":"{{name}}",
                "platformId":"{{platformId}}",
                "spec":{
                    "updateBehavior":"Notify",
                    "image":{
                        "$type":"External",
                        "registryId":"{{Constants.DefaultRegistryId}}",
                        "imageTag":"nginx"
                     },
                     "ports":[],
                     "networks":["96da77baf016bb722c40f10c023b2f5d1a4296b4bc6593cfad74de4bd31e8b14"]
                },
                "tagIds":[{{tagIdsJson}}]
            }
            """;
    }

    private static StringContent JsonContent(string json) => new(json, Encoding.UTF8, "application/json");

    private static async Task<JsonDocument> ReadJsonAsync(HttpResponseMessage response)
        => await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

    private static void AssertContainsTag(JsonElement tags, Guid tagId)
    {
        Assert.Contains(
            tags.EnumerateArray(),
            tag => tag.GetProperty("id").GetGuid() == tagId);
    }

    private static Platform CreatePlatform(string name)
        => new(
            name: name,
            address: $"https://{name}.test",
            networkCount: 1,
            volumeCount: 1,
            imageCount: 1,
            cpuCount: 2,
            memTotal: 2048,
            serverVersion: "1.0.0",
            agentVersion: "1.0.0",
            status: PlatformStatus.Online,
            connectorType: PlatformConnectorType.Agent,
            platformDescriptor: new DockerPlatformDescriptor(
                DaemonId: Guid.NewGuid().ToString("N"),
                ContainerCount: 0,
                ContainersRunning: 0,
                ContainersPaused: 0,
                ContainersStopped: 0));

    private static StackEntity CreateStack(string name, Guid platformId)
        => StackEntity.Create(
            name,
            Constants.SystemId,
            StackSource.WebEditor,
            platformId,
            new ManualStack("docker-compose.yml", StackUpdateBehavior.Disabled));

    private static GitRepository CreateGitRepository(string name)
        => new(
            name: name,
            description: $"{name} description",
            url: $"https://github.com/citadel-p/{name}.git",
            defaultBranch: "main",
            gitAccountId: null,
            createdByActorId: Constants.SystemId);

    private static Registry CreateRegistry(string name)
        => new(
            name,
            $"{name}.registry.test",
            RegistryStatus.Active,
            Constants.SystemId,
            DockerHubRegistry.Create("dummy-user", "dummy-pat123"),
            $"{name} description");
}
