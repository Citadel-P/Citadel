using System.Diagnostics;
using System.Net.Http.Json;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Tests.Acceptance.Infrastructure;

namespace Tests.Acceptance.Compatibility;

[Collection("AcceptancePostgres")]
public sealed class VaultKvV2CompatibilityTests(
    AcceptancePostgresFixture postgres)
{
    [Fact]
    public async Task VaultKvV2_ShouldValidateResolveInjectAndRedactSecret()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var connectionString =
            await postgres.CreateDatabaseAsync(cancellationToken);
        var candidateDirectory = Path.Combine(
            Path.GetTempPath(),
            $"citadel-vault-{Guid.NewGuid():N}");
        Directory.CreateDirectory(candidateDirectory);

        Guid providerId = Guid.Empty;
        Guid secretId = Guid.Empty;
        Guid platformId = Guid.Empty;
        Guid stackId = Guid.Empty;
        string? containerId = null;
        string? projectName = null;
        CandidateApplicationProcess? candidate = null;

        try
        {
            await using var vault =
                await VaultInstance.StartAsync(cancellationToken);
            candidate = await CandidateApplicationProcess.StartAsync(
                connectionString,
                candidateDirectory,
                cancellationToken,
                new CandidateApplicationOptions(HttpTimeoutSeconds: 120));
            await candidate.AuthenticateAsAdminAsync(cancellationToken);

            await AssertConnectionAsync(
                candidate,
                vault.BaseAddress,
                providerId: null,
                token: VaultInstance.RootToken,
                expectedSuccess: true,
                expectedMessage: "token is valid",
                cancellationToken);
            await AssertConnectionAsync(
                candidate,
                vault.BaseAddress,
                providerId: null,
                token: "invalid-vault-token",
                expectedSuccess: false,
                expectedMessage: "token was rejected",
                cancellationToken);

            providerId = await CreateProviderAsync(
                candidate,
                vault.BaseAddress,
                cancellationToken);

            await AssertExternalSecretAsync(
                candidate,
                providerId,
                VaultInstance.SecretPath,
                VaultInstance.SecretKey,
                expectedSuccess: true,
                expectedMessage: "resolved successfully",
                cancellationToken);
            await AssertExternalSecretAsync(
                candidate,
                providerId,
                "citadel/missing",
                VaultInstance.SecretKey,
                expectedSuccess: false,
                expectedMessage: "was not found",
                cancellationToken);
            await AssertExternalSecretAsync(
                candidate,
                providerId,
                VaultInstance.SecretPath,
                "missing_key",
                expectedSuccess: false,
                expectedMessage: "key 'missing_key' was not found",
                cancellationToken);

            await PatchProviderTokenAsync(
                candidate,
                providerId,
                string.Empty,
                cancellationToken);
            await AssertStoredConnectionAsync(
                candidate,
                vault.BaseAddress,
                providerId,
                expectedSuccess: true,
                expectedMessage: "stored token",
                cancellationToken);

            await PatchProviderTokenAsync(
                candidate,
                providerId,
                "invalid-replacement-token",
                cancellationToken);
            await AssertStoredConnectionAsync(
                candidate,
                vault.BaseAddress,
                providerId,
                expectedSuccess: false,
                expectedMessage: "token was rejected",
                cancellationToken);

            await PatchProviderTokenAsync(
                candidate,
                providerId,
                VaultInstance.RootToken,
                cancellationToken);
            await AssertStoredConnectionAsync(
                candidate,
                vault.BaseAddress,
                providerId,
                expectedSuccess: true,
                expectedMessage: "stored token",
                cancellationToken);

            secretId = await CreateExternalSecretAsync(
                candidate,
                providerId,
                cancellationToken);
            platformId = await CreatePlatformAsync(
                candidate,
                cancellationToken);
            projectName = $"acceptance-vault-{Guid.NewGuid():N}";
            stackId = await CreateStackAsync(
                candidate,
                platformId,
                projectName,
                cancellationToken);
            var bindingId = await CreateStackBindingAsync(
                candidate,
                stackId,
                secretId,
                cancellationToken);

            var applyBody = await ApplyStackAsync(
                candidate,
                stackId,
                cancellationToken);
            AssertDoesNotContainSecret(applyBody);

            containerId = await WaitForContainerIdAsync(
                candidate,
                stackId,
                cancellationToken);
            await AssertContainerReceivedSecretAsync(
                containerId,
                cancellationToken);
            await DeleteStackBindingAsync(
                candidate,
                stackId,
                bindingId,
                cancellationToken);
            await AssertInspectionIsRedactedAsync(
                candidate,
                stackId,
                containerId,
                cancellationToken);
            await AssertPersistedOutputsAreRedactedAsync(
                candidate,
                stackId,
                cancellationToken);
        }
        finally
        {
            if (candidate is not null)
            {
                await CleanupApiResourcesAsync(
                    candidate.Client,
                    providerId,
                    secretId,
                    platformId,
                    stackId);
                await candidate.DisposeAsync();
            }

            await CleanupDockerResourcesAsync(containerId, projectName);
            await postgres.DropDatabaseAsync(
                connectionString,
                CancellationToken.None);
            await DeleteDirectoryAsync(candidateDirectory);
        }
    }

    private static async Task AssertConnectionAsync(
        CandidateApplicationProcess candidate,
        Uri vaultAddress,
        Guid? providerId,
        string? token,
        bool expectedSuccess,
        string expectedMessage,
        CancellationToken cancellationToken)
    {
        using var response = await candidate.Client.PostAsJsonAsync(
            "/api/v1/resourceBindings/secret-providers/vault-kv2/test",
            new
            {
                providerId,
                name = "Vault acceptance",
                address = vaultAddress.ToString(),
                mountPath = "secret",
                token
            },
            cancellationToken);
        var result = await ReadTestResultAsync(
            response,
            "Testing the Vault provider connection",
            cancellationToken);

        Assert.Equal(expectedSuccess, result.Success);
        Assert.Contains(
            expectedMessage,
            result.Message,
            StringComparison.OrdinalIgnoreCase);
    }

    private static Task AssertStoredConnectionAsync(
        CandidateApplicationProcess candidate,
        Uri vaultAddress,
        Guid providerId,
        bool expectedSuccess,
        string expectedMessage,
        CancellationToken cancellationToken)
        => AssertConnectionAsync(
            candidate,
            vaultAddress,
            providerId,
            token: null,
            expectedSuccess,
            expectedMessage,
            cancellationToken);

    private static async Task<Guid> CreateProviderAsync(
        CandidateApplicationProcess candidate,
        Uri vaultAddress,
        CancellationToken cancellationToken)
    {
        using var response = await candidate.Client.PostAsJsonAsync(
            "/api/v1/resourceBindings/secret-providers/vault-kv2",
            new
            {
                name = $"acceptance-vault-provider-{Guid.NewGuid():N}",
                address = vaultAddress.ToString(),
                mountPath = "secret",
                token = VaultInstance.RootToken
            },
            cancellationToken);
        return await ReadCreatedIdAsync(
            response,
            "Creating the Vault provider",
            cancellationToken);
    }

    private static async Task PatchProviderTokenAsync(
        CandidateApplicationProcess candidate,
        Guid providerId,
        string token,
        CancellationToken cancellationToken)
    {
        using var request = new HttpRequestMessage(
            HttpMethod.Patch,
            $"/api/v1/resourceBindings/secret-providers/vault-kv2/{providerId:D}")
        {
            Content = new StringContent(
                JsonSerializer.Serialize(new { token }),
                Encoding.UTF8,
                "application/merge-patch+json")
        };
        using var response = await candidate.Client.SendAsync(
            request,
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        AssertDoesNotContainSecret(body);
        Assert.True(
            response.IsSuccessStatusCode,
            $"Updating the Vault provider failed with HTTP {(int)response.StatusCode}: {body}");
    }

    private static async Task AssertExternalSecretAsync(
        CandidateApplicationProcess candidate,
        Guid providerId,
        string path,
        string key,
        bool expectedSuccess,
        string expectedMessage,
        CancellationToken cancellationToken)
    {
        using var response = await candidate.Client.PostAsJsonAsync(
            "/api/v1/resourceBindings/secrets/external/test",
            new
            {
                providerId,
                externalPath = path,
                externalKey = key,
                externalVersion = (int?)null
            },
            cancellationToken);
        var result = await ReadTestResultAsync(
            response,
            "Testing the external secret reference",
            cancellationToken);

        Assert.Equal(expectedSuccess, result.Success);
        Assert.Contains(
            expectedMessage,
            result.Message,
            StringComparison.OrdinalIgnoreCase);
    }

    private static async Task<Guid> CreateExternalSecretAsync(
        CandidateApplicationProcess candidate,
        Guid providerId,
        CancellationToken cancellationToken)
    {
        using var response = await candidate.Client.PostAsJsonAsync(
            "/api/v1/resourceBindings/secrets/external",
            new
            {
                name = "CITADEL_VAULT_ACCEPTANCE_SECRET",
                providerId,
                externalPath = VaultInstance.SecretPath,
                externalKey = VaultInstance.SecretKey,
                externalVersion = (int?)null
            },
            cancellationToken);
        return await ReadCreatedIdAsync(
            response,
            "Creating the external secret reference",
            cancellationToken);
    }

    private static async Task<Guid> CreatePlatformAsync(
        CandidateApplicationProcess candidate,
        CancellationToken cancellationToken)
    {
        using var response = await candidate.Client.PostAsJsonAsync(
            "/api/v1/platforms",
            new
            {
                name = $"acceptance-vault-platform-{Guid.NewGuid():N}",
                address = (string?)null,
                description = "Disposable Vault acceptance platform",
                type = "Docker",
                connectorType = "Local",
                tagIds = Array.Empty<Guid>()
            },
            cancellationToken);
        return await ReadCreatedIdAsync(
            response,
            "Creating the Vault acceptance platform",
            cancellationToken);
    }

    private static async Task<Guid> CreateStackAsync(
        CandidateApplicationProcess candidate,
        Guid platformId,
        string projectName,
        CancellationToken cancellationToken)
    {
        var expectedHash = Convert.ToHexString(
                SHA256.HashData(
                    Encoding.UTF8.GetBytes(VaultInstance.SecretValue)))
            .ToLowerInvariant();
        using var response = await candidate.Client.PostAsJsonAsync(
            "/api/v1/stacks",
            new
            {
                name = projectName,
                platformId,
                description = "Vault secret delivery acceptance fixture",
                stackSource = "WebEditor",
                spec = new Dictionary<string, object?>
                {
                    ["$type"] = "WebEditor",
                    ["composeFile"] = ComposeFile(expectedHash),
                    ["updateBehavior"] = "Disabled",
                    ["projectName"] = projectName,
                    ["destroyBeforeDeploy"] = true,
                    ["buildImageBindings"] = Array.Empty<object>()
                },
                tagIds = Array.Empty<Guid>()
            },
            cancellationToken);
        return await ReadCreatedIdAsync(
            response,
            "Creating the Vault acceptance stack",
            cancellationToken);
    }

    private static async Task<Guid> CreateStackBindingAsync(
        CandidateApplicationProcess candidate,
        Guid stackId,
        Guid secretId,
        CancellationToken cancellationToken)
    {
        using var response = await candidate.Client.PostAsJsonAsync(
            $"/api/v1/resourceBindings/Stack/{stackId:D}",
            new
            {
                name = "CITADEL_VAULT_SECRET",
                kind = "Secret",
                value = (string?)null,
                secretId,
                secretDeliveryMode = "EnvironmentVariable",
                targetPath = (string?)null
            },
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        AssertDoesNotContainSecret(body);
        Assert.True(
            response.IsSuccessStatusCode,
            $"Creating the stack secret binding failed with HTTP {(int)response.StatusCode}: {body}");

        using var json = JsonDocument.Parse(body);
        return json.RootElement
            .GetProperty("entries")
            .EnumerateArray()
            .Single(entry => string.Equals(
                entry.GetProperty("name").GetString(),
                "CITADEL_VAULT_SECRET",
                StringComparison.Ordinal))
            .GetProperty("id")
            .GetGuid();
    }

    private static async Task DeleteStackBindingAsync(
        CandidateApplicationProcess candidate,
        Guid stackId,
        Guid bindingId,
        CancellationToken cancellationToken)
    {
        using var response = await candidate.Client.DeleteAsync(
            $"/api/v1/resourceBindings/Stack/{stackId:D}/{bindingId:D}",
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        AssertDoesNotContainSecret(body);
        Assert.True(
            response.IsSuccessStatusCode,
            $"Deleting the applied stack binding failed with HTTP {(int)response.StatusCode}: {body}");
    }

    private static async Task<string> ApplyStackAsync(
        CandidateApplicationProcess candidate,
        Guid stackId,
        CancellationToken cancellationToken)
    {
        using var response = await candidate.Client.PostAsJsonAsync(
            "/api/v1/stacks/apply",
            new
            {
                id = stackId,
                recreate = false
            },
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        AssertDoesNotContainSecret(body);
        Assert.True(
            response.IsSuccessStatusCode,
            $"""
            Applying Vault stack {stackId:D} failed with HTTP {(int)response.StatusCode}: {body}
            {candidate.Output}
            """);

        using var json = JsonDocument.Parse(body);
        var events = json.RootElement.EnumerateArray().ToArray();
        Assert.NotEmpty(events);
        var completion = events[^1];
        Assert.True(
            completion.GetProperty("exitCode").GetInt32() == 0
            && string.Equals(
                completion.GetProperty("severity").GetString(),
                "success",
                StringComparison.Ordinal)
            && string.Equals(
                completion.GetProperty("stackStatus").GetString(),
                "Healthy",
                StringComparison.Ordinal),
            $"""
            Applying Vault stack {stackId:D} did not finish healthy.
            Apply response: {body}
            {candidate.Output}
            """);
        return body;
    }

    private static async Task<string> WaitForContainerIdAsync(
        CandidateApplicationProcess candidate,
        Guid stackId,
        CancellationToken cancellationToken)
    {
        var elapsed = Stopwatch.StartNew();
        var lastBody = string.Empty;
        while (elapsed.Elapsed < TimeSpan.FromSeconds(60))
        {
            using var response = await candidate.Client.GetAsync(
                $"/api/v1/stacks/{stackId:D}/data",
                cancellationToken);
            lastBody = await response.Content.ReadAsStringAsync(
                cancellationToken);
            AssertDoesNotContainSecret(lastBody);

            if (response.IsSuccessStatusCode)
            {
                using var json = JsonDocument.Parse(lastBody);
                var containers = json.RootElement
                    .GetProperty("containers")
                    .EnumerateArray()
                    .ToArray();
                if (containers.Length > 0)
                    return containers[0].GetProperty("id").GetString()
                           ?? throw new InvalidOperationException(
                               "Stack container id was null.");
            }

            await Task.Delay(
                TimeSpan.FromMilliseconds(250),
                cancellationToken);
        }

        throw new TimeoutException(
            $"""
            Timed out waiting for stack {stackId:D} container data.
            Last response: {lastBody}
            {candidate.Output}
            """);
    }

    private static async Task AssertContainerReceivedSecretAsync(
        string containerId,
        CancellationToken cancellationToken)
    {
        var environment = await RunDockerAsync(
            cancellationToken,
            "inspect",
            "--format",
            "{{range .Config.Env}}{{println .}}{{end}}",
            containerId);
        Assert.True(
            environment
                .Split(
                    ['\r', '\n'],
                    StringSplitOptions.RemoveEmptyEntries)
                .Contains(
                    $"CITADEL_VAULT_SECRET={VaultInstance.SecretValue}",
                    StringComparer.Ordinal),
            "The deployed container did not receive the resolved Vault secret.");

        var logs = await RunDockerAsync(
            cancellationToken,
            "logs",
            containerId);
        AssertDoesNotContainSecret(logs);
        Assert.Contains(
            "citadel-vault-secret-injected",
            logs,
            StringComparison.Ordinal);
    }

    private static async Task AssertInspectionIsRedactedAsync(
        CandidateApplicationProcess candidate,
        Guid stackId,
        string containerId,
        CancellationToken cancellationToken)
    {
        foreach (var path in new[]
                 {
                     $"/api/v1/stacks/{stackId:D}/containers/{containerId}/inspect",
                     $"/api/v1/containers/{containerId}/inspect"
                 })
        {
            using var response = await candidate.Client.GetAsync(
                path,
                cancellationToken);
            var body = await response.Content.ReadAsStringAsync(
                cancellationToken);
            AssertDoesNotContainSecret(body);
            Assert.True(
                response.IsSuccessStatusCode,
                $"Inspecting the Vault stack container failed with HTTP {(int)response.StatusCode}: {body}");

            using var json = JsonDocument.Parse(body);
            Assert.Contains(
                "CITADEL_VAULT_SECRET=********",
                json.RootElement
                    .GetProperty("config")
                    .GetProperty("env")
                    .EnumerateArray()
                    .Select(value => value.GetString()),
                StringComparer.Ordinal);
        }
    }

    private static async Task AssertPersistedOutputsAreRedactedAsync(
        CandidateApplicationProcess candidate,
        Guid stackId,
        CancellationToken cancellationToken)
    {
        foreach (var path in new[]
                 {
                     $"/api/v1/stacks/{stackId:D}",
                     $"/api/v1/stacks/{stackId:D}/releases",
                     $"/api/v1/resourceBindings/Stack/{stackId:D}",
                     "/api/v1/resourceBindings/secrets",
                     "/api/v1/resourceBindings/secret-providers",
                     $"/api/v1/activities?resourceId={stackId:D}&pageSize=50"
                 })
        {
            using var response = await candidate.Client.GetAsync(
                path,
                cancellationToken);
            var body = await response.Content.ReadAsStringAsync(
                cancellationToken);
            AssertDoesNotContainSecret(body);
            Assert.True(
                response.IsSuccessStatusCode,
                $"Reading '{path}' failed with HTTP {(int)response.StatusCode}: {body}");

            if (!path.StartsWith(
                    "/api/v1/activities?",
                    StringComparison.Ordinal))
            {
                continue;
            }

            using var json = JsonDocument.Parse(body);
            foreach (var activity in json.RootElement
                         .GetProperty("pagedResult")
                         .GetProperty("items")
                         .EnumerateArray())
            {
                var activityId = activity.GetProperty("id").GetGuid();
                using var detailResponse = await candidate.Client.GetAsync(
                    $"/api/v1/activities/{activityId:D}",
                    cancellationToken);
                var detailBody = await detailResponse.Content.ReadAsStringAsync(
                    cancellationToken);
                AssertDoesNotContainSecret(detailBody);
                Assert.True(
                    detailResponse.IsSuccessStatusCode,
                    $"Reading activity {activityId:D} failed with HTTP {(int)detailResponse.StatusCode}: {detailBody}");
            }
        }
    }

    private static async Task<(bool Success, string Message)> ReadTestResultAsync(
        HttpResponseMessage response,
        string operation,
        CancellationToken cancellationToken)
    {
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        AssertDoesNotContainSecret(body);
        Assert.True(
            response.IsSuccessStatusCode,
            $"{operation} failed with HTTP {(int)response.StatusCode}: {body}");

        using var json = JsonDocument.Parse(body);
        return (
            json.RootElement.GetProperty("success").GetBoolean(),
            json.RootElement.GetProperty("message").GetString() ?? string.Empty);
    }

    private static async Task<Guid> ReadCreatedIdAsync(
        HttpResponseMessage response,
        string operation,
        CancellationToken cancellationToken)
    {
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        AssertDoesNotContainSecret(body);
        Assert.True(
            response.IsSuccessStatusCode,
            $"{operation} failed with HTTP {(int)response.StatusCode}: {body}");

        using var json = JsonDocument.Parse(body);
        return json.RootElement.GetProperty("id").GetGuid();
    }

    private static void AssertDoesNotContainSecret(string value)
    {
        Assert.DoesNotContain(
            VaultInstance.SecretValue,
            value,
            StringComparison.Ordinal);
        Assert.DoesNotContain(
            VaultInstance.RootToken,
            value,
            StringComparison.Ordinal);
    }

    private static async Task CleanupApiResourcesAsync(
        HttpClient client,
        Guid providerId,
        Guid secretId,
        Guid platformId,
        Guid stackId)
    {
        if (stackId != Guid.Empty)
        {
            await TryDeleteAsync(
                client,
                "/api/v1/stacks",
                new[] { stackId });
        }

        if (secretId != Guid.Empty)
        {
            await TryDeleteAsync(
                client,
                $"/api/v1/resourceBindings/secrets/{secretId:D}");
        }

        if (providerId != Guid.Empty)
        {
            await TryDeleteAsync(
                client,
                $"/api/v1/resourceBindings/secret-providers/{providerId:D}");
        }

        if (platformId != Guid.Empty)
        {
            await TryDeleteAsync(
                client,
                "/api/v1/platforms",
                new
                {
                    ids = new[] { platformId }
                });
        }
    }

    private static async Task CleanupDockerResourcesAsync(
        string? containerId,
        string? projectName)
    {
        if (containerId is not null)
        {
            await RunDockerAsync(
                CancellationToken.None,
                throwOnFailure: false,
                "rm",
                "--force",
                containerId);
        }

        if (projectName is not null)
        {
            await RunDockerAsync(
                CancellationToken.None,
                throwOnFailure: false,
                "network",
                "rm",
                $"{projectName}_default");
        }
    }

    private static async Task TryDeleteAsync(
        HttpClient client,
        string path,
        object? body = null)
    {
        try
        {
            using var request = new HttpRequestMessage(
                HttpMethod.Delete,
                path)
            {
                Content = body is null ? null : JsonContent.Create(body)
            };
            using var response = await client.SendAsync(
                request,
                CancellationToken.None);
            _ = await response.Content.ReadAsStringAsync(
                CancellationToken.None);
        }
        catch (Exception error) when (
            error is HttpRequestException
                or TaskCanceledException
                or ObjectDisposedException)
        {
        }
    }

    private static async Task<string> RunDockerAsync(
        CancellationToken cancellationToken,
        params string[] arguments)
        => await RunDockerAsync(
            cancellationToken,
            throwOnFailure: true,
            arguments);

    private static async Task<string> RunDockerAsync(
        CancellationToken cancellationToken,
        bool throwOnFailure,
        params string[] arguments)
    {
        var startInfo = new ProcessStartInfo("docker")
        {
            UseShellExecute = false,
            CreateNoWindow = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true
        };
        foreach (var argument in arguments)
            startInfo.ArgumentList.Add(argument);

        using var process = Process.Start(startInfo)
            ?? throw new InvalidOperationException("Failed to start Docker.");
        var outputTask = process.StandardOutput.ReadToEndAsync(
            cancellationToken);
        var errorTask = process.StandardError.ReadToEndAsync(
            cancellationToken);
        await process.WaitForExitAsync(cancellationToken);
        var output = await outputTask;
        var error = await errorTask;

        if (throwOnFailure)
        {
            Assert.True(
                process.ExitCode == 0,
                $"docker {string.Join(' ', arguments)} failed with exit code {process.ExitCode}: {error}");
        }

        return string.Concat(output, error);
    }

    private static async Task DeleteDirectoryAsync(string path)
    {
        for (var attempt = 0; attempt < 5; attempt++)
        {
            if (!Directory.Exists(path))
                return;

            try
            {
                Directory.Delete(path, recursive: true);
                return;
            }
            catch (IOException) when (attempt < 4)
            {
            }
            catch (UnauthorizedAccessException) when (attempt < 4)
            {
            }

            await Task.Delay(TimeSpan.FromMilliseconds(200));
        }
    }

    private static string ComposeFile(string expectedHash) =>
        $$"""
        services:
          consumer:
            image: busybox:1.36.1
            network_mode: none
            environment:
              CITADEL_VAULT_SECRET: ${CITADEL_VAULT_SECRET}
            command:
              - sh
              - -c
              - >-
                actual=$$(printf '%s' "$$CITADEL_VAULT_SECRET" | sha256sum | cut -d ' ' -f 1);
                test "$$actual" = "{{expectedHash}}" && echo citadel-vault-secret-injected;
                exec tail -f /dev/null
            stop_grace_period: 1s
        """;
}
