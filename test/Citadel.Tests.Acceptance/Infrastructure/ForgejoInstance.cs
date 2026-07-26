using DotNet.Testcontainers.Builders;
using DotNet.Testcontainers.Containers;
using System.Diagnostics;
using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Text;
using System.Text.Json;

namespace Tests.Acceptance.Infrastructure;

internal sealed class ForgejoInstance : IAsyncDisposable
{
    private const ushort HttpPort = 3000;
    private const string Image = "codeberg.org/forgejo/forgejo:16.0.1-rootless";
    private const string Username = "citadel-acceptance";
    private const string Password = "CitadelAcceptance2026";
    private const string RepositoryName = "citadel-acceptance";

    private readonly IContainer container;
    private readonly HttpClient client;

    private ForgejoInstance(
        IContainer container,
        HttpClient client,
        Uri baseAddress,
        string accessToken)
    {
        this.container = container;
        this.client = client;
        BaseAddress = baseAddress;
        AccessToken = accessToken;
    }

    public Uri BaseAddress { get; }

    public string AccessToken { get; }

    public string CloneUrl =>
        new Uri(BaseAddress, $"{Username}/{RepositoryName}.git").ToString();

    public string RepositoryFullName => $"{Username}/{RepositoryName}";

    public static async Task<ForgejoInstance> StartAsync(
        CancellationToken cancellationToken)
    {
        var container = new ContainerBuilder(Image)
            .WithPortBinding(HttpPort, assignRandomHostPort: true)
            .WithEnvironment("FORGEJO__database__DB_TYPE", "sqlite3")
            .WithEnvironment(
                "FORGEJO__database__PATH",
                "/var/lib/gitea/data/forgejo.db")
            .WithEnvironment("FORGEJO__security__INSTALL_LOCK", "true")
            .WithEnvironment("FORGEJO__service__DISABLE_REGISTRATION", "true")
            .WithEnvironment("FORGEJO__repository__DEFAULT_BRANCH", "main")
            .WithEnvironment("FORGEJO__webhook__ALLOWED_HOST_LIST", "*")
            .WithWaitStrategy(
                Wait.ForUnixContainer().UntilHttpRequestIsSucceeded(
                    request => request
                        .ForPort(HttpPort)
                        .ForPath("/api/healthz")))
            .Build();

        try
        {
            await container.StartAsync(cancellationToken);
            var baseAddress = new Uri(
                $"http://127.0.0.1:{container.GetMappedPublicPort(HttpPort)}");
            var client = new HttpClient
            {
                BaseAddress = baseAddress,
                Timeout = TimeSpan.FromSeconds(30)
            };

            var createUser = await container.ExecAsync(
                [
                    "forgejo",
                    "admin",
                    "user",
                    "create",
                    "--username",
                    Username,
                    "--password",
                    Password,
                    "--email",
                    "citadel-acceptance@example.test",
                    "--admin",
                    "--must-change-password=false"
                ],
                cancellationToken);
            Assert.Equal(0L, createUser.ExitCode ?? -1L);

            client.DefaultRequestHeaders.Authorization =
                new AuthenticationHeaderValue(
                    "Basic",
                    Convert.ToBase64String(
                        Encoding.UTF8.GetBytes($"{Username}:{Password}")));

            var token = await CreateAccessTokenAsync(
                client,
                cancellationToken);
            await CreateRepositoryAsync(client, cancellationToken);

            return new ForgejoInstance(
                container,
                client,
                baseAddress,
                token);
        }
        catch
        {
            await container.DisposeAsync();
            throw;
        }
    }

    public async Task InitializeLocalRepositoryAsync(
        string repositoryDirectory,
        string composeFile,
        CancellationToken cancellationToken)
    {
        Directory.CreateDirectory(repositoryDirectory);
        await RunGitAsync(
            repositoryDirectory,
            cancellationToken,
            "init",
            "--initial-branch=main");
        await RunGitAsync(
            repositoryDirectory,
            cancellationToken,
            "config",
            "user.name",
            "Citadel Acceptance");
        await RunGitAsync(
            repositoryDirectory,
            cancellationToken,
            "config",
            "user.email",
            "citadel-acceptance@example.test");

        await File.WriteAllTextAsync(
            Path.Combine(repositoryDirectory, "compose.yml"),
            composeFile,
            cancellationToken);
        await File.WriteAllTextAsync(
            Path.Combine(repositoryDirectory, "README.md"),
            "Citadel Forgejo acceptance fixture.",
            cancellationToken);
        await RunGitAsync(repositoryDirectory, cancellationToken, "add", ".");
        await RunGitAsync(
            repositoryDirectory,
            cancellationToken,
            "commit",
            "-m",
            "Initial stack");
        await RunGitAsync(
            repositoryDirectory,
            cancellationToken,
            "remote",
            "add",
            "origin",
            GetAuthenticatedCloneUrl());
        await RunGitAsync(
            repositoryDirectory,
            cancellationToken,
            "push",
            "--set-upstream",
            "origin",
            "main");
    }

