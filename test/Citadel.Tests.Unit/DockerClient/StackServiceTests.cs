using Hosting.DockerClient.Models.Stacks;
using Hosting.DockerClient.Services;

namespace Tests.Unit.DockerClient;

public class StackServiceTests
{
    [Fact]
    public async Task ApplyStreamAsync_SourceBackedStack_UsesOrderedComposeFilesAndProjectDirectory()
    {
        using var temp = new TempDirectory();
        var sourceRoot = Path.Combine(temp.Path, "source");
        var workingDirectory = Path.Combine(sourceRoot, "app");
        var generatedDirectory = Path.Combine(temp.Path, "citadel");
        Directory.CreateDirectory(workingDirectory);
        Directory.CreateDirectory(generatedDirectory);

        var baseCompose = Path.Combine(workingDirectory, "compose.yml");
        var overrideCompose = Path.Combine(workingDirectory, "compose.prod.yml");
        var labelsOverride = Path.Combine(generatedDirectory, "citadel.labels.yml");
        var repoEnvFile = Path.Combine(workingDirectory, ".env");
        File.WriteAllText(baseCompose, "services: {}");
        File.WriteAllText(overrideCompose, "services: {}");
        File.WriteAllText(labelsOverride, "services: {}");
        File.WriteAllText(repoEnvFile, "APP_ENV=repo");

        var executor = new CapturingCommandExecutor();
        var service = CreateStackService(executor);

        var command = new StackApplyCommand(
            PlatformAddress: "http://localhost.docker",
            StackName: "demo",
            ComposeFileContent: null,
            ProjectName: "demo",
            EnvironmentFilePath: null,
            RegistryAuth: null,
            RegistryName: null,
            RegistryHost: null,
            DestroyBeforeDeploy: false,
            EnvironmentVariables: ["APP_ENV=prod"],
            PreDeploy: null,
            PostDeploy: null,
            ServiceNames: null,
            PullImages: false,
            SourceWorkingDirectory: workingDirectory,
            SourceComposeFilePaths: [baseCompose, overrideCompose],
            SourceEnvFilePaths: [repoEnvFile],
            LabelsOverrideFilePath: labelsOverride,
            GeneratedFilesDirectory: generatedDirectory);

        await foreach (var _ in service.ApplyStreamAsync(command, TestContext.Current.CancellationToken))
        {
        }

        var invocation = Assert.Single(executor.Invocations);
        Assert.Equal("docker", invocation.FileName);
        Assert.Equal(workingDirectory, invocation.WorkingDirectory);
        Assert.Equal("prod", invocation.EnvironmentVariables["APP_ENV"]);
        var generatedEnvFile = Path.Combine(generatedDirectory, ".env");
        Assert.False(File.Exists(generatedEnvFile));
        Assert.Equal(
            [
                "compose",
                "--project-directory", workingDirectory,
                "-p", "demo",
                "--env-file", repoEnvFile,
                "--env-file", generatedEnvFile,
                "-f", baseCompose,
                "-f", overrideCompose,
                "-f", labelsOverride,
                "up",
                "-d"
            ],
            invocation.Arguments);
    }

