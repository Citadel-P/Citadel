using Application.Configs;
using Application.Features.Platforms.Queries;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Platforms;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.Options;
using Moq;

namespace Tests.Unit.Application.Features.Platforms;

public sealed class GetSwarmNodeAgentCoverageTests
{
    [Fact]
    public async Task Handler_ShouldInitializeAnEmptySwarmInventoryBeforeCalculatingCoverage()
    {
        var platform = CreateSwarmPlatform();
        var node = new SwarmNodeProjection(
            platform.Id, "node-1", 1, "manager-1", "Manager", true, "Reachable", "Ready",
            null, "Active", "28.0", "linux", "x86_64", "10.0.0.1", new Dictionary<string, string>(), 0, 0,
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
        swarm
            .Setup(value => value.GetServicesAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);
        swarm
            .Setup(value => value.GetNodeRuntimeStatesAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);
        var edgeAgents = new Mock<IEdgeAgentRepository>();
        edgeAgents
            .Setup(value => value.GetNodeBindingsAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(value => value.Platforms).Returns(platforms.Object);
        unitOfWork.SetupGet(value => value.Swarm).Returns(swarm.Object);
        unitOfWork.SetupGet(value => value.EdgeAgents).Returns(edgeAgents.Object);
        var coordinator = new Mock<ISwarmReconciliationCoordinator>();
        coordinator
            .Setup(value => value.EnsureInitializedAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
        var handler = new GetSwarmNodeAgentCoverageHandler(
            unitOfWork.Object,
            Mock.Of<IEdgeAgentSessionStatus>(),
            coordinator.Object,
            Options.Create(new EdgeAgentOptions()));

        var result = await handler.Handle(
            new GetSwarmNodeAgentCoverage(platform.Id),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var coverage, out var error), error?.Message);
        Assert.Equal(1, coverage.TotalNodes);
        coordinator.Verify(
            value => value.EnsureInitializedAsync(platform.Id, It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task Handler_ShouldReturnInitializationFailure()
    {
        var platform = CreateSwarmPlatform();
        var platforms = new Mock<IPlatformRepository>();
        platforms
            .Setup(value => value.GetByIdAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(platform);
        var swarm = new Mock<ISwarmProjectionRepository>();
        swarm
            .Setup(value => value.GetNodesAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(value => value.Platforms).Returns(platforms.Object);
        unitOfWork.SetupGet(value => value.Swarm).Returns(swarm.Object);
        var coordinator = new Mock<ISwarmReconciliationCoordinator>();
        coordinator
            .Setup(value => value.EnsureInitializedAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Failure(new InternalServerError("Swarm inventory refresh failed.")));
        var handler = new GetSwarmNodeAgentCoverageHandler(
            unitOfWork.Object,
            Mock.Of<IEdgeAgentSessionStatus>(),
            coordinator.Object,
            Options.Create(new EdgeAgentOptions()));

        var result = await handler.Handle(
            new GetSwarmNodeAgentCoverage(platform.Id),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Equal("Swarm inventory refresh failed.", error.Message);
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
