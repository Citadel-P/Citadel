using System.Diagnostics;
using System.Net.Http.Json;
using System.Text.Json;
using DotNet.Testcontainers.Builders;
using DotNet.Testcontainers.Containers;
using DotNet.Testcontainers.Networks;
using DotNet.Testcontainers.Volumes;
using Minio;
using Minio.DataModel.Args;

namespace Tests.Acceptance.Infrastructure;

public enum SwarmManagerConnectorMode
{
    Local,
    Agent,
    EdgeAgent
}

internal sealed class SwarmCompatibilityEnvironment : IAsyncDisposable
{
    private const ushort CoreHttpPort = 8000;
    private const ushort CoreGrpcPort = 8001;
    private const ushort RegistryPort = 5000;
    private const ushort RustFsPort = 9000;
    private const string CoreNetworkAlias = "citadel-core";
    private const string ManagerNetworkAlias = "swarm-manager";
    private const string WorkerOneNetworkAlias = "swarm-worker-one";
    private const string WorkerTwoNetworkAlias = "swarm-worker-two";
    private const string RegistryNetworkAlias = "swarm-registry";
    private const string RustFsNetworkAlias = "swarm-rustfs";
    internal const string PostgresNetworkAlias = "swarm-postgres";
    private const string DockerImage = "docker:27.5.1-dind";
    private const string RegistryImage = "registry:2.8.3";
    private const string RustFsImage =
        "rustfs/rustfs@sha256:60f4f2f41ce95216f8cac676e69f9d90c0bfec458a3bc7fd7fb9b7c2452ac57a";
    private const string NodeAgentTag = "candidate";
    private const string ManagerAgentContainerName = "citadel-acceptance-manager-agent";
    private const string ManagerAgentStateVolume = "citadel-acceptance-manager-agent-state";
    private const string InnerDockerBridge = "172.30.0.1/24";
    private const string SwarmGatewaySubnet = "172.31.0.0/24";
    private const string SwarmGateway = "172.31.0.1";

    public const string RustFsAccessKey = "citadel-swarm-acceptance";
    public const string RustFsSecretKey = "citadel-swarm-acceptance-secret-2026";

    private readonly string sourceCoreImage;
    private readonly string sourceAgentImage;
    private readonly string postgresContainerId;
    private readonly string nodeAgentRepository;
    private readonly string backupHelperImage;
    private readonly INetwork network;
    private readonly IVolume managerSocketVolume;
    private readonly IVolume managerDataVolume;
    private readonly IVolume workerOneSocketVolume;
    private readonly IVolume workerOneDataVolume;
    private readonly IVolume workerTwoSocketVolume;
    private readonly IVolume workerTwoDataVolume;
    private readonly IVolume coreDataVolume;
    private readonly IContainer registry;
    private readonly IContainer manager;
    private readonly IContainer workerOne;
    private readonly IContainer workerTwo;
    private readonly IContainer core;
    private readonly IContainer? rustFs;
    private IMinioClient? objectStorage;
    private bool postgresConnected;