    [Fact]
    public async Task ApplyStreamAsync_SourceBackedDestroyBeforeDeploy_UsesSourceWorkingDirectoryForDown()
    {
        using var temp = new TempDirectory();
        var sourceRoot = Path.Combine(temp.Path, "source");
        var workingDirectory = Path.Combine(sourceRoot, "app");
        var generatedDirectory = Path.Combine(temp.Path, "citadel");
        Directory.CreateDirectory(workingDirectory);
        Directory.CreateDirectory(generatedDirectory);

        var composePath = Path.Combine(workingDirectory, "compose.yml");
        File.WriteAllText(composePath, "services: {}");

        var executor = new CapturingCommandExecutor();
        var service = CreateStackService(executor);

        var command = new StackApplyCommand(
            PlatformAddress: "http://localhost.docker",
            StackName: "demo",
            ComposeFileContent: null,
            ProjectName: "demo",
            EnvironmentFilePath: null,
            RegistryAuth: null,
            RegistryName: null,
            RegistryHost: null,
            DestroyBeforeDeploy: true,
            EnvironmentVariables: ["APP_ENV=prod"],
            PreDeploy: null,
            PostDeploy: null,
            ServiceNames: null,
            PullImages: false,
            SourceWorkingDirectory: workingDirectory,
            SourceComposeFilePaths: [composePath],
            LabelsOverrideFilePath: null,
            GeneratedFilesDirectory: generatedDirectory);

        await foreach (var _ in service.ApplyStreamAsync(command, TestContext.Current.CancellationToken))
        {
        }

        Assert.Collection(
            executor.Invocations,
            down =>
            {
                Assert.Equal(workingDirectory, down.WorkingDirectory);
                Assert.Equal("prod", down.EnvironmentVariables["APP_ENV"]);
                Assert.EndsWith("down", string.Join(' ', down.Arguments));
            },
            up =>
            {
                Assert.Equal(workingDirectory, up.WorkingDirectory);
                Assert.Equal("prod", up.EnvironmentVariables["APP_ENV"]);
                Assert.EndsWith("up -d", string.Join(' ', up.Arguments));
            });
    }

    [Fact]
    public async Task ApplyStreamAsync_Should_Suppress_Missing_Variable_Warnings_During_Down_Only()
    {
        using var temp = new TempDirectory();
        var warning = """time="2026-07-02T14:22:18Z" level=warning msg="The \"stripe_api_key\" variable is not set. Defaulting to a blank string." """;
        var executor = new CapturingCommandExecutor(stdErr: warning);
        var service = CreateStackService(executor);

        var command = new StackApplyCommand(
            PlatformAddress: "http://localhost.docker",
            StackName: "demo",
            ComposeFileContent: "services:\n  app:\n    image: nginx\n    environment:\n      API_KEY: ${stripe_api_key}\n",
            ProjectName: "demo",
            EnvironmentFilePath: null,
            RegistryAuth: null,
            RegistryName: null,
            RegistryHost: null,
            DestroyBeforeDeploy: true,
            EnvironmentVariables: null,
            PreDeploy: null,
            PostDeploy: null,
            ServiceNames: null,
            PullImages: false,
            GeneratedFilesDirectory: temp.Path);

        var results = new List<StackApplyResult>();
        await foreach (var result in service.ApplyStreamAsync(command, TestContext.Current.CancellationToken))
        {
            results.Add(result);
        }

        Assert.Equal(2, executor.Invocations.Count);
        Assert.Equal(1, results.Count(result => result.Type == StackApplyEventType.StdErr && result.Message == warning));
        Assert.Equal("up -d", string.Join(' ', executor.Invocations[1].Arguments[^2..]));
    }

