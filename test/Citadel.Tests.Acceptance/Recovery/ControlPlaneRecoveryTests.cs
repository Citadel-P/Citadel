using Domain;
using Domain.Entities.Backups;
using Domain.Entities.Platforms;
using Npgsql;
using System.IO.Compression;
using System.Net.Http.Json;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using Tests.Acceptance.Infrastructure;

namespace Tests.Acceptance.Recovery;

[Collection("AcceptancePostgres")]
public sealed class ControlPlaneRecoveryTests(AcceptancePostgresFixture postgres)
{
    private static readonly Guid PlatformId =
        Guid.Parse("019f1000-0000-7000-8000-000000000001");
    private static readonly Guid GitRepositoryId =
        Guid.Parse("019f1000-0000-7000-8000-000000000002");
    private static readonly Guid EdgeAgentBindingId =
        Guid.Parse("019f1000-0000-7000-8000-000000000003");
    private static readonly Guid BackupRepositoryId =
        Guid.Parse("019f1000-0000-7000-8000-000000000004");
    private static readonly Guid BackupPolicyId =
        Guid.Parse("019f1000-0000-7000-8000-000000000005");
    private static readonly Guid BackupRunId =
        Guid.Parse("019f1000-0000-7000-8000-000000000006");
    private static readonly Guid ActivityId =
        Guid.Parse("019f1000-0000-7000-8000-000000000007");

    [Fact]
    public async Task Candidate_ShouldRestoreControlPlaneAndOperateInCleanEnvironment()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var root = CreateTestRoot();
        var sourceWorkingDirectory = Path.Combine(root, "source");
        var targetWorkingDirectory = Path.Combine(root, "target");
        var archivePath = Path.Combine(root, "citadel-control-plane.zip");
        Directory.CreateDirectory(sourceWorkingDirectory);