    private SwarmCompatibilityEnvironment(
        string coreImage,
        string agentImage,
        string postgresConnectionString,
        string postgresContainerId,
        bool enableBackupInfrastructure)
    {
        sourceCoreImage = coreImage;
        sourceAgentImage = agentImage;
        this.postgresContainerId = postgresContainerId;
        var repositoryName = $"citadel-agent-{Guid.NewGuid():N}";
        var helperRepositoryName = $"citadel-backup-helper-{Guid.NewGuid():N}";
        nodeAgentRepository = $"{RegistryNetworkAlias}:{RegistryPort}/{repositoryName}";
        backupHelperImage = $"{RegistryNetworkAlias}:{RegistryPort}/{helperRepositoryName}:candidate";

        network = new NetworkBuilder().Build();
        managerSocketVolume = new VolumeBuilder().Build();
        managerDataVolume = new VolumeBuilder().Build();
        workerOneSocketVolume = new VolumeBuilder().Build();
        workerOneDataVolume = new VolumeBuilder().Build();
        workerTwoSocketVolume = new VolumeBuilder().Build();
        workerTwoDataVolume = new VolumeBuilder().Build();
        coreDataVolume = new VolumeBuilder().Build();

        registry = new ContainerBuilder(RegistryImage)
            .WithPortBinding(RegistryPort, assignRandomHostPort: true)
            .WithNetwork(network)
            .WithNetworkAliases(RegistryNetworkAlias)
            .WithWaitStrategy(
                Wait.ForUnixContainer().UntilHttpRequestIsSucceeded(
                    request => request
                        .ForPort(RegistryPort)
                        .ForPath("/v2/")))
            .Build();

        rustFs = enableBackupInfrastructure
            ? new ContainerBuilder(RustFsImage)
                .WithPortBinding(RustFsPort, assignRandomHostPort: true)
                .WithEnvironment("RUSTFS_ACCESS_KEY", RustFsAccessKey)
                .WithEnvironment("RUSTFS_SECRET_KEY", RustFsSecretKey)
                .WithNetwork(network)
                .WithNetworkAliases(RustFsNetworkAlias)
                .WithWaitStrategy(
                    Wait.ForUnixContainer()
                        .UntilInternalTcpPortIsAvailable(RustFsPort))
                .Build()
            : null;

        manager = BuildDockerDaemon(
            ManagerNetworkAlias,
            managerSocketVolume,
            managerDataVolume);
        workerOne = BuildDockerDaemon(
            WorkerOneNetworkAlias,
            workerOneSocketVolume,
            workerOneDataVolume);
        workerTwo = BuildDockerDaemon(
            WorkerTwoNetworkAlias,
            workerTwoSocketVolume,
            workerTwoDataVolume);

        core = new ContainerBuilder(coreImage)
            .WithPortBinding(CoreHttpPort, assignRandomHostPort: true)
            .WithEnvironment("ASPNETCORE_ENVIRONMENT", "Production")
            .WithEnvironment("Transport__Mode", "Disabled")
            .WithEnvironment(
                "Transport__PublicUrl",
                $"http://{CoreNetworkAlias}:{CoreHttpPort}")
            .WithEnvironment(
                "EdgeAgent__PublicGrpcUrl",
                $"http://{SwarmGateway}:{CoreGrpcPort}")
            .WithEnvironment("AgentTransport__AllowInsecure", "true")
            .WithEnvironment("EdgeAgent__AgentImageRepository", nodeAgentRepository)
            .WithEnvironment("EdgeAgent__AgentImageTag", NodeAgentTag)
            .WithEnvironment("EdgeAgent__NodeAgentSetupMinutes", "3")
            .WithEnvironment("EdgeAgent__NodeAgentBootstrapMinutes", "5")
            .WithEnvironment("VolumeBrowser__HelperImage", backupHelperImage)
            .WithEnvironment(
                "DOCKER_HOST",
                $"tcp://{ManagerNetworkAlias}:2375")
            .WithEnvironment("ConnectionStrings__Postgres", postgresConnectionString)
            .WithEnvironment("Jwt__Issuer", "http://citadel-core")
            .WithEnvironment("Jwt__Audience", "http://citadel-core")
            .WithEnvironment(
                "Jwt__Key",
                "citadel-swarm-compatibility-signing-key-000000000000000000000")
            .WithEnvironment(
                "Secrets__EncryptionKey",
                "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=")
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

    public string WorkerOneNodeId { get; private set; } = string.Empty;

    public string WorkerTwoNodeId { get; private set; } = string.Empty;

    public string ManagerNodeId { get; private set; } = string.Empty;

    public string? PlatformS3Endpoint { get; private set; }

    public static async Task<SwarmCompatibilityEnvironment> StartAsync(
        string coreImage,
        string agentImage,
        string postgresConnectionString,
        string postgresContainerId,
        CancellationToken cancellationToken) => await StartAsync(
            coreImage,
            agentImage,
            postgresConnectionString,
            postgresContainerId,
            enableBackupInfrastructure: false,
            cancellationToken);

    public static async Task<SwarmCompatibilityEnvironment> StartWithBackupAsync(
        string coreImage,
        string agentImage,
        string postgresConnectionString,
        string postgresContainerId,
        CancellationToken cancellationToken) => await StartAsync(
            coreImage,
            agentImage,
            postgresConnectionString,
            postgresContainerId,
            enableBackupInfrastructure: true,
            cancellationToken);

    private static async Task<SwarmCompatibilityEnvironment> StartAsync(
        string coreImage,
        string agentImage,
        string postgresConnectionString,
        string postgresContainerId,
        bool enableBackupInfrastructure,
        CancellationToken cancellationToken)
    {
        Assert.False(
            string.IsNullOrWhiteSpace(coreImage),
            "Set CITADEL_ACCEPTANCE_CORE_IMAGE to the Core image under test.");
        Assert.False(
            string.IsNullOrWhiteSpace(agentImage),
            "Set CITADEL_ACCEPTANCE_AGENT_IMAGE to the Agent image under test.");

        var environment = new SwarmCompatibilityEnvironment(
            coreImage,
            agentImage,
            postgresConnectionString,
            postgresContainerId,
            enableBackupInfrastructure);
        try
        {
            await environment.StartCoreInfrastructureAsync(cancellationToken);
            return environment;
        }
        catch (Exception exception)
        {
            var diagnostics = await environment.GetDiagnosticsAsync(
                CancellationToken.None);
            await environment.DisposeAsync();
            throw new InvalidOperationException(
                $"Starting the disposable Swarm environment failed.{Environment.NewLine}{diagnostics}",
                exception);
        }
    }

    public async Task AuthenticateAsAdminAsync(CancellationToken cancellationToken)
    {
        try
        {
            await InitialAdministratorSession.AuthenticateAsync(
                Client,
                "Swarm compatibility Core image",
                cancellationToken);
        }
        catch (Exception exception)
        {
            throw new InvalidOperationException(
                $"Authenticating with the disposable Core failed.{Environment.NewLine}{await GetDiagnosticsAsync(CancellationToken.None)}",
                exception);
        }
    }

    public async Task<Guid> CreatePlatformAsync(
        SwarmManagerConnectorMode connectorMode,
        CancellationToken cancellationToken)
    {
        if (connectorMode == SwarmManagerConnectorMode.Agent)
            await StartRegularManagerAgentAsync(cancellationToken);

        using var response = await Client.PostAsJsonAsync(
            "/api/v1/platforms",
            new
            {
                name = $"acceptance-swarm-{Guid.NewGuid():N}",
                address = connectorMode == SwarmManagerConnectorMode.Agent
                    ? $"http://{ManagerNetworkAlias}:9000"
                    : null,
                description = "Real multi-node Swarm compatibility platform",
                type = "DockerSwarm",
                connectorType = connectorMode == SwarmManagerConnectorMode.EdgeAgent
                    ? "edgeAgent"
                    : connectorMode.ToString()
            },
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        if (!response.IsSuccessStatusCode)
        {
            Assert.Fail(
                $"Creating the Swarm platform failed with HTTP {(int)response.StatusCode}: {body}{Environment.NewLine}{await GetDiagnosticsAsync(CancellationToken.None)}");
        }

        using var json = JsonDocument.Parse(body);
        var platformId = json.RootElement.GetProperty("id").GetGuid();
        if (connectorMode == SwarmManagerConnectorMode.EdgeAgent)
        {
            var enrollmentToken = await CreateEdgeEnrollmentAsync(
                platformId,
                cancellationToken);
            await StartEdgeManagerAgentAsync(enrollmentToken, cancellationToken);
            await WaitForEdgeManagerConnectionAsync(platformId, cancellationToken);
        }

        return platformId;
    }

    public async Task InstallNodeAgentsAsync(
        Guid platformId,
        CancellationToken cancellationToken)
    {
        HttpResponseMessage response;
        string body;
        try
        {
            response = await Client.PostAsync(
                $"/api/v1/platforms/{platformId:D}/node-agents/install",
                content: null,
                cancellationToken);
            body = await response.Content.ReadAsStringAsync(cancellationToken);
        }
        catch (Exception exception)
        {
            throw new InvalidOperationException(
                $"The node-agent installation response ended unexpectedly.{Environment.NewLine}{await GetNodeAgentDiagnosticsAsync(platformId, CancellationToken.None)}{Environment.NewLine}{await GetDiagnosticsAsync(CancellationToken.None)}",
                exception);
        }

        using (response)
        {
            if (!response.IsSuccessStatusCode)
            {
                Assert.Fail(
                    $"Installing Swarm node Agents failed with HTTP {(int)response.StatusCode}: {body}{Environment.NewLine}{await GetNodeAgentDiagnosticsAsync(platformId, CancellationToken.None)}{Environment.NewLine}{await GetDiagnosticsAsync(CancellationToken.None)}");
            }

            using var json = JsonDocument.Parse(body);
            var progress = json.RootElement.EnumerateArray().ToArray();
            Assert.NotEmpty(progress);
            var completion = progress[^1];
            var hasError = completion.TryGetProperty("errorMessage", out var errorMessage)
                && errorMessage.ValueKind != JsonValueKind.Null
                && !string.IsNullOrWhiteSpace(errorMessage.GetString());
            if (!completion.GetProperty("isCompleted").GetBoolean()
                || !string.Equals(completion.GetProperty("stage").GetString(), "completed", StringComparison.Ordinal)
                || hasError)
            {
                Assert.Fail(
                    $"Swarm node Agent installation failed: {(hasError ? errorMessage.GetString() : body)}{Environment.NewLine}{await GetNodeAgentDiagnosticsAsync(platformId, CancellationToken.None)}{Environment.NewLine}{await GetDiagnosticsAsync(CancellationToken.None)}");
            }
        }
    }

    private async Task<string> GetNodeAgentDiagnosticsAsync(
        Guid platformId,
        CancellationToken cancellationToken)
    {
        var sections = new List<string>(3);
        try
        {
            using var response = await Client.GetAsync(
                $"/api/v1/platforms/{platformId:D}/node-agent-coverage",
                cancellationToken);
            sections.Add(
                $"Node Agent coverage ({(int)response.StatusCode}):{Environment.NewLine}{await response.Content.ReadAsStringAsync(cancellationToken)}");
        }
        catch (Exception exception)
        {
            sections.Add($"Node Agent coverage diagnostics unavailable: {exception.Message}");
        }

        var serviceName = $"citadel-node-agent-{platformId:N}";
        await AddNestedDockerCommandDiagnosticsAsync(
            sections,
            "Node Agent Service tasks",
            ["docker", "service", "ps", "--no-trunc", serviceName],
            cancellationToken);
        await AddNestedDockerCommandDiagnosticsAsync(
            sections,
            "Node Agent Service logs",
            ["docker", "service", "logs", "--raw", "--tail", "200", serviceName],
            cancellationToken);
        await AddNestedDockerCommandDiagnosticsAsync(
            sections,
            "Node Agent Service specification",
            ["docker", "service", "inspect", "--format", "{{json .Spec}}", serviceName],
            cancellationToken);
        return string.Join(Environment.NewLine, sections);
    }

    public async Task<JsonElement> WaitForCoverageAsync(
        Guid platformId,
        Func<JsonElement, bool> predicate,
        string expectation,
        CancellationToken cancellationToken)
    {
        string? lastBody = null;
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeout.CancelAfter(TimeSpan.FromMinutes(2));

        try
        {
            while (!timeout.IsCancellationRequested)
            {
                using var response = await Client.GetAsync(
                    $"/api/v1/platforms/{platformId:D}/node-agent-coverage",
                    timeout.Token);
                lastBody = await response.Content.ReadAsStringAsync(timeout.Token);
                if (response.IsSuccessStatusCode)
                {
                    using var json = JsonDocument.Parse(lastBody);
                    if (predicate(json.RootElement))
                        return json.RootElement.Clone();
                }

                await Task.Delay(TimeSpan.FromSeconds(1), timeout.Token);
            }
        }
        catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
        }

        Assert.Fail(
            $"Timed out waiting for {expectation}. Last coverage: {lastBody}{Environment.NewLine}{await GetNodeAgentDiagnosticsAsync(platformId, CancellationToken.None)}{Environment.NewLine}{await GetDiagnosticsAsync(CancellationToken.None)}");
        return default;
    }

    public async Task WaitForSwarmNodesAsync(
        Guid platformId,
        int expectedCount,
        CancellationToken cancellationToken)
    {
        string? lastBody = null;
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeout.CancelAfter(TimeSpan.FromMinutes(2));

        try
        {
            while (!timeout.IsCancellationRequested)
            {
                using var response = await Client.GetAsync(
                    $"/api/v1/platforms/{platformId:D}/swarm/nodes",
                    timeout.Token);
                lastBody = await response.Content.ReadAsStringAsync(timeout.Token);
                if (response.IsSuccessStatusCode)
                {
                    using var json = JsonDocument.Parse(lastBody);
                    if (json.RootElement.GetProperty("items").GetArrayLength() == expectedCount)
                        return;
                }

                await Task.Delay(TimeSpan.FromSeconds(1), timeout.Token);
            }
        }
        catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
        }

        Assert.Fail(
            $"Timed out waiting for {expectedCount} Swarm Nodes. Last response: {lastBody}{Environment.NewLine}{await GetDiagnosticsAsync(CancellationToken.None)}");
    }