    public async Task<string> CommitAndPushAsync(
        string repositoryDirectory,
        string relativePath,
        string content,
        string message,
        CancellationToken cancellationToken)
    {
        var filePath = Path.Combine(
            repositoryDirectory,
            relativePath.Replace('/', Path.DirectorySeparatorChar));
        Directory.CreateDirectory(
            Path.GetDirectoryName(filePath)
            ?? throw new InvalidOperationException("Commit path has no parent."));
        await File.WriteAllTextAsync(filePath, content, cancellationToken);
        await RunGitAsync(
            repositoryDirectory,
            cancellationToken,
            "add",
            relativePath);
        await RunGitAsync(
            repositoryDirectory,
            cancellationToken,
            "commit",
            "-m",
            message);
        var commit = await RunGitAsync(
            repositoryDirectory,
            cancellationToken,
            "rev-parse",
            "HEAD");
        await RunGitAsync(
            repositoryDirectory,
            cancellationToken,
            "push",
            "origin",
            "main");
        return commit.Trim();
    }

    public async Task CreateWebhookAsync(
        Uri target,
        string secret,
        CancellationToken cancellationToken)
    {
        using var response = await client.PostAsJsonAsync(
            $"/api/v1/repos/{RepositoryFullName}/hooks",
            new
            {
                type = "gitea",
                active = true,
                branchFilter = "main",
                events = new[] { "push" },
                config = new Dictionary<string, string>
                {
                    ["url"] = target.ToString(),
                    ["content_type"] = "json",
                    ["secret"] = secret
                }
            },
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(
            response.IsSuccessStatusCode,
            $"Creating the Forgejo webhook failed with HTTP {(int)response.StatusCode}: {body}");
    }

    public ValueTask DisposeAsync()
    {
        client.Dispose();
        return container.DisposeAsync();
    }

    private static async Task<string> CreateAccessTokenAsync(
        HttpClient client,
        CancellationToken cancellationToken)
    {
        using var response = await client.PostAsJsonAsync(
            $"/api/v1/users/{Username}/tokens",
            new
            {
                name = "citadel-acceptance",
                scopes = new[] { "read:repository" }
            },
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(
            response.IsSuccessStatusCode,
            $"Creating the Forgejo token failed with HTTP {(int)response.StatusCode}: {body}");

        using var json = JsonDocument.Parse(body);
        var root = json.RootElement;
        var token = root.TryGetProperty("sha1", out var sha1)
            ? sha1.GetString()
            : root.TryGetProperty("token", out var value)
                ? value.GetString()
                : null;
        Assert.False(string.IsNullOrWhiteSpace(token));
        return token!;
    }

    private static async Task CreateRepositoryAsync(
        HttpClient client,
        CancellationToken cancellationToken)
    {
        using var response = await client.PostAsJsonAsync(
            "/api/v1/user/repos",
            new
            {
                name = RepositoryName,
                description = "Citadel Forgejo compatibility fixture",
                @private = true,
                autoInit = false,
                defaultBranch = "main"
            },
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(
            response.IsSuccessStatusCode,
            $"Creating the Forgejo repository failed with HTTP {(int)response.StatusCode}: {body}");
    }

    private string GetAuthenticatedCloneUrl()
    {
        var builder = new UriBuilder(CloneUrl)
        {
            UserName = Username,
            Password = Password
        };
        return builder.Uri.ToString();
    }

    private static async Task<string> RunGitAsync(
        string workingDirectory,
        CancellationToken cancellationToken,
        params string[] arguments)
    {
        var startInfo = new ProcessStartInfo("git")
        {
            WorkingDirectory = workingDirectory,
            UseShellExecute = false,
            CreateNoWindow = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true
        };
        startInfo.Environment["GIT_TERMINAL_PROMPT"] = "0";
        foreach (var argument in arguments)
            startInfo.ArgumentList.Add(argument);

        using var process = Process.Start(startInfo)
            ?? throw new InvalidOperationException("Failed to start git.");
        var standardOutput = process.StandardOutput.ReadToEndAsync(
            cancellationToken);
        var standardError = process.StandardError.ReadToEndAsync(
            cancellationToken);
        await process.WaitForExitAsync(cancellationToken);
        var output = await standardOutput;
        var error = await standardError;

        Assert.True(
            process.ExitCode == 0,
            $"""
            git {FormatGitArguments(arguments)} failed with exit code {process.ExitCode}.
            {output}
            {error}
            """);
        return output;
    }

    private static string FormatGitArguments(IEnumerable<string> arguments) =>
        string.Join(
            ' ',
            arguments.Select(argument =>
            {
                if (!Uri.TryCreate(argument, UriKind.Absolute, out var uri)
                    || string.IsNullOrWhiteSpace(uri.UserInfo))
                {
                    return argument;
                }

                return new UriBuilder(uri)
                {
                    UserName = "redacted",
                    Password = "redacted"
                }.Uri.ToString();
            }));
}
