using DomainModel = global::Domain;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Stacks;
using Hosting.DockerClient.Services;
using Infrastructure.Connectors.LocalConnectors;
using Moq;

namespace Tests.Unit.Infrastructure.Connectors;

public class LocalStackConnectorTests
{
    [Fact]
    public async Task StackApplyAsync_Runs_PreDeploy_RegistryLogin_Compose_And_PostDeploy_In_Order()
    {
        var executor = new Mock<ICommandExecutor>(MockBehavior.Strict);
        var callLog = new List<(string FileName, IReadOnlyList<string> Arguments)>();
        string[]? composeArguments = null;

        executor
            .Setup(x => x.ExecuteAsync(It.IsAny<string>(), It.IsAny<IEnumerable<string>>(), It.IsAny<IDictionary<string, string>?>(), It.IsAny<string?>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((string fileName, IEnumerable<string> arguments, IDictionary<string, string>? env, string? workingDirectory, CancellationToken _) =>
            {
                var args = arguments.ToArray();
                callLog.Add((fileName, args));

                if (args.Contains("docker login") || args.Contains("CITADEL_REGISTRY_PASSWORD"))
                {
                    Assert.NotNull(env);
                    Assert.True(env!.ContainsKey("CITADEL_REGISTRY_PASSWORD"));
                }

                return new ProcessExecutionResult(0, "ok", string.Empty);
            });

        executor
            .Setup(x => x.StreamAsync(It.IsAny<string>(), It.IsAny<IEnumerable<string>>(), It.IsAny<IDictionary<string, string>?>(), It.IsAny<string?>(), It.IsAny<string?>(), It.IsAny<CancellationToken>()))
            .Returns((string fileName, IEnumerable<string> arguments, IDictionary<string, string>? env, string? workingDirectory, string? configDirectory, CancellationToken token) =>
            {
                composeArguments = arguments.ToArray();
                return StreamCompose();
            });

        var connector = new LocalStackConnector(executor.Object);
        var command = new StackApplyCommand(
            PlatformAddress: "local",
            StackName: "demo-stack",
            ComposeFileContent: "services:{}",
            ProjectName: "demo",
            EnvironmentFilePath: ".env",
            EnvironmentVariables: ["FOO=bar"],
            PreDeploy: new StackCommand(["echo pre"]),
            PostDeploy: new StackCommand(["echo post"]),
            RegistryAuth: "secret",
            RegistryName: "docker.io",
            Spec: new ManualStack("services:{}", DomainModel.StackUpdateBehavior.Disabled));

        var results = await connector.StackApplyAsync(command, CancellationToken.None).ToListAsync();

        Assert.Contains(results, x => x.Message == "Running pre-deploy commands...");
        Assert.Contains(results, x => x.Message != null && x.Message.Contains("Setting up registry config for", StringComparison.Ordinal));
        Assert.Contains(results, x => x.Message == "compose-up");
        Assert.Contains(results, x => x.Message == "Running post-deploy commands...");
        Assert.Contains(results, x => x.ExitCode == 0);

        Assert.Equal(2, callLog.Count);
        Assert.Contains(callLog[0].Arguments, x => x.Contains("echo pre", StringComparison.Ordinal));
        Assert.Contains(callLog[1].Arguments, x => x.Contains("echo post", StringComparison.Ordinal));

        Assert.NotNull(composeArguments);
        Assert.Equal("compose", composeArguments![0]);
        Assert.Equal("-f", composeArguments[1]);
        Assert.EndsWith("compose.yml", composeArguments[2], StringComparison.OrdinalIgnoreCase);
        Assert.Equal("--env-file", composeArguments[3]);
        Assert.EndsWith(".env", composeArguments[4], StringComparison.OrdinalIgnoreCase);
        Assert.Equal("--project-name", composeArguments[5]);
        Assert.Equal("demo", composeArguments[6]);
        Assert.Equal("up", composeArguments[7]);
        Assert.Equal("-d", composeArguments[8]);
    }

    [Fact]
    public async Task StackApplyAsync_Stops_When_PreDeploy_Fails()
    {
        var executor = new Mock<ICommandExecutor>(MockBehavior.Strict);

        executor
            .Setup(x => x.ExecuteAsync(It.IsAny<string>(), It.IsAny<IEnumerable<string>>(), It.IsAny<IDictionary<string, string>?>(), It.IsAny<string?>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(new ProcessExecutionResult(42, string.Empty, "pre failed"));

        var connector = new LocalStackConnector(executor.Object);
        var command = new StackApplyCommand(
            PlatformAddress: "local",
            StackName: "demo-stack",
            ComposeFileContent: "services:{}",
            ProjectName: "demo",
            EnvironmentFilePath: null,
            EnvironmentVariables: null,
            PreDeploy: new StackCommand(["exit 42"]),
            PostDeploy: null,
            RegistryAuth: "",
            RegistryName: "docker.io",
            Spec: new ManualStack("services:{}", DomainModel.StackUpdateBehavior.Disabled));

        var results = await connector.StackApplyAsync(command, CancellationToken.None).ToListAsync();

        Assert.Contains(results, x => x.Message == "Running pre-deploy commands...");
        Assert.Contains(results, x => x.Message == "pre failed");
        Assert.Contains(results, x => x.ExitCode == 42);

        executor.Verify(x => x.StreamAsync(It.IsAny<string>(), It.IsAny<IEnumerable<string>>(), It.IsAny<IDictionary<string, string>?>(), It.IsAny<string?>(), It.IsAny<string?>(), It.IsAny<CancellationToken>()), Times.Never);
    }

    private static async IAsyncEnumerable<ProcessOutput> StreamCompose()
    {
        yield return new ProcessOutput("compose-up", null, null);
        await Task.Yield();
        yield return new ProcessOutput(null, null, 0);
    }
}