    public async Task<(string ContainerId, string NodeId)> StartWorkerContainerAsync(
        int workerNumber,
        CancellationToken cancellationToken)
    {
        var (worker, nodeId) = workerNumber switch
        {
            1 => (workerOne, WorkerOneNodeId),
            2 => (workerTwo, WorkerTwoNodeId),
            _ => throw new ArgumentOutOfRangeException(nameof(workerNumber))
        };
        var name = $"citadel-acceptance-worker-{workerNumber}";
        var result = await worker.ExecAsync(
            [
                "docker", "run", "--detach", "--restart", "unless-stopped",
                "--name", name,
                "--label", "citadel.acceptance=swarm-node",
                "busybox:1.36.1",
                "sh", "-c", "while true; do sleep 3600; done"
            ],
            cancellationToken);
        AssertCommandSucceeded(result, $"Starting {name}");
        return (result.Stdout.Trim(), nodeId);
    }

    public async Task CreateBackupBucketAsync(
        string bucket,
        CancellationToken cancellationToken)
    {
        Assert.NotNull(objectStorage);
        var exists = await objectStorage.BucketExistsAsync(
            new BucketExistsArgs().WithBucket(bucket),
            cancellationToken);
        if (!exists)
        {
            await objectStorage.MakeBucketAsync(
                new MakeBucketArgs().WithBucket(bucket),
                cancellationToken);
        }
    }

    public async Task SeedWorkerVolumeAsync(
        int workerNumber,
        string volumeName,
        byte[] content,
        CancellationToken cancellationToken)
    {
        var worker = GetWorker(workerNumber);
        AssertCommandSucceeded(
            await worker.ExecAsync(["docker", "pull", "busybox:1.36.1"], cancellationToken),
            $"Pulling the worker {workerNumber} volume fixture image");
        AssertCommandSucceeded(
            await worker.ExecAsync(["docker", "volume", "create", volumeName], cancellationToken),
            $"Creating worker {workerNumber} Volume {volumeName}");

        var payload = Convert.ToBase64String(content);
        AssertCommandSucceeded(
            await worker.ExecAsync(
                [
                    "docker", "run", "--rm",
                    "--volume", $"{volumeName}:/data",
                    "busybox:1.36.1",
                    "sh", "-c", $"printf '%s' '{payload}' | base64 -d > /data/payload.bin"
                ],
                cancellationToken),
            $"Seeding worker {workerNumber} Volume {volumeName}");
    }

    public async Task<byte[]> ReadWorkerVolumePayloadAsync(
        int workerNumber,
        string volumeName,
        CancellationToken cancellationToken)
    {
        var result = await GetWorker(workerNumber).ExecAsync(
            [
                "docker", "run", "--rm",
                "--volume", $"{volumeName}:/data:ro",
                "busybox:1.36.1",
                "base64", "/data/payload.bin"
            ],
            cancellationToken);
        AssertCommandSucceeded(
            result,
            $"Reading worker {workerNumber} Volume {volumeName}");
        return Convert.FromBase64String(result.Stdout);
    }

