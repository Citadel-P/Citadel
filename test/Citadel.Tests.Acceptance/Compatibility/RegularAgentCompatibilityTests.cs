using DotNet.Testcontainers.Configurations;
using Npgsql;
using System.Net;
using System.Net.Http.Json;
using System.Text.Json;
using Tests.Acceptance.Infrastructure;

namespace Tests.Acceptance.Compatibility;

[Collection("AcceptancePostgres")]
public sealed class RegularAgentCompatibilityTests(
    AcceptancePostgresFixture postgres)
{
    public static bool HasRequiredCandidateImages =>
        CandidateImageTestEnvironment.ShouldRun(
            "CITADEL_ACCEPTANCE_CORE_IMAGE",
            "CITADEL_ACCEPTANCE_AGENT_IMAGE");

    [Fact(
        Skip = "Set CITADEL_ACCEPTANCE_CORE_IMAGE and CITADEL_ACCEPTANCE_AGENT_IMAGE to run the regular Agent compatibility test.",
        SkipUnless = nameof(HasRequiredCandidateImages))]
    public async Task RealAgent_ShouldAuthenticateRouteDockerWorkAndRejectInvalidHubKey()
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

            var setup = await GetAgentSetupAsync(
                environment,
                cancellationToken);
            Assert.Equal(
                setup.HubPublicKey,
                setup.Environment["HUB_PUBLIC_KEY"]);
            Assert.Contains(
                "HUB_PUBLIC_KEY",
                setup.DockerRunCommand,
                StringComparison.Ordinal);
            Assert.Contains(
                "-p 9000:9000",
                setup.DockerRunCommand,
                StringComparison.Ordinal);

            await environment.StartInvalidRegularAgentAsync(
                CreateDifferentPublicKey(setup.HubPublicKey),
                cancellationToken);
            await AssertInvalidHubKeyIsRejectedAsync(
                environment,
                cancellationToken);

            await environment.StartRegularAgentAsync(
                setup.HubPublicKey,
                cancellationToken);
            var platform = await CreatePlatformAsync(
                environment,
                cancellationToken);

            Assert.Equal(
                "Agent",
                platform.ConnectorType,
                ignoreCase: true);
            Assert.Equal(
                "Online",
                platform.Status,
                ignoreCase: true);
            Assert.False(string.IsNullOrWhiteSpace(platform.AgentVersion));
            if (!string.IsNullOrWhiteSpace(expectedAgentVersion))
            {
                Assert.Equal(
                    expectedAgentVersion,
                    platform.AgentVersion);
            }

            await AssertPlatformOperationSucceedsAsync(
                environment,
                platform.Id,
                cancellationToken);

            var stackId = await CreateStackAsync(
                environment,
                platform.Id,
                cancellationToken);
            await ApplyStackAsync(
                environment,
                stackId,
                cancellationToken);
            await AssertRuntimeContainerIsRunningAsync(
                environment,
                stackId,
                cancellationToken);
            await DeleteStackAsync(
                environment,
                stackId,
                cancellationToken);
        }
        finally
        {
            await postgres.DropDatabaseAsync(
                hostConnectionString,
                CancellationToken.None);
        }
    }

    private static async Task<AgentSetup> GetAgentSetupAsync(
        CoreAgentCompatibilityEnvironment environment,
        CancellationToken cancellationToken)
    {
        using var response = await environment.Client.GetAsync(
            "/api/v1/platforms/agent/setup",
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(
            response.IsSuccessStatusCode,
            $"Reading regular Agent setup failed with HTTP {(int)response.StatusCode}: {body}");

        using var json = JsonDocument.Parse(body);
        var root = json.RootElement;
        var setupEnvironment = root
            .GetProperty("environment")
            .EnumerateObject()
            .ToDictionary(
                property => property.Name,
                property => property.Value.GetString()!);
        return new AgentSetup(
            root.GetProperty("hubPublicKey").GetString()!,
            setupEnvironment,
            root.GetProperty("dockerRunCommand").GetString()!);
    }

    private static async Task AssertInvalidHubKeyIsRejectedAsync(
        CoreAgentCompatibilityEnvironment environment,
        CancellationToken cancellationToken)
    {
        using var response = await environment.Client.PostAsJsonAsync(
            "/api/v1/platforms",
            new
            {
                name = $"acceptance-invalid-agent-{Guid.NewGuid():N}",
                address = environment.InvalidRegularAgentAddress,
                description = "Agent with an invalid Core public key",
                type = "Docker",
                connectorType = "Agent",
                tagIds = Array.Empty<Guid>()
            },
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);

        Assert.Equal(HttpStatusCode.Unauthorized, response.StatusCode);
        Assert.Contains(
            "Unauthenticated",
            body,
            StringComparison.OrdinalIgnoreCase);
        Assert.Contains(
            "Invalid signature",
            body,
            StringComparison.OrdinalIgnoreCase);
    }

    private static async Task<PlatformFixture> CreatePlatformAsync(
        CoreAgentCompatibilityEnvironment environment,
        CancellationToken cancellationToken)
    {
        using var response = await environment.Client.PostAsJsonAsync(
            "/api/v1/platforms",
            new
            {
                name = $"acceptance-real-agent-{Guid.NewGuid():N}",
                address = environment.RegularAgentAddress,
                description = "Real regular Agent compatibility platform",
                type = "Docker",
                connectorType = "Agent",
                tagIds = Array.Empty<Guid>()
            },
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(
            response.IsSuccessStatusCode,
            $"""
            Creating the regular Agent platform failed with HTTP {(int)response.StatusCode}: {body}
            {await environment.GetDiagnosticsAsync(CancellationToken.None)}
            """);

        using var json = JsonDocument.Parse(body);
        var root = json.RootElement;
        return new PlatformFixture(
            root.GetProperty("id").GetGuid(),
            root.GetProperty("connectorType").GetString()!,
            root.GetProperty("status").GetString()!,
            root.GetProperty("agentVersion").GetString());
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
            Routing a platform operation through the regular Agent failed with HTTP {(int)response.StatusCode}: {body}
            {await environment.GetDiagnosticsAsync(CancellationToken.None)}
            """);
    }

    private static async Task<Guid> CreateStackAsync(
        CoreAgentCompatibilityEnvironment environment,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        var name = $"acceptance-real-agent-stack-{Guid.NewGuid():N}";
        using var response = await environment.Client.PostAsJsonAsync(
            "/api/v1/stacks",
            new
            {
                name,
                platformId,
                description = "Real regular Agent compatibility stack",
                stackSource = "WebEditor",
                spec = new Dictionary<string, object?>
                {
                    ["$type"] = "WebEditor",
                    ["composeFile"] =
                        """
                        services:
                          runtime:
                            image: busybox:1.36.1
                            command: ["sh", "-c", "echo citadel-real-agent; exec tail -f /dev/null"]
                            stop_grace_period: 1s
                            labels:
                              citadel.acceptance: regular-agent
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
            "Creating the regular Agent stack",
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
            Applying the stack through the regular Agent failed with HTTP {(int)response.StatusCode}: {body}
            {await environment.GetDiagnosticsAsync(CancellationToken.None)}
            """);

        using var json = JsonDocument.Parse(body);
        var events = json.RootElement.EnumerateArray().ToArray();
        Assert.True(events.Length > 1);
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
            $"Listing containers through the regular Agent failed with HTTP {(int)response.StatusCode}: {body}");

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
            $"Inspecting the runtime container through the regular Agent failed with HTTP {(int)inspectResponse.StatusCode}: {inspectBody}");

        using var inspectJson = JsonDocument.Parse(inspectBody);
        Assert.Contains(
            "busybox",
            inspectJson.RootElement
                .GetProperty("config")
                .GetProperty("image")
                .GetString(),
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
            $"Deleting the regular Agent stack failed with HTTP {(int)response.StatusCode}: {body}");
    }

    private static string CreateDifferentPublicKey(string publicKey)
    {
        var bytes = Convert.FromBase64String(publicKey);
        bytes[0] ^= 0xFF;
        return Convert.ToBase64String(bytes);
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

    private sealed record AgentSetup(
        string HubPublicKey,
        IReadOnlyDictionary<string, string> Environment,
        string DockerRunCommand);

    private sealed record PlatformFixture(
        Guid Id,
        string ConnectorType,
        string Status,
        string? AgentVersion);
}
