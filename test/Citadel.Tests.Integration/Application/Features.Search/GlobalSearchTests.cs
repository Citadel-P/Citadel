using System.Net;
using System.Net.Http.Headers;
using System.Text.Json;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Domain.Entities.Platforms;
using Domain.Entities.SwarmServices;
using Hosting.Common;

namespace Tests.Integration.Application.Features.Search;

public sealed class GlobalSearchTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private Guid _exactPlatformId;
    private Guid _prefixPlatformId;
    private Guid _containsPlatformId;
    private Guid _literalWildcardPlatformId;
    private Guid _deploymentId;
    private Guid _swarmPlatformId;
    private Guid _swarmServiceId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var exactPlatform = CreatePlatform("prod", "https://prod.local", PlatformStatus.Online);
        var prefixPlatform = CreatePlatform("production-east", "https://production-east.local", PlatformStatus.Online);
        var containsPlatform = CreatePlatform("east-production", "https://east-production.local", PlatformStatus.Offline);
        var literalWildcardPlatform = CreatePlatform("literal%prod", "https://literal-percent.local", PlatformStatus.Online);

        await uow.Platforms.AddAsync(exactPlatform, TestContext.Current.CancellationToken);
        await uow.Platforms.AddAsync(prefixPlatform, TestContext.Current.CancellationToken);
        await uow.Platforms.AddAsync(containsPlatform, TestContext.Current.CancellationToken);
        await uow.Platforms.AddAsync(literalWildcardPlatform, TestContext.Current.CancellationToken);

        var swarmPlatform = CreateSwarmPlatform("swarm-search-host");
        await uow.Platforms.AddAsync(swarmPlatform, TestContext.Current.CancellationToken);
        _swarmPlatformId = swarmPlatform.Id;

        _exactPlatformId = exactPlatform.Id;
        _prefixPlatformId = prefixPlatform.Id;
        _containsPlatformId = containsPlatform.Id;
        _literalWildcardPlatformId = literalWildcardPlatform.Id;

        var deployment = new Deployment(
            name: "production-api",
            createdByActorId: Constants.SystemId,
            platformId: exactPlatform.Id,
            spec: new DeploymentSpec(
                new ExternalImage(Constants.DefaultRegistryId, "nginx:latest"),
                UpdateBehavior.Notify));
        await uow.Deployments.AddAsync(deployment, TestContext.Current.CancellationToken);
        _deploymentId = deployment.Id;

        var swarmService = new SwarmService(
            "production-cache",
            swarmPlatform.Id,
            Constants.SystemId,
            new SwarmServiceSpec
            {
                Image = new SwarmExternalImage(Constants.DefaultRegistryId, "redis:latest")
            });
        await uow.SwarmServices.AddAsync(swarmService, TestContext.Current.CancellationToken);
        _swarmServiceId = swarmService.Id;

        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    [Fact]
    public async Task Search_Admin_Should_Rank_Exact_Prefix_And_Contains_Matches()
    {
        var response = await Client.GetAsync(
            "/api/v1/search?q=prod&types=Platform&limitPerType=5",
            TestContext.Current.CancellationToken);
        var body = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        Assert.True(response.IsSuccessStatusCode, body);

        using var document = JsonDocument.Parse(body);
        var items = Assert.Single(document.RootElement.GetProperty("groups").EnumerateArray())
            .GetProperty("items")
            .EnumerateArray()
            .ToArray();

        Assert.Equal(4, items.Length);
        Assert.Equal(_exactPlatformId, items[0].GetProperty("id").GetGuid());
        Assert.Equal(_prefixPlatformId, items[1].GetProperty("id").GetGuid());
        Assert.Contains(items.Skip(2), item => item.GetProperty("id").GetGuid() == _containsPlatformId);
        Assert.Equal("Positive", items[0].GetProperty("status").GetProperty("tone").GetString());
    }

    [Fact]
    public async Task Search_Should_Treat_Like_Wildcards_As_Literals()
    {
        var response = await Client.GetAsync(
            "/api/v1/search?q=%25p&types=Platform",
            TestContext.Current.CancellationToken);
        var body = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        Assert.True(response.IsSuccessStatusCode, body);

        using var document = JsonDocument.Parse(body);
        var item = Assert.Single(
            Assert.Single(document.RootElement.GetProperty("groups").EnumerateArray())
                .GetProperty("items")
                .EnumerateArray());

        Assert.Equal(_literalWildcardPlatformId, item.GetProperty("id").GetGuid());
    }

    [Fact]
    public async Task Search_Should_Filter_Resources_And_Omit_Inaccessible_Parent()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.Deployment, _deploymentId, PermissionLevel.Read)
            ]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync(
            "/api/v1/search?q=production&types=Platform,Deployment",
            TestContext.Current.CancellationToken);
        var body = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        Assert.True(response.IsSuccessStatusCode, body);

        using var document = JsonDocument.Parse(body);
        var groups = document.RootElement.GetProperty("groups").EnumerateArray().ToArray();
        var group = Assert.Single(groups);
        Assert.Equal("Deployments", group.GetProperty("category").GetString());

        var item = Assert.Single(group.GetProperty("items").EnumerateArray());
        Assert.Equal(_deploymentId, item.GetProperty("id").GetGuid());
        Assert.False(item.TryGetProperty("parent", out _));
        Assert.False(item.TryGetProperty("secondaryText", out _));
    }

    [Fact]
    public async Task Search_Should_Include_Independently_Authorized_Parent()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.Deployment, _deploymentId, PermissionLevel.Read),
                new ResourceGrant(ResourceType.Platform, _exactPlatformId, PermissionLevel.Read)
            ]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync(
            "/api/v1/search?q=production-api&types=Deployment",
            TestContext.Current.CancellationToken);
        var body = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        Assert.True(response.IsSuccessStatusCode, body);

        using var document = JsonDocument.Parse(body);
        var item = Assert.Single(
            Assert.Single(document.RootElement.GetProperty("groups").EnumerateArray())
                .GetProperty("items")
                .EnumerateArray());
        var parent = item.GetProperty("parent");

        Assert.Equal(_exactPlatformId, parent.GetProperty("id").GetGuid());
        Assert.Equal("Platform", parent.GetProperty("resourceType").GetString());
        Assert.Equal("prod", parent.GetProperty("name").GetString());
    }

    [Fact]
    public async Task Search_Should_Return_Authorized_Managed_Swarm_Service()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.SwarmService, _swarmServiceId, PermissionLevel.Read),
                new ResourceGrant(ResourceType.Platform, _swarmPlatformId, PermissionLevel.Read)
            ]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync(
            "/api/v1/search?q=production-cache&types=SwarmService",
            TestContext.Current.CancellationToken);
        var body = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        Assert.True(response.IsSuccessStatusCode, body);

        using var document = JsonDocument.Parse(body);
        var group = Assert.Single(document.RootElement.GetProperty("groups").EnumerateArray());
        Assert.Equal("SwarmServices", group.GetProperty("category").GetString());

        var item = Assert.Single(group.GetProperty("items").EnumerateArray());
        Assert.Equal(_swarmServiceId, item.GetProperty("id").GetGuid());
        Assert.Equal("SwarmService", item.GetProperty("resourceType").GetString());
        Assert.Equal("Created", item.GetProperty("status").GetProperty("label").GetString());
        Assert.Equal("swarm-search-host", item.GetProperty("parent").GetProperty("name").GetString());
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public async Task Search_Should_Use_Direct_And_Team_Global_Permissions(bool useTeamRole)
    {
        var subject = await CreateAuthorizationSubjectAsync(
            directRoleId: useTeamRole ? null : ViewerRoleId,
            teamRoleId: useTeamRole ? ViewerRoleId : null);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync(
            "/api/v1/search?q=prod&types=Platform",
            TestContext.Current.CancellationToken);
        var body = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        Assert.True(response.IsSuccessStatusCode, body);

        using var document = JsonDocument.Parse(body);
        var items = Assert.Single(document.RootElement.GetProperty("groups").EnumerateArray())
            .GetProperty("items")
            .EnumerateArray()
            .ToArray();

        Assert.Equal(4, items.Length);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _exactPlatformId);
    }

    [Theory]
    [InlineData("/api/v1/search?q=p")]
    [InlineData("/api/v1/search?q=prod&types=Unknown")]
    [InlineData("/api/v1/search?q=prod&limitPerType=11")]
    public async Task Search_Should_Reject_Invalid_Parameters(string requestUri)
    {
        var response = await Client.GetAsync(requestUri, TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    private static Platform CreatePlatform(string name, string address, PlatformStatus status)
        => new(
            name: name,
            address: address,
            networkCount: 1,
            volumeCount: 2,
            imageCount: 3,
            cpuCount: 4,
            memTotal: 500,
            serverVersion: "1.0.0",
            agentVersion: "1.0.0",
            status: status,
            connectorType: PlatformConnectorType.Agent,
            platformDescriptor: new DockerPlatformDescriptor(
                DaemonId: $"{name}-daemon",
                ContainerCount: 0,
                ContainersRunning: 0,
                ContainersPaused: 0,
                ContainersStopped: 0));

    private static Platform CreateSwarmPlatform(string name) => new(
        name: name,
        address: $"https://{name}.local",
        networkCount: 1,
        volumeCount: 1,
        imageCount: 1,
        cpuCount: 2,
        memTotal: 512,
        serverVersion: "1.0.0",
        agentVersion: "1.0.0",
        status: PlatformStatus.Online,
        connectorType: PlatformConnectorType.Agent,
        platformDescriptor: new DockerSwarmPlatformDescriptor(
            NodeID: $"{name}-node",
            NodeAddr: "10.0.0.1",
            LocalNodeState: "Active",
            ControlAvailable: true,
            Nodes: 1,
            Managers: 1,
            DaemonId: $"{name}-daemon",
            ContainerCount: 0,
            ContainersRunning: 0,
            ContainersPaused: 0,
            ContainersStopped: 0,
            ClusterId: $"{name}-cluster"),
        clusterId: $"{name}-cluster");
}
