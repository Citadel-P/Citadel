using Npgsql;
using System.Net.Http.Json;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Tests.Acceptance.Infrastructure;

namespace Tests.Acceptance.Compatibility;

[Collection("AcceptancePostgres")]
public sealed class SwarmBackupCompatibilityTests(
    AcceptancePostgresFixture postgres)
{
    public static bool HasRequiredCandidateImages =>
        CandidateImageTestEnvironment.ShouldRun(
            "CITADEL_ACCEPTANCE_CORE_IMAGE",
            "CITADEL_ACCEPTANCE_AGENT_IMAGE");

    [Fact(
        Skip = "Set CITADEL_ACCEPTANCE_CORE_IMAGE and CITADEL_ACCEPTANCE_AGENT_IMAGE to run the real multi-node Swarm backup test.",
        SkipUnless = nameof(HasRequiredCandidateImages))]
    public async Task WorkerVolume_ShouldBackupToRustFsAndRestoreOnAnotherNode()
    {
        var coreImage = CandidateImageTestEnvironment.GetRequiredImage(
            "CITADEL_ACCEPTANCE_CORE_IMAGE");
        var agentImage = CandidateImageTestEnvironment.GetRequiredImage(
            "CITADEL_ACCEPTANCE_AGENT_IMAGE");
        var cancellationToken = TestContext.Current.CancellationToken;
        var hostConnectionString =
            await postgres.CreateDatabaseAsync(cancellationToken);

        try
        {
            var databaseConnection =
                new NpgsqlConnectionStringBuilder(hostConnectionString);
            databaseConnection.Host =
                SwarmCompatibilityEnvironment.PostgresNetworkAlias;
            databaseConnection.Port = 5432;

            await using var environment =
                await SwarmCompatibilityEnvironment.StartWithBackupAsync(
                    coreImage,
                    agentImage,
                    databaseConnection.ConnectionString,
                    postgres.ContainerId,
                    cancellationToken);
            await environment.AuthenticateAsAdminAsync(cancellationToken);

            var platformId = await environment.CreatePlatformAsync(
                SwarmManagerConnectorMode.Local,
                cancellationToken);
            await environment.WaitForSwarmNodesAsync(platformId, 3, cancellationToken);
            await environment.WaitForCoverageAsync(
                platformId,
                static coverage =>
                    coverage.GetProperty("totalNodes").GetInt32() == 3
                    && coverage.GetProperty("eligibleNodes").GetInt32() == 3,
                "the three Swarm Nodes to appear",
                cancellationToken);
            await environment.InstallNodeAgentsAsync(platformId, cancellationToken);
            await environment.WaitForCoverageAsync(
                platformId,
                static coverage =>
                    coverage.GetProperty("state").GetString() == "Complete"
                    && coverage.GetProperty("connectedNodes").GetInt32() == 3,
                "complete node-agent coverage",
                cancellationToken);

            var bucket = $"citadel-swarm-{Guid.NewGuid():N}";
            await environment.CreateBackupBucketAsync(bucket, cancellationToken);
            var passwordSecretId = await CreateSecretAsync(
                environment,
                "SWARM_RESTIC_PASSWORD",
                "swarm-restic-acceptance-password",
                cancellationToken);
            var accessKeySecretId = await CreateSecretAsync(
                environment,
                "SWARM_RUSTFS_ACCESS_KEY",
                SwarmCompatibilityEnvironment.RustFsAccessKey,
                cancellationToken);
            var secretKeySecretId = await CreateSecretAsync(
                environment,
                "SWARM_RUSTFS_SECRET_KEY",
                SwarmCompatibilityEnvironment.RustFsSecretKey,
                cancellationToken);
            var repositoryId = await CreateRepositoryAsync(
                environment,
                bucket,
                passwordSecretId,
                accessKeySecretId,
                secretKeySecretId,
                cancellationToken);
            await InitializeRepositoryAsync(
                environment,
                repositoryId,
                platformId,
                cancellationToken);

            var sourceVolume = $"citadel-swarm-source-{Guid.NewGuid():N}";
            var restoredVolume = $"citadel-swarm-restored-{Guid.NewGuid():N}";
            var originalContent = Encoding.UTF8.GetBytes(
                string.Concat(Enumerable.Repeat("citadel-swarm-backup|", 512)));
            await environment.SeedWorkerVolumeAsync(
                1,
                sourceVolume,
                originalContent,
                cancellationToken);
            await environment.WaitForNodeVolumeAsync(
                platformId,
                environment.WorkerOneNodeId,
                sourceVolume,
                cancellationToken);

            var policyId = await CreatePolicyAsync(
                environment,
                platformId,
                environment.WorkerOneNodeId,
                sourceVolume,
                repositoryId,
                cancellationToken);
            var backupRunId = await RunBackupAsync(
                environment,
                policyId,
                cancellationToken);
            await AssertBackupNodeAsync(
                environment,
                backupRunId,
                environment.WorkerOneNodeId,
                sourceVolume,
                cancellationToken);

            await RestoreVolumeAsync(
                environment,
                backupRunId,
                platformId,
                environment.WorkerTwoNodeId,
                restoredVolume,
                cancellationToken);
            var restoredContent = await environment.ReadWorkerVolumePayloadAsync(
                2,
                restoredVolume,
                cancellationToken);

            Assert.Equal(
                Convert.ToHexString(SHA256.HashData(originalContent)),
                Convert.ToHexString(SHA256.HashData(restoredContent)));
            await environment.AssertNoBackupHelpersAsync(1, cancellationToken);
            await environment.AssertNoBackupHelpersAsync(2, cancellationToken);
        }
        catch (Exception testFailure)
        {
            try
            {
                await postgres.DropDatabaseAsync(
                    hostConnectionString,
                    CancellationToken.None);
            }
            catch (Exception cleanupFailure)
            {
                throw new AggregateException(
                    "The Swarm backup test failed and its temporary database could not be removed.",
                    testFailure,
                    cleanupFailure);
            }

            throw;
        }

        await postgres.DropDatabaseAsync(
            hostConnectionString,
            CancellationToken.None);
    }

    private static async Task<Guid> CreateSecretAsync(
        SwarmCompatibilityEnvironment environment,
        string name,
        string value,
        CancellationToken cancellationToken)
    {
        using var response = await environment.Client.PostAsJsonAsync(
            "/api/v1/resourceBindings/secrets",
            new
            {
                name = $"{name}_{Guid.NewGuid():N}".ToUpperInvariant(),
                value
            },
            cancellationToken);
        return await ReadIdAsync(
            environment,
            response,
            $"Creating {name} secret",
            cancellationToken);
    }

    private static async Task<Guid> CreateRepositoryAsync(
        SwarmCompatibilityEnvironment environment,
        string bucket,
        Guid passwordSecretId,
        Guid accessKeySecretId,
        Guid secretKeySecretId,
        CancellationToken cancellationToken)
    {
        Assert.NotNull(environment.PlatformS3Endpoint);
        using var response = await environment.Client.PostAsJsonAsync(
            "/api/v1/backupRepositories",
            new
            {
                name = $"acceptance-swarm-rustfs-{Guid.NewGuid():N}",
                description = "Disposable multi-node Swarm backup repository",
                spec = new Dictionary<string, object?>
                {
                    ["$type"] = "S3Compatible",
                    ["endpoint"] = environment.PlatformS3Endpoint,
                    ["bucket"] = bucket,
                    ["prefix"] = $"swarm/{Guid.NewGuid():N}",
                    ["region"] = "us-east-1",
                    ["bucketLookup"] = "Path",
                    ["accessKeySecretId"] = accessKeySecretId,
                    ["secretKeySecretId"] = secretKeySecretId,
                    ["sessionTokenSecretId"] = null,
                    ["allowInsecureHttp"] = true
                },
                passwordSecretId
            },
            cancellationToken);
        return await ReadIdAsync(
            environment,
            response,
            "Creating the Swarm RustFS repository",
            cancellationToken);
    }

    private static async Task InitializeRepositoryAsync(
        SwarmCompatibilityEnvironment environment,
        Guid repositoryId,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        using var response = await environment.Client.PostAsJsonAsync(
            $"/api/v1/backupRepositories/{repositoryId:D}/initialize",
            new
            {
                location = "Platform",
                platformId
            },
            cancellationToken);
        await AssertSuccessAsync(
            environment,
            response,
            "Initializing the Swarm RustFS repository",
            cancellationToken);
    }

    private static async Task<Guid> CreatePolicyAsync(
        SwarmCompatibilityEnvironment environment,
        Guid platformId,
        string dockerNodeId,
        string volumeName,
        Guid repositoryId,
        CancellationToken cancellationToken)
    {
        using var response = await environment.Client.PostAsJsonAsync(
            "/api/v1/backupPolicies",
            new
            {
                name = $"acceptance-swarm-policy-{Guid.NewGuid():N}",
                description = "Disposable worker-node Volume backup policy",
                source = new Dictionary<string, object?>
                {
                    ["$type"] = "DockerVolume",
                    ["platformId"] = platformId,
                    ["volumeName"] = volumeName,
                    ["consistency"] = "Live",
                    ["dockerNodeId"] = dockerNodeId
                },
                backupRepositoryId = repositoryId,
                enabled = true,
                cron = (string?)null,
                timeZone = (string?)null,
                webhook = (object?)null,
                keepLastSuccessful = 1,
                timeoutSeconds = 180,
                alertOnFailure = false,
                tagIds = Array.Empty<Guid>()
            },
            cancellationToken);
        return await ReadIdAsync(
            environment,
            response,
            "Creating the worker-node backup policy",
            cancellationToken);
    }

    private static async Task<Guid> RunBackupAsync(
        SwarmCompatibilityEnvironment environment,
        Guid policyId,
        CancellationToken cancellationToken)
    {
        using var response = await environment.Client.PostAsJsonAsync(
            $"/api/v1/backupPolicies/{policyId:D}/run",
            new
            {
                trigger = "Manual",
                triggerSourceId = (Guid?)null
            },
            cancellationToken);
        var body = await AssertSuccessAsync(
            environment,
            response,
            "Running the worker-node backup",
            cancellationToken);
        using var json = JsonDocument.Parse(body);
        var items = json.RootElement.EnumerateArray().ToArray();
        Assert.NotEmpty(items);
        Assert.Contains(
            items,
            static item => item.TryGetProperty("status", out var status)
                           && status.GetString() == "Succeeded");
        return items[0].GetProperty("runId").GetGuid();
    }

    private static async Task AssertBackupNodeAsync(
        SwarmCompatibilityEnvironment environment,
        Guid runId,
        string dockerNodeId,
        string volumeName,
        CancellationToken cancellationToken)
    {
        using var response = await environment.Client.GetAsync(
            $"/api/v1/backupRuns/{runId:D}",
            cancellationToken);
        var body = await AssertSuccessAsync(
            environment,
            response,
            "Reading the completed worker-node backup",
            cancellationToken);
        using var json = JsonDocument.Parse(body);
        var run = json.RootElement;
        Assert.Equal("Succeeded", run.GetProperty("status").GetString());
        Assert.False(string.IsNullOrWhiteSpace(run.GetProperty("resticSnapshotId").GetString()));
        var item = Assert.Single(run.GetProperty("items").EnumerateArray());
        Assert.Equal(dockerNodeId, item.GetProperty("dockerNodeId").GetString());
        Assert.Equal(volumeName, item.GetProperty("volumeName").GetString());
        Assert.Equal("Succeeded", item.GetProperty("status").GetString());
    }

    private static async Task RestoreVolumeAsync(
        SwarmCompatibilityEnvironment environment,
        Guid backupRunId,
        Guid platformId,
        string targetDockerNodeId,
        string targetVolume,
        CancellationToken cancellationToken)
    {
        using var response = await environment.Client.PostAsJsonAsync(
            $"/api/v1/backupRuns/{backupRunId:D}/restoreVolume/run",
            new
            {
                targetPlatformId = platformId,
                targetVolumeName = targetVolume,
                overwriteExisting = false,
                targetDockerNodeId
            },
            cancellationToken);
        var body = await AssertSuccessAsync(
            environment,
            response,
            "Restoring the worker-node backup on another Node",
            cancellationToken);
        using var json = JsonDocument.Parse(body);
        var items = json.RootElement.EnumerateArray().ToArray();
        Assert.NotEmpty(items);
        Assert.Contains(
            items,
            static item => item.TryGetProperty("status", out var status)
                           && status.GetString() == "Succeeded");
    }

    private static async Task<Guid> ReadIdAsync(
        SwarmCompatibilityEnvironment environment,
        HttpResponseMessage response,
        string operation,
        CancellationToken cancellationToken)
    {
        var body = await AssertSuccessAsync(
            environment,
            response,
            operation,
            cancellationToken);
        using var json = JsonDocument.Parse(body);
        return json.RootElement.GetProperty("id").GetGuid();
    }

    private static async Task<string> AssertSuccessAsync(
        SwarmCompatibilityEnvironment environment,
        HttpResponseMessage response,
        string operation,
        CancellationToken cancellationToken)
    {
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        if (!response.IsSuccessStatusCode)
        {
            Assert.Fail(
                $"""
                {operation} failed with HTTP {(int)response.StatusCode}: {body}
                {await environment.GetDiagnosticsAsync(CancellationToken.None)}
                """);
        }

        return body;
    }
}
