using DotNet.Testcontainers.Builders;
using DotNet.Testcontainers.Containers;
using DotNet.Testcontainers.Networks;
using DotNet.Testcontainers.Volumes;
using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Text.Json;

namespace Tests.Acceptance.Infrastructure;

internal sealed class CoreAgentCompatibilityEnvironment : IAsyncDisposable
{
    private const ushort CoreHttpPort = 8000;
    private const ushort AgentGrpcPort = 9000;
    private const string CoreNetworkAlias = "citadel-core";
    private const string RegularAgentNetworkAlias = "citadel-agent";
    private const string InvalidRegularAgentNetworkAlias = "citadel-agent-invalid";
    private const string DockerImage = "docker:27.5.1-dind";

    private readonly string agentImage;
    private readonly INetwork network;
    private readonly IVolume dockerSocketVolume;
    private readonly IVolume dockerDataVolume;
    private readonly IVolume coreDataVolume;
    private readonly IVolume agentDataVolume;
    private readonly IContainer dockerDaemon;
    private readonly IContainer core;
    private IContainer? agent;
    private IContainer? regularAgent;
    private IContainer? invalidRegularAgent;

    private CoreAgentCompatibilityEnvironment(
        string coreImage,
        string agentImage,
        string postgresConnectionString)
    {
        this.agentImage = agentImage;
        network = new NetworkBuilder().Build();
        dockerSocketVolume = new VolumeBuilder().Build();
        dockerDataVolume = new VolumeBuilder().Build();
        coreDataVolume = new VolumeBuilder().Build();
        agentDataVolume = new VolumeBuilder().Build();

        dockerDaemon = new ContainerBuilder(DockerImage)
            .WithPrivileged(true)
            .WithEnvironment("DOCKER_TLS_CERTDIR", string.Empty)
            .WithVolumeMount(dockerSocketVolume, "/var/run")
            .WithVolumeMount(dockerDataVolume, "/var/lib/docker")
            .WithNetwork(network)
            .WithWaitStrategy(
                Wait.ForUnixContainer().UntilCommandIsCompleted(
                    "docker",
                    "info"))
            .Build();

        core = new ContainerBuilder(coreImage)
            .WithPortBinding(CoreHttpPort, assignRandomHostPort: true)
            .WithEnvironment("ASPNETCORE_ENVIRONMENT", "Production")
            .WithEnvironment(
                "ConnectionStrings__Postgres",
                postgresConnectionString)
            .WithEnvironment("Jwt__Issuer", "http://citadel-core")
            .WithEnvironment("Jwt__Audience", "http://citadel-core")
            .WithEnvironment(
                "Jwt__Key",
                "citadel-edge-compatibility-signing-key-000000000000000000000000")
            .WithEnvironment(
                "Secrets__EncryptionKey",
                "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=")
            .WithEnvironment(
                "EdgeAgent__PublicGrpcUrl",
                $"http://{CoreNetworkAlias}:8001")
            .WithVolumeMount(coreDataVolume, "/app/data")
            .WithNetwork(network)
            .WithNetworkAliases(CoreNetworkAlias)
            .WithWaitStrategy(
                Wait.ForUnixContainer().UntilHttpRequestIsSucceeded(
                    request => request
                        .ForPort(CoreHttpPort)
                        .ForPath("/health")))
            .Build();
    }

    public HttpClient Client { get; private set; } = null!;
    public string RegularAgentAddress =>
        $"http://{RegularAgentNetworkAlias}:{AgentGrpcPort}";
    public string InvalidRegularAgentAddress =>
        $"http://{InvalidRegularAgentNetworkAlias}:{AgentGrpcPort}";

