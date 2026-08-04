using Application.Features.Swarm.Queries;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Platforms;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Moq;

namespace Tests.Unit.Application.Features.Swarm;

public sealed class GetSwarmNodesTests
{
    [Fact]
    public void Query_ShouldRequirePlatformReadPermission()
    {
        Assert.Contains(
            typeof(GetSwarmNodes).GetCustomAttributes(inherit: false),
            attribute => attribute is RequirePermissionAttribute);
        Assert.Contains(
            typeof(GetSwarmNode).GetCustomAttributes(inherit: false),
            attribute => attribute is RequirePermissionAttribute);
    }

    [Theory]
    [MemberData(nameof(InventoryQueryTypes))]
    public void InventoryQuery_ShouldRequirePlatformReadPermission(Type queryType)
    {
        var permission = Assert.IsType<RequirePermissionAttribute>(
            Assert.Single(queryType.GetCustomAttributes(typeof(RequirePermissionAttribute), inherit: false)));

        Assert.Equal(ResourceType.Platform, permission.ResourceType);
        Assert.Equal(PermissionLevel.Read, permission.PermissionLevel);
    }

    public static TheoryData<Type> InventoryQueryTypes => new()
    {
        typeof(GetSwarmServices), typeof(GetSwarmService),
        typeof(GetSwarmTasks), typeof(GetSwarmTask),
        typeof(GetSwarmNetworks), typeof(GetSwarmNetwork),
        typeof(GetSwarmSecrets), typeof(GetSwarmSecret),
        typeof(GetSwarmConfigs), typeof(GetSwarmConfig)
    };

    [Theory]
    [InlineData(0)]
    [InlineData(201)]
    public void TaskQuery_ShouldRejectAnUnboundedLimit(int limit)
    {
        var result = new GetSwarmTasks.Validator().Validate(new GetSwarmTasks(Guid.CreateVersion7(), limit));

        Assert.False(result.IsValid);
        Assert.Contains(result.Errors, error => error.PropertyName == nameof(GetSwarmTasks.Limit));
    }

    [Fact]
    public async Task Handler_ShouldRejectStandaloneDockerPlatform()
    {
        var platform = new Platform(
            "docker",
            "unix:///var/run/docker.sock",
            0,
            0,
            0,
            1,
            1024,
            "28.0",
            null,
            PlatformStatus.Online,
            PlatformConnectorType.Local,
            new DockerPlatformDescriptor("daemon-1", 0, 0, 0, 0));
        var platforms = new Mock<IPlatformRepository>();
        platforms
            .Setup(value => value.GetByIdAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(platform);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(value => value.Platforms).Returns(platforms.Object);
        var coordinator = new Mock<ISwarmReconciliationCoordinator>();
        var handler = new GetSwarmNodesHandler(unitOfWork.Object, coordinator.Object);

        var result = await handler.Handle(
            new GetSwarmNodes(platform.Id),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Contains("only available", error.Message, StringComparison.OrdinalIgnoreCase);
        coordinator.Verify(
            value => value.EnsureInitializedAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task Handler_ShouldInitializeAnEmptyProjectionBeforeReturningIt()
    {
        var platform = CreateSwarmPlatform();
        var node = new SwarmNodeProjection(
            platform.Id, "node-1", 1, "manager-1", "Manager", true, "Reachable", "Ready",
            null, "Active", "28.0", "linux", "x86_64", "10.0.0.1", new Dictionary<string, string>(), 1, 1,
            null, null, DateTimeOffset.UtcNow, false);
        var platforms = new Mock<IPlatformRepository>();
        platforms
            .Setup(value => value.GetByIdAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(platform);
        var swarm = new Mock<ISwarmProjectionRepository>();
        swarm
            .SetupSequence(value => value.GetNodesAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([])
            .ReturnsAsync([node]);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(value => value.Platforms).Returns(platforms.Object);
        unitOfWork.SetupGet(value => value.Swarm).Returns(swarm.Object);
        var coordinator = new Mock<ISwarmReconciliationCoordinator>();
        coordinator
            .Setup(value => value.EnsureInitializedAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
        var handler = new GetSwarmNodesHandler(unitOfWork.Object, coordinator.Object);

        var result = await handler.Handle(
            new GetSwarmNodes(platform.Id),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var nodes, out var error), error?.Message);
        Assert.Same(node, Assert.Single(nodes));
        coordinator.Verify(
            value => value.EnsureInitializedAsync(platform.Id, It.IsAny<CancellationToken>()),
            Times.Once);
    }

    private static Platform CreateSwarmPlatform() => new(
        "swarm",
        "unix:///var/run/docker.sock",
        0,
        0,
        0,
        1,
        1024,
        "28.0",
        null,
        PlatformStatus.Online,
        PlatformConnectorType.Local,
        new DockerSwarmPlatformDescriptor(
            "node-1", "10.0.0.1", "Active", true, 1, 1, "daemon-1", 0, 0, 0, 0));
}
