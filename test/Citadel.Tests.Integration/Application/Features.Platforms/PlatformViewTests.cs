using Domain.Contracts.Interfaces;
using Domain;
using Domain.Entities.Platforms;
using Domain.Entities.Tags;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Text;
using System.Text.Json;

namespace Tests.Integration.Application.Features.Platforms;

public class PlatformViewTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private Guid platformId;
    private Guid otherPlatformId;
    private Tag platformTag = null!;
    private Tag otherPlatformTag = null!;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = CreatePlatform("platform-counts-a", "https://platform-counts-a");
        var otherPlatform = CreatePlatform("platform-counts-b", "https://platform-counts-b");
        platformTag = Tag.Create("platform-filter-match", "#1144AA", Constants.SystemId);
        otherPlatformTag = Tag.Create("platform-filter-other", "#AA4411", Constants.SystemId);

        await uow.Tags.AddAsync(platformTag, TestContext.Current.CancellationToken);
        await uow.Tags.AddAsync(otherPlatformTag, TestContext.Current.CancellationToken);
        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken, [platformTag.Id], Constants.SystemId);
        await uow.Platforms.AddAsync(otherPlatform, TestContext.Current.CancellationToken, [otherPlatformTag.Id], Constants.SystemId);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        platformId = platform.Id;
        otherPlatformId = otherPlatform.Id;
    }

    [Fact]
    public async Task List_Platforms_Should_Return_Deployment_And_Stack_Counts()
    {
        await CreateDeploymentAsync("platform-counts-deployment-a", platformId);
        await CreateDeploymentAsync("platform-counts-deployment-b", otherPlatformId);
        await CreateStackAsync("platform-counts-stack-a", platformId);
        await CreateStackAsync("platform-counts-stack-b", otherPlatformId);

        var response = await Client.GetAsync("/api/v1/platforms", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var platforms = document.RootElement.GetProperty("platforms").EnumerateArray();
        var target = platforms.Single(platform => platform.GetProperty("id").GetGuid() == platformId);

        Assert.Equal(1, target.GetProperty("deploymentCount").GetInt64());
        Assert.Equal(1, target.GetProperty("stackCount").GetInt64());
    }

    [Fact]
    public async Task GetPlatformsWithLatestStatByIdsAsync_Should_Return_Selected_Platforms_With_Counts_And_Tags()
    {
        await CreateDeploymentAsync("platform-batch-counts-deployment-a", platformId);
        await CreateDeploymentAsync("platform-batch-counts-deployment-b", otherPlatformId);
        await CreateStackAsync("platform-batch-counts-stack-a", platformId);
        await CreateStackAsync("platform-batch-counts-stack-b", otherPlatformId);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var platforms = (await uow.Platforms.GetPlatformsWithLatestStatByIdsAsync(
            [platformId],
            TestContext.Current.CancellationToken)).ToArray();

        var platform = Assert.Single(platforms);
        Assert.Equal(platformId, platform.Id);
        Assert.Equal(1, platform.DeploymentCount);
        Assert.Equal(1, platform.StackCount);
        Assert.Contains(platform.Tags, tag => tag.Id == platformTag.Id);
    }

    [Fact]
    public async Task List_Platforms_Should_Filter_By_Tag_Name()
    {
        var response = await Client.GetAsync(
            $"/api/v1/platforms?tags={Uri.EscapeDataString(platformTag.Name)}",
            TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var platforms = document.RootElement.GetProperty("platforms").EnumerateArray().ToArray();
        var platform = Assert.Single(platforms);

        Assert.Equal(platformId, platform.GetProperty("id").GetGuid());
        Assert.Contains(
            platform.GetProperty("tags").EnumerateArray(),
            tag => tag.GetProperty("id").GetGuid() == platformTag.Id);
    }

    private async Task CreateDeploymentAsync(string name, Guid selectedPlatformId)
    {
        var createJson = $$"""
            {
                "name":"{{name}}",
                "platformId":"{{selectedPlatformId}}",
                "spec":{
                    "updateBehavior":"Notify",
                    "image":{
                        "$type":"External",
                        "registryId":"{{Constants.DefaultRegistryId}}",
                        "imageTag":"nginx"
                     },
                     "ports":[],
                     "networks":["96da77baf016bb722c40f10c023b2f5d1a4296b4bc6593cfad74de4bd31e8b14"]
                }
            }
            """;

        var response = await Client.PostAsync(
            "/api/v1/deployments",
            new StringContent(createJson, Encoding.UTF8, "application/json"),
            cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
    }

    private async Task CreateStackAsync(string name, Guid selectedPlatformId)
    {
        var createJson = $$"""
            {
                "name":"{{name}}",
                "platformId":"{{selectedPlatformId}}",
                "stackSource":"WebEditor",
                "spec":{
                    "$type":"WebEditor",
                    "composeFile":"docker-compose.yml"
                }
            }
            """;

        var response = await Client.PostAsync(
            "/api/v1/stacks",
            new StringContent(createJson, Encoding.UTF8, "application/json"),
            cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
    }

    private static Platform CreatePlatform(string name, string address) => new(
        name: name,
        address: address,
        networkCount: 1,
        volumeCount: 1,
        imageCount: 1,
        cpuCount: 2,
        memTotal: 512,
        serverVersion: "1.0.0",
        agentVersion: "1.0.0",
        status: PlatformStatus.Online,
        connectorType: PlatformConnectorType.Agent,
        platformDescriptor: new DockerPlatformDescriptor(
            DaemonId: name,
            ContainerCount: 0,
            ContainersRunning: 0,
            ContainersPaused: 0,
            ContainersStopped: 0));
}
