using DotNet.Testcontainers.Configurations;
using System.Diagnostics;
using System.Net;
using System.Net.Http.Json;
using System.Text;
using System.Text.Json;
using Tests.Acceptance.Infrastructure;

namespace Tests.Acceptance.Compatibility;

[Collection("AcceptancePostgres")]
public sealed class ForgejoGitWebhookTests(AcceptancePostgresFixture postgres)
{
    private const string WebhookSecret =
        "citadel-forgejo-acceptance-webhook-secret";

    [Fact]
    public async Task ForgejoPush_ShouldUpdateAndDeployGitStack()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var connectionString =
            await postgres.CreateDatabaseAsync(cancellationToken);
        var root = Path.Combine(
            Path.GetTempPath(),
            $"cfa-{Guid.NewGuid():N}"[..12]);
        var candidateDirectory = Path.Combine(root, "c");
        var repositoryDirectory = Path.Combine(root, "r");
        Directory.CreateDirectory(candidateDirectory);

        try
        {
            await using var dockerDaemon =
                await StandaloneDockerDaemon.StartAsync(cancellationToken);
            await using var candidate =
                await CandidateApplicationProcess.StartAsync(
                    connectionString,
                    candidateDirectory,
                    cancellationToken,
                    new CandidateApplicationOptions(
                        HttpTimeoutSeconds: 120,
                        DockerHost: dockerDaemon.DockerHost));
            await candidate.AuthenticateAsAdminAsync(cancellationToken);

            var candidatePort = checked(
                (ushort)candidate.Client.BaseAddress!.Port);
            await TestcontainersSettings.ExposeHostPortsAsync(
                candidatePort,
                cancellationToken);

            await using var forgejo =
                await ForgejoInstance.StartAsync(cancellationToken);
            await forgejo.InitializeLocalRepositoryAsync(
                repositoryDirectory,
                ComposeFile("one"),
                cancellationToken);
            var initialCommit = await GetHeadCommitAsync(
                repositoryDirectory,
                cancellationToken);

            Guid platformId = Guid.Empty;
            Guid gitAccountId = Guid.Empty;
            Guid gitRepositoryId = Guid.Empty;
            Guid stackId = Guid.Empty;

            try
            {
                platformId = await CreatePlatformAsync(
                    candidate,
                    cancellationToken);
                gitAccountId = await CreateGitAccountAsync(
                    candidate,
                    forgejo,
                    cancellationToken);
                gitRepositoryId = await CreateGitRepositoryAsync(
                    candidate,
                    forgejo,
                    gitAccountId,
                    cancellationToken);
                await WaitForRepositoryCommitAsync(
                    candidate,
                    gitRepositoryId,
                    initialCommit,
                    cancellationToken);

                stackId = await CreateStackAsync(
                    candidate,
                    gitRepositoryId,
                    platformId,
                    cancellationToken);
                await ApplyStackAsync(
                    candidate,
                    stackId,
                    cancellationToken);
                var initialStack = await GetStackAsync(
                    candidate,
                    stackId,
                    cancellationToken);
                Assert.Equal("Healthy", GetString(initialStack, "status"));
                Assert.Equal("1", GetString(initialStack, "version"));
                Assert.Equal(
                    initialCommit,
                    GetString(initialStack, "source", "resolvedCommitSha"));

                var webhookTarget = new Uri(
                    $"http://host.testcontainers.internal:{candidatePort}/listener/github/stack/{stackId:D}/deploy");
                await forgejo.CreateWebhookAsync(
                    webhookTarget,
                    WebhookSecret,
                    cancellationToken);

                await AssertInvalidSignatureRejectedAsync(
                    candidate,
                    stackId,
                    forgejo.RepositoryFullName,
                    cancellationToken);

                var secondCommit = await forgejo.CommitAndPushAsync(
                    repositoryDirectory,
                    "compose.yml",
                    ComposeFile("two"),
                    "Deploy release two",
                    cancellationToken);
                await WaitForStackRemoteCommitAsync(
                    candidate,
                    stackId,
                    secondCommit,
                    cancellationToken);

                await ApplyStackAsync(
                    candidate,
                    stackId,
                    cancellationToken);
                var secondStack = await GetStackAsync(
                    candidate,
                    stackId,
                    cancellationToken);
                Assert.Equal("Healthy", GetString(secondStack, "status"));
                Assert.Equal("2", GetString(secondStack, "version"));
                Assert.Equal(
                    secondCommit,
                    GetString(secondStack, "source", "resolvedCommitSha"));
                Assert.Null(GetOptionalString(
                    secondStack,
                    "stackUpdateState",
                    "recreateStackOnNewCommitState",
                    "remoteCommitSha"));

                var ignoredCommit = await forgejo.CommitAndPushAsync(
                    repositoryDirectory,
                    "README.md",
                    "This change is intentionally outside the stack watch paths.",
                    "Update documentation only",
                    cancellationToken);
                await WaitForWebhookActivityAsync(
                    candidate,
                    stackId,
                    ignoredCommit,
                    "noop",
                    "No relevant path changes",
                    cancellationToken);

                var ignoredStack = await GetStackAsync(
                    candidate,
                    stackId,
                    cancellationToken);
                Assert.Equal("2", GetString(ignoredStack, "version"));
                Assert.Equal(
                    secondCommit,
                    GetString(ignoredStack, "source", "resolvedCommitSha"));
                Assert.Null(GetOptionalString(
                    ignoredStack,
                    "stackUpdateState",
                    "recreateStackOnNewCommitState",
                    "remoteCommitSha"));
                Assert.Equal(
                    secondCommit,
                    await GetRepositoryCommitAsync(
                        candidate,
                        gitRepositoryId,
                        cancellationToken));
            }
            finally
            {
                await CleanupAsync(
                    candidate,
                    stackId,
                    gitRepositoryId,
                    gitAccountId,
                    platformId);
            }
        }
        finally
        {
            await postgres.DropDatabaseAsync(
                connectionString,
                CancellationToken.None);
            await DeleteDirectoryAsync(root);
        }
    }

    private static async Task<Guid> CreatePlatformAsync(
        CandidateApplicationProcess candidate,
        CancellationToken cancellationToken)
    {
        using var response = await candidate.Client.PostAsJsonAsync(
            "/api/v1/platforms",
            new
            {
                name = $"acceptance-forgejo-platform-{Guid.NewGuid():N}",
                address = (string?)null,
                description = "Disposable Forgejo acceptance platform",
                type = "Docker",
                connectorType = "Local",
                tagIds = Array.Empty<Guid>()
            },
            cancellationToken);
        return await ReadCreatedIdAsync(
            response,
            "Creating the Forgejo acceptance platform",
            cancellationToken);
    }

    private static async Task<Guid> CreateGitAccountAsync(
        CandidateApplicationProcess candidate,
        ForgejoInstance forgejo,
        CancellationToken cancellationToken)
    {
        using var response = await candidate.Client.PostAsJsonAsync(
            "/api/v1/gitAccounts",
            new
            {
                name = $"acceptance-forgejo-account-{Guid.NewGuid():N}",
                domain = forgejo.BaseAddress.Host,
                transport = "Http",
                authType = "Token",
                configuration = new Dictionary<string, object?>
                {
                    ["$type"] = "Token",
                    ["token"] = forgejo.AccessToken
                }
            },
            cancellationToken);
        return await ReadCreatedIdAsync(
            response,
            "Creating the Forgejo Git account",
            cancellationToken);
    }

    private static async Task<Guid> CreateGitRepositoryAsync(
        CandidateApplicationProcess candidate,
        ForgejoInstance forgejo,
        Guid gitAccountId,
        CancellationToken cancellationToken)
    {
        using var response = await candidate.Client.PostAsJsonAsync(
            "/api/v1/gitRepositories",
            new
            {
                name = $"acceptance-forgejo-repo-{Guid.NewGuid():N}",
                description = "Disposable private Forgejo repository",
                url = forgejo.CloneUrl,
                defaultBranch = "main",
                gitAccountId,
                syncMode = "Manual",
                syncIntervalMinutes = (int?)null,
                webhook = (object?)null,
                onClone = (object?)null,
                onPull = (object?)null,
                tagIds = Array.Empty<Guid>()
            },
            cancellationToken);
        return await ReadCreatedIdAsync(
            response,
            "Creating the Citadel Git repository",
            cancellationToken);
    }

    private static async Task<Guid> CreateStackAsync(
        CandidateApplicationProcess candidate,
        Guid gitRepositoryId,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        var name = $"acceptance-forgejo-stack-{Guid.NewGuid():N}";
        using var response = await candidate.Client.PostAsJsonAsync(
            "/api/v1/stacks",
            new
            {
                name,
                platformId,
                description = "Forgejo Git stack acceptance fixture",
                stackSource = "Git",
                spec = new Dictionary<string, object?>
                {
                    ["$type"] = "Git",
                    ["gitRepoId"] = gitRepositoryId,
                    ["branch"] = "main",
                    ["commitSha"] = null,
                    ["updateBehavior"] = "Notify",
                    ["projectName"] = name,
                    ["webhook"] = new
                    {
                        enabled = true,
                        provider = "GitHub",
                        authScheme = "GitHubHmacSha256",
                        secret = WebhookSecret,
                        branchFilter = "main",
                        forceDeploy = false
                    },
                    ["composePaths"] = new[] { "compose.yml" },
                    ["workingDirectory"] = ".",
                    ["watchPaths"] = new[] { "compose.yml" },
                    ["destroyBeforeDeploy"] = true,
                    ["buildImageBindings"] = Array.Empty<object>()
                },
                tagIds = Array.Empty<Guid>()
            },
            cancellationToken);
        return await ReadCreatedIdAsync(
            response,
            "Creating the Forgejo Git stack",
            cancellationToken);
    }

    private static async Task ApplyStackAsync(
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
        Assert.True(
            response.IsSuccessStatusCode,
            $"""
            Applying Forgejo stack {stackId:D} failed with HTTP {(int)response.StatusCode}: {body}
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
            Applying Forgejo stack {stackId:D} did not finish healthy.
            {body}
            {candidate.Output}
            """);
    }

    private static async Task AssertInvalidSignatureRejectedAsync(
        CandidateApplicationProcess candidate,
        Guid stackId,
        string repositoryFullName,
        CancellationToken cancellationToken)
    {
        var payload = JsonSerializer.Serialize(new
        {
            @ref = "refs/heads/main",
            after = new string('0', 40),
            repository = new
            {
                full_name = repositoryFullName
            },
            commits = new[]
            {
                new
                {
                    added = Array.Empty<string>(),
                    modified = new[] { "compose.yml" },
                    removed = Array.Empty<string>()
                }
            }
        });
        using var request = new HttpRequestMessage(
            HttpMethod.Post,
            $"/listener/github/stack/{stackId:D}/deploy")
        {
            Content = new StringContent(
                payload,
                Encoding.UTF8,
                "application/json")
        };
        request.Headers.TryAddWithoutValidation(
            "X-Gitea-Signature",
            new string('0', 64));
        request.Headers.TryAddWithoutValidation(
            "X-Gitea-Event",
            "push");

        using var response = await candidate.Client.SendAsync(
            request,
            cancellationToken);
        Assert.Equal(HttpStatusCode.Unauthorized, response.StatusCode);
    }

    private static async Task WaitForRepositoryCommitAsync(
        CandidateApplicationProcess candidate,
        Guid repositoryId,
        string expectedCommit,
        CancellationToken cancellationToken)
    {
        await WaitUntilAsync(
            async () => string.Equals(
                await GetRepositoryCommitAsync(
                    candidate,
                    repositoryId,
                    cancellationToken),
                expectedCommit,
                StringComparison.OrdinalIgnoreCase),
            $"repository {repositoryId:D} to synchronize commit {expectedCommit}",
            candidate,
            cancellationToken);
    }

    private static async Task WaitForStackRemoteCommitAsync(
        CandidateApplicationProcess candidate,
        Guid stackId,
        string expectedCommit,
        CancellationToken cancellationToken)
    {
        await WaitUntilAsync(
            async () =>
            {
                var stack = await GetStackAsync(
                    candidate,
                    stackId,
                    cancellationToken);
                return string.Equals(
                    GetOptionalString(
                        stack,
                        "stackUpdateState",
                        "recreateStackOnNewCommitState",
                        "remoteCommitSha"),
                    expectedCommit,
                    StringComparison.OrdinalIgnoreCase);
            },
            $"stack {stackId:D} to report remote commit {expectedCommit}",
            candidate,
            cancellationToken);
    }

    private static async Task WaitForWebhookActivityAsync(
        CandidateApplicationProcess candidate,
        Guid stackId,
        string commitSha,
        string expectedStatus,
        string expectedReason,
        CancellationToken cancellationToken)
    {
        var lastBody = string.Empty;
        try
        {
            await WaitUntilAsync(
                async () =>
                {
                    using var response = await candidate.Client.GetAsync(
                        $"/api/v1/activities?resourceId={stackId:D}&eventType=StackWebhookReceived&pageSize=50",
                        cancellationToken);
                    lastBody = await response.Content.ReadAsStringAsync(
                        cancellationToken);
                    Assert.True(
                        response.IsSuccessStatusCode,
                        $"Reading webhook activities failed with HTTP {(int)response.StatusCode}: {lastBody}");

                    using var json = JsonDocument.Parse(lastBody);
                    var activities = json.RootElement
                        .GetProperty("pagedResult")
                        .GetProperty("items")
                        .EnumerateArray();
                    foreach (var activity in activities)
                    {
                        var activityId = activity
                            .GetProperty("id")
                            .GetGuid();
                        using var detailResponse = await candidate.Client.GetAsync(
                            $"/api/v1/activities/{activityId:D}",
                            cancellationToken);
                        lastBody = await detailResponse.Content.ReadAsStringAsync(
                            cancellationToken);
                        Assert.True(
                            detailResponse.IsSuccessStatusCode,
                            $"Reading webhook activity {activityId:D} failed with HTTP {(int)detailResponse.StatusCode}: {lastBody}");

                        using var detailJson = JsonDocument.Parse(lastBody);
                        var info = detailJson.RootElement.GetProperty("info");
                        if (string.Equals(
                                GetOptionalString(info, "commitSha"),
                                commitSha,
                                StringComparison.OrdinalIgnoreCase)
                            && string.Equals(
                                GetOptionalString(info, "status"),
                                expectedStatus,
                                StringComparison.Ordinal)
                            && string.Equals(
                                GetOptionalString(info, "reason"),
                                expectedReason,
                                StringComparison.Ordinal))
                        {
                            return true;
                        }
                    }

                    return false;
                },
                $"stack {stackId:D} webhook activity for commit {commitSha}",
                candidate,
                cancellationToken);
        }
        catch (TimeoutException error)
        {
            throw new TimeoutException(
                $"{error.Message}{Environment.NewLine}Last activities response: {lastBody}",
                error);
        }
    }

    private static async Task<string?> GetRepositoryCommitAsync(
        CandidateApplicationProcess candidate,
        Guid repositoryId,
        CancellationToken cancellationToken)
    {
        using var response = await candidate.Client.GetAsync(
            $"/api/v1/gitRepositories/{repositoryId:D}/refs",
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(
            response.IsSuccessStatusCode,
            $"Reading repository refs failed with HTTP {(int)response.StatusCode}: {body}");

        using var json = JsonDocument.Parse(body);
        foreach (var reference in json.RootElement
                     .GetProperty("refs")
                     .EnumerateArray())
        {
            if (string.Equals(
                    reference.GetProperty("branch").GetString(),
                    "main",
                    StringComparison.Ordinal))
            {
                return reference
                    .GetProperty("resolvedCommitSha")
                    .GetString();
            }
        }

        return null;
    }

    private static async Task<JsonElement> GetStackAsync(
        CandidateApplicationProcess candidate,
        Guid stackId,
        CancellationToken cancellationToken)
    {
        using var response = await candidate.Client.GetAsync(
            $"/api/v1/stacks/{stackId:D}",
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(
            response.IsSuccessStatusCode,
            $"Reading stack {stackId:D} failed with HTTP {(int)response.StatusCode}: {body}");

        using var json = JsonDocument.Parse(body);
        return json.RootElement.Clone();
    }

    private static async Task<Guid> ReadCreatedIdAsync(
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

    private static async Task<string> GetHeadCommitAsync(
        string repositoryDirectory,
        CancellationToken cancellationToken)
    {
        var startInfo = new ProcessStartInfo(
            "git",
            "rev-parse HEAD")
        {
            WorkingDirectory = repositoryDirectory,
            UseShellExecute = false,
            CreateNoWindow = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true
        };
        using var process = Process.Start(startInfo)
            ?? throw new InvalidOperationException("Failed to start git.");
        var output = await process.StandardOutput.ReadToEndAsync(
            cancellationToken);
        var error = await process.StandardError.ReadToEndAsync(
            cancellationToken);
        await process.WaitForExitAsync(cancellationToken);
        Assert.True(
            process.ExitCode == 0,
            $"Resolving the fixture commit failed: {error}");
        return output.Trim();
    }

    private static async Task WaitUntilAsync(
        Func<Task<bool>> condition,
        string description,
        CandidateApplicationProcess candidate,
        CancellationToken cancellationToken)
    {
        var elapsed = Stopwatch.StartNew();
        while (elapsed.Elapsed < TimeSpan.FromSeconds(60))
        {
            cancellationToken.ThrowIfCancellationRequested();
            if (await condition())
                return;

            await Task.Delay(TimeSpan.FromMilliseconds(250), cancellationToken);
        }

        throw new TimeoutException(
            $"Timed out waiting for {description}.{Environment.NewLine}{candidate.Output}");
    }

    private static string GetString(
        JsonElement element,
        params string[] path)
        => GetOptionalString(element, path)
           ?? throw new InvalidOperationException(
               $"JSON path '{string.Join('.', path)}' is null.");

    private static string? GetOptionalString(
        JsonElement element,
        params string[] path)
    {
        var current = element;
        foreach (var property in path)
        {
            if (!current.TryGetProperty(property, out current))
                return null;
        }

        return current.ValueKind is JsonValueKind.Null
            ? null
            : current.GetString();
    }

    private static async Task CleanupAsync(
        CandidateApplicationProcess candidate,
        Guid stackId,
        Guid gitRepositoryId,
        Guid gitAccountId,
        Guid platformId)
    {
        if (stackId != Guid.Empty)
        {
            await TryDeleteAsync(
                candidate.Client,
                "/api/v1/stacks",
                new[] { stackId });
        }

        if (gitRepositoryId != Guid.Empty)
        {
            await TryDeleteAsync(
                candidate.Client,
                "/api/v1/gitRepositories",
                new
                {
                    ids = new[] { gitRepositoryId }
                });
        }

        if (gitAccountId != Guid.Empty)
        {
            await TryDeleteAsync(
                candidate.Client,
                "/api/v1/gitAccounts",
                new
                {
                    ids = new[] { gitAccountId }
                });
        }

        if (platformId != Guid.Empty)
        {
            await TryDeleteAsync(
                candidate.Client,
                "/api/v1/platforms",
                new
                {
                    ids = new[] { platformId }
                });
        }
    }

    private static async Task TryDeleteAsync(
        HttpClient client,
        string path,
        object body)
    {
        try
        {
            using var request = new HttpRequestMessage(
                HttpMethod.Delete,
                path)
            {
                Content = JsonContent.Create(body)
            };
            using var response = await client.SendAsync(
                request,
                CancellationToken.None);
        }
        catch
        {
        }
    }

    private static async Task DeleteDirectoryAsync(string path)
    {
        for (var attempt = 0; attempt < 5; attempt++)
        {
            if (!Directory.Exists(path))
                return;

            try
            {
                foreach (var file in Directory.EnumerateFiles(
                             path,
                             "*",
                             SearchOption.AllDirectories))
                {
                    File.SetAttributes(file, FileAttributes.Normal);
                }

                Directory.Delete(path, recursive: true);
                return;
            }
            catch (IOException)
            {
                if (attempt == 4)
                    return;
            }
            catch (UnauthorizedAccessException)
            {
                if (attempt == 4)
                    return;
            }

            await Task.Delay(TimeSpan.FromMilliseconds(200));
        }
    }

    private static string ComposeFile(string release) =>
        $"""
        services:
          runtime:
            image: busybox:1.36.1
            network_mode: none
            command: ["sh", "-c", "echo citadel-forgejo-release-{release}; exec tail -f /dev/null"]
            stop_grace_period: 1s
            labels:
              citadel.acceptance.release: {release}
        """;
}
