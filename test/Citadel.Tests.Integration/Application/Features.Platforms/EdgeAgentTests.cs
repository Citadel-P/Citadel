using System.Net;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Application.Services;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Activities;
using Domain.Entities.Platforms;
using Hosting.Common;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Moq;

namespace Tests.Integration.Application.Features.Platforms;

public class EdgeAgentTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<IPlatformHealthMonitorJob> healthMonitorMock = new();

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.AddSingleton(_ => healthMonitorMock.Object);
    }

    [Fact]
    public async Task EdgeAgentEnrollment_ShouldPersistHashedTokenAndSupportLifecycle()
    {
        var platformId = await CreateEdgePlatformAsync("edge-platform");

        using var enrollmentRequest = new HttpRequestMessage(
            HttpMethod.Post,
            $"/api/v1/platforms/{platformId:D}/edge/enrollments");
        enrollmentRequest.Headers.Host = "localhost:8000";
        var enrollmentResponse = await Client.SendAsync(
            enrollmentRequest,
            TestContext.Current.CancellationToken);
        var enrollmentBody = await enrollmentResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        enrollmentResponse.EnsureSuccessStatusCode();
        using var enrollmentJson = JsonDocument.Parse(enrollmentBody);
        var token = enrollmentJson.RootElement.GetProperty("token").GetString()!;
        var expiresAtUtc = enrollmentJson.RootElement.GetProperty("expiresAtUtc").GetDateTime();
        var instructions = enrollmentJson.RootElement.GetProperty("instructions");
        var environment = instructions.GetProperty("environment");

        Assert.False(string.IsNullOrWhiteSpace(token));
        Assert.InRange(expiresAtUtc - DateTime.UtcNow, TimeSpan.FromHours(23), TimeSpan.FromHours(24).Add(TimeSpan.FromMinutes(1)));
        Assert.Equal("http://localhost:8001", instructions.GetProperty("coreUrl").GetString());
        Assert.Equal("edge", environment.GetProperty("CITADEL_AGENT_MODE").GetString());
        Assert.Equal("http://localhost:8001", environment.GetProperty("CITADEL_CORE_URL").GetString());
        Assert.Equal("/app/data/edge-agent.key", environment.GetProperty("CITADEL_EDGE_AGENT_KEY_PATH").GetString());
        Assert.Contains(
            "CITADEL_CORE_URL=\"http://localhost:8001\"",
            instructions.GetProperty("dockerRunCommand").GetString());

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var enrollment = await uow.EdgeAgents.GetActiveEnrollmentAsync(platformId, DateTime.UtcNow, TestContext.Current.CancellationToken);
            Assert.NotNull(enrollment);
            Assert.NotEqual(token, enrollment.TokenHash);
            Assert.Equal(HashToken(token), enrollment.TokenHash);
        }

        var publicKey = RandomNumberGenerator.GetBytes(32);
        var fingerprint = GetFingerprint(publicKey);

        await using (var scope = Services.CreateAsyncScope())
        {
            var service = scope.ServiceProvider.GetRequiredService<IEdgeAgentManagementService>();
            var completed = await service.CompleteEnrollmentAsync(
                new EdgeAgentEnrollmentRequest(
                    token,
                    Convert.ToBase64String(publicKey),
                    fingerprint,
                    "edge-host",
                    "edge-agent-test",
                    "{}",
                    2,
                    "edge-daemon"),
                DateTime.UtcNow,
                TestContext.Current.CancellationToken);

            Assert.True(completed.IsSuccess(out var result, out var error), error?.Message);
            Assert.Equal(platformId, result.PlatformId);

            var reconnect = await service.GetReconnectBindingAsync(
                platformId,
                result.AgentId,
                fingerprint,
                "edge-daemon",
                TestContext.Current.CancellationToken);
            Assert.True(
                reconnect.IsSuccess(out _, out var reconnectError),
                reconnectError?.Message);

            var movedAgent = await service.GetReconnectBindingAsync(
                platformId,
                result.AgentId,
                fingerprint,
                "different-daemon",
                TestContext.Current.CancellationToken);
            Assert.True(movedAgent.IsFailure(out var movedAgentError));
            Assert.Contains(
                "different Docker engine",
                movedAgentError.Message);

            var reused = await service.CompleteEnrollmentAsync(
                new EdgeAgentEnrollmentRequest(
                    token,
                    Convert.ToBase64String(publicKey),
                    fingerprint,
                    "edge-host",
                    "edge-agent-test",
                    "{}",
                    2,
                    "edge-daemon"),
                DateTime.UtcNow,
                TestContext.Current.CancellationToken);

            Assert.True(reused.IsFailure());
        }

        var statusResponse = await Client.GetAsync(
            $"/api/v1/platforms/{platformId:D}/edge/status",
            TestContext.Current.CancellationToken);
        var statusBody = await statusResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        statusResponse.EnsureSuccessStatusCode();
        using var statusJson = JsonDocument.Parse(statusBody);
        Assert.Equal("Offline", statusJson.RootElement.GetProperty("connectionStatus").GetString());
        Assert.StartsWith("SHA256:", statusJson.RootElement.GetProperty("agentFingerprint").GetString());

        await using (var scope = Services.CreateAsyncScope())
        {
            var service = scope.ServiceProvider.GetRequiredService<IEdgeAgentManagementService>();
            var connectedAt = DateTime.UtcNow;

            await service.MarkConnectedAsync(
                platformId,
                "edge-host",
                "edge-agent-test",
                """{"containers":true}""",
                connectedAt,
                TestContext.Current.CancellationToken);

            await service.MarkHeartbeatAsync(
                platformId,
                new EdgeAgentHeartbeatSnapshot(
                    DockerReachable: true,
                    DockerVersion: "27.5.1",
                    Hostname: "edge-host-updated",
                    AgentVersion: "edge-agent-test-updated",
                    CapabilitiesJson: """{"containers":true,"logs":true}"""),
                connectedAt.AddSeconds(5),
                TestContext.Current.CancellationToken);
        }

        var connectedResponse = await Client.GetAsync(
            $"/api/v1/platforms/{platformId:D}/edge/status",
            TestContext.Current.CancellationToken);
        var connectedBody = await connectedResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        connectedResponse.EnsureSuccessStatusCode();
        using var connectedJson = JsonDocument.Parse(connectedBody);
        Assert.Equal("Connected", connectedJson.RootElement.GetProperty("connectionStatus").GetString());
        Assert.Equal("edge-host-updated", connectedJson.RootElement.GetProperty("lastSeenHostname").GetString());
        Assert.Equal("edge-agent-test-updated", connectedJson.RootElement.GetProperty("lastSeenVersion").GetString());

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var platform = await uow.Platforms.GetByIdAsync(platformId, TestContext.Current.CancellationToken);
            Assert.Equal(PlatformStatus.Online, platform?.Status);
            Assert.Equal(
                "edge-daemon",
                Assert.IsType<DockerPlatformDescriptor>(
                    platform?.PlatformDescriptor).DaemonId);

            var activities = await uow.ActivityEventRepository.GetPagedAsync(
                platformId,
                ActivityResourceType.Platform,
                ActivityEventType.PlatformConnected,
                1,
                10,
                TestContext.Current.CancellationToken);

            var activitySummary = Assert.Single(activities.Items);
            var activity = await uow.ActivityEventRepository.GetByIdAsync(activitySummary.Id, TestContext.Current.CancellationToken);
            var connected = Assert.IsType<PlatformConnected>(activity?.Info);
            Assert.Equal(PlatformStatus.Offline, connected.PreviousStatus);
            Assert.Equal(PlatformStatus.Online, connected.Platform.Status);
            Assert.Equal("edge-agent-test", connected.Platform.AgentVersion);
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var service = scope.ServiceProvider.GetRequiredService<IEdgeAgentManagementService>();
            await service.MarkDisconnectedAsync(
                platformId,
                DateTime.UtcNow,
                TestContext.Current.CancellationToken);
        }

        var disconnectedResponse = await Client.GetAsync(
            $"/api/v1/platforms/{platformId:D}/edge/status",
            TestContext.Current.CancellationToken);
        disconnectedResponse.EnsureSuccessStatusCode();
        using (var disconnectedJson = JsonDocument.Parse(
                   await disconnectedResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken)))
        {
            Assert.Equal("Offline", disconnectedJson.RootElement.GetProperty("connectionStatus").GetString());
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var platform = await uow.Platforms.GetByIdAsync(platformId, TestContext.Current.CancellationToken);
            Assert.Equal(PlatformStatus.Offline, platform?.Status);

            var activities = await uow.ActivityEventRepository.GetPagedAsync(
                platformId,
                ActivityResourceType.Platform,
                ActivityEventType.PlatformDisconnected,
                1,
                10,
                TestContext.Current.CancellationToken);

            var activitySummary = Assert.Single(activities.Items);
            var activity = await uow.ActivityEventRepository.GetByIdAsync(activitySummary.Id, TestContext.Current.CancellationToken);
            var disconnected = Assert.IsType<PlatformDisconnected>(activity?.Info);
            Assert.Equal(PlatformStatus.Online, disconnected.PreviousStatus);
            Assert.Equal(PlatformStatus.Offline, disconnected.Platform.Status);
        }

        var revokeResponse = await Client.PostAsync(
            $"/api/v1/platforms/{platformId:D}/edge/revoke",
            content: null,
            cancellationToken: TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.NoContent, revokeResponse.StatusCode);

        var revokedResponse = await Client.GetAsync(
            $"/api/v1/platforms/{platformId:D}/edge/status",
            TestContext.Current.CancellationToken);
        var revokedBody = await revokedResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        revokedResponse.EnsureSuccessStatusCode();
        using var revokedJson = JsonDocument.Parse(revokedBody);
        Assert.Equal("Revoked", revokedJson.RootElement.GetProperty("connectionStatus").GetString());
    }

    [Fact]
    public async Task CreateEdgeEnrollment_ShouldRejectNonEdgePlatform()
    {
        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var platform = new Platform(
                name: "agent-platform",
                address: "https://localhost:9000",
                networkCount: 0,
                volumeCount: 0,
                imageCount: 0,
                cpuCount: 1,
                memTotal: 1,
                serverVersion: null,
                agentVersion: null,
                status: PlatformStatus.Offline,
                connectorType: PlatformConnectorType.Agent,
                platformDescriptor: new DockerPlatformDescriptor("daemon", 0, 0, 0, 0));
            await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);

            var response = await Client.PostAsync(
                $"/api/v1/platforms/{platform.Id:D}/edge/enrollments",
                content: null,
                cancellationToken: TestContext.Current.CancellationToken);

            Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
        }
    }

    [Fact]
    public async Task CompleteEdgeEnrollment_ShouldRejectDockerDaemonAlreadyRegisteredByAnotherPlatform()
    {
        const string daemonId = "shared-docker-daemon";
        var edgePlatformId = await CreateEdgePlatformAsync(
            "duplicate-edge-platform");

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var existingPlatform = new Platform(
            name: "existing-platform",
            address: "unix:///existing-docker.sock",
            networkCount: 0,
            volumeCount: 0,
            imageCount: 0,
            cpuCount: 1,
            memTotal: 1,
            serverVersion: null,
            agentVersion: null,
            status: PlatformStatus.Online,
            connectorType: PlatformConnectorType.Local,
            platformDescriptor: new DockerPlatformDescriptor(
                daemonId,
                0,
                0,
                0,
                0));
        await uow.Platforms.AddAsync(
            existingPlatform,
            TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        var service =
            scope.ServiceProvider.GetRequiredService<
                IEdgeAgentManagementService>();
        var enrollment = await service.CreateEnrollmentAsync(
            edgePlatformId,
            "http://localhost:8001",
            Constants.SystemId,
            TimeSpan.FromHours(1),
            TestContext.Current.CancellationToken);
        Assert.True(
            enrollment.IsSuccess(
                out var enrollmentResult,
                out var enrollmentError),
            enrollmentError?.Message);

        var publicKey = RandomNumberGenerator.GetBytes(32);
        var result = await service.CompleteEnrollmentAsync(
            new EdgeAgentEnrollmentRequest(
                enrollmentResult.Token,
                Convert.ToBase64String(publicKey),
                GetFingerprint(publicKey),
                "edge-host",
                "edge-agent-test",
                "{}",
                2,
                daemonId),
            DateTime.UtcNow,
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Contains("existing-platform", error.Message);
        Assert.Null(
            await uow.EdgeAgents.GetBindingByPlatformIdAsync(
                edgePlatformId,
                TestContext.Current.CancellationToken));
    }

    private async Task<Guid> CreateEdgePlatformAsync(string name)
    {
        var createJson = $$"""
        {
          "name": "{{name}}",
          "type": "Docker",
          "connectorType": "edgeAgent"
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/platforms", content, TestContext.Current.CancellationToken);
        var body = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        using var json = JsonDocument.Parse(body);
        var id = json.RootElement.GetProperty("id").GetGuid();
        var address = json.RootElement.GetProperty("address").GetString()!;

        Assert.StartsWith("edge://", address);
        healthMonitorMock.Verify(x => x.TrackPlatform(address, id, PlatformConnectorType.EdgeAgent), Times.Once);

        return id;
    }

    private static string HashToken(string token)
    {
        var hash = SHA256.HashData(Encoding.UTF8.GetBytes(token));
        return Base64Url(hash);
    }

    private static string GetFingerprint(ReadOnlySpan<byte> publicKey)
    {
        var hash = SHA256.HashData(publicKey);
        return $"SHA256:{Convert.ToHexString(hash).ToLowerInvariant()}";
    }

    private static string Base64Url(ReadOnlySpan<byte> bytes)
        => Convert.ToBase64String(bytes)
            .TrimEnd('=')
            .Replace('+', '-')
            .Replace('/', '_');
}
