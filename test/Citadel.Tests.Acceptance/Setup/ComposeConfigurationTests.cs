using System.Diagnostics;
using System.Text.Json;
using Application.Configs;

namespace Tests.Acceptance.Setup;

public sealed class ComposeConfigurationTests
{
    private static readonly string[] RequiredProductionSettings =
    [
        "CITADEL_IMAGE_TAG",
        "Transport__Mode",
        "Transport__PublicUrl",
        "EdgeAgent__PublicGrpcUrl",
        "AllowedHosts",
        "Transport__ForwardedHeaders__KnownProxies",
        "Jwt__Issuer",
        "Jwt__Audience",
        "PG_USER",
        "PG_PASSWORD",
        "PG_DATABASE"
    ];

    [Fact]
    public void ProductionEnvironmentTemplate_ShouldExposeTheSupportedContract()
    {
        var path = FixturePath(".env.example");
        var environment = ReadEnvironment(path);
        var template = File.ReadAllText(path);

        Assert.All(
            RequiredProductionSettings,
            key => Assert.True(
                environment.ContainsKey(key),
                $".env.example does not define {key}."));
        Assert.Equal(
            RequiredProductionSettings.Order(StringComparer.Ordinal),
            environment.Keys.Order(StringComparer.Ordinal));
        Assert.Equal("ReverseProxy", environment["Transport__Mode"]);
        Assert.Contains(
            new Uri(new AutomationOptions().InternalBaseUrl).Host,
            environment["AllowedHosts"].Split(';'),
            StringComparer.OrdinalIgnoreCase);
        Assert.StartsWith("replace-", environment["PG_PASSWORD"]);
        Assert.Equal(
            "replace-with-release-version",
            environment["CITADEL_IMAGE_TAG"]);
        Assert.DoesNotContain("CITADEL_IMAGE_TAG=latest", template);
        Assert.Contains("# PG_HOST=pg_db", template);
        Assert.Contains("# Jwt__Key=", template);
        Assert.Contains(
            "# JobConfiguration__MonitoringInterval=10",
            template);
        Assert.Contains("# Transport__Certificate__Path=", template);
        Assert.Contains("# Bootstrap__AdminPasswordFile=", template);
        Assert.Contains("# AgentTransport__CaCertificatePath=", template);
    }

    [Fact]
    public async Task ComposeFiles_ShouldRenderProductionDevelopmentAndDirectModes()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var directory = Path.Combine(
            Path.GetTempPath(),
            $"citadel-compose-acceptance-{Guid.NewGuid():N}");
        Directory.CreateDirectory(directory);

