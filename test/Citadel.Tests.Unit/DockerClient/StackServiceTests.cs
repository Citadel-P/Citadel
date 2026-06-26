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
        File.WriteAllText(baseCompose, "services: {}");
        File.WriteAllText(overrideCompose, "services: {}");
        File.WriteAllText(labelsOverride, "services: {}");

        var executor = new CapturingCommandExecutor();
        var service = new StackService(executor);

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
            LabelsOverrideFilePath: labelsOverride,
            GeneratedFilesDirectory: generatedDirectory);

        await foreach (var _ in service.ApplyStreamAsync(command, TestContext.Current.CancellationToken))
        {
        }

        var invocation = Assert.Single(executor.Invocations);
        Assert.Equal("docker", invocation.FileName);
        Assert.Equal(workingDirectory, invocation.WorkingDirectory);
        Assert.Equal("prod", invocation.EnvironmentVariables["APP_ENV"]);
        Assert.Equal(generatedDirectory, Path.GetDirectoryName(invocation.Arguments[6]));
        Assert.Equal(
            [
                "compose",
                "--project-directory", workingDirectory,
                "-p", "demo",
                "--env-file", Path.Combine(generatedDirectory, ".env"),
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
        var service = new StackService(executor);

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

    private sealed class CapturingCommandExecutor : ICommandExecutor
    {
        public List<Invocation> Invocations { get; } = [];

        public Task<ProcessExecutionResult> ExecuteAsync(
            string fileName,
            IEnumerable<string> arguments,
            IDictionary<string, string>? environmentVariables = null,
            string? workingDirectory = null,
            CancellationToken cancellationToken = default)
            => Task.FromResult(new ProcessExecutionResult(0, string.Empty, string.Empty));

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
            yield return new ProcessOutput(null, null, 0);
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
