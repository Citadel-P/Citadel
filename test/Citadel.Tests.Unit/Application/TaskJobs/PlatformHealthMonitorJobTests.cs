using Application.Services;
using Application.Services.Alerts;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
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

    private static ServiceProvider CreateProvider(IEdgeAgentRepository edgeAgents)
    {
        var unitOfWork = new Mock<IUnitOfWork>(MockBehavior.Strict);
        unitOfWork.SetupGet(x => x.EdgeAgents).Returns(edgeAgents);
        unitOfWork.Setup(x => x.DisposeAsync()).Returns(ValueTask.CompletedTask);

        return new ServiceCollection()
            .AddScoped(_ => unitOfWork.Object)
            .BuildServiceProvider();
    }

    private static PlatformHealthMonitorJob CreateJob(IServiceScopeFactory scopeFactory)
        => new(
            Mock.Of<IAlertService>(),
            scopeFactory,
            Mock.Of<IPlatformHealthBroadCaster>(),
            Mock.Of<IConnectorFactory<IPlatformConnector>>(),
            Mock.Of<ILogger<PlatformHealthMonitorJob>>());

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
