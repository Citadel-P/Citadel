using Application.Services;
using Application.Services.Alerts;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Platforms;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using Moq;

namespace Tests.Unit.Application.TaskJobs;

public sealed class PlatformHealthMonitorJobTests
{
    [Fact]
    public async Task ShouldRaisePlatformUnreachableAlertAsync_ShouldAllowNonEdgePlatforms()
    {
        await using var provider = new ServiceCollection().BuildServiceProvider();
        var job = CreateJob(provider.GetRequiredService<IServiceScopeFactory>());

        var shouldRaise = await job.ShouldRaisePlatformUnreachableAlertAsync(
            Guid.CreateVersion7(),
            PlatformConnectorType.Agent,
            TestContext.Current.CancellationToken);

        Assert.True(shouldRaise);
    }

    [Fact]
    public async Task ShouldRaisePlatformUnreachableAlertAsync_ShouldSuppressUnenrolledEdgePlatforms()
    {
        var platformId = Guid.CreateVersion7();
        var edgeAgents = new Mock<IEdgeAgentRepository>(MockBehavior.Strict);
        edgeAgents
            .Setup(x => x.GetBindingByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync((EdgeAgentBinding?)null);

        await using var provider = CreateProvider(edgeAgents.Object);
        var job = CreateJob(provider.GetRequiredService<IServiceScopeFactory>());

        var shouldRaise = await job.ShouldRaisePlatformUnreachableAlertAsync(
            platformId,
            PlatformConnectorType.EdgeAgent,
            TestContext.Current.CancellationToken);

        Assert.False(shouldRaise);
        edgeAgents.VerifyAll();
    }

    [Fact]
    public async Task ShouldRaisePlatformUnreachableAlertAsync_ShouldAllowEnrolledEdgePlatforms()
    {
        var platformId = Guid.CreateVersion7();
        var edgeAgents = new Mock<IEdgeAgentRepository>(MockBehavior.Strict);
        edgeAgents
            .Setup(x => x.GetBindingByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(CreateBinding(platformId));

        await using var provider = CreateProvider(edgeAgents.Object);
        var job = CreateJob(provider.GetRequiredService<IServiceScopeFactory>());

        var shouldRaise = await job.ShouldRaisePlatformUnreachableAlertAsync(
            platformId,
            PlatformConnectorType.EdgeAgent,
            TestContext.Current.CancellationToken);

        Assert.True(shouldRaise);
        edgeAgents.VerifyAll();
    }

    [Fact]
    public async Task ShouldRaisePlatformUnreachableAlertAsync_ShouldSuppressRevokedEdgeBindings()
    {
        var platformId = Guid.CreateVersion7();
        var edgeAgents = new Mock<IEdgeAgentRepository>(MockBehavior.Strict);
        edgeAgents
            .Setup(x => x.GetBindingByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(CreateBinding(platformId, EdgeAgentConnectionStatus.Revoked, DateTime.UtcNow));

        await using var provider = CreateProvider(edgeAgents.Object);
        var job = CreateJob(provider.GetRequiredService<IServiceScopeFactory>());

        var shouldRaise = await job.ShouldRaisePlatformUnreachableAlertAsync(
            platformId,
            PlatformConnectorType.EdgeAgent,
            TestContext.Current.CancellationToken);

        Assert.False(shouldRaise);
        edgeAgents.VerifyAll();
    }

    [Fact]
    public async Task CheckHealthAsync_ShouldTreatTimeoutAsOffline()
    {
        var connector = new Mock<IPlatformConnector>();
        connector
            .Setup(x => x.CheckHealthAsync(It.IsAny<string>(), It.IsAny<CancellationToken>()))
            .Returns(async (string _, CancellationToken cancellationToken) =>
            {
                await Task.Delay(Timeout.InfiniteTimeSpan, cancellationToken);
                return new PlatformHealthResult(true);
            });

        var connectorFactory = new Mock<IConnectorFactory<IPlatformConnector>>();
        connectorFactory
            .Setup(x => x.GetConnector(PlatformConnectorType.Agent))
            .Returns(connector.Object);

        await using var provider = new ServiceCollection().BuildServiceProvider();
        var job = CreateJob(
            provider.GetRequiredService<IServiceScopeFactory>(),
            connectorFactory.Object);

        var result = await job.CheckHealthAsync(
            "https://unresponsive.example",
            PlatformConnectorType.Agent,
            TimeSpan.FromMilliseconds(10),
            TestContext.Current.CancellationToken);

        Assert.False(result);
    }

    [Theory]
    [InlineData(PlatformConnectorType.Local, "unix:///var/run/docker.sock")]
    [InlineData(PlatformConnectorType.EdgeAgent, "edge://10000000-0000-0000-0000-000000000001")]
    public async Task UntrackPlatform_ShouldNotEvictDirectAgentCacheForNonAgentPlatforms(
        PlatformConnectorType connectorType,
        string address)
    {
        var connectionCache = new Mock<IPlatformConnectionCache>(MockBehavior.Strict);
        await using var provider = new ServiceCollection().BuildServiceProvider();
        var job = CreateJob(
            provider.GetRequiredService<IServiceScopeFactory>(),
            connectionCache: connectionCache.Object);

        Assert.True(job.TrackPlatform(address, Guid.CreateVersion7(), connectorType));
        Assert.True(await job.UntrackPlatform(address, TestContext.Current.CancellationToken));

        connectionCache.VerifyNoOtherCalls();
    }

    [Fact]
    public async Task UntrackPlatform_ShouldEvictDirectAgentCacheForAgentPlatform()
    {
        const string address = "https://agent.example";
        var connectionCache = new Mock<IPlatformConnectionCache>(MockBehavior.Strict);
        connectionCache.Setup(x => x.Evict(address));

        await using var provider = new ServiceCollection().BuildServiceProvider();
        var job = CreateJob(
            provider.GetRequiredService<IServiceScopeFactory>(),
            connectionCache: connectionCache.Object);

        Assert.True(job.TrackPlatform(address, Guid.CreateVersion7(), PlatformConnectorType.Agent));
        Assert.True(await job.UntrackPlatform(address, TestContext.Current.CancellationToken));

        connectionCache.Verify(x => x.Evict(address), Times.Once);
        connectionCache.VerifyNoOtherCalls();
    }

    private static ServiceProvider CreateProvider(IEdgeAgentRepository edgeAgents)
    {
        var unitOfWork = new Mock<IUnitOfWork>(MockBehavior.Strict);
        unitOfWork.SetupGet(x => x.EdgeAgents).Returns(edgeAgents);
        unitOfWork.Setup(x => x.DisposeAsync()).Returns(ValueTask.CompletedTask);

        return new ServiceCollection()
            .AddScoped(_ => unitOfWork.Object)
            .BuildServiceProvider();
    }

    private static PlatformHealthMonitorJob CreateJob(
        IServiceScopeFactory scopeFactory,
        IConnectorFactory<IPlatformConnector>? connectorFactory = null,
        IPlatformConnectionCache? connectionCache = null)
        => new(
            Mock.Of<IAlertService>(),
            scopeFactory,
            Mock.Of<IPlatformHealthBroadCaster>(),
            connectorFactory ?? Mock.Of<IConnectorFactory<IPlatformConnector>>(),
            Mock.Of<ILogger<PlatformHealthMonitorJob>>(),
            connectionCache);

    private static EdgeAgentBinding CreateBinding(
        Guid platformId,
        EdgeAgentConnectionStatus status = EdgeAgentConnectionStatus.Offline,
        DateTime? revokedAtUtc = null)
        => new(
            Id: Guid.CreateVersion7(),
            PlatformId: platformId,
            ResourceType: EdgeAgentResourceType.Platform,
            ResourceId: platformId,
            AgentId: Guid.CreateVersion7(),
            AgentPublicKey: "public-key",
            AgentFingerprint: "SHA256:fingerprint",
            ConnectionStatus: status,
            LastConnectedAtUtc: null,
            LastDisconnectedAtUtc: null,
            LastHeartbeatAtUtc: null,
            LastSeenVersion: "edge-agent-test",
            LastSeenHostname: "edge-host",
            CapabilitiesJson: "{}",
            ProtocolVersion: 1,
            RevokedAtUtc: revokedAtUtc,
            CreatedAtUtc: DateTime.UtcNow,
            UpdatedAtUtc: DateTime.UtcNow);
}
