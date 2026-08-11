using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Platforms;
using LightResults;
using Moq;

namespace Tests.Unit.Application.Services;

public sealed class SwarmManagerIdentityValidatorTests
{
    [Fact]
    public async Task ValidateAsync_ShouldSucceedWhenPinnedManagerIdentityMatches()
    {
        var (validator, platform) = CreateSubject(CreateDescriptor());

        var result = await validator.ValidateAsync(platform, TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(), string.Join(Environment.NewLine, result.Errors.Select(error => error.Message)));
    }

    [Theory]
    [InlineData("other-node", "daemon-1", "cluster-1")]
    [InlineData("node-1", "other-daemon", "cluster-1")]
    [InlineData("node-1", "daemon-1", "other-cluster")]
    public async Task ValidateAsync_ShouldRejectChangedManagerIdentity(
        string nodeId,
        string daemonId,
        string clusterId)
    {
        var (validator, platform) = CreateSubject(CreateDescriptor(nodeId, daemonId, clusterId));

        var result = await validator.ValidateAsync(platform, TestContext.Current.CancellationToken);

        Assert.False(result.IsSuccess());
        Assert.Contains(result.Errors, error => error.Message.StartsWith("ManagerIdentityChanged:", StringComparison.Ordinal));
    }

    private static (SwarmManagerIdentityValidator Validator, Platform Platform) CreateSubject(
        DockerSwarmPlatformDescriptor observedDescriptor)
    {
        var connector = new Mock<IPlatformConnector>(MockBehavior.Strict);
        connector
            .Setup(value => value.GetPlatformAsync(
                It.Is<GetPlatformCommand>(command =>
                    command.PlatformAddress == "agent://manager-1"
                    && command.PlatformName == "swarm"),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new PlatformResult(
                "swarm",
                "agent://manager-1",
                0,
                0,
                0,
                1,
                1024,
                "28.0",
                "1.0",
                observedDescriptor,
                ClusterId: observedDescriptor.ClusterId)));

        var connectorFactory = new Mock<IConnectorFactory<IPlatformConnector>>(MockBehavior.Strict);
        connectorFactory
            .Setup(value => value.GetConnector(PlatformConnectorType.Agent))
            .Returns(connector.Object);

        var platform = new Platform(
            "swarm",
            "agent://manager-1",
            0,
            0,
            0,
            1,
            1024,
            "28.0",
            "1.0",
            PlatformStatus.Online,
            PlatformConnectorType.Agent,
            CreateDescriptor(),
            clusterId: "cluster-1");

        return (new SwarmManagerIdentityValidator(connectorFactory.Object), platform);
    }

    private static DockerSwarmPlatformDescriptor CreateDescriptor(
        string nodeId = "node-1",
        string daemonId = "daemon-1",
        string clusterId = "cluster-1") =>
        new(
            nodeId,
            "10.0.0.1",
            "Active",
            true,
            1,
            1,
            daemonId,
            0,
            0,
            0,
            0,
            ClusterId: clusterId);
}
