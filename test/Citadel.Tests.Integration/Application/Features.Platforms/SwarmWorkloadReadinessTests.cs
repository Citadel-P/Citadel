using System.Net;
using System.Net.Http.Headers;
using System.Text;
using System.Text.Json;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Platforms;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Platforms;

public sealed class SwarmWorkloadReadinessTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private Guid platformId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = new Platform(
            name: "swarm-manager",
            address: "https://swarm.example.test",
            networkCount: 1,
            volumeCount: 1,
            imageCount: 1,
            cpuCount: 4,
            memTotal: 1024,
            serverVersion: "28.0.0",
            agentVersion: "1.0.0",
            status: PlatformStatus.Online,
            connectorType: PlatformConnectorType.Agent,
            platformDescriptor: new DockerSwarmPlatformDescriptor(
                NodeID: "node-1",
                NodeAddr: "10.0.0.1",
                LocalNodeState: "Active",
                ControlAvailable: true,
                Nodes: 1,
                Managers: 1,
                DaemonId: "daemon-1",
                ContainerCount: 0,
                ContainersRunning: 0,
                ContainersPaused: 0,
                ContainersStopped: 0,
                ClusterId: "cluster-1"),
            clusterId: "cluster-1");

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        platformId = platform.Id;
    }

    [Fact]
    public async Task CreateDeployment_WhenSwarmApplyIsUnavailable_ReturnsNotFound()
    {
        var json = $$"""
        {
          "name": "swarm-deployment",
          "platformId": "{{platformId}}",
          "spec": {
            "updateBehavior": "Disabled",
            "image": {
              "$type": "External",
              "registryId": "{{Constants.DefaultRegistryId}}",
              "imageTag": "nginx:latest"
            }
          }
        }
        """;

        using var response = await Client.PostAsync(
            "/api/v1/deployments",
            new StringContent(json, Encoding.UTF8, "application/json"),
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task CreateStack_WhenPlatformIsSwarm_CreatesLockedSwarmDraft()
    {
        var json = $$"""
        {
          "name": "swarm-stack",
          "platformId": "{{platformId}}",
          "stackSource": "WebEditor",
          "spec": {
            "$type": "WebEditor",
            "composeFile": "services:\n  web:\n    image: nginx:latest",
            "updateBehavior": "Disabled",
            "destroyBeforeDeploy": false
          }
        }
        """;

        using var response = await Client.PostAsync(
            "/api/v1/stacks",
            new StringContent(json, Encoding.UTF8, "application/json"),
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        using var body = JsonDocument.Parse(await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
        Assert.Equal("DockerSwarm", body.RootElement.GetProperty("platformType").GetString());

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stack = Assert.Single(await uow.Stacks.GetInfoAsync(TestContext.Current.CancellationToken));
        Assert.Equal(platformId, stack.CurrentStackRelease?.PlatformId);
        Assert.Equal(PlatformType.DockerSwarm, stack.CurrentStackRelease?.Platform?.PlatformDescriptor.Type);
        Assert.Equal(StackReleaseStatus.Created, stack.CurrentStackRelease?.Status);
    }

    [Fact]
    public async Task PreflightSwarmStack_WhenComposeIsSupported_ReturnsCompatibleReport()
    {
        var json = $$"""
        {
          "name": "swarm-stack",
          "platformId": "{{platformId}}",
          "stackSource": "WebEditor",
          "spec": {
            "$type": "WebEditor",
            "composeFile": "services:\n  web:\n    image: nginx:latest\n    deploy:\n      replicas: 2",
            "updateBehavior": "Disabled",
            "destroyBeforeDeploy": false
          }
        }
        """;

        using var response = await Client.PostAsync(
            "/api/v1/stacks/preflight/swarm",
            new StringContent(json, Encoding.UTF8, "application/json"),
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        using var body = JsonDocument.Parse(await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
        Assert.True(body.RootElement.GetProperty("isCompatible").GetBoolean());
        Assert.Empty(body.RootElement.GetProperty("issues").EnumerateArray());
    }

    [Fact]
    public async Task PreflightSwarmStack_WhenComposeUsesStandaloneField_ReturnsCompatibilityError()
    {
        var json = $$"""
        {
          "name": "swarm-stack",
          "platformId": "{{platformId}}",
          "stackSource": "WebEditor",
          "spec": {
            "$type": "WebEditor",
            "composeFile": "services:\n  web:\n    image: nginx:latest\n    container_name: fixed-web",
            "updateBehavior": "Disabled",
            "destroyBeforeDeploy": false
          }
        }
        """;

        using var response = await Client.PostAsync(
            "/api/v1/stacks/preflight/swarm",
            new StringContent(json, Encoding.UTF8, "application/json"),
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        using var body = JsonDocument.Parse(await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
        Assert.False(body.RootElement.GetProperty("isCompatible").GetBoolean());
        Assert.Contains(
            body.RootElement.GetProperty("issues").EnumerateArray(),
            issue => issue.GetProperty("code").GetString() == "compose.unsupported_key");
    }

    [Fact]
    public async Task StackPlatformLookup_InAddMode_IncludesSwarmPlatforms()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Platform, platformId, PermissionLevel.Read)]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        using var response = await Client.GetAsync(
            "/api/v1/lookup?sourceResourceType=Stack&targetResourceType=Platform",
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        using var body = JsonDocument.Parse(await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
        Assert.Contains(
            body.RootElement.EnumerateArray(),
            item => item.GetProperty("id").GetGuid() == platformId);
    }
}
