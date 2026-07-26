using System.Net;
using System.Net.Http.Json;
using System.Text.Json;
using Tests.Acceptance.Infrastructure;

namespace Tests.Acceptance.Recovery;

[Collection("AcceptancePostgres")]
public sealed class EdgeAgentRecoveryTests(AcceptancePostgresFixture postgres)
{
    [Fact]
    public async Task EdgeAgent_ShouldRecoverAndRejectRevokedIdentity()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var connectionString =
            await postgres.CreateDatabaseAsync(cancellationToken);
        var workingDirectory = Path.Combine(
            Path.GetTempPath(),
            "citadel-acceptance",
            $"edge-agent-{Guid.NewGuid():N}");
        Directory.CreateDirectory(workingDirectory);

        await using var agent = new EdgeAgentProtocolClient();
        try
        {
            await using (var firstCandidate =
                await CandidateApplicationProcess.StartAsync(
                    connectionString,
                    workingDirectory,
                    cancellationToken))
            {
                await firstCandidate.AuthenticateAsAdminAsync(cancellationToken);
                var platformId = await CreateEdgePlatformAsync(
                    firstCandidate,
                    cancellationToken);
                var enrollmentToken = await CreateEnrollmentAsync(
                    firstCandidate,
                    platformId,
                    cancellationToken);

                await agent.EnrollAsync(
                    firstCandidate.GrpcAddress,
                    enrollmentToken,
                    cancellationToken);
                Assert.Equal(platformId, agent.PlatformId);
                await agent.SendHeartbeatAsync(cancellationToken);
                await WaitForStatusAsync(
                    firstCandidate,
                    platformId,
                    "Connected",
                    cancellationToken);
                await AssertPruneSucceedsAsync(
                    firstCandidate,
                    platformId,
                    cancellationToken);

                await agent.DisconnectAsync(cancellationToken);
                await WaitForStatusAsync(
                    firstCandidate,
                    platformId,
                    "Offline",
                    cancellationToken);
                await AssertPruneUnavailableAsync(
                    firstCandidate,
                    platformId,
                    cancellationToken);

                Assert.Null(await agent.ReconnectAsync(
                    firstCandidate.GrpcAddress,
                    cancellationToken));
                await WaitForStatusAsync(
                    firstCandidate,
                    platformId,
                    "Connected",
                    cancellationToken);
                await AssertPruneSucceedsAsync(
                    firstCandidate,
                    platformId,
                    cancellationToken);
            }

            await agent.WaitForDisconnectAsync(
                TimeSpan.FromSeconds(10),
                cancellationToken);

            await using var restartedCandidate =
                await CandidateApplicationProcess.StartAsync(
                    connectionString,
                    workingDirectory,
                    cancellationToken);
            await restartedCandidate.AuthenticateAsAdminAsync(cancellationToken);

            Assert.Null(await agent.ReconnectAsync(
                restartedCandidate.GrpcAddress,
                cancellationToken));
            await WaitForStatusAsync(
                restartedCandidate,
                agent.PlatformId,
                "Connected",
                cancellationToken);
            await AssertPruneSucceedsAsync(
                restartedCandidate,
                agent.PlatformId,
                cancellationToken);

            await agent.DisconnectAsync(cancellationToken);
            await WaitForStatusAsync(
                restartedCandidate,
                agent.PlatformId,
                "Offline",
                cancellationToken);

            using var revokeResponse = await restartedCandidate.Client.PostAsync(
                $"/api/v1/platforms/{agent.PlatformId:D}/edge/revoke",
                content: null,
                cancellationToken);
            Assert.Equal(HttpStatusCode.NoContent, revokeResponse.StatusCode);
            await WaitForStatusAsync(
                restartedCandidate,
                agent.PlatformId,
                "Revoked",
                cancellationToken);

            var rejection = await agent.ReconnectAsync(
                restartedCandidate.GrpcAddress,
                cancellationToken);
            Assert.NotNull(rejection);
            Assert.Contains(
                "revoked",
                rejection,
                StringComparison.OrdinalIgnoreCase);
            await AssertPruneUnavailableAsync(
                restartedCandidate,
                agent.PlatformId,
                cancellationToken);
            Assert.Equal(3, agent.CompletedCommandCount);
        }
        finally
        {
            await postgres.DropDatabaseAsync(
                connectionString,
                CancellationToken.None);
            Directory.Delete(workingDirectory, recursive: true);
        }
    }

    private static async Task<Guid> CreateEdgePlatformAsync(
        CandidateApplicationProcess candidate,
        CancellationToken cancellationToken)
    {
        using var response = await candidate.Client.PostAsJsonAsync(
            "/api/v1/platforms",
            new
            {
                name = $"acceptance-edge-{Guid.NewGuid():N}",
                type = "Docker",
                connectorType = "edgeAgent"
            },
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(
            response.IsSuccessStatusCode,
            $"Creating the Edge Agent platform failed: {body}");

        using var json = JsonDocument.Parse(body);
        return json.RootElement.GetProperty("id").GetGuid();
    }

    private static async Task<string> CreateEnrollmentAsync(
        CandidateApplicationProcess candidate,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        using var response = await candidate.Client.PostAsync(
            $"/api/v1/platforms/{platformId:D}/edge/enrollments",
            content: null,
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(
            response.IsSuccessStatusCode,
            $"Creating the Edge Agent enrollment failed: {body}");

        using var json = JsonDocument.Parse(body);
        return json.RootElement.GetProperty("token").GetString()!;
    }

    private static async Task AssertPruneSucceedsAsync(
        CandidateApplicationProcess candidate,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        using var response = await candidate.Client.PostAsJsonAsync(
            $"/api/v1/platforms/{platformId:D}/prune",
            new { resource = "All" },
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);

        Assert.True(
            response.IsSuccessStatusCode,
            $"The routed Edge Agent operation failed: {body}");
        using var json = JsonDocument.Parse(body);
        Assert.Equal(
            4096,
            json.RootElement.GetProperty("spaceReclaimed").GetInt64());
    }

    private static async Task AssertPruneUnavailableAsync(
        CandidateApplicationProcess candidate,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        using var response = await candidate.Client.PostAsJsonAsync(
            $"/api/v1/platforms/{platformId:D}/prune",
            new { resource = "All" },
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);

        Assert.Equal(HttpStatusCode.ServiceUnavailable, response.StatusCode);
        Assert.Contains(
            "offline",
            body,
            StringComparison.OrdinalIgnoreCase);
    }

    private static async Task WaitForStatusAsync(
        CandidateApplicationProcess candidate,
        Guid platformId,
        string expectedStatus,
        CancellationToken cancellationToken)
    {
        var deadline = DateTime.UtcNow.AddSeconds(10);
        string? observedStatus = null;
        do
        {
            using var response = await candidate.Client.GetAsync(
                $"/api/v1/platforms/{platformId:D}/edge/status",
                cancellationToken);
            var body =
                await response.Content.ReadAsStringAsync(cancellationToken);
            Assert.True(
                response.IsSuccessStatusCode,
                $"Reading Edge Agent status failed: {body}");

            using var json = JsonDocument.Parse(body);
            observedStatus = json.RootElement
                .GetProperty("connectionStatus")
                .GetString();
            if (string.Equals(
                observedStatus,
                expectedStatus,
                StringComparison.Ordinal))
            {
                return;
            }

            await Task.Delay(TimeSpan.FromMilliseconds(100), cancellationToken);
        }
        while (DateTime.UtcNow < deadline);

        Assert.Fail(
            $"Expected Edge Agent status '{expectedStatus}', observed '{observedStatus}'.");
    }
}
