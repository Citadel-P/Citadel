using System.Text.Json;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Platforms;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Infrastructure.Persistence;

public sealed class EdgeAgentRepositoryTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task BindingLifecycle_ShouldPersistConnectedAndHeartbeatCapabilitiesJson()
    {
        var platform = CreateEdgePlatform();
        var binding = new EdgeAgentBinding(
            Id: Guid.CreateVersion7(),
            PlatformId: platform.Id,
            AgentId: Guid.CreateVersion7(),
            AgentPublicKey: "public-key",
            AgentFingerprint: "SHA256:fingerprint",
            ConnectionStatus: EdgeAgentConnectionStatus.Offline,
            LastConnectedAtUtc: null,
            LastDisconnectedAtUtc: null,
            LastHeartbeatAtUtc: null,
            LastSeenVersion: "edge-agent-initial",
            LastSeenHostname: "edge-host-initial",
            CapabilitiesJson: """{"enrolled":true}""",
            ProtocolVersion: 1,
            RevokedAtUtc: null,
            CreatedAtUtc: DateTime.UtcNow,
            UpdatedAtUtc: DateTime.UtcNow);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.EdgeAgents.AddBindingAsync(binding, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        var added = await uow.EdgeAgents.GetBindingByPlatformIdAsync(platform.Id, TestContext.Current.CancellationToken);
        Assert.NotNull(added);
        AssertJsonProperty(added.CapabilitiesJson, "enrolled", true);

        var connectedAt = DateTime.UtcNow.AddSeconds(1);
        await uow.EdgeAgents.UpdateBindingConnectedAsync(
            platform.Id,
            connectedAt,
            "edge-host-connected",
            "edge-agent-connected",
            """{"containers":true}""",
            TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        var connected = await uow.EdgeAgents.GetBindingByPlatformIdAsync(platform.Id, TestContext.Current.CancellationToken);
        Assert.NotNull(connected);
        Assert.Equal(EdgeAgentConnectionStatus.Connected, connected.ConnectionStatus);
        Assert.Equal("edge-host-connected", connected.LastSeenHostname);
        Assert.Equal("edge-agent-connected", connected.LastSeenVersion);
        AssertJsonProperty(connected.CapabilitiesJson, "containers", true);

        var heartbeatAt = connectedAt.AddSeconds(5);
        await uow.EdgeAgents.UpdateBindingHeartbeatAsync(
            platform.Id,
            heartbeatAt,
            "edge-host-heartbeat",
            "edge-agent-heartbeat",
            """{"containers":true,"logs":true}""",
            TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        var heartbeat = await uow.EdgeAgents.GetBindingByPlatformIdAsync(platform.Id, TestContext.Current.CancellationToken);
        Assert.NotNull(heartbeat);
        Assert.Equal("edge-host-heartbeat", heartbeat.LastSeenHostname);
        Assert.Equal("edge-agent-heartbeat", heartbeat.LastSeenVersion);
        AssertJsonProperty(heartbeat.CapabilitiesJson, "logs", true);

        await uow.EdgeAgents.UpdateBindingHeartbeatAsync(
            platform.Id,
            heartbeatAt.AddSeconds(5),
            hostname: null,
            agentVersion: null,
            capabilitiesJson: null,
            TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        var preserved = await uow.EdgeAgents.GetBindingByPlatformIdAsync(platform.Id, TestContext.Current.CancellationToken);
        Assert.NotNull(preserved);
        Assert.Equal("edge-host-heartbeat", preserved.LastSeenHostname);
        Assert.Equal("edge-agent-heartbeat", preserved.LastSeenVersion);
        AssertJsonProperty(preserved.CapabilitiesJson, "logs", true);
    }

    private static Platform CreateEdgePlatform()
        => new(
            name: $"edge-platform-{Guid.CreateVersion7():N}",
            address: $"edge://{Guid.CreateVersion7():N}",
            networkCount: 0,
            volumeCount: 0,
            imageCount: 0,
            cpuCount: 1,
            memTotal: 1,
            serverVersion: null,
            agentVersion: null,
            status: PlatformStatus.Offline,
            connectorType: PlatformConnectorType.EdgeAgent,
            platformDescriptor: new DockerPlatformDescriptor("daemon", 0, 0, 0, 0));

    private static void AssertJsonProperty(string? json, string propertyName, bool expected)
    {
        Assert.False(string.IsNullOrWhiteSpace(json));
        using var document = JsonDocument.Parse(json);
        Assert.Equal(expected, document.RootElement.GetProperty(propertyName).GetBoolean());
    }
}