    public async Task AssertNoBackupHelpersAsync(
        int workerNumber,
        CancellationToken cancellationToken)
    {
        var result = await GetWorker(workerNumber).ExecAsync(
            [
                "docker", "ps", "--all", "--quiet",
                "--filter", "label=citadel.backup-helper=true"
            ],
            cancellationToken);
        AssertCommandSucceeded(
            result,
            $"Listing worker {workerNumber} backup helpers");
        Assert.True(
            string.IsNullOrWhiteSpace(result.Stdout),
            $"Worker {workerNumber} retained backup helper containers: {result.Stdout}");
    }

    public Task<JsonElement> WaitForNodeVolumeAsync(
        Guid platformId,
        string dockerNodeId,
        string volumeName,
        CancellationToken cancellationToken) => WaitForNodeLocalResourceAsync(
            $"/api/v1/volumes/{platformId:D}",
            "volumes",
            item => item.GetProperty("dockerNodeId").GetString() == dockerNodeId
                    && item.GetProperty("name").GetString() == volumeName
                    && !item.GetProperty("isStale").GetBoolean(),
            $"Node {dockerNodeId} Volume {volumeName}",
            cancellationToken);

    public async Task AssertNodeLocalResourcesRouteAsync(
        Guid platformId,
        int workerNumber,
        string dockerNodeId,
        CancellationToken cancellationToken)
    {
        var worker = workerNumber switch
        {
            1 => workerOne,
            2 => workerTwo,
            _ => throw new ArgumentOutOfRangeException(nameof(workerNumber))
        };
        var volumeName = $"citadel-acceptance-volume-{workerNumber}";
        var networkName = $"citadel-acceptance-network-{workerNumber}";
        AssertCommandSucceeded(
            await worker.ExecAsync(["docker", "volume", "create", volumeName], cancellationToken),
            $"Creating {volumeName}");
        AssertCommandSucceeded(
            await worker.ExecAsync(["docker", "network", "create", networkName], cancellationToken),
            $"Creating {networkName}");

        var image = await WaitForNodeLocalResourceAsync(
            $"/api/v1/images/{platformId:D}",
            "images",
            item => item.GetProperty("dockerNodeId").GetString() == dockerNodeId
                    && item.GetProperty("tags").EnumerateArray().Any(tag =>
                        tag.GetString()?.Contains("busybox", StringComparison.OrdinalIgnoreCase) == true),
            $"worker {workerNumber} Image inventory",
            cancellationToken);
        var volume = await WaitForNodeLocalResourceAsync(
            $"/api/v1/volumes/{platformId:D}",
            "volumes",
            item => item.GetProperty("dockerNodeId").GetString() == dockerNodeId
                    && item.GetProperty("name").GetString() == volumeName,
            $"worker {workerNumber} Volume inventory",
            cancellationToken);
        var network = await WaitForNodeLocalResourceAsync(
            $"/api/v1/networks/{platformId:D}",
            "networks",
            item => item.GetProperty("dockerNodeId").GetString() == dockerNodeId
                    && item.GetProperty("name").GetString() == networkName,
            $"worker {workerNumber} local Network inventory",
            cancellationToken);

        var imageId = image.GetProperty("dockerImageId").GetString() ?? string.Empty;
        if (imageId.StartsWith("sha256:", StringComparison.OrdinalIgnoreCase))
            imageId = imageId[7..];
        await AssertNodeInspectAsync(
            $"/api/v1/images/{platformId:D}/{imageId[..Math.Min(12, imageId.Length)]}?dockerNodeId={Uri.EscapeDataString(dockerNodeId)}",
            dockerNodeId,
            cancellationToken);
        await AssertNodeInspectAsync(
            $"/api/v1/volumes/{platformId:D}/{Uri.EscapeDataString(volume.GetProperty("name").GetString()!)}?dockerNodeId={Uri.EscapeDataString(dockerNodeId)}",
            dockerNodeId,
            cancellationToken);
        using (var browseResponse = await Client.GetAsync(
                   $"/api/v1/platforms/{platformId:D}/volumes/{Uri.EscapeDataString(volumeName)}/files?dockerNodeId={Uri.EscapeDataString(dockerNodeId)}",
                   cancellationToken))
        {
            var browseBody = await browseResponse.Content.ReadAsStringAsync(cancellationToken);
            Assert.True(
                browseResponse.IsSuccessStatusCode,
                $"Browsing a worker Volume failed with HTTP {(int)browseResponse.StatusCode}: {browseBody}");
        }
        await AssertNodeInspectAsync(
            $"/api/v1/networks/{platformId:D}/{network.GetProperty("id").GetString()}?dockerNodeId={Uri.EscapeDataString(dockerNodeId)}",
            dockerNodeId,
            cancellationToken);
    }

    public Task<JsonElement> WaitForNodeVolumeStateAsync(
        Guid platformId,
        string dockerNodeId,
        bool isStale,
        CancellationToken cancellationToken) => WaitForNodeLocalResourceAsync(
        $"/api/v1/volumes/{platformId:D}",
        "volumes",
        item => item.GetProperty("dockerNodeId").GetString() == dockerNodeId
                && item.GetProperty("isStale").GetBoolean() == isStale,
        $"Node {dockerNodeId} Volume projection stale={isStale}",
        cancellationToken);

    private async Task<JsonElement> WaitForNodeLocalResourceAsync(
        string url,
        string collection,
        Func<JsonElement, bool> predicate,
        string expectation,
        CancellationToken cancellationToken)
    {
        string? lastBody = null;
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeout.CancelAfter(TimeSpan.FromMinutes(2));
        try
        {
            while (!timeout.IsCancellationRequested)
            {
                using var response = await Client.GetAsync(url, timeout.Token);
                lastBody = await response.Content.ReadAsStringAsync(timeout.Token);
                if (response.IsSuccessStatusCode)
                {
                    using var json = JsonDocument.Parse(lastBody);
                    var resource = json.RootElement.GetProperty(collection).EnumerateArray().FirstOrDefault(predicate);
                    if (resource.ValueKind != JsonValueKind.Undefined)
                        return resource.Clone();
                }

                await Task.Delay(TimeSpan.FromSeconds(1), timeout.Token);
            }
        }
        catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
        }