        try
        {
            var sourceConnectionString =
                await postgres.CreateDatabaseAsync(cancellationToken);

            await StartAndStopCandidateAsync(
                sourceConnectionString,
                sourceWorkingDirectory,
                cancellationToken);
            var administratorActorId = await GetInitialAdministratorActorIdAsync(
                sourceConnectionString,
                cancellationToken);
            await SeedInfrastructureStateAsync(
                sourceConnectionString,
                cancellationToken);

            Guid secretId;
            Guid stackId;
            JsonElement sourceLicense;
            await using (var sourceCandidate =
                await CandidateApplicationProcess.StartAsync(
                    sourceConnectionString,
                    sourceWorkingDirectory,
                    cancellationToken,
                    CandidateApplicationOptions.FileBackedRecoveryAssets))
            {
                await sourceCandidate.AuthenticateAsAdminAsync(cancellationToken);
                secretId = await CreateSecretAsync(
                    sourceCandidate,
                    cancellationToken);
                stackId = await CreateStackAsync(
                    sourceCandidate,
                    cancellationToken);
                await CreateTagAsync(
                    sourceCandidate,
                    "Before recovery",
                    cancellationToken);
                sourceLicense = await ReadLicenseAsync(
                    sourceCandidate,
                    cancellationToken);
            }

            await SeedBackupStateAsync(
                sourceConnectionString,
                secretId,
                administratorActorId,
                cancellationToken);

            var sourceDataDirectory =
                Path.Combine(sourceWorkingDirectory, "data");
            await ControlPlaneRecoveryArchive.CreateAsync(
                postgres,
                sourceConnectionString,
                sourceDataDirectory,
                archivePath,
                cancellationToken);

            var originalSecretKeyHash = HashFile(
                Path.Combine(
                    sourceDataDirectory,
                    "secret-encryption-key"));
            var originalHubKeyHash = HashFile(
                Path.Combine(
                    sourceDataDirectory,
                    "keys",
                    "id_ed25519"));

            await postgres.DropDatabaseAsync(
                sourceConnectionString,
                cancellationToken);
            Directory.Delete(sourceWorkingDirectory, recursive: true);

            var targetConnectionString =
                await postgres.CreateDatabaseAsync(cancellationToken);
            var targetDataDirectory =
                Path.Combine(targetWorkingDirectory, "data");
            await ControlPlaneRecoveryArchive.RestoreAsync(
                postgres,
                targetConnectionString,
                archivePath,
                targetDataDirectory,
                cancellationToken);

            Assert.Equal(
                originalSecretKeyHash,
                HashFile(Path.Combine(
                    targetDataDirectory,
                    "secret-encryption-key")));
            Assert.Equal(
                originalHubKeyHash,
                HashFile(Path.Combine(
                    targetDataDirectory,
                    "keys",
                    "id_ed25519")));

            await using (var restoredCandidate =
                await CandidateApplicationProcess.StartAsync(
                    targetConnectionString,
                    targetWorkingDirectory,
                    cancellationToken,
                    CandidateApplicationOptions.FileBackedRecoveryAssets))
            {
                await restoredCandidate.AuthenticateAsAdminAsync(
                    cancellationToken);
                await AssertTagExistsAsync(
                    restoredCandidate,
                    "Before recovery",
                    cancellationToken);
                Assert.Equal(
                    GetLicenseState(sourceLicense),
                    GetLicenseState(await ReadLicenseAsync(
                        restoredCandidate,
                        cancellationToken)));

                await CreateTagAsync(
                    restoredCandidate,
                    "After recovery",
                    cancellationToken);
            }

            await AssertRestoredStateAsync(
                targetConnectionString,
                secretId,
                stackId,
                administratorActorId,
                targetDataDirectory,
                cancellationToken);
        }
        finally
        {
            DeleteDirectory(root);
        }
    }

    [Fact]
    public async Task Restore_ShouldRejectDatabaseArchiveWithoutSecretEncryptionKey()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var root = CreateTestRoot();
        var sourceWorkingDirectory = Path.Combine(root, "source");
        var archivePath = Path.Combine(root, "citadel-control-plane.zip");
        var incompleteArchivePath =
            Path.Combine(root, "citadel-control-plane-missing-key.zip");
        Directory.CreateDirectory(sourceWorkingDirectory);

        try
        {
            var sourceConnectionString =
                await postgres.CreateDatabaseAsync(cancellationToken);
            await StartAndStopCandidateAsync(
                sourceConnectionString,
                sourceWorkingDirectory,
                cancellationToken);
            await ControlPlaneRecoveryArchive.CreateAsync(
                postgres,
                sourceConnectionString,
                Path.Combine(sourceWorkingDirectory, "data"),
                archivePath,
                cancellationToken);
            CreateArchiveWithoutEntry(
                archivePath,
                incompleteArchivePath,
                "recovery/secret-encryption-key");

            await postgres.DropDatabaseAsync(
                sourceConnectionString,
                cancellationToken);
            Directory.Delete(sourceWorkingDirectory, recursive: true);

            var targetConnectionString =
                await postgres.CreateDatabaseAsync(cancellationToken);
            var error = await Assert.ThrowsAsync<InvalidDataException>(
                () => ControlPlaneRecoveryArchive.RestoreAsync(
                    postgres,
                    targetConnectionString,
                    incompleteArchivePath,
                    Path.Combine(root, "target", "data"),
                    cancellationToken));

            Assert.Contains(
                "secret-encryption-key",
                error.Message,
                StringComparison.Ordinal);
            Assert.True(
                await AcceptancePostgresFixture.IsDatabaseEmptyAsync(
                    targetConnectionString,
                    cancellationToken),
                "An invalid recovery archive must be rejected before PostgreSQL is modified.");
        }
        finally
        {
            DeleteDirectory(root);
        }
    }

    private static async Task StartAndStopCandidateAsync(
        string connectionString,
        string workingDirectory,
        CancellationToken cancellationToken)
    {
        await using var candidate =
            await CandidateApplicationProcess.StartAsync(
                connectionString,
                workingDirectory,
                cancellationToken,
                CandidateApplicationOptions.FileBackedRecoveryAssets);
        await candidate.AuthenticateAsAdminAsync(cancellationToken);
        await ReadLicenseAsync(candidate, cancellationToken);
        var response = await candidate.Client.PostAsJsonAsync(
            "/api/v1/resourceBindings/secrets",
            new
            {
                name = "RECOVERY_BOOTSTRAP_SECRET",
                value = "bootstrap-recovery-secret"
            },
            cancellationToken);
        await AssertSuccessAsync(candidate, response, cancellationToken);
    }

    private static async Task SeedInfrastructureStateAsync(
        string connectionString,
        CancellationToken cancellationToken)
    {
        var descriptor = JsonSerializer.Serialize<PlatformDescriptor>(
            new DockerPlatformDescriptor(
                DaemonId: "recovery-daemon",
                ContainerCount: 2,
                ContainersRunning: 1,
                ContainersPaused: 0,
                ContainersStopped: 1,
                Driver: "overlay2",
                OperatingSystem: "Linux",
                OsVersion: "6.8",
                OsType: "linux",
                Architecture: "x86_64"),
            PlatformJsonContext.Default.PlatformDescriptor);

        await using var connection = new NpgsqlConnection(connectionString);
        await connection.OpenAsync(cancellationToken);
        await using var command = new NpgsqlCommand(
            """
            INSERT INTO platforms (
                id, address, agentversion, connectortype, cpucount,
                description, imagecount, memtotal, name, networkcount,
                platformdescriptor, serverversion, status, volumecount)
            VALUES (
                @platformId, 'http://127.0.0.1:1', '1.0.0', 'Agent', 4,
                'Recovery acceptance platform', 3, 8589934592,
                'Recovery platform', 1, @descriptor::json, '27.0.0',
                'Offline', 2);

            INSERT INTO gitrepositories (
                id, controlstate, createdat, createdbyactorid, defaultbranch,
                description, name, rowversion, status, syncintervalminutes,
                syncmode, url)
            VALUES (
                @gitRepositoryId, 'Idle', CURRENT_TIMESTAMP,
                '00000000-0000-0000-0000-000000000001', 'main',
                'Recovery acceptance repository', 'Recovery repository', 0,
                'Healthy', NULL, 'Manual',
                'https://example.invalid/citadel/recovery.git');

            INSERT INTO edgeagentbindings (
                id, agentfingerprint, agentid, agentpublickey,
                capabilitiesjson, connectionstatus, createdatutc,
                lastconnectedatutc, lastdisconnectedatutc,
                lastheartbeatatutc, lastseenhostname, lastseenversion,
                platformid, protocolversion, resourceid, resourcetype,
                updatedatutc)
            VALUES (
                @edgeAgentBindingId, 'sha256:recovery-agent',
                '019f1000-0000-7000-8000-000000000008',
                'cmVjb3ZlcnktYWdlbnQtcHVibGljLWtleQ==',
                '{"docker":true}'::json, 'Disconnected',
                CURRENT_TIMESTAMP - INTERVAL '1 day',
                CURRENT_TIMESTAMP - INTERVAL '1 hour',
                CURRENT_TIMESTAMP - INTERVAL '30 minutes',
                CURRENT_TIMESTAMP - INTERVAL '30 minutes',
                'recovery-agent', '1.0.0', @platformId, 1, @platformId,
                'Platform', CURRENT_TIMESTAMP);

            INSERT INTO activityevents (
                id, createdat, createdbyactorid, eventtype, info, platformid,
                resourceid, resourcename, resourcetype, status)
            VALUES (
                @activityId, CURRENT_TIMESTAMP,
                '00000000-0000-0000-0000-000000000001',
                'PlatformDisconnected', '{}'::json, @platformId, @platformId,
                'Recovery platform', 'Platform', 'Warning');
            """,
            connection);
        command.Parameters.AddWithValue("platformId", PlatformId);
        command.Parameters.AddWithValue("descriptor", descriptor);
        command.Parameters.AddWithValue("gitRepositoryId", GitRepositoryId);
        command.Parameters.AddWithValue(
            "edgeAgentBindingId",
            EdgeAgentBindingId);
        command.Parameters.AddWithValue("activityId", ActivityId);
        await command.ExecuteNonQueryAsync(cancellationToken);
    }

    private static async Task SeedBackupStateAsync(
        string connectionString,
        Guid secretId,
        Guid administratorActorId,
        CancellationToken cancellationToken)
    {
        var repositorySpec = SerializeWithType(
            "FileSystem",
            JsonSerializer.SerializeToNode(
                new FileSystemBackupRepositorySpec(
                    BackupExecutionLocation.Core,
                    PlatformId: null,
                    Path: "/tmp/citadel-recovery-acceptance"),
                BackupJsonContext.Default.FileSystemBackupRepositorySpec)!);
        var source = SerializeWithType(
            "CitadelSystem",
            JsonSerializer.SerializeToNode(
                new CitadelSystemBackupSource(),
                BackupJsonContext.Default.CitadelSystemBackupSource)!);

        await using var connection = new NpgsqlConnection(connectionString);
        await connection.OpenAsync(cancellationToken);
        await using var command = new NpgsqlCommand(
            """
            INSERT INTO backuprepositories (
                id, controlstate, createdat, createdbyactorid, description,
                name, normalizedname, passwordsecretid, rowversion, spec,
                status, type, updatedat)
            VALUES (
                @backupRepositoryId, 'Idle', CURRENT_TIMESTAMP,
                '00000000-0000-0000-0000-000000000001',
                'Recovery acceptance destination', 'Recovery destination',
                'recovery destination', @secretId, 0, @repositorySpec::jsonb,
                'Ready', 'FileSystem', CURRENT_TIMESTAMP);

            INSERT INTO backuppolicies (
                id, alertonfailure, backuprepositoryid, controlstate,
                createdat, createdbyactorid, description, enabled,
                keeplastsuccessful, name, normalizedname, rowversion,
                runasactorid, source, timeoutseconds, updatedat)
            VALUES (
                @backupPolicyId, TRUE, @backupRepositoryId, 'Idle',
                CURRENT_TIMESTAMP,
                '00000000-0000-0000-0000-000000000001',
                'Recovery acceptance policy', FALSE, 3, 'Recovery policy',
                'recovery policy', 0,
                @administratorActorId,
                @source::jsonb, 300, CURRENT_TIMESTAMP);

            INSERT INTO backupruns (
                id, backuppolicyid, backuprepositoryid, bytesadded,
                bytesprocessed, completedat, filesprocessed,
                policynamesnapshot, queuedat, repositorytypesnapshot,
                resticsnapshotid, snapshotavailability, sourcesnapshot,
                startedat, status, trigger, triggeredbyactorid, warnings)
            VALUES (
                @backupRunId, @backupPolicyId, @backupRepositoryId, 1024,
                4096, CURRENT_TIMESTAMP - INTERVAL '2 minutes', 12,
                'Recovery policy', CURRENT_TIMESTAMP - INTERVAL '5 minutes',
                'FileSystem', 'recovery-snapshot-001', 'Available',
                @source::jsonb, CURRENT_TIMESTAMP - INTERVAL '4 minutes',
                'Succeeded', 'Manual',
                @administratorActorId, '[]'::jsonb);
            """,
            connection);
        command.Parameters.AddWithValue(
            "backupRepositoryId",
            BackupRepositoryId);
        command.Parameters.AddWithValue("backupPolicyId", BackupPolicyId);
        command.Parameters.AddWithValue("backupRunId", BackupRunId);
        command.Parameters.AddWithValue("secretId", secretId);
        command.Parameters.AddWithValue(
            "administratorActorId",
            administratorActorId);
        command.Parameters.AddWithValue("repositorySpec", repositorySpec);
        command.Parameters.AddWithValue("source", source);
        await command.ExecuteNonQueryAsync(cancellationToken);
    }

    private static async Task<Guid> CreateSecretAsync(
        CandidateApplicationProcess candidate,
        CancellationToken cancellationToken)
    {
        var response = await candidate.Client.PostAsJsonAsync(
            "/api/v1/resourceBindings/secrets",
            new
            {
                name = "RECOVERY_SECRET",
                value = "recovery-secret-value"
            },
            cancellationToken);
        await AssertSuccessAsync(candidate, response, cancellationToken);
        var payload = await response.Content.ReadFromJsonAsync<JsonElement>(
            cancellationToken: cancellationToken);
        return payload.GetProperty("id").GetGuid();
    }

    private static async Task<Guid> CreateStackAsync(
        CandidateApplicationProcess candidate,
        CancellationToken cancellationToken)
    {
        var response = await candidate.Client.PostAsJsonAsync(
            "/api/v1/stacks",
            new
            {
                name = "recovery-stack",
                platformId = PlatformId,
                stackSource = "WebEditor",
                spec = new JsonObject
                {
                    ["$type"] = "WebEditor",
                    ["composeFile"] =
                        "services:\n  recovery:\n    image: hello-world"
                }
            },
            cancellationToken);
        await AssertSuccessAsync(candidate, response, cancellationToken);
        var payload = await response.Content.ReadFromJsonAsync<JsonElement>(
            cancellationToken: cancellationToken);
        return payload.GetProperty("id").GetGuid();
    }

    private static async Task CreateTagAsync(
        CandidateApplicationProcess candidate,
        string name,
        CancellationToken cancellationToken)
    {
        var response = await candidate.Client.PostAsJsonAsync(
            "/api/v1/tags/",
            new
            {
                name,
                color = "#2563eb"
            },
            cancellationToken);
        await AssertSuccessAsync(candidate, response, cancellationToken);
    }

    private static async Task AssertTagExistsAsync(
        CandidateApplicationProcess candidate,
        string expectedName,
        CancellationToken cancellationToken)
    {
        var tags = await candidate.Client.GetFromJsonAsync<JsonElement>(
            "/api/v1/tags/",
            cancellationToken);
        Assert.Contains(
            tags.GetProperty("tags").EnumerateArray(),
            tag => string.Equals(
                tag.GetProperty("name").GetString(),
                expectedName,
                StringComparison.Ordinal));
    }

    private static async Task<JsonElement> ReadLicenseAsync(
        CandidateApplicationProcess candidate,
        CancellationToken cancellationToken)
    {
        var response = await candidate.Client.GetAsync(
            "/api/v1/license/",
            cancellationToken);
        await AssertSuccessAsync(candidate, response, cancellationToken);
        return await response.Content.ReadFromJsonAsync<JsonElement>(
            cancellationToken: cancellationToken);
    }

    private static string GetLicenseState(JsonElement license)
        => string.Join(
            ":",
            license.GetProperty("effectiveEdition").GetString(),
            license.GetProperty("status").GetString(),
            license.GetProperty("instanceId").GetGuid());

    private static async Task AssertRestoredStateAsync(
        string connectionString,
        Guid secretId,
        Guid stackId,
        Guid administratorActorId,
        string dataDirectory,
        CancellationToken cancellationToken)
    {
        await using var connection = new NpgsqlConnection(connectionString);
        await connection.OpenAsync(cancellationToken);

        Assert.Equal(
            "recovery-secret-value",
            DecryptSecret(
                await ScalarAsync<string>(
                    connection,
                    """
                    SELECT encryptedvalue
                    FROM internalsecretvalues
                    WHERE secretid = @secretId;
                    """,
                    cancellationToken,
                    new NpgsqlParameter("secretId", secretId)),
                Path.Combine(dataDirectory, "secret-encryption-key")));
        Assert.Equal(
            1L,
            await CountAsync(
                connection,
                "SELECT COUNT(*) FROM users WHERE email = 'admin@citadel.local';",
                cancellationToken));
        Assert.True(
            await CountAsync(
                connection,
                """
                SELECT COUNT(*)
                FROM actorroles AS ar
                JOIN permissions AS p ON p.roleid = ar.roleid
                WHERE ar.actorid = @administratorActorId;
                """,
                cancellationToken,
                new NpgsqlParameter(
                    "administratorActorId",
                    administratorActorId)) > 0);
        Assert.Equal(
            1L,
            await CountAsync(
                connection,
                $"SELECT COUNT(*) FROM platforms WHERE id = '{PlatformId}';",
                cancellationToken));
        Assert.Equal(
            1L,
            await CountAsync(
                connection,
                $"SELECT COUNT(*) FROM gitrepositories WHERE id = '{GitRepositoryId}';",
                cancellationToken));
        Assert.Equal(
            1L,
            await CountAsync(
                connection,
                $"SELECT COUNT(*) FROM stacks WHERE id = '{stackId}';",
                cancellationToken));
        Assert.Equal(
            1L,
            await CountAsync(
                connection,
                $"SELECT COUNT(*) FROM stackreleases WHERE stackid = '{stackId}' AND status = 'Created';",
                cancellationToken));
        Assert.Equal(
            1L,
            await CountAsync(
                connection,
                $"SELECT COUNT(*) FROM backuprepositories WHERE id = '{BackupRepositoryId}';",
                cancellationToken));
        Assert.Equal(
            1L,
            await CountAsync(
                connection,
                $"SELECT COUNT(*) FROM backupruns WHERE id = '{BackupRunId}' AND status = 'Succeeded';",
                cancellationToken));
        Assert.Equal(
            1L,
            await CountAsync(
                connection,
                $"SELECT COUNT(*) FROM activityevents WHERE id = '{ActivityId}';",
                cancellationToken));
        Assert.True(
            await CountAsync(
                connection,
                "SELECT COUNT(*) FROM actions WHERE scheduleenabled = TRUE;",
                cancellationToken) > 0);
        Assert.Equal(
            1L,
            await CountAsync(
                connection,
                $"SELECT COUNT(*) FROM edgeagentbindings WHERE id = '{EdgeAgentBindingId}' AND revokedatutc IS NULL;",
                cancellationToken));
        Assert.Equal(
            2L,
            await CountAsync(
                connection,
                """
                SELECT COUNT(*)
                FROM tags
                WHERE name IN ('Before recovery', 'After recovery');
                """,
                cancellationToken));
    }

    private static async Task<long> CountAsync(
        NpgsqlConnection connection,
        string sql,
        CancellationToken cancellationToken,
        params NpgsqlParameter[] parameters)
        => await ScalarAsync<long>(
            connection,
            sql,
            cancellationToken,
            parameters);

    private static async Task<Guid> GetInitialAdministratorActorIdAsync(
        string connectionString,
        CancellationToken cancellationToken)
    {
        await using var connection = new NpgsqlConnection(connectionString);
        await connection.OpenAsync(cancellationToken);
        return await ScalarAsync<Guid>(
            connection,
            """
            SELECT initialadministratoractorid
            FROM instancesetupstates
            WHERE id = 1;
            """,
            cancellationToken);
    }

    private static async Task<T> ScalarAsync<T>(
        NpgsqlConnection connection,
        string sql,
        CancellationToken cancellationToken,
        params NpgsqlParameter[] parameters)
    {
        await using var command = new NpgsqlCommand(sql, connection);
        command.Parameters.AddRange(parameters);
        return (T)(await command.ExecuteScalarAsync(cancellationToken))!;
    }

    private static string SerializeWithType(
        string type,
        JsonNode payload)
    {
        var source = payload.AsObject();
        var result = new JsonObject
        {
            ["$type"] = type
        };
        foreach (var property in source.ToArray())
        {
            source.Remove(property.Key);
            result[property.Key] = property.Value;
        }

        return result.ToJsonString();
    }

    private static string DecryptSecret(
        string protectedValue,
        string keyPath)
    {
        const int nonceSize = 12;
        const int tagSize = 16;
        var key = Convert.FromBase64String(
            File.ReadAllText(keyPath).Trim());
        var payload = Convert.FromBase64String(protectedValue);
        var plaintext = new byte[payload.Length - nonceSize - tagSize];

        using var aes = new AesGcm(key, tagSize);
        aes.Decrypt(
            payload.AsSpan(0, nonceSize),
            payload.AsSpan(nonceSize + tagSize),
            payload.AsSpan(nonceSize, tagSize),
            plaintext);
        return Encoding.UTF8.GetString(plaintext);
    }

    private static async Task AssertSuccessAsync(
        CandidateApplicationProcess candidate,
        HttpResponseMessage response,
        CancellationToken cancellationToken)
    {
        Assert.True(
            response.IsSuccessStatusCode,
            $"""
            Candidate request failed with HTTP {(int)response.StatusCode}.
            {await response.Content.ReadAsStringAsync(cancellationToken)}
            {candidate.Output}
            """);
    }

    private static string HashFile(string path)
        => Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));

    private static void CreateArchiveWithoutEntry(
        string sourcePath,
        string destinationPath,
        string omittedEntry)
    {
        using var source = ZipFile.OpenRead(sourcePath);
        using var destination = ZipFile.Open(
            destinationPath,
            ZipArchiveMode.Create);
        foreach (var entry in source.Entries)
        {
            if (string.Equals(
                    entry.FullName,
                    omittedEntry,
                    StringComparison.Ordinal))
            {
                continue;
            }

            var copy = destination.CreateEntry(
                entry.FullName,
                CompressionLevel.NoCompression);
            using var input = entry.Open();
            using var output = copy.Open();
            input.CopyTo(output);
        }
    }

    private static string CreateTestRoot()
    {
        var root = Path.Combine(
            Path.GetTempPath(),
            $"citadel-recovery-acceptance-{Guid.NewGuid():N}");
        Directory.CreateDirectory(root);
        return root;
    }

    private static void DeleteDirectory(string path)
    {
        if (Directory.Exists(path))
            Directory.Delete(path, recursive: true);
    }
}