    [Fact]
    public async Task ApplyStreamAsync_Should_Query_Compose_Status_After_Successful_Apply()
    {
        using var temp = new TempDirectory();
        var workingDirectory = Path.Combine(temp.Path, "source", "app");
        var generatedDirectory = Path.Combine(temp.Path, "citadel");
        Directory.CreateDirectory(workingDirectory);
        Directory.CreateDirectory(generatedDirectory);

        var composePath = Path.Combine(workingDirectory, "compose.yml");
        File.WriteAllText(composePath, "services:\n  minio:\n    image: minio/minio\n  minio-init:\n    image: minio/mc\n");

        var executor = new CapturingCommandExecutor(executeStdOut: """
            [
              { "Service": "minio", "State": "running", "Health": "", "ExitCode": 0 },
              { "Service": "minio-init", "State": "exited", "Health": "", "ExitCode": 1 }
            ]
            """);
        var settleDelays = new List<TimeSpan>();
        var service = CreateStackService(executor, settleDelays);

        var command = new StackApplyCommand(
            PlatformAddress: "http://localhost.docker",
            StackName: "demo",
            ComposeFileContent: null,
            ProjectName: "demo",
            EnvironmentFilePath: null,
            RegistryAuth: null,
            RegistryName: null,
            RegistryHost: null,
            DestroyBeforeDeploy: false,
            EnvironmentVariables: null,
            PreDeploy: null,
            PostDeploy: null,
            ServiceNames: null,
            PullImages: false,
            SourceWorkingDirectory: workingDirectory,
            SourceComposeFilePaths: [composePath],
            GeneratedFilesDirectory: generatedDirectory);

        var results = new List<StackApplyResult>();
        await foreach (var result in service.ApplyStreamAsync(command, TestContext.Current.CancellationToken))
        {
            results.Add(result);
        }

        var status = Assert.Single(results, result => result.StackStatus is not null);
        Assert.Equal("Degraded", status.StackStatus);
        Assert.Equal(TimeSpan.FromSeconds(4), Assert.Single(settleDelays));
        var invocation = Assert.Single(executor.ExecuteInvocations);
        Assert.Equal("docker", invocation.FileName);
        Assert.Equal(workingDirectory, invocation.WorkingDirectory);
        Assert.Equal(
            [
                "compose",
                "--project-directory", workingDirectory,
                "-p", "demo",
                "-f", composePath,
                "ps",
                "--all",
                "--format", "json"
            ],
            invocation.Arguments);
    }

    [Fact]
    public async Task ApplyStreamAsync_Should_Mark_Compose_Status_Degraded_For_Mixed_Running_And_Stopped()
    {
        var results = await ApplyAndCollectStatusAsync("""
            [
              { "Service": "app", "State": "running", "Health": "", "ExitCode": 0 },
              { "Service": "init", "State": "exited", "Health": "", "ExitCode": 0 }
            ]
            """);

        Assert.Equal("Degraded", Assert.Single(results, result => result.StackStatus is not null).StackStatus);
    }

    [Fact]
    public async Task ApplyStreamAsync_Should_Mark_Compose_Status_Stopped_When_All_Containers_Are_Stopped()
    {
        var results = await ApplyAndCollectStatusAsync("""
            [
              { "Service": "app", "State": "exited", "Health": "", "ExitCode": 0 },
              { "Service": "init", "State": "exited", "Health": "", "ExitCode": 0 }
            ]
            """);

        Assert.Equal("Stopped", Assert.Single(results, result => result.StackStatus is not null).StackStatus);
    }

    [Fact]
    public async Task ApplyStreamAsync_Should_Delete_Generated_Docker_Config_After_Apply()
    {
        using var temp = new TempDirectory();
        var generatedDirectory = Path.Combine(temp.Path, "citadel");
        Directory.CreateDirectory(generatedDirectory);

        var executor = new CapturingCommandExecutor();
        var service = CreateStackService(executor);

        var command = new StackApplyCommand(
            PlatformAddress: "http://localhost.docker",
            StackName: "demo",
            ComposeFileContent: "services:\n  app:\n    image: nginx",
            ProjectName: "demo",
            EnvironmentFilePath: null,
            RegistryAuth: "registry-token",
            RegistryName: "ghcr",
            RegistryHost: "https://ghcr.io",
            DestroyBeforeDeploy: false,
            EnvironmentVariables: null,
            PreDeploy: null,
            PostDeploy: null,
            ServiceNames: null,
            PullImages: false,
            GeneratedFilesDirectory: generatedDirectory);

        await foreach (var _ in service.ApplyStreamAsync(command, TestContext.Current.CancellationToken))
        {
        }

        var invocation = Assert.Single(executor.Invocations);
        Assert.NotNull(invocation.DockerConfigDirectory);
        Assert.False(Directory.Exists(invocation.DockerConfigDirectory));
    }