    public static async Task<CoreAgentCompatibilityEnvironment> StartAsync(
        string coreImage,
        string agentImage,
        string postgresConnectionString,
        CancellationToken cancellationToken)
    {
        Assert.False(
            string.IsNullOrWhiteSpace(coreImage),
            "Set CITADEL_ACCEPTANCE_CORE_IMAGE to the Core image under test.");
        Assert.False(
            string.IsNullOrWhiteSpace(agentImage),
            "Set CITADEL_ACCEPTANCE_AGENT_IMAGE to the Agent image under test.");

        var environment = new CoreAgentCompatibilityEnvironment(
            coreImage,
            agentImage,
            postgresConnectionString);
        try
        {
            await environment.network.CreateAsync(cancellationToken);
            await environment.dockerSocketVolume.CreateAsync(cancellationToken);
            await environment.dockerDataVolume.CreateAsync(cancellationToken);
            await environment.coreDataVolume.CreateAsync(cancellationToken);
            await environment.agentDataVolume.CreateAsync(cancellationToken);
            await environment.dockerDaemon.StartAsync(cancellationToken);
            await environment.core.StartAsync(cancellationToken);

            environment.Client = new HttpClient
            {
                BaseAddress = new Uri(
                    $"http://127.0.0.1:{environment.core.GetMappedPublicPort(CoreHttpPort)}"),
                Timeout = TimeSpan.FromMinutes(3)
            };
            return environment;
        }
        catch
        {
            await environment.DisposeAsync();
            throw;
        }
    }

