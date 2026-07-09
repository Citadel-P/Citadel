using Application.Services;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Microsoft.Extensions.DependencyInjection;
using Moq;

namespace Tests.Unit.Application.Services;

public sealed class EdgeAgentManagementServiceTests
{
    [Fact]
    public async Task MarkHeartbeatAsync_ShouldUpdateBindingAndCommit()
    {
        var platformId = Guid.CreateVersion7();
        var utcNow = new DateTime(2026, 7, 9, 18, 30, 0, DateTimeKind.Utc);
        var heartbeat = new EdgeAgentHeartbeatSnapshot(
            DockerReachable: true,
            DockerVersion: "27.5.1",
            Hostname: "edge-host",
            AgentVersion: "edge-agent-test",
            CapabilitiesJson: """{"containers":true,"logs":true}""");

        var edgeAgents = new Mock<IEdgeAgentRepository>(MockBehavior.Strict);
        edgeAgents
            .Setup(x => x.UpdateBindingHeartbeatAsync(
                platformId,
                utcNow,
                heartbeat.Hostname,
                heartbeat.AgentVersion,
                heartbeat.CapabilitiesJson,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var unitOfWork = new Mock<IUnitOfWork>(MockBehavior.Strict);
        unitOfWork.SetupGet(x => x.EdgeAgents).Returns(edgeAgents.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);
        unitOfWork
            .Setup(x => x.DisposeAsync())
            .Returns(ValueTask.CompletedTask);

        await using var provider = new ServiceCollection()
            .AddScoped(_ => unitOfWork.Object)
            .BuildServiceProvider();

        var service = new EdgeAgentManagementService(provider.GetRequiredService<IServiceScopeFactory>());

        await service.MarkHeartbeatAsync(platformId, heartbeat, utcNow, TestContext.Current.CancellationToken);

        edgeAgents.VerifyAll();
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
    }
}