        try
        {
            foreach (var fileName in new[]
                     {
                         ".env.example",
                         "docker-compose.yml",
                         "docker-compose.override.yml",
                         "docker-compose.local.yml",
                         "docker-compose.direct-tls.yml"
                     })
            {
                File.Copy(
                    FixturePath(fileName),
                    Path.Combine(directory, fileName));
            }

            File.Copy(
                Path.Combine(directory, ".env.example"),
                Path.Combine(directory, ".env"));

            using (var production = await RenderComposeAsync(
                       directory,
                       ["docker-compose.yml"],
                       cancellationToken))
            {
                var server = GetServer(production);
                var environment = server.GetProperty("environment");

                Assert.Equal(
                    "ReverseProxy",
                    environment.GetProperty("Transport__Mode").GetString());
                Assert.Equal(
                    "Host=pg_db;Database=citadel;Username=citadel;Password=replace-with-a-strong-database-password",
                    environment
                        .GetProperty("ConnectionStrings__Postgres")
                        .GetString());
                Assert.False(server.TryGetProperty("ports", out _));
            }

            using (var development = await RenderComposeAsync(
                       directory,
                       [
                           "docker-compose.yml",
                           "docker-compose.override.yml"
                       ],
                       cancellationToken))
            {
                var server = GetServer(development);
                var environment = server.GetProperty("environment");

                Assert.Equal(
                    "Disabled",
                    environment.GetProperty("Transport__Mode").GetString());
                Assert.Equal(
                    string.Empty,
                    environment
                        .GetProperty(
                            "Transport__ForwardedHeaders__KnownProxies")
                        .GetString());
                Assert.Equal(
                    "localhost;host.docker.internal",
                    environment.GetProperty("AllowedHosts").GetString());
                Assert.Equal(2, server.GetProperty("ports").GetArrayLength());
            }

            using (var local = await RenderComposeAsync(
                       directory,
                       [
                           "docker-compose.yml",
                           "docker-compose.local.yml"
                       ],
                       cancellationToken))
            {
                var server = GetServer(local);
                var environment = server.GetProperty("environment");

                Assert.Equal(
                    "Disabled",
                    environment.GetProperty("Transport__Mode").GetString());
                Assert.Equal(
                    "http://localhost:8000",
                    environment.GetProperty("Transport__PublicUrl").GetString());
                Assert.Equal(
                    "http://localhost:8000",
                    environment.GetProperty("Jwt__Issuer").GetString());
                var port = Assert.Single(server.GetProperty("ports").EnumerateArray());
                Assert.Equal("127.0.0.1", port.GetProperty("host_ip").GetString());
                Assert.Equal("8000", port.GetProperty("published").GetString());
            }

            var directEnvironment = ReadEnvironment(
                Path.Combine(directory, ".env"));
            directEnvironment["Transport__Mode"] = "Direct";
            directEnvironment.Remove(
                "Transport__ForwardedHeaders__KnownProxies");
            directEnvironment.Remove(
                "Transport__ForwardedHeaders__KnownNetworks");
            directEnvironment.Remove(
                "Transport__ForwardedHeaders__ForwardLimit");
            directEnvironment["Transport__Certificate__Path"] =
                "/etc/citadel/tls/core-fullchain.pem";
            directEnvironment["Transport__Certificate__PrivateKeyPath"] =
                "/etc/citadel/tls/core-key.pem";
            directEnvironment["CITADEL_TLS_HOST_DIRECTORY"] = "./tls";
            WriteEnvironment(
                Path.Combine(directory, ".env"),
                directEnvironment);
            Directory.CreateDirectory(Path.Combine(directory, "tls"));

            using var direct = await RenderComposeAsync(
                directory,
                [
                    "docker-compose.yml",
                    "docker-compose.direct-tls.yml"
                ],
                cancellationToken);
            var directServer = GetServer(direct);
            Assert.Equal(
                "Direct",
                directServer
                    .GetProperty("environment")
                    .GetProperty("Transport__Mode")
                    .GetString());
            Assert.Equal(
                2,
                directServer.GetProperty("ports").GetArrayLength());
            Assert.Contains(
                "https://localhost:",
                directServer
                    .GetProperty("healthcheck")
                    .GetProperty("test")[1]
                    .GetString());
        }
        finally
        {
            if (Directory.Exists(directory))
            {
                Directory.Delete(directory, recursive: true);
            }
        }
    }

    [Fact]
    public void ProductionImage_ShouldRetainItsHealthProbeAndConserveGcMemory()
    {
        var dependencies = File.ReadAllText(
            FixturePath("install-server-deps.sh"));
        var webApiProject = File.ReadAllText(
            FixturePath("Citadel.WebApi.csproj"));
        var dockerfile = File.ReadAllText(
            FixturePath("Dockerfile"));

        Assert.Contains("apk add --no-cache curl", dependencies);
        Assert.DoesNotContain("apk del curl", dependencies);
        Assert.Contains(
            "<ServerGarbageCollection>false</ServerGarbageCollection>",
            webApiProject);
        Assert.Contains(
            "<RuntimeHostConfigurationOption Include=\"System.GC.ConserveMemory\" Value=\"7\" />",
            webApiProject);
        Assert.DoesNotContain(
            "<PreserveCompilationContext>true</PreserveCompilationContext>",
            webApiProject);
        Assert.Contains("rm -rf /app/publish/refs", dockerfile);
        Assert.Contains("/app/publish/*.pdb", dockerfile);
    }

    private static string FixturePath(string fileName)
        => Path.Combine(
            AppContext.BaseDirectory,
            "Fixtures",
            "Compose",
            fileName);

    private static Dictionary<string, string> ReadEnvironment(string path)
    {
        var environment = new Dictionary<string, string>(
            StringComparer.Ordinal);
        foreach (var sourceLine in File.ReadLines(path))
        {
            var line = sourceLine.Trim();
            if (line.Length == 0 || line.StartsWith('#'))
            {
                continue;
            }

            var separator = line.IndexOf('=');
            Assert.True(
                separator > 0,
                $"Invalid environment entry: {sourceLine}");
            var key = line[..separator].Trim();
            var value = line[(separator + 1)..];
            Assert.True(
                environment.TryAdd(key, value),
                $"Duplicate environment setting: {key}");
        }

        return environment;
    }

    private static void WriteEnvironment(
        string path,
        IReadOnlyDictionary<string, string> environment)
        => File.WriteAllLines(
            path,
            environment.Select(pair => $"{pair.Key}={pair.Value}"));

    private static JsonElement GetServer(JsonDocument document)
        => document.RootElement
            .GetProperty("services")
            .GetProperty("server");

    private static async Task<JsonDocument> RenderComposeAsync(
        string workingDirectory,
        IReadOnlyList<string> composeFiles,
        CancellationToken cancellationToken)
    {
        var startInfo = new ProcessStartInfo("docker")
        {
            WorkingDirectory = workingDirectory,
            UseShellExecute = false,
            CreateNoWindow = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true
        };
        startInfo.ArgumentList.Add("compose");
        foreach (var composeFile in composeFiles)
        {
            startInfo.ArgumentList.Add("-f");
            startInfo.ArgumentList.Add(composeFile);
        }

        startInfo.ArgumentList.Add("config");
        startInfo.ArgumentList.Add("--format");
        startInfo.ArgumentList.Add("json");

        using var process = Process.Start(startInfo);
        Assert.NotNull(process);
        var standardOutput = process.StandardOutput.ReadToEndAsync(
            cancellationToken);
        var standardError = process.StandardError.ReadToEndAsync(
            cancellationToken);
        await process.WaitForExitAsync(cancellationToken);
        var output = await standardOutput;
        var error = await standardError;

        Assert.True(
            process.ExitCode == 0,
            $"docker compose config failed with exit code {process.ExitCode}:{Environment.NewLine}{error}");
        return JsonDocument.Parse(output);
    }
}