    public async Task AuthenticateAsAdminAsync(
        CancellationToken cancellationToken)
    {
        using var response = await Client.PostAsJsonAsync(
            "/api/v1/authentication/login",
            new
            {
                emailOrName = "admin@citadel.local",
                password = "admin123"
            },
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(
            response.IsSuccessStatusCode,
            $"Core image login failed with HTTP {(int)response.StatusCode}: {body}");

        using var json = JsonDocument.Parse(body);
        var accessToken = json.RootElement
            .GetProperty("accessToken")
            .GetString();
        Assert.False(string.IsNullOrWhiteSpace(accessToken));
        Client.DefaultRequestHeaders.Authorization =
            new AuthenticationHeaderValue("Bearer", accessToken);
    }

    public async Task StartAgentAsync(
        string enrollmentToken,
        CancellationToken cancellationToken)
    {
        if (agent is null)
        {
            agent = new ContainerBuilder(agentImage)
                .WithEnvironment("ASPNETCORE_ENVIRONMENT", "Production")
                .WithEnvironment("CITADEL_AGENT_MODE", "edge")
                .WithEnvironment(
                    "CITADEL_EDGE_AGENT_PROFILE",
                    "edge-agent-compatibility")
                .WithEnvironment(
                    "CITADEL_CORE_URL",
                    $"http://{CoreNetworkAlias}:8001")
                .WithEnvironment(
                    "CITADEL_EDGE_ENROLLMENT_TOKEN",
                    enrollmentToken)
                .WithEnvironment(
                    "CITADEL_EDGE_AGENT_KEY_PATH",
                    "/app/data/edge-agent.key")
                .WithEnvironment(
                    "CITADEL_EDGE_IDENTITY_PATH",
                    "/app/data/edge-agent.identity.json")
                .WithEnvironment(
                    "DOCKER_HOST",
                    "unix:///var/run/docker.sock")
                .WithVolumeMount(dockerSocketVolume, "/var/run")
                .WithVolumeMount(agentDataVolume, "/app/data")
                .WithNetwork(network)
                .WithWaitStrategy(
                    Wait.ForUnixContainer().UntilMessageIsLogged(
                        "Starting Citadel Edge Agent"))
                .Build();
        }

        await agent.StartAsync(cancellationToken);
    }

    public Task StopAgentAsync(CancellationToken cancellationToken)
    {
        Assert.NotNull(agent);
        return agent.StopAsync(cancellationToken);
    }

    public async Task StartRegularAgentAsync(
        string hubPublicKey,
        CancellationToken cancellationToken)
    {
        regularAgent ??= BuildRegularAgent(
            hubPublicKey,
            RegularAgentNetworkAlias);
        await regularAgent.StartAsync(cancellationToken);
    }

    public async Task StartInvalidRegularAgentAsync(
        string hubPublicKey,
        CancellationToken cancellationToken)
    {
        invalidRegularAgent ??= BuildRegularAgent(
            hubPublicKey,
            InvalidRegularAgentNetworkAlias);
        await invalidRegularAgent.StartAsync(cancellationToken);
    }

    public async Task<string> GetDiagnosticsAsync(
        CancellationToken cancellationToken)
    {
        var (coreStdout, coreStderr) = await core.GetLogsAsync(
            DateTime.UnixEpoch,
            DateTime.UtcNow,
            timestampsEnabled: false,
            cancellationToken);
        var agentLogs = string.Empty;
        if (agent is not null)
        {
            var (agentStdout, agentStderr) = await agent.GetLogsAsync(
                DateTime.UnixEpoch,
                DateTime.UtcNow,
                timestampsEnabled: false,
                cancellationToken);
            agentLogs =
                $"{Environment.NewLine}Agent stdout:{Environment.NewLine}{agentStdout}"
                + $"{Environment.NewLine}Agent stderr:{Environment.NewLine}{agentStderr}";
        }

        var regularAgentLogs = regularAgent is null
            ? string.Empty
            : await GetContainerLogsAsync(
                "Regular Agent",
                regularAgent,
                cancellationToken);
        var invalidRegularAgentLogs = invalidRegularAgent is null
            ? string.Empty
            : await GetContainerLogsAsync(
                "Invalid-key Agent",
                invalidRegularAgent,
                cancellationToken);

        return
            $"Core stdout:{Environment.NewLine}{coreStdout}"
            + $"{Environment.NewLine}Core stderr:{Environment.NewLine}{coreStderr}"
            + agentLogs
            + regularAgentLogs
            + invalidRegularAgentLogs;
    }

    public async ValueTask DisposeAsync()
    {
        Client?.Dispose();

        if (agent is not null)
            await agent.DisposeAsync();

        if (regularAgent is not null)
            await regularAgent.DisposeAsync();

        if (invalidRegularAgent is not null)
            await invalidRegularAgent.DisposeAsync();

        await core.DisposeAsync();
        await dockerDaemon.DisposeAsync();
        await agentDataVolume.DisposeAsync();
        await coreDataVolume.DisposeAsync();
        await dockerDataVolume.DisposeAsync();
        await dockerSocketVolume.DisposeAsync();
        await network.DisposeAsync();
    }

    private IContainer BuildRegularAgent(
        string hubPublicKey,
        string networkAlias)
    {
        return new ContainerBuilder(agentImage)
            .WithEnvironment("ASPNETCORE_ENVIRONMENT", "Production")
            .WithEnvironment("HUB_PUBLIC_KEY", hubPublicKey)
            .WithEnvironment(
                "DOCKER_HOST",
                "unix:///var/run/docker.sock")
            .WithVolumeMount(dockerSocketVolume, "/var/run")
            .WithNetwork(network)
            .WithNetworkAliases(networkAlias)
            .WithWaitStrategy(
                Wait.ForUnixContainer()
                    .UntilInternalTcpPortIsAvailable(AgentGrpcPort))
            .Build();
    }

    private static async Task<string> GetContainerLogsAsync(
        string label,
        IContainer container,
        CancellationToken cancellationToken)
    {
        var (stdout, stderr) = await container.GetLogsAsync(
            DateTime.UnixEpoch,
            DateTime.UtcNow,
            timestampsEnabled: false,
            cancellationToken);
        return
            $"{Environment.NewLine}{label} stdout:{Environment.NewLine}{stdout}"
            + $"{Environment.NewLine}{label} stderr:{Environment.NewLine}{stderr}";
    }
}