        Assert.Fail(
            $"Timed out waiting for {expectation}. Last response: {lastBody}{Environment.NewLine}{await GetDiagnosticsAsync(CancellationToken.None)}");
        return default;
    }

    private async Task AssertNodeInspectAsync(
        string url,
        string dockerNodeId,
        CancellationToken cancellationToken)
    {
        using var response = await Client.GetAsync(url, cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(response.IsSuccessStatusCode, $"Node-local inspect failed with HTTP {(int)response.StatusCode}: {body}");
        using var json = JsonDocument.Parse(body);
        Assert.Equal(dockerNodeId, json.RootElement.GetProperty("dockerNodeId").GetString());
    }

    public async Task<JsonElement> WaitForContainerAsync(
        Guid platformId,
        string dockerContainerId,
        Func<JsonElement, bool> predicate,
        string expectation,
        CancellationToken cancellationToken)
    {
        string? lastBody = null;
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeout.CancelAfter(TimeSpan.FromMinutes(2));

        try
        {
            while (!timeout.IsCancellationRequested)
            {
                using var response = await Client.GetAsync(
                    $"/api/v1/platforms/{platformId:D}/containers",
                    timeout.Token);
                lastBody = await response.Content.ReadAsStringAsync(timeout.Token);
                if (response.IsSuccessStatusCode)
                {
                    using var json = JsonDocument.Parse(lastBody);
                    var container = json.RootElement
                        .GetProperty("containers")
                        .EnumerateArray()
                        .FirstOrDefault(item => string.Equals(
                            item.GetProperty("containerId").GetString(),
                            dockerContainerId,
                            StringComparison.Ordinal));
                    if (container.ValueKind != JsonValueKind.Undefined && predicate(container))
                        return container.Clone();
                }

                await Task.Delay(TimeSpan.FromSeconds(1), timeout.Token);
            }
        }
        catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
        }

        Assert.Fail(
            $"Timed out waiting for {expectation}. Last Container response: {lastBody}{Environment.NewLine}{await GetDiagnosticsAsync(CancellationToken.None)}");
        return default;
    }

    public async Task AssertContainerInspectRoutesAsync(
        JsonElement container,
        CancellationToken cancellationToken)
    {
        var id = container.GetProperty("id").GetGuid();
        using var response = await Client.GetAsync(
            $"/api/v1/containers/{id:D}/inspect",
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(
            response.IsSuccessStatusCode,
            $"Inspecting a worker Container failed with HTTP {(int)response.StatusCode}: {body}");
        using var json = JsonDocument.Parse(body);
        Assert.Contains(
            "busybox",
            json.RootElement.GetProperty("config").GetProperty("image").GetString(),
            StringComparison.OrdinalIgnoreCase);
    }

    public Task StopWorkerTwoAsync(CancellationToken cancellationToken) =>
        workerTwo.StopAsync(cancellationToken);

    public async Task RestartWorkerTwoAsync(CancellationToken cancellationToken)
    {
        await workerTwo.StartAsync(cancellationToken);
        await ConfigureCoreForwardingAsync(workerTwo, cancellationToken);
    }

    public async Task DisconnectManagerConnectorAsync(
        SwarmManagerConnectorMode connectorMode,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        if (connectorMode == SwarmManagerConnectorMode.Local)
        {
            await manager.StopAsync(cancellationToken);
        }
        else
        {
            var result = await manager.ExecAsync(
                ["docker", "stop", ManagerAgentContainerName],
                cancellationToken);
            AssertCommandSucceeded(result, "Stopping the manager Agent");
        }

        await WaitForManagerControlPlaneAsync(
            platformId,
            available: false,
            cancellationToken);
    }

    public async Task ReconnectManagerConnectorAsync(
        SwarmManagerConnectorMode connectorMode,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        if (connectorMode == SwarmManagerConnectorMode.Local)
        {
            await manager.StartAsync(cancellationToken);
            await ConfigureCoreForwardingAsync(manager, cancellationToken);
        }
        else
        {
            var result = await manager.ExecAsync(
                ["docker", "start", ManagerAgentContainerName],
                cancellationToken);
            AssertCommandSucceeded(result, "Restarting the manager Agent");
            if (connectorMode == SwarmManagerConnectorMode.Agent)
                await WaitForManagerAgentPortAsync(cancellationToken);
            else
                await WaitForEdgeManagerConnectionAsync(platformId, cancellationToken);
        }

        await WaitForManagerControlPlaneAsync(
            platformId,
            available: true,
            cancellationToken);
    }

    public async Task<string> GetDiagnosticsAsync(CancellationToken cancellationToken)
    {
        var sections = new List<string>(6);
        await AddLogsAsync(sections, "Core", core, cancellationToken);
        await AddLogsAsync(sections, "Manager", manager, cancellationToken);
        await AddLogsAsync(sections, "Worker one", workerOne, cancellationToken);
        await AddLogsAsync(sections, "Worker two", workerTwo, cancellationToken);
        await AddLogsAsync(sections, "Registry", registry, cancellationToken);
        if (rustFs is not null)
            await AddLogsAsync(sections, "RustFS", rustFs, cancellationToken);
        await AddNestedManagerAgentLogsAsync(sections, cancellationToken);
        return string.Join(Environment.NewLine, sections);
    }

    public async ValueTask DisposeAsync()
    {
        Client?.Dispose();
        objectStorage?.Dispose();

        await core.DisposeAsync();
        await workerTwo.DisposeAsync();
        await workerOne.DisposeAsync();
        await manager.DisposeAsync();
        if (rustFs is not null)
            await rustFs.DisposeAsync();
        await registry.DisposeAsync();
        await coreDataVolume.DisposeAsync();
        await workerTwoDataVolume.DisposeAsync();
        await workerTwoSocketVolume.DisposeAsync();
        await workerOneDataVolume.DisposeAsync();
        await workerOneSocketVolume.DisposeAsync();
        await managerDataVolume.DisposeAsync();
        await managerSocketVolume.DisposeAsync();
        if (postgresConnected)
        {
            await RunProcessAsync(
                "docker",
                ["network", "disconnect", "--force", network.Name, postgresContainerId],
                throwOnError: false,
                CancellationToken.None);
            postgresConnected = false;
        }
        await network.DisposeAsync();

    }

    private async Task StartCoreInfrastructureAsync(CancellationToken cancellationToken)
    {
        await network.CreateAsync(cancellationToken);
        await RunProcessAsync(
            "docker",
            ["network", "connect", "--alias", PostgresNetworkAlias, network.Name, postgresContainerId],
            throwOnError: true,
            cancellationToken);
        postgresConnected = true;
        await managerSocketVolume.CreateAsync(cancellationToken);
        await managerDataVolume.CreateAsync(cancellationToken);
        await workerOneSocketVolume.CreateAsync(cancellationToken);
        await workerOneDataVolume.CreateAsync(cancellationToken);
        await workerTwoSocketVolume.CreateAsync(cancellationToken);
        await workerTwoDataVolume.CreateAsync(cancellationToken);
        await coreDataVolume.CreateAsync(cancellationToken);
        await registry.StartAsync(cancellationToken);
        if (rustFs is not null)
        {
            await rustFs.StartAsync(cancellationToken);
            await InitializeObjectStorageAsync(cancellationToken);
        }

        await manager.StartAsync(cancellationToken);
        await workerOne.StartAsync(cancellationToken);
        await workerTwo.StartAsync(cancellationToken);
        await CreateSwarmGatewayAsync(manager, cancellationToken);
        await CreateSwarmGatewayAsync(workerOne, cancellationToken);
        await CreateSwarmGatewayAsync(workerTwo, cancellationToken);
        await InitializeSwarmAsync(cancellationToken);
        await core.StartAsync(cancellationToken);
        await ConfigureCoreForwardingAsync(manager, cancellationToken);
        await ConfigureCoreForwardingAsync(workerOne, cancellationToken);
        await ConfigureCoreForwardingAsync(workerTwo, cancellationToken);
        await ImportCandidateImageAsync(
            sourceAgentImage,
            $"{nodeAgentRepository}:{NodeAgentTag}",
            cancellationToken);
        await ImportCandidateImageAsync(
            sourceCoreImage,
            backupHelperImage,
            cancellationToken);

        Client = new HttpClient
        {
            BaseAddress = new Uri(
                $"http://127.0.0.1:{core.GetMappedPublicPort(CoreHttpPort)}"),
            Timeout = TimeSpan.FromMinutes(5)
        };
    }

    private IContainer GetWorker(int workerNumber) => workerNumber switch
    {
        1 => workerOne,
        2 => workerTwo,
        _ => throw new ArgumentOutOfRangeException(nameof(workerNumber))
    };

    private async Task InitializeObjectStorageAsync(CancellationToken cancellationToken)
    {
        Assert.NotNull(rustFs);
        PlatformS3Endpoint = $"http://{rustFs.IpAddress}:{RustFsPort}";
        var hostEndpoint = new Uri(
            $"http://127.0.0.1:{rustFs.GetMappedPublicPort(RustFsPort)}");
        objectStorage = new MinioClient()
            .WithEndpoint(hostEndpoint.Host, hostEndpoint.Port)
            .WithCredentials(RustFsAccessKey, RustFsSecretKey)
            .Build();

        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeout.CancelAfter(TimeSpan.FromMinutes(2));
        while (!timeout.IsCancellationRequested)
        {
            try
            {
                await objectStorage.ListBucketsAsync(timeout.Token);
                return;
            }
            catch when (!timeout.IsCancellationRequested)
            {
                await Task.Delay(TimeSpan.FromMilliseconds(500), timeout.Token);
            }
        }

        throw new TimeoutException("RustFS did not become ready for the Swarm backup compatibility test.");
    }

    private IContainer BuildDockerDaemon(
        string networkAlias,
        IVolume socketVolume,
        IVolume dataVolume) =>
        new ContainerBuilder(DockerImage)
            .WithPrivileged(true)
            .WithEnvironment("DOCKER_TLS_CERTDIR", string.Empty)
            .WithCommand(
                "--host=unix:///var/run/docker.sock",
                "--host=tcp://0.0.0.0:2375",
                $"--bip={InnerDockerBridge}",
                $"--insecure-registry={RegistryNetworkAlias}:{RegistryPort}")
            .WithVolumeMount(socketVolume, "/var/run")
            .WithVolumeMount(dataVolume, "/var/lib/docker")
            .WithNetwork(network)
            .WithNetworkAliases(networkAlias)
            .WithWaitStrategy(
                Wait.ForUnixContainer().UntilCommandIsCompleted(
                    "docker",
                    "info"))
            .Build();

    private async Task InitializeSwarmAsync(CancellationToken cancellationToken)
    {
        var initialize = await manager.ExecAsync(
            ["docker", "swarm", "init", "--advertise-addr", manager.IpAddress],
            cancellationToken);
        AssertCommandSucceeded(initialize, "Initializing the acceptance Swarm");

        var tokenResult = await manager.ExecAsync(
            ["docker", "swarm", "join-token", "--quiet", "worker"],
            cancellationToken);
        AssertCommandSucceeded(tokenResult, "Reading the Swarm worker token");
        var token = tokenResult.Stdout.Trim();
        Assert.False(string.IsNullOrWhiteSpace(token));

        await JoinWorkerAsync(workerOne, token, cancellationToken);
        await JoinWorkerAsync(workerTwo, token, cancellationToken);
        ManagerNodeId = await ReadNodeIdAsync(manager, cancellationToken);
        WorkerOneNodeId = await ReadNodeIdAsync(workerOne, cancellationToken);
        WorkerTwoNodeId = await ReadNodeIdAsync(workerTwo, cancellationToken);

        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeout.CancelAfter(TimeSpan.FromMinutes(1));
        while (!timeout.IsCancellationRequested)
        {
            var nodes = await manager.ExecAsync(
                ["docker", "node", "ls", "--format", "{{.ID}} {{.Status}}"],
                timeout.Token);
            if ((nodes.ExitCode ?? -1) == 0
                && nodes.Stdout.Split('\n', StringSplitOptions.RemoveEmptyEntries)
                    .Count(static line => line.EndsWith(" Ready", StringComparison.OrdinalIgnoreCase)) == 3)
            {
                return;
            }

            await Task.Delay(TimeSpan.FromSeconds(1), timeout.Token);
        }

        Assert.Fail("The acceptance Swarm did not report three Ready nodes.");
    }

    private async Task StartRegularManagerAgentAsync(CancellationToken cancellationToken)
    {
        using var response = await Client.GetAsync(
            "/api/v1/platforms/agent/setup",
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(
            response.IsSuccessStatusCode,
            $"Reading regular Agent setup failed with HTTP {(int)response.StatusCode}: {body}");
        using var json = JsonDocument.Parse(body);
        var hubPublicKey = json.RootElement.GetProperty("hubPublicKey").GetString();
        Assert.False(string.IsNullOrWhiteSpace(hubPublicKey));

        var result = await manager.ExecAsync(
            [
                "docker", "run", "--detach",
                "--name", ManagerAgentContainerName,
                "--network", "host",
                "--env", "ASPNETCORE_ENVIRONMENT=Production",
                "--env", $"HUB_PUBLIC_KEY={hubPublicKey}",
                "--env", "DOCKER_HOST=unix:///var/run/docker.sock",
                "--volume", "/var/run/docker.sock:/var/run/docker.sock",
                $"{nodeAgentRepository}:{NodeAgentTag}"
            ],
            cancellationToken);
        AssertCommandSucceeded(result, "Starting the regular manager Agent");
        await WaitForManagerAgentPortAsync(cancellationToken);
    }

    private async Task<string> CreateEdgeEnrollmentAsync(
        Guid platformId,
        CancellationToken cancellationToken)
    {
        using var response = await Client.PostAsync(
            $"/api/v1/platforms/{platformId:D}/edge/enrollments",
            content: null,
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(
            response.IsSuccessStatusCode,
            $"Creating the manager Edge Agent enrollment failed with HTTP {(int)response.StatusCode}: {body}");
        using var json = JsonDocument.Parse(body);
        return json.RootElement.GetProperty("token").GetString()!;
    }

    private async Task StartEdgeManagerAgentAsync(
        string enrollmentToken,
        CancellationToken cancellationToken)
    {
        var result = await manager.ExecAsync(
            [
                "docker", "run", "--detach",
                "--name", ManagerAgentContainerName,
                "--network", "host",
                "--env", "ASPNETCORE_ENVIRONMENT=Production",
                "--env", "CITADEL_AGENT_MODE=edge",
                "--env", "CITADEL_EDGE_AGENT_PROFILE=edge-agent-compatibility",
                "--env", $"CITADEL_CORE_URL=http://{SwarmGateway}:{CoreGrpcPort}",
                "--env", $"CITADEL_EDGE_ENROLLMENT_TOKEN={enrollmentToken}",
                "--env", "CITADEL_EDGE_AGENT_KEY_PATH=/app/data/edge-agent.key",
                "--env", "CITADEL_EDGE_IDENTITY_PATH=/app/data/edge-agent.identity.json",
                "--env", "DOCKER_HOST=unix:///var/run/docker.sock",
                "--volume", "/var/run/docker.sock:/var/run/docker.sock",
                "--volume", $"{ManagerAgentStateVolume}:/app/data",
                $"{nodeAgentRepository}:{NodeAgentTag}"
            ],
            cancellationToken);
        AssertCommandSucceeded(result, "Starting the Edge manager Agent");
    }

    private async Task WaitForManagerAgentPortAsync(CancellationToken cancellationToken)
    {
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeout.CancelAfter(TimeSpan.FromMinutes(1));
        try
        {
            while (!timeout.IsCancellationRequested)
            {
                var result = await manager.ExecAsync(
                    ["sh", "-c", "nc -z 127.0.0.1 9000"],
                    timeout.Token);
                if ((result.ExitCode ?? -1) == 0)
                    return;

                await Task.Delay(TimeSpan.FromSeconds(1), timeout.Token);
            }
        }
        catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
        }

        Assert.Fail(
            $"The regular manager Agent did not open port 9000.{Environment.NewLine}{await GetDiagnosticsAsync(CancellationToken.None)}");
    }

    private async Task WaitForEdgeManagerConnectionAsync(
        Guid platformId,
        CancellationToken cancellationToken)
    {
        string? lastBody = null;
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeout.CancelAfter(TimeSpan.FromMinutes(1));
        try
        {
            while (!timeout.IsCancellationRequested)
            {
                using var response = await Client.GetAsync(
                    $"/api/v1/platforms/{platformId:D}/edge/status",
                    timeout.Token);
                lastBody = await response.Content.ReadAsStringAsync(timeout.Token);
                if (response.IsSuccessStatusCode)
                {
                    using var json = JsonDocument.Parse(lastBody);
                    if (json.RootElement.GetProperty("connectionStatus").GetString() == "Connected")
                        return;
                }

                await Task.Delay(TimeSpan.FromSeconds(1), timeout.Token);
            }
        }
        catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
        }

        Assert.Fail(
            $"The Edge manager Agent did not connect. Last status: {lastBody}{Environment.NewLine}{await GetDiagnosticsAsync(CancellationToken.None)}");
    }

    private async Task WaitForManagerControlPlaneAsync(
        Guid platformId,
        bool available,
        CancellationToken cancellationToken)
    {
        string? lastBody = null;
        int? lastStatusCode = null;
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeout.CancelAfter(TimeSpan.FromMinutes(1));
        try
        {
            while (!timeout.IsCancellationRequested)
            {
                using var response = await Client.GetAsync(
                    $"/api/v1/platforms/{platformId:D}/swarm/nodes/{ManagerNodeId}/inspect",
                    timeout.Token);
                lastStatusCode = (int)response.StatusCode;
                lastBody = await response.Content.ReadAsStringAsync(timeout.Token);
                if (response.IsSuccessStatusCode == available)
                    return;

                await Task.Delay(TimeSpan.FromSeconds(1), timeout.Token);
            }
        }
        catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
        }

        Assert.Fail(
            $"Manager control plane did not become {(available ? "available" : "unavailable")}. Last HTTP status: {lastStatusCode}. Body: {lastBody}{Environment.NewLine}{await GetDiagnosticsAsync(CancellationToken.None)}");
    }

    private static async Task CreateSwarmGatewayAsync(
        IContainer dockerDaemon,
        CancellationToken cancellationToken)
    {
        var result = await dockerDaemon.ExecAsync(
            [
                "docker", "network", "create",
                "--driver", "bridge",
                "--subnet", SwarmGatewaySubnet,
                "--gateway", SwarmGateway,
                "--opt", "com.docker.network.bridge.name=docker_gwbridge",
                "docker_gwbridge"
            ],
            cancellationToken);
        AssertCommandSucceeded(result, "Creating the disposable Swarm gateway");
    }

    private async Task JoinWorkerAsync(
        IContainer worker,
        string token,
        CancellationToken cancellationToken)
    {
        var result = await worker.ExecAsync(
            [
                "docker", "swarm", "join",
                "--token", token,
                "--advertise-addr", worker.IpAddress,
                $"{manager.IpAddress}:2377"
            ],
            cancellationToken);
        AssertCommandSucceeded(result, "Joining a worker to the acceptance Swarm");
    }

    private static async Task<string> ReadNodeIdAsync(
        IContainer worker,
        CancellationToken cancellationToken)
    {
        var result = await worker.ExecAsync(
            ["docker", "info", "--format", "{{.Swarm.NodeID}}"],
            cancellationToken);
        AssertCommandSucceeded(result, "Reading the worker Node ID");
        var nodeId = result.Stdout.Trim();
        Assert.False(string.IsNullOrWhiteSpace(nodeId));
        return nodeId;
    }

    private async Task ConfigureCoreForwardingAsync(
        IContainer dockerDaemon,
        CancellationToken cancellationToken)
    {
        var result = await dockerDaemon.ExecAsync(
            [
                "sh", "-c",
                $"iptables -t nat -C PREROUTING -p tcp -d {SwarmGateway} --dport {CoreGrpcPort} -j DNAT --to-destination {core.IpAddress}:{CoreGrpcPort} 2>/dev/null || "
                + $"iptables -t nat -A PREROUTING -p tcp -d {SwarmGateway} --dport {CoreGrpcPort} -j DNAT --to-destination {core.IpAddress}:{CoreGrpcPort}; "
                + $"iptables -t nat -C OUTPUT -p tcp -d {SwarmGateway} --dport {CoreGrpcPort} -j DNAT --to-destination {core.IpAddress}:{CoreGrpcPort} 2>/dev/null || "
                + $"iptables -t nat -A OUTPUT -p tcp -d {SwarmGateway} --dport {CoreGrpcPort} -j DNAT --to-destination {core.IpAddress}:{CoreGrpcPort}; "
                + $"iptables -t nat -C POSTROUTING -p tcp -d {core.IpAddress} --dport {CoreGrpcPort} -j MASQUERADE 2>/dev/null || "
                + $"iptables -t nat -A POSTROUTING -p tcp -d {core.IpAddress} --dport {CoreGrpcPort} -j MASQUERADE"
            ],
            cancellationToken);
        AssertCommandSucceeded(result, "Configuring the disposable Core route");
    }

    private static void AssertCommandSucceeded(
        DotNet.Testcontainers.Containers.ExecResult result,
        string operation)
    {
        Assert.True(
            (result.ExitCode ?? -1) == 0,
            $"{operation} failed with exit code {result.ExitCode}: {result.Stderr}{Environment.NewLine}{result.Stdout}");
    }

    private static async Task AddLogsAsync(
        ICollection<string> sections,
        string label,
        IContainer container,
        CancellationToken cancellationToken)
    {
        try
        {
            var (stdout, stderr) = await container.GetLogsAsync(
                DateTime.UnixEpoch,
                DateTime.UtcNow,
                timestampsEnabled: false,
                cancellationToken);
            sections.Add(
                $"{label} stdout:{Environment.NewLine}{stdout}{Environment.NewLine}{label} stderr:{Environment.NewLine}{stderr}");
        }
        catch (Exception exception)
        {
            sections.Add($"{label} diagnostics unavailable: {exception.Message}");
        }
    }

    private async Task AddNestedManagerAgentLogsAsync(
        ICollection<string> sections,
        CancellationToken cancellationToken)
    {
        try
        {
            var result = await manager.ExecAsync(
                ["docker", "logs", ManagerAgentContainerName],
                cancellationToken);
            if ((result.ExitCode ?? -1) == 0)
            {
                sections.Add(
                    $"Manager Agent stdout:{Environment.NewLine}{result.Stdout}{Environment.NewLine}Manager Agent stderr:{Environment.NewLine}{result.Stderr}");
            }
        }
        catch (Exception exception)
        {
            sections.Add($"Manager Agent diagnostics unavailable: {exception.Message}");
        }
    }

    private async Task AddNestedDockerCommandDiagnosticsAsync(
        ICollection<string> sections,
        string label,
        IList<string> command,
        CancellationToken cancellationToken)
    {
        try
        {
            var result = await manager.ExecAsync(command, cancellationToken);
            sections.Add(
                $"{label} (exit code {result.ExitCode}):{Environment.NewLine}{result.Stdout}{Environment.NewLine}{result.Stderr}");
        }
        catch (Exception exception)
        {
            sections.Add($"{label} diagnostics unavailable: {exception.Message}");
        }
    }

    private async Task ImportCandidateImageAsync(
        string sourceImage,
        string registryImage,
        CancellationToken cancellationToken)
    {
        using var save = StartProcess(
            "docker",
            ["image", "save", sourceImage],
            redirectStandardInput: false);
        using var load = StartProcess(
            "docker",
            ["exec", "--interactive", manager.Id, "docker", "image", "load"],
            redirectStandardInput: true);
        var saveErrorTask = save.StandardError.ReadToEndAsync(cancellationToken);
        var loadOutputTask = load.StandardOutput.ReadToEndAsync(cancellationToken);
        var loadErrorTask = load.StandardError.ReadToEndAsync(cancellationToken);

        try
        {
            await save.StandardOutput.BaseStream.CopyToAsync(
                load.StandardInput.BaseStream,
                cancellationToken);
            load.StandardInput.Close();
            await Task.WhenAll(
                save.WaitForExitAsync(cancellationToken),
                load.WaitForExitAsync(cancellationToken));
        }
        catch
        {
            load.StandardInput.Close();
            await StopProcessAsync(save);
            await StopProcessAsync(load);
            throw;
        }

        var saveError = await saveErrorTask;
        var loadOutput = await loadOutputTask;
        var loadError = await loadErrorTask;
        Assert.True(
            save.ExitCode == 0 && load.ExitCode == 0,
            $"""
            Importing candidate image {sourceImage} into the Swarm manager failed.
            docker save exit code: {save.ExitCode}
            docker save stderr: {saveError}
            docker load exit code: {load.ExitCode}
            docker load stdout: {loadOutput}
            docker load stderr: {loadError}
            """);

        var tag = await manager.ExecAsync(
            ["docker", "image", "tag", sourceImage, registryImage],
            cancellationToken);
        AssertCommandSucceeded(tag, $"Tagging candidate image {sourceImage}");
        var push = await manager.ExecAsync(
            ["docker", "image", "push", registryImage],
            cancellationToken);
        AssertCommandSucceeded(push, $"Pushing candidate image {registryImage}");
    }

    private static Process StartProcess(
        string fileName,
        IReadOnlyList<string> arguments,
        bool redirectStandardInput)
    {
        var startInfo = new ProcessStartInfo(fileName)
        {
            UseShellExecute = false,
            CreateNoWindow = true,
            RedirectStandardInput = redirectStandardInput,
            RedirectStandardOutput = true,
            RedirectStandardError = true
        };
        foreach (var argument in arguments)
            startInfo.ArgumentList.Add(argument);

        return Process.Start(startInfo)
            ?? throw new InvalidOperationException($"Failed to start {fileName}.");
    }

    private static async Task RunProcessAsync(
        string fileName,
        IReadOnlyList<string> arguments,
        bool throwOnError,
        CancellationToken cancellationToken)
    {
        using var process = StartProcess(
            fileName,
            arguments,
            redirectStandardInput: false);
        var outputTask = process.StandardOutput.ReadToEndAsync(cancellationToken);
        var errorTask = process.StandardError.ReadToEndAsync(cancellationToken);
        try
        {
            await process.WaitForExitAsync(cancellationToken);
        }
        catch
        {
            await StopProcessAsync(process);
            throw;
        }

        var output = await outputTask;
        var error = await errorTask;
        if (throwOnError)
        {
            Assert.True(
                process.ExitCode == 0,
                $"{fileName} {string.Join(' ', arguments)} failed with exit code {process.ExitCode}: {error}{Environment.NewLine}{output}");
        }
    }

    private static async Task StopProcessAsync(Process process)
    {
        if (process.HasExited)
            return;

        process.Kill(entireProcessTree: true);
        await process.WaitForExitAsync(CancellationToken.None);
    }
}
