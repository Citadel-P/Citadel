using DotNet.Testcontainers.Configurations;
using Npgsql;
using System.Net;
using System.Net.Http.Json;
using System.Text.Json;
using Tests.Acceptance.Infrastructure;

namespace Tests.Acceptance.Compatibility;

[Collection("AcceptancePostgres")]
public sealed class EdgeAgentCompatibilityTests(
    AcceptancePostgresFixture postgres)
{
    public static bool HasRequiredCandidateImages =>
        CandidateImageTestEnvironment.ShouldRun(
            "CITADEL_ACCEPTANCE_CORE_IMAGE",
            "CITADEL_ACCEPTANCE_AGENT_IMAGE");

    [Fact(
        Skip = "Set CITADEL_ACCEPTANCE_CORE_IMAGE and CITADEL_ACCEPTANCE_AGENT_IMAGE to run the real image compatibility test.",
        SkipUnless = nameof(HasRequiredCandidateImages))]
    public async Task RealAgent_ShouldEnrollRouteDockerWorkAndRejectRevocation()
    {
        var coreImage = CandidateImageTestEnvironment.GetRequiredImage(
            "CITADEL_ACCEPTANCE_CORE_IMAGE");
        var agentImage = CandidateImageTestEnvironment.GetRequiredImage(
            "CITADEL_ACCEPTANCE_AGENT_IMAGE");
        var expectedAgentVersion = Environment.GetEnvironmentVariable(
            "CITADEL_ACCEPTANCE_AGENT_VERSION");
        var cancellationToken = TestContext.Current.CancellationToken;
        var hostConnectionString =
            await postgres.CreateDatabaseAsync(cancellationToken);

        try
        {
            var databaseConnection =
                new NpgsqlConnectionStringBuilder(hostConnectionString);
            await TestcontainersSettings.ExposeHostPortsAsync(
                checked((ushort)databaseConnection.Port),
                cancellationToken);
            databaseConnection.Host = "host.testcontainers.internal";

            await using var environment =
                await CoreAgentCompatibilityEnvironment.StartAsync(
                    coreImage,
                    agentImage,
                    databaseConnection.ConnectionString,
                    cancellationToken);
            await environment.AuthenticateAsAdminAsync(cancellationToken);

            var platformId = await CreateEdgePlatformAsync(
                environment,
                "target",
                cancellationToken);
            var otherPlatformId = await CreateEdgePlatformAsync(
                environment,
                "other",
                cancellationToken);
            var enrollmentToken = await CreateEnrollmentAsync(
                environment,
                platformId,
                cancellationToken);

            await environment.StartAgentAsync(
                enrollmentToken,
                cancellationToken);
            var connectedStatus = await WaitForStatusAsync(
                environment,
                platformId,
                "Connected",
                cancellationToken);
            Assert.Equal(
                1,
                connectedStatus.GetProperty("protocolVersion").GetInt32());
            var observedAgentVersion = connectedStatus
                .GetProperty("lastSeenVersion")
                .GetString();
            Assert.False(string.IsNullOrWhiteSpace(observedAgentVersion));
            if (!string.IsNullOrWhiteSpace(expectedAgentVersion))
            {
                Assert.Equal(
                    expectedAgentVersion,
                    observedAgentVersion);
            }

            await AssertPlatformOperationSucceedsAsync(
                environment,
                platformId,
                cancellationToken);
            await AssertPlatformOperationIsIsolatedAsync(
                environment,
                otherPlatformId,
                cancellationToken);
            await WaitForDockerRouteAsync(
                environment,
                platformId,
                cancellationToken);

            var stackId = await CreateStackAsync(
                environment,
                platformId,
                cancellationToken);
            await ApplyStackAsync(
                environment,
                stackId,
                cancellationToken);
            await AssertRuntimeContainerIsRunningAsync(
                environment,
                stackId,
                cancellationToken);

            await environment.StopAgentAsync(cancellationToken);
            await WaitForStatusAsync(
                environment,
                platformId,
                "Offline",
                cancellationToken);
            await AssertPlatformOperationIsOfflineAsync(
                environment,
                platformId,
                cancellationToken);

            await environment.StartAgentAsync(
                enrollmentToken,
                cancellationToken);
            await WaitForStatusAsync(
                environment,
                platformId,
                "Connected",
                cancellationToken);
            await AssertPlatformOperationSucceedsAsync(
                environment,
                platformId,
                cancellationToken);
            await AssertRuntimeContainerIsRunningAsync(
                environment,
                stackId,
                cancellationToken);
            await DeleteStackAsync(
                environment,
                stackId,
                cancellationToken);

            await environment.StopAgentAsync(cancellationToken);
            await WaitForStatusAsync(
                environment,
                platformId,
                "Offline",
                cancellationToken);
            await RevokeAgentAsync(
                environment,
                platformId,
                cancellationToken);
            await environment.StartAgentAsync(
                enrollmentToken,
                cancellationToken);
            await WaitForStatusAsync(
                environment,
                platformId,
                "Revoked",
                cancellationToken);
            await WaitForAgentRejectionAsync(
                environment,
                cancellationToken);
            await AssertPlatformOperationIsUnavailableAsync(
                environment,
                platformId,
                cancellationToken);
        }
        finally
        {
            await postgres.DropDatabaseAsync(
                hostConnectionString,
                CancellationToken.None);
        }
    }

    private static async Task<Guid> CreateEdgePlatformAsync(
        CoreAgentCompatibilityEnvironment environment,
        string suffix,
        CancellationToken cancellationToken)
    {
        using var response = await environment.Client.PostAsJsonAsync(
            "/api/v1/platforms",
            new
            {
                name = $"acceptance-real-edge-{suffix}-{Guid.NewGuid():N}",
                type = "Docker",
                connectorType = "edgeAgent",
                tagIds = Array.Empty<Guid>()
            },
            cancellationToken);
        return await ReadIdAsync(
            response,
            $"Creating the {suffix} Edge Agent platform",
            cancellationToken);
    }

    private static async Task<string> CreateEnrollmentAsync(
        CoreAgentCompatibilityEnvironment environment,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        using var response = await environment.Client.PostAsync(
            $"/api/v1/platforms/{platformId:D}/edge/enrollments",
            content: null,
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(
            response.IsSuccessStatusCode,
            $"Creating the real Agent enrollment failed with HTTP {(int)response.StatusCode}: {body}");

        using var json = JsonDocument.Parse(body);
        Assert.Equal(
            "http://citadel-core:8001",
            json.RootElement
                .GetProperty("instructions")
                .GetProperty("coreUrl")
                .GetString());
        return json.RootElement.GetProperty("token").GetString()!;
    }

    private static async Task<Guid> CreateStackAsync(
        CoreAgentCompatibilityEnvironment environment,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        var name = $"acceptance-real-edge-stack-{Guid.NewGuid():N}";
        using var response = await environment.Client.PostAsJsonAsync(
            "/api/v1/stacks",
            new
            {
                name,
                platformId,
                description = "Real Edge Agent compatibility stack",
                stackSource = "WebEditor",
                spec = new Dictionary<string, object?>
                {
                    ["$type"] = "WebEditor",
                    ["composeFile"] =
                        """
                        services:
                          runtime:
                            image: busybox:1.36.1
                            command: ["sh", "-c", "echo citadel-real-edge-agent; exec tail -f /dev/null"]
                            stop_grace_period: 1s
                            labels:
                              citadel.acceptance: edge-agent
                        """,
                    ["updateBehavior"] = "Disabled",
                    ["projectName"] = name,
                    ["destroyBeforeDeploy"] = true,
                    ["buildImageBindings"] = Array.Empty<object>()
                },
                tagIds = Array.Empty<Guid>()
            },
            cancellationToken);
        return await ReadIdAsync(
            response,
            "Creating the real Agent stack",
            cancellationToken);
    }

    private static async Task ApplyStackAsync(
        CoreAgentCompatibilityEnvironment environment,
        Guid stackId,
        CancellationToken cancellationToken)
    {
        using var response = await environment.Client.PostAsJsonAsync(
            "/api/v1/stacks/apply",
            new
            {
                id = stackId,
                recreate = false
            },
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(
            response.IsSuccessStatusCode,
            $"""
            Applying the stack through the real Agent failed with HTTP {(int)response.StatusCode}: {body}
            {await environment.GetDiagnosticsAsync(CancellationToken.None)}
            """);

        using var json = JsonDocument.Parse(body);
        var events = json.RootElement.EnumerateArray().ToArray();
        Assert.True(
            events.Length > 1,
            $"""
            Expected streamed stack progress, received: {body}
            {await environment.GetDiagnosticsAsync(CancellationToken.None)}
            """);
        var completion = events[^1];
        Assert.Equal(0, completion.GetProperty("exitCode").GetInt32());
        Assert.Equal(
            "success",
            completion.GetProperty("severity").GetString());
        Assert.Equal(
            "Healthy",
            completion.GetProperty("stackStatus").GetString());
    }

    private static async Task AssertRuntimeContainerIsRunningAsync(
        CoreAgentCompatibilityEnvironment environment,
        Guid stackId,
        CancellationToken cancellationToken)
    {
        using var response = await environment.Client.GetAsync(
            $"/api/v1/stacks/{stackId:D}/data",
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(
            response.IsSuccessStatusCode,
            $"Listing containers through the real Agent failed with HTTP {(int)response.StatusCode}: {body}");

        using var json = JsonDocument.Parse(body);
        var container = Assert.Single(
            json.RootElement.GetProperty("containers").EnumerateArray());
        Assert.Equal(
            "running",
            container.GetProperty("state").GetString(),
            ignoreCase: true);

        var containerId = container.GetProperty("id").GetString();
        Assert.False(string.IsNullOrWhiteSpace(containerId));
        using var inspectResponse = await environment.Client.GetAsync(
            $"/api/v1/containers/{containerId}/inspect",
            cancellationToken);
        var inspectBody = await inspectResponse.Content.ReadAsStringAsync(
            cancellationToken);
        Assert.True(
            inspectResponse.IsSuccessStatusCode,
            $"Inspecting the runtime container through the real Agent failed with HTTP {(int)inspectResponse.StatusCode}: {inspectBody}");

        using var inspectJson = JsonDocument.Parse(inspectBody);
        Assert.Contains(
            "busybox",
            inspectJson.RootElement
                .GetProperty("config")
                .GetProperty("image")
                .GetString(),
            StringComparison.OrdinalIgnoreCase);
    }

    private static async Task AssertPlatformOperationIsOfflineAsync(
        CoreAgentCompatibilityEnvironment environment,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        using var response = await environment.Client.PostAsJsonAsync(
            $"/api/v1/platforms/{platformId:D}/prune",
            new
            {
                resource = "All"
            },
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        Assert.Equal(HttpStatusCode.ServiceUnavailable, response.StatusCode);
        Assert.Contains(
            "offline",
            body,
            StringComparison.OrdinalIgnoreCase);
    }

    private static async Task WaitForDockerRouteAsync(
        CoreAgentCompatibilityEnvironment environment,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        var deadline = DateTime.UtcNow.AddSeconds(30);
        string lastBody = string.Empty;
        HttpStatusCode lastStatus = default;

        while (DateTime.UtcNow < deadline)
        {
            using var response = await environment.Client.GetAsync(
                $"/api/v1/volumes/{platformId:D}",
                cancellationToken);
            lastStatus = response.StatusCode;
            lastBody = await response.Content.ReadAsStringAsync(
                cancellationToken);
            if (response.IsSuccessStatusCode)
                return;

            await Task.Delay(
                TimeSpan.FromMilliseconds(250),
                cancellationToken);
        }

        Assert.Fail(
            $"""
            The connected Agent Docker route did not become ready. Last response was HTTP {(int)lastStatus}: {lastBody}
            {await environment.GetDiagnosticsAsync(CancellationToken.None)}
            """);
    }

    private static async Task AssertPlatformOperationSucceedsAsync(
        CoreAgentCompatibilityEnvironment environment,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        using var response = await environment.Client.PostAsJsonAsync(
            $"/api/v1/platforms/{platformId:D}/prune",
            new
            {
                resource = "All"
            },
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(
            response.IsSuccessStatusCode,
            $"""
            Routing a platform operation through the real Agent failed with HTTP {(int)response.StatusCode}: {body}
            {await environment.GetDiagnosticsAsync(CancellationToken.None)}
            """);
    }

    private static async Task AssertPlatformOperationIsIsolatedAsync(
        CoreAgentCompatibilityEnvironment environment,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        using var response = await environment.Client.PostAsJsonAsync(
            $"/api/v1/platforms/{platformId:D}/prune",
            new
            {
                resource = "All"
            },
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        Assert.Equal(HttpStatusCode.ServiceUnavailable, response.StatusCode);
        Assert.Contains(
            "offline",
            body,
            StringComparison.OrdinalIgnoreCase);
    }

    private static async Task AssertPlatformOperationIsUnavailableAsync(
        CoreAgentCompatibilityEnvironment environment,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        using var response = await environment.Client.PostAsJsonAsync(
            $"/api/v1/platforms/{platformId:D}/prune",
            new
            {
                resource = "All"
            },
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        Assert.Equal(HttpStatusCode.ServiceUnavailable, response.StatusCode);
        Assert.Contains(
            "offline",
            body,
            StringComparison.OrdinalIgnoreCase);
    }

    private static async Task DeleteStackAsync(
        CoreAgentCompatibilityEnvironment environment,
        Guid stackId,
        CancellationToken cancellationToken)
    {
        using var request = new HttpRequestMessage(
            HttpMethod.Delete,
            "/api/v1/stacks")
        {
            Content = JsonContent.Create(new[] { stackId })
        };
        using var response = await environment.Client.SendAsync(
            request,
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(
            response.IsSuccessStatusCode,
            $"Deleting the real Agent stack failed with HTTP {(int)response.StatusCode}: {body}");
    }

    private static async Task RevokeAgentAsync(
        CoreAgentCompatibilityEnvironment environment,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        using var response = await environment.Client.PostAsync(
            $"/api/v1/platforms/{platformId:D}/edge/revoke",
            content: null,
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(
            response.IsSuccessStatusCode,
            $"Revoking the real Agent failed with HTTP {(int)response.StatusCode}: {body}");
    }

    private static async Task<JsonElement> WaitForStatusAsync(
        CoreAgentCompatibilityEnvironment environment,
        Guid platformId,
        string expectedStatus,
        CancellationToken cancellationToken)
    {
        JsonElement lastStatus = default;
        var deadline = DateTime.UtcNow.AddSeconds(30);
        while (DateTime.UtcNow < deadline)
        {
            using var response = await environment.Client.GetAsync(
                $"/api/v1/platforms/{platformId:D}/edge/status",
                cancellationToken);
            var body = await response.Content.ReadAsStringAsync(
                cancellationToken);
            Assert.True(
                response.IsSuccessStatusCode,
                $"Reading real Agent status failed with HTTP {(int)response.StatusCode}: {body}");

            using var json = JsonDocument.Parse(body);
            lastStatus = json.RootElement.Clone();
            if (string.Equals(
                    lastStatus.GetProperty("connectionStatus").GetString(),
                    expectedStatus,
                    StringComparison.Ordinal))
            {
                return lastStatus;
            }

            await Task.Delay(
                TimeSpan.FromMilliseconds(200),
                cancellationToken);
        }

        Assert.Fail(
            $"""
            Expected Edge Agent status '{expectedStatus}', observed '{lastStatus}'.
            {await environment.GetDiagnosticsAsync(CancellationToken.None)}
            """);
        return default;
    }

    private static async Task WaitForAgentRejectionAsync(
        CoreAgentCompatibilityEnvironment environment,
        CancellationToken cancellationToken)
    {
        var deadline = DateTime.UtcNow.AddSeconds(30);
        string diagnostics;
        do
        {
            diagnostics = await environment.GetDiagnosticsAsync(
                cancellationToken);
            if (diagnostics.Contains(
                    "revoked",
                    StringComparison.OrdinalIgnoreCase))
            {
                return;
            }

            await Task.Delay(
                TimeSpan.FromMilliseconds(250),
                cancellationToken);
        }
        while (DateTime.UtcNow < deadline);

        Assert.Fail(
            $"The restarted Agent did not report revocation.{Environment.NewLine}{diagnostics}");
    }

    private static async Task<Guid> ReadIdAsync(
        HttpResponseMessage response,
        string operation,
        CancellationToken cancellationToken)
    {
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(
            response.IsSuccessStatusCode,
            $"{operation} failed with HTTP {(int)response.StatusCode}: {body}");
        using var json = JsonDocument.Parse(body);
        return json.RootElement.GetProperty("id").GetGuid();
    }
}