    [Fact]
    public async Task ApplyStreamAsync_Should_Write_Mounted_Secret_Files_And_Use_Secret_Override()
    {
        using var temp = new TempDirectory();
        var workingDirectory = Path.Combine(temp.Path, "source", "app");
        var generatedDirectory = Path.Combine(temp.Path, "citadel");
        Directory.CreateDirectory(workingDirectory);
        Directory.CreateDirectory(generatedDirectory);

        var composePath = Path.Combine(workingDirectory, "compose.yml");
        File.WriteAllText(composePath, "services:\n  db:\n    image: postgres\n  web:\n    image: nginx\n");
        var staleSecretDirectory = Path.Combine(generatedDirectory, "secrets");
        Directory.CreateDirectory(staleSecretDirectory);
        var staleSecretFile = Path.Combine(staleSecretDirectory, "OLD_SECRET");
        File.WriteAllText(staleSecretFile, "old-secret");

        var executor = new CapturingCommandExecutor();
        var service = CreateStackService(executor);

        var command = new StackApplyCommand(
            PlatformAddress: "http://localhost.docker",
            StackName: "demo",
            ComposeFileContent: null,
            ProjectName: "demo",
            EnvironmentFilePath: null,
            RegistryAuth: null,
            RegistryName: null,
            RegistryHost: null,
            DestroyBeforeDeploy: false,
            EnvironmentVariables: null,
            PreDeploy: null,
            PostDeploy: null,
            ServiceNames: null,
            PullImages: false,
            SourceWorkingDirectory: workingDirectory,
            SourceComposeFilePaths: [composePath],
            GeneratedFilesDirectory: generatedDirectory,
            SecretFiles:
            [
                new StackSecretFile(
                    Name: "POSTGRES_PASSWORD",
                    TargetPath: "/run/secrets/postgres_password",
                    Content: "super-secret")
            ],
            SecretTargetServiceNames: ["db", "web"]);

        await foreach (var _ in service.ApplyStreamAsync(command, TestContext.Current.CancellationToken))
        {
        }

        var secretFile = Assert.Single(Directory.GetFiles(Path.Combine(generatedDirectory, "secrets"), "POSTGRES_PASSWORD", SearchOption.AllDirectories));
        var secretOverride = Assert.Single(Directory.GetFiles(generatedDirectory, "citadel.secrets*.yml"));
        Assert.False(File.Exists(staleSecretFile));
        Assert.True(File.Exists(secretFile));
        Assert.Equal("super-secret", File.ReadAllText(secretFile));
        Assert.True(File.Exists(secretOverride));

        var overrideContent = File.ReadAllText(secretOverride);
        Assert.Contains("target: '/run/secrets/postgres_password'", overrideContent);
        Assert.Contains("read_only: true", overrideContent);
        Assert.Contains("super-secret", File.ReadAllText(secretFile));
        Assert.DoesNotContain("super-secret", overrideContent);
        Assert.Contains($"source: '{secretFile}'", overrideContent);

        var invocation = Assert.Single(executor.Invocations);
        Assert.Equal(
            [
                "compose",
                "--project-directory", workingDirectory,
                "-p", "demo",
                "-f", composePath,
                "-f", secretOverride,
                "up",
                "-d"
            ],
            invocation.Arguments);
    }

    [Fact]
    public async Task ApplyStreamAsync_Should_Delete_Previous_Mounted_Secret_Files_When_No_Secrets_Are_Applied()
    {
        using var temp = new TempDirectory();
        var workingDirectory = Path.Combine(temp.Path, "source", "app");
        var generatedDirectory = Path.Combine(temp.Path, "citadel");
        Directory.CreateDirectory(workingDirectory);
        Directory.CreateDirectory(generatedDirectory);

        var composePath = Path.Combine(workingDirectory, "compose.yml");
        File.WriteAllText(composePath, "services:\n  app:\n    image: nginx\n");
        var secretsDirectory = Path.Combine(generatedDirectory, "secrets");
        Directory.CreateDirectory(secretsDirectory);
        File.WriteAllText(Path.Combine(secretsDirectory, "OLD_SECRET"), "old-secret");
        var secretOverride = Path.Combine(generatedDirectory, "citadel.secrets.old.yml");
        File.WriteAllText(secretOverride, "services: {}\n");

        var executor = new CapturingCommandExecutor();
        var service = CreateStackService(executor);

        var command = new StackApplyCommand(
            PlatformAddress: "http://localhost.docker",
            StackName: "demo",
            ComposeFileContent: null,
            ProjectName: "demo",
            EnvironmentFilePath: null,
            RegistryAuth: null,
            RegistryName: null,
            RegistryHost: null,
            DestroyBeforeDeploy: false,
            EnvironmentVariables: null,
            PreDeploy: null,
            PostDeploy: null,
            ServiceNames: null,
            PullImages: false,
            SourceWorkingDirectory: workingDirectory,
            SourceComposeFilePaths: [composePath],
            GeneratedFilesDirectory: generatedDirectory);

        await foreach (var _ in service.ApplyStreamAsync(command, TestContext.Current.CancellationToken))
        {
        }

        Assert.False(Directory.Exists(secretsDirectory));
        Assert.False(File.Exists(secretOverride));
        var invocation = Assert.Single(executor.Invocations);
        Assert.DoesNotContain(secretOverride, invocation.Arguments);
    }

