using System.Net;
using System.Text;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Platforms;
using Hosting.Common;
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
    public async Task CreateStack_WhenSwarmApplyIsUnavailable_ReturnsNotFound()
    {
        var json = $$"""
        {
          "name": "swarm-stack",
          "platformId": "{{platformId}}",
          "stackSource": "WebEditor",
          "spec": {
            "$type": "WebEditor",
            "composeFile": "services:\n  web:\n    image: nginx:latest",
            "updateBehavior": "Disabled"
          }
        }
        """;

        using var response = await Client.PostAsync(
            "/api/v1/stacks",
            new StringContent(json, Encoding.UTF8, "application/json"),
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }
}
