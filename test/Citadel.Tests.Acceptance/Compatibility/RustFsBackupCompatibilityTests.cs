using DotNet.Testcontainers.Configurations;
using Npgsql;
using System.Net.Http.Json;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Tests.Acceptance.Infrastructure;

namespace Tests.Acceptance.Compatibility;

[Collection("AcceptancePostgres")]
public sealed class RustFsBackupCompatibilityTests(
    AcceptancePostgresFixture postgres)
{
    public static bool HasRequiredCandidateImage =>
        CandidateImageTestEnvironment.ShouldRun(
            "CITADEL_ACCEPTANCE_CORE_IMAGE");

    [Fact(
        Skip = "Set CITADEL_ACCEPTANCE_CORE_IMAGE to run the real Core and RustFS compatibility test.",
        SkipUnless = nameof(HasRequiredCandidateImage))]
    public async Task S3Repository_ShouldBackupRestoreAndIsolateItsPrefix()
    {
        var coreImage = CandidateImageTestEnvironment.GetRequiredImage(
            "CITADEL_ACCEPTANCE_CORE_IMAGE");
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
                await RustFsBackupCompatibilityEnvironment.StartAsync(
                    coreImage,
                    databaseConnection.ConnectionString,
                    cancellationToken);
            await environment.AuthenticateAsAdminAsync(cancellationToken);

            var bucket = $"citadel-acceptance-{Guid.NewGuid():N}";
            var repositoryPrefix = $"suite/{Guid.NewGuid():N}/repository";
            var sentinelPrefix = $"suite/{Guid.NewGuid():N}/sentinel";
            var sentinelObject = $"{sentinelPrefix}/do-not-delete.txt";
            await environment.CreateBucketAsync(
                bucket,
                cancellationToken);
            await environment.PutObjectAsync(
                bucket,
                sentinelObject,
                Encoding.UTF8.GetBytes("prefix-isolation"),
                cancellationToken);

            var platformId = await CreateLocalPlatformAsync(
                environment,
                cancellationToken);
            var passwordSecretId = await CreateSecretAsync(
                environment,
                "RUSTFS_RESTIC_PASSWORD",
                "restic-acceptance-password",
                cancellationToken);
            var accessKeySecretId = await CreateSecretAsync(
                environment,
                "RUSTFS_ACCESS_KEY",
                RustFsBackupCompatibilityEnvironment.AccessKey,
                cancellationToken);
            var secretKeySecretId = await CreateSecretAsync(
                environment,
                "RUSTFS_SECRET_KEY",
                RustFsBackupCompatibilityEnvironment.SecretKey,
                cancellationToken);

            var repositoryId = await CreateRepositoryAsync(
                environment,
                "ready",
                bucket,
                repositoryPrefix,
                passwordSecretId,
                accessKeySecretId,
                secretKeySecretId,
                cancellationToken);
            await InitializeRepositoryAsync(
                environment,
                repositoryId,
                platformId,
                cancellationToken);
            await AssertRepositoryStatusAsync(
                environment,
                repositoryId,
                platformId,
                expectedStatus: "Ready",
                cancellationToken);

            await AssertInvalidCredentialsAreRejectedAsync(
                environment,
                bucket,
                platformId,
                passwordSecretId,
                secretKeySecretId,
                cancellationToken);
            await AssertMissingBucketIsRejectedAsync(
                environment,
                platformId,
                passwordSecretId,
                accessKeySecretId,
                secretKeySecretId,
                cancellationToken);

            var sourceVolume = $"citadel-acceptance-source-{Guid.NewGuid():N}";
            var restoredVolume = $"citadel-acceptance-restored-{Guid.NewGuid():N}";
            var originalContent = BuildPayload("first-snapshot");
            await environment.SeedVolumeAsync(
                sourceVolume,
                originalContent,
                cancellationToken);

            var policyId = await CreatePolicyAsync(
                environment,
                platformId,
                sourceVolume,
                repositoryId,
                cancellationToken);
            var firstRunId = await RunBackupAsync(
                environment,
                policyId,
                cancellationToken);

            await environment.SeedVolumeAsync(
                sourceVolume,
                BuildPayload("source-was-altered"),
                cancellationToken);
            await RestoreVolumeAsync(
                environment,
                firstRunId,
                platformId,
                restoredVolume,
                cancellationToken);

            var restoredContent = await environment.ReadVolumePayloadAsync(
                restoredVolume,
                cancellationToken);
            Assert.Equal(originalContent.LongLength, restoredContent.LongLength);
            Assert.Equal(
                Convert.ToHexString(SHA256.HashData(originalContent)),
                Convert.ToHexString(SHA256.HashData(restoredContent)));

            await environment.SeedVolumeAsync(
                sourceVolume,
                BuildPayload("second-snapshot"),
                cancellationToken);
            await RunBackupAsync(
                environment,
                policyId,
                cancellationToken);
            await PruneRepositoryAsync(
                environment,
                repositoryId,
                platformId,
                cancellationToken);

            Assert.NotEmpty(
                await environment.ListObjectNamesAsync(
                    bucket,
                    repositoryPrefix,
                    cancellationToken));
            Assert.Contains(
                sentinelObject,
                await environment.ListObjectNamesAsync(
                    bucket,
                    sentinelPrefix,
                    cancellationToken));
        }
        finally
        {
            await postgres.DropDatabaseAsync(
                hostConnectionString,
                CancellationToken.None);
        }
    }

    private static byte[] BuildPayload(string marker)
    {
        var line = $"{marker}|citadel-rustfs-acceptance|";
        return Encoding.UTF8.GetBytes(
            string.Concat(Enumerable.Repeat(line, 256)));
    }

    private static async Task<Guid> CreateLocalPlatformAsync(
        RustFsBackupCompatibilityEnvironment environment,
        CancellationToken cancellationToken)
    {
        using var response = await environment.Client.PostAsJsonAsync(
            "/api/v1/platforms",
            new
            {
                name = $"acceptance-rustfs-platform-{Guid.NewGuid():N}",
                address = (string?)null,
                description = "Disposable RustFS backup platform",
                type = "Docker",
                connectorType = "Local",
                tagIds = Array.Empty<Guid>()
            },
            cancellationToken);
        return await ReadIdAsync(
            response,
            "Creating the local backup platform",
            cancellationToken);
    }

    private static async Task<Guid> CreateSecretAsync(
        RustFsBackupCompatibilityEnvironment environment,
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
            response,
            $"Creating {name} secret",
            cancellationToken);
    }

    private static async Task<Guid> CreateRepositoryAsync(
        RustFsBackupCompatibilityEnvironment environment,
        string suffix,
        string bucket,
        string prefix,
        Guid passwordSecretId,
        Guid accessKeySecretId,
        Guid secretKeySecretId,
        CancellationToken cancellationToken)
    {
        using var response = await environment.Client.PostAsJsonAsync(
            "/api/v1/backupRepositories",
            new
            {
                name = $"acceptance-rustfs-{suffix}-{Guid.NewGuid():N}",
                description = "Disposable RustFS backup repository",
                spec = new Dictionary<string, object?>
                {
                    ["$type"] = "S3Compatible",
                    ["endpoint"] = environment.PlatformS3Endpoint,
                    ["bucket"] = bucket,
                    ["prefix"] = prefix,
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
            response,
            $"Creating the {suffix} RustFS repository",
            cancellationToken);
    }

    private static async Task InitializeRepositoryAsync(
        RustFsBackupCompatibilityEnvironment environment,
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
            "Initializing the RustFS repository",
            cancellationToken);
    }

    private static async Task AssertRepositoryStatusAsync(
        RustFsBackupCompatibilityEnvironment environment,
        Guid repositoryId,
        Guid platformId,
        string expectedStatus,
        CancellationToken cancellationToken)
    {
        using var response = await environment.Client.PostAsJsonAsync(
            $"/api/v1/backupRepositories/{repositoryId:D}/validate",
            new
            {
                location = "Platform",
                platformId
            },
            cancellationToken);
        var body = await AssertSuccessAsync(
            environment,
            response,
            "Validating the RustFS repository",
            cancellationToken);
        using var json = JsonDocument.Parse(body);
        Assert.Equal(
            expectedStatus,
            json.RootElement.GetProperty("status").GetString());
    }

    private static async Task AssertInvalidCredentialsAreRejectedAsync(
        RustFsBackupCompatibilityEnvironment environment,
        string bucket,
        Guid platformId,
        Guid passwordSecretId,
        Guid secretKeySecretId,
        CancellationToken cancellationToken)
    {
        var invalidAccessKeyId = await CreateSecretAsync(
            environment,
            "RUSTFS_INVALID_ACCESS_KEY",
            "invalid-access-key",
            cancellationToken);
        var repositoryId = await CreateRepositoryAsync(
            environment,
            "invalid-credentials",
            bucket,
            $"negative/{Guid.NewGuid():N}",
            passwordSecretId,
            invalidAccessKeyId,
            secretKeySecretId,
            cancellationToken);

        using var response = await environment.Client.PostAsJsonAsync(
            $"/api/v1/backupRepositories/{repositoryId:D}/validate",
            new
            {
                location = "Platform",
                platformId
            },
            cancellationToken);
        var body = await AssertSuccessAsync(
            environment,
            response,
            "Validating invalid RustFS credentials",
            cancellationToken);
        using var json = JsonDocument.Parse(body);
        Assert.NotEqual(
            "Ready",
            json.RootElement.GetProperty("status").GetString());
        Assert.False(
            string.IsNullOrWhiteSpace(
                json.RootElement
                    .GetProperty("lastErrorMessage")
                    .GetString()));
    }

    private static async Task AssertMissingBucketIsRejectedAsync(
        RustFsBackupCompatibilityEnvironment environment,
        Guid platformId,
        Guid passwordSecretId,
        Guid accessKeySecretId,
        Guid secretKeySecretId,
        CancellationToken cancellationToken)
    {
        var repositoryId = await CreateRepositoryAsync(
            environment,
            "missing-bucket",
            $"missing-{Guid.NewGuid():N}",
            $"negative/{Guid.NewGuid():N}",
            passwordSecretId,
            accessKeySecretId,
            secretKeySecretId,
            cancellationToken);

        using var response = await environment.Client.PostAsJsonAsync(
            $"/api/v1/backupRepositories/{repositoryId:D}/validate",
            new
            {
                location = "Platform",
                platformId
            },
            cancellationToken);
        var body = await AssertSuccessAsync(
            environment,
            response,
            "Validating a missing RustFS bucket",
            cancellationToken);
        using var json = JsonDocument.Parse(body);
        Assert.NotEqual(
            "Ready",
            json.RootElement.GetProperty("status").GetString());
        Assert.False(
            string.IsNullOrWhiteSpace(
                json.RootElement
                    .GetProperty("lastErrorMessage")
                    .GetString()));
    }

    private static async Task<Guid> CreatePolicyAsync(
        RustFsBackupCompatibilityEnvironment environment,
        Guid platformId,
        string volumeName,
        Guid repositoryId,
        CancellationToken cancellationToken)
    {
        using var response = await environment.Client.PostAsJsonAsync(
            "/api/v1/backupPolicies",
            new
            {
                name = $"acceptance-rustfs-policy-{Guid.NewGuid():N}",
                description = "Disposable RustFS volume backup policy",
                source = new Dictionary<string, object?>
                {
                    ["$type"] = "DockerVolume",
                    ["platformId"] = platformId,
                    ["volumeName"] = volumeName,
                    ["consistency"] = "Live"
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
            response,
            "Creating the RustFS backup policy",
            cancellationToken);
    }

    private static async Task<Guid> RunBackupAsync(
        RustFsBackupCompatibilityEnvironment environment,
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
            "Running the RustFS volume backup",
            cancellationToken);
        using var json = JsonDocument.Parse(body);
        var items = json.RootElement.EnumerateArray().ToArray();
        Assert.NotEmpty(items);
        var runId = items[0].GetProperty("runId").GetGuid();
        Assert.Contains(
            items,
            item =>
                item.TryGetProperty("status", out var status)
                && status.ValueKind == JsonValueKind.String
                && status.GetString() == "Succeeded");
        return runId;
    }

    private static async Task RestoreVolumeAsync(
        RustFsBackupCompatibilityEnvironment environment,
        Guid backupRunId,
        Guid platformId,
        string targetVolume,
        CancellationToken cancellationToken)
    {
        using var response = await environment.Client.PostAsJsonAsync(
            $"/api/v1/backupRuns/{backupRunId:D}/restoreVolume/run",
            new
            {
                targetPlatformId = platformId,
                targetVolumeName = targetVolume,
                overwriteExisting = false
            },
            cancellationToken);
        var body = await AssertSuccessAsync(
            environment,
            response,
            "Restoring the RustFS volume snapshot",
            cancellationToken);
        using var json = JsonDocument.Parse(body);
        var items = json.RootElement.EnumerateArray().ToArray();
        Assert.NotEmpty(items);
        Assert.Contains(
            items,
            item =>
                item.TryGetProperty("status", out var status)
                && status.ValueKind == JsonValueKind.String
                && status.GetString() == "Succeeded");
    }

    private static async Task PruneRepositoryAsync(
        RustFsBackupCompatibilityEnvironment environment,
        Guid repositoryId,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        using var response = await environment.Client.PostAsJsonAsync(
            $"/api/v1/backupRepositories/{repositoryId:D}/prune",
            new
            {
                location = "Platform",
                platformId
            },
            cancellationToken);
        await AssertSuccessAsync(
            environment,
            response,
            "Pruning the RustFS repository",
            cancellationToken);
    }

    private static async Task<Guid> ReadIdAsync(
        HttpResponseMessage response,
        string operation,
        CancellationToken cancellationToken)
    {
        var body = await response.Content.ReadAsStringAsync(
            cancellationToken);
        Assert.True(
            response.IsSuccessStatusCode,
            $"{operation} failed with HTTP {(int)response.StatusCode}: {body}");
        using var json = JsonDocument.Parse(body);
        return json.RootElement.GetProperty("id").GetGuid();
    }

    private static async Task<string> AssertSuccessAsync(
        RustFsBackupCompatibilityEnvironment environment,
        HttpResponseMessage response,
        string operation,
        CancellationToken cancellationToken)
    {
        var body = await response.Content.ReadAsStringAsync(
            cancellationToken);
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
