using Application.Services.Backups;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Volumes;
using Domain.Entities.Platforms;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Moq;

namespace Tests.Unit.Application.Services.Backups;

public sealed class SwarmWorkloadBackupVolumeResolverTests
{
    [Fact]
    public async Task ResolveAsync_EqualVolumeNamesOnDifferentNodes_RemainDistinctAndUseExactNodes()
    {
        var platform = CreatePlatform();
        var service = CreateService(platform.Id, desiredTasks: 2);
        var tasks = new[]
        {
            CreateTask(platform.Id, service.DockerServiceId, "task-2", "node-2", "worker-2", "container-2"),
            CreateTask(platform.Id, service.DockerServiceId, "task-1", "node-1", "worker-1", "container-1")
        };
        var swarm = new Mock<ISwarmProjectionRepository>();
        swarm.Setup(x => x.GetNodesAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([CreateNode(platform.Id, "node-1", "worker-1"), CreateNode(platform.Id, "node-2", "worker-2")]);
        swarm.Setup(x => x.GetTasksAsync(
                platform.Id,
                5_000,
                It.IsAny<CancellationToken>(),
                service.DockerServiceId))
            .ReturnsAsync(tasks);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Swarm).Returns(swarm.Object);
        using var services = new ServiceCollection().AddSingleton(unitOfWork.Object).BuildServiceProvider();
        var nodeConnector = new Mock<ISwarmNodeRuntimeConnector>();
        nodeConnector.Setup(x => x.InspectContainerAsync(
                platform,
                It.IsAny<string>(),
                It.IsAny<string>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync((Platform _, string _, string containerId, CancellationToken _) =>
                Result.Success(Inspection(containerId, "database")));
        nodeConnector.Setup(x => x.InspectVolumeAsync(
                platform,
                It.IsAny<string>(),
                "database",
                It.IsAny<CancellationToken>()))
            .ReturnsAsync((Platform _, string nodeId, string _, CancellationToken _) =>
                Result.Success(Volume("database") with { DockerNodeId = nodeId }));
        var resolver = new SwarmWorkloadBackupVolumeResolver(
            services.GetRequiredService<IServiceScopeFactory>(),
            nodeConnector.Object);

        var result = await resolver.ResolveAsync(platform, [service], TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var volumes, out var error), error?.Message);
        Assert.Collection(
            volumes,
            volume =>
            {
                Assert.Equal("database", volume.VolumeName);
                Assert.Equal("node-1", volume.DockerNodeId);
                Assert.Equal("worker-1", volume.NodeHostname);
            },
            volume =>
            {
                Assert.Equal("database", volume.VolumeName);
                Assert.Equal("node-2", volume.DockerNodeId);
                Assert.Equal("worker-2", volume.NodeHostname);
            });
        nodeConnector.Verify(x => x.InspectContainerAsync(
            platform,
            "node-1",
            "container-1",
            It.IsAny<CancellationToken>()), Times.Once);
        nodeConnector.Verify(x => x.InspectContainerAsync(
            platform,
            "node-2",
            "container-2",
            It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task ResolveAsync_IncompleteTaskPlacement_FailsBeforeInspectingNodes()
    {
        var platform = CreatePlatform();
        var service = CreateService(platform.Id, desiredTasks: 2);
        var swarm = new Mock<ISwarmProjectionRepository>();
        swarm.Setup(x => x.GetNodesAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([CreateNode(platform.Id, "node-1", "worker-1")]);
        swarm.Setup(x => x.GetTasksAsync(
                platform.Id,
                5_000,
                It.IsAny<CancellationToken>(),
                service.DockerServiceId))
            .ReturnsAsync([CreateTask(platform.Id, service.DockerServiceId, "task-1", "node-1", "worker-1", "container-1")]);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Swarm).Returns(swarm.Object);
        using var services = new ServiceCollection().AddSingleton(unitOfWork.Object).BuildServiceProvider();
        var nodeConnector = new Mock<ISwarmNodeRuntimeConnector>(MockBehavior.Strict);
        var resolver = new SwarmWorkloadBackupVolumeResolver(
            services.GetRequiredService<IServiceScopeFactory>(),
            nodeConnector.Object);

        var result = await resolver.ResolveAsync(platform, [service], TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Contains("placement is not stable", error.Message, StringComparison.OrdinalIgnoreCase);
        nodeConnector.VerifyNoOtherCalls();
    }

    [Fact]
    public async Task ResolveAsync_NonRunningTask_FailsBeforeInspectingNodes()
    {
        var platform = CreatePlatform();
        var service = CreateService(platform.Id, desiredTasks: 1);
        var swarm = new Mock<ISwarmProjectionRepository>();
        swarm.Setup(x => x.GetNodesAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([CreateNode(platform.Id, "node-1", "worker-1")]);
        swarm.Setup(x => x.GetTasksAsync(
                platform.Id,
                5_000,
                It.IsAny<CancellationToken>(),
                service.DockerServiceId))
            .ReturnsAsync([
                CreateTask(platform.Id, service.DockerServiceId, "task-1", "node-1", "worker-1", "container-1")
                    with { State = "failed" }
            ]);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Swarm).Returns(swarm.Object);
        using var services = new ServiceCollection().AddSingleton(unitOfWork.Object).BuildServiceProvider();
        var nodeConnector = new Mock<ISwarmNodeRuntimeConnector>(MockBehavior.Strict);
        var resolver = new SwarmWorkloadBackupVolumeResolver(
            services.GetRequiredService<IServiceScopeFactory>(),
            nodeConnector.Object);

        var result = await resolver.ResolveAsync(platform, [service], TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Contains("is not running", error.Message, StringComparison.OrdinalIgnoreCase);
        nodeConnector.VerifyNoOtherCalls();
    }

    [Fact]
    public async Task ResolveAsync_StaleNode_FailsBeforeInspectingNodeRuntime()
    {
        var platform = CreatePlatform();
        var service = CreateService(platform.Id, desiredTasks: 1);
        var swarm = new Mock<ISwarmProjectionRepository>();
        swarm.Setup(x => x.GetNodesAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([CreateNode(platform.Id, "node-1", "worker-1") with { IsStale = true }]);
        swarm.Setup(x => x.GetTasksAsync(
                platform.Id,
                5_000,
                It.IsAny<CancellationToken>(),
                service.DockerServiceId))
            .ReturnsAsync([CreateTask(platform.Id, service.DockerServiceId, "task-1", "node-1", "worker-1", "container-1")]);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Swarm).Returns(swarm.Object);
        using var services = new ServiceCollection().AddSingleton(unitOfWork.Object).BuildServiceProvider();
        var nodeConnector = new Mock<ISwarmNodeRuntimeConnector>(MockBehavior.Strict);
        var resolver = new SwarmWorkloadBackupVolumeResolver(
            services.GetRequiredService<IServiceScopeFactory>(),
            nodeConnector.Object);

        var result = await resolver.ResolveAsync(platform, [service], TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Contains("unavailable or stale", error.Message, StringComparison.OrdinalIgnoreCase);
        nodeConnector.VerifyNoOtherCalls();
    }

    private static Platform CreatePlatform() => new(
        "swarm",
        "http://localhost.docker",
        0,
        0,
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
            "daemon-1",
            2,
            2,
            0,
            0));

    private static SwarmServiceProjection CreateService(Guid platformId, int desiredTasks) => new(
        platformId,
        "service-1",
        1,
        "database",
        "Replicated",
        "postgres:latest",
        desiredTasks,
        desiredTasks,
        "Completed",
        null,
        [],
        [],
        [],
        [],
        new Dictionary<string, string>(),
        DateTimeOffset.UtcNow,
        DateTimeOffset.UtcNow,
        DateTimeOffset.UtcNow,
        IsStale: false);

    private static SwarmTaskProjection CreateTask(
        Guid platformId,
        string serviceId,
        string taskId,
        string nodeId,
        string hostname,
        string containerId) => new(
        platformId,
        taskId,
        1,
        taskId,
        serviceId,
        "database",
        1,
        nodeId,
        hostname,
        "running",
        "running",
        null,
        null,
        "postgres:latest",
        [],
        DateTimeOffset.UtcNow,
        DateTimeOffset.UtcNow,
        DateTimeOffset.UtcNow,
        DateTimeOffset.UtcNow,
        IsStale: false,
        DockerContainerId: containerId);

    private static SwarmNodeProjection CreateNode(Guid platformId, string nodeId, string hostname) => new(
        platformId,
        nodeId,
        1,
        hostname,
        "worker",
        false,
        "reachable",
        "ready",
        null,
        "active",
        "28.0",
        "linux",
        "x86_64",
        "10.0.0.2",
        new Dictionary<string, string>(),
        1,
        1,
        null,
        null,
        DateTimeOffset.UtcNow,
        IsStale: false);

    private static ContainerInspectionInfo Inspection(string containerId, string volumeName) => new(
        containerId,
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
        [new MountPointInfo("volume", volumeName, volumeName, "/data", "local", "rw", true, string.Empty)],
        null,
        null);

    private static DockerVolumeResult Volume(string name) => new(
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
}