    [Fact]
    public async Task ApplyStreamAsync_Should_Preserve_Previous_Mounted_Secret_Files_When_Apply_Fails()
    {
        using var temp = new TempDirectory();
        var workingDirectory = Path.Combine(temp.Path, "source", "app");
        var generatedDirectory = Path.Combine(temp.Path, "citadel");
        Directory.CreateDirectory(workingDirectory);
        Directory.CreateDirectory(generatedDirectory);

        var composePath = Path.Combine(workingDirectory, "compose.yml");
        File.WriteAllText(composePath, "services:\n  db:\n    image: postgres\n");
        var previousSecretDirectory = Path.Combine(generatedDirectory, "secrets", "previous");
        Directory.CreateDirectory(previousSecretDirectory);
        var previousSecretFile = Path.Combine(previousSecretDirectory, "POSTGRES_PASSWORD");
        File.WriteAllText(previousSecretFile, "old-secret");
        var previousOverride = Path.Combine(generatedDirectory, "citadel.secrets.previous.yml");
        File.WriteAllText(previousOverride, "services: {}\n");

        var executor = new CapturingCommandExecutor(exitCode: 1);
        var service = CreateStackService(executor);

        var command = new StackApplyCommand(
            PlatformAddress: "http://localhost.docker",
            StackName: "demo",
            ComposeFileContent: null,
            ProjectName: "demo",
            EnvironmentFilePath: null,
            RegistryAuth: null,
            RegistryName: null,
            RegistryHost: null,
            DestroyBeforeDeploy: false,
            EnvironmentVariables: null,
            PreDeploy: null,
            PostDeploy: null,
            ServiceNames: null,
            PullImages: false,
            SourceWorkingDirectory: workingDirectory,
            SourceComposeFilePaths: [composePath],
            GeneratedFilesDirectory: generatedDirectory,
            SecretFiles:
            [
                new StackSecretFile(
                    Name: "POSTGRES_PASSWORD",
                    TargetPath: "/run/secrets/postgres_password",
                    Content: "new-secret")
            ],
            SecretTargetServiceNames: ["db"]);

        await foreach (var _ in service.ApplyStreamAsync(command, TestContext.Current.CancellationToken))
        {
        }

        Assert.True(File.Exists(previousSecretFile));
        Assert.Equal("old-secret", File.ReadAllText(previousSecretFile));
        Assert.True(File.Exists(previousOverride));
        Assert.True(Directory.GetFiles(Path.Combine(generatedDirectory, "secrets"), "POSTGRES_PASSWORD", SearchOption.AllDirectories).Length >= 2);
    }

    private static async Task<List<StackApplyResult>> ApplyAndCollectStatusAsync(string composePsJson)
    {
        using var temp = new TempDirectory();
        var workingDirectory = Path.Combine(temp.Path, "source", "app");
        var generatedDirectory = Path.Combine(temp.Path, "citadel");
        Directory.CreateDirectory(workingDirectory);
        Directory.CreateDirectory(generatedDirectory);

        var composePath = Path.Combine(workingDirectory, "compose.yml");
        File.WriteAllText(composePath, "services:\n  app:\n    image: nginx\n");

        var executor = new CapturingCommandExecutor(executeStdOut: composePsJson);
        var service = CreateStackService(executor);

        var command = new StackApplyCommand(
            PlatformAddress: "http://localhost.docker",
            StackName: "demo",
            ComposeFileContent: null,
            ProjectName: "demo",
            EnvironmentFilePath: null,
            RegistryAuth: null,
            RegistryName: null,
            RegistryHost: null,
            DestroyBeforeDeploy: false,
            EnvironmentVariables: null,
            PreDeploy: null,
            PostDeploy: null,
            ServiceNames: null,
            PullImages: false,
            SourceWorkingDirectory: workingDirectory,
            SourceComposeFilePaths: [composePath],
            GeneratedFilesDirectory: generatedDirectory);

        var results = new List<StackApplyResult>();
        await foreach (var result in service.ApplyStreamAsync(command, TestContext.Current.CancellationToken))
        {
            results.Add(result);
        }

        return results;
    }

    private static StackService CreateStackService(
        CapturingCommandExecutor executor,
        List<TimeSpan>? settleDelays = null)
        => new(
            executor,
            (delay, _) =>
            {
                settleDelays?.Add(delay);
                return ValueTask.CompletedTask;
            });

    private sealed class CapturingCommandExecutor(
        int exitCode = 0,
        string? stdOut = null,
        string? stdErr = null,
        int executeExitCode = 0,
        string? executeStdOut = null,
        string? executeStdErr = null) : ICommandExecutor
    {
        public List<Invocation> Invocations { get; } = [];
        public List<Invocation> ExecuteInvocations { get; } = [];

        public Task<ProcessExecutionResult> ExecuteAsync(
            string fileName,
            IEnumerable<string> arguments,
            IDictionary<string, string>? environmentVariables = null,
            string? workingDirectory = null,
            CancellationToken cancellationToken = default)
        {
            ExecuteInvocations.Add(new Invocation(
                fileName,
                [.. arguments],
                new Dictionary<string, string>(environmentVariables ?? new Dictionary<string, string>()),
                workingDirectory,
                DockerConfigDirectory: null));

            return Task.FromResult(new ProcessExecutionResult(
                executeExitCode,
                executeStdOut ?? string.Empty,
                executeStdErr ?? string.Empty));
        }

        public async IAsyncEnumerable<ProcessOutput> StreamAsync(
            string fileName,
            IEnumerable<string> arguments,
            IDictionary<string, string>? environmentVariables = null,
            string? workingDirectory = null,
            string? dockerConfigDirectory = null,
            [System.Runtime.CompilerServices.EnumeratorCancellation] CancellationToken cancellationToken = default)
        {
            Invocations.Add(new Invocation(
                fileName,
                [.. arguments],
                new Dictionary<string, string>(environmentVariables ?? new Dictionary<string, string>()),
                workingDirectory,
                dockerConfigDirectory));
            if (!string.IsNullOrWhiteSpace(stdOut))
            {
                yield return new ProcessOutput(stdOut, null);
            }

            if (!string.IsNullOrWhiteSpace(stdErr))
            {
                yield return new ProcessOutput(null, stdErr);
            }

            yield return new ProcessOutput(null, null, exitCode);
            await Task.CompletedTask;
        }
    }

    private sealed record Invocation(
        string FileName,
        string[] Arguments,
        IReadOnlyDictionary<string, string> EnvironmentVariables,
        string? WorkingDirectory,
        string? DockerConfigDirectory);

    private sealed class TempDirectory : IDisposable
    {
        public string Path { get; } = System.IO.Path.Combine(System.IO.Path.GetTempPath(), Guid.NewGuid().ToString("N"));

        public TempDirectory()
        {
            Directory.CreateDirectory(Path);
        }

        public void Dispose()
        {
            if (Directory.Exists(Path))
                Directory.Delete(Path, recursive: true);
        }
    }
}
