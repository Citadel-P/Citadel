using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using Infrastructure.Repositories;
using Moq;
using System.Runtime.CompilerServices;

namespace Tests.Unit.Infrastructure.Repositories;

public sealed class BuildProcessRunnerTests
{
    [Fact]
    public async Task RunAsync_ShouldBuildAndPushThroughDockerApiStreams()
    {
        var imageConnector = new Mock<IImageConnector>(MockBehavior.Strict);
        var connectorFactory = new Mock<IConnectorFactory<IImageConnector>>(MockBehavior.Strict);
        BuildImageCommand? buildCommand = null;
        PushImageCommand? pushCommand = null;
        var digest = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        var runner = new BuildProcessRunner(connectorFactory.Object);
        var command = CreateCommand(connectorType: PlatformConnectorType.Agent);

        connectorFactory.Setup(x => x.GetConnector(PlatformConnectorType.Agent)).Returns(imageConnector.Object);
        imageConnector
            .Setup(x => x.BuildImageProgressStreamAsync(It.IsAny<BuildImageCommand>(), It.IsAny<CancellationToken>()))
            .Callback<BuildImageCommand, CancellationToken>((cmd, _) => buildCommand = cmd)
            .Returns(Messages(new ImageBuildStreamItem(null, "Step 1/1 : FROM scratch", null, null, null, null, null)));
        imageConnector
            .Setup(x => x.PushImageProgressStreamAsync(It.IsAny<PushImageCommand>(), It.IsAny<CancellationToken>()))
            .Callback<PushImageCommand, CancellationToken>((cmd, _) => pushCommand = cmd)
            .Returns(Messages(new ImageBuildStreamItem(null, null, $"latest: digest: {digest} size: 123", null, null, null, null)));

        var events = await runner.RunAsync(command, TestContext.Current.CancellationToken)
            .ToListAsync(TestContext.Current.CancellationToken);

        Assert.NotNull(buildCommand);
        Assert.Equal(command.PlatformAddress, buildCommand.PlatformAddress);
        Assert.Equal(command.ContextPath, buildCommand.ContextDirectory);
        Assert.Equal(command.DockerfilePath, buildCommand.DockerfilePath);
        Assert.NotNull(buildCommand.ContextArchive);
        Assert.NotEmpty(buildCommand.ContextArchive);
        Assert.Equal("Dockerfile", buildCommand.DockerfileArchivePath);
        Assert.Equal(command.ImageReferences, buildCommand.Tags);
        Assert.Equal("Release", buildCommand.BuildArgs["CONFIGURATION"]);
        Assert.Equal(command.RegistryCredential!.RegistryHost, buildCommand.RegistryHost);
        Assert.Equal(command.RegistryCredential.RegistryAuth, buildCommand.RegistryAuth);
        Assert.NotNull(pushCommand);
        Assert.Equal(command.ImageReferences[0], pushCommand.ImageReference);
        Assert.Equal(command.RegistryCredential.RegistryAuth, pushCommand.RegistryAuth);
        Assert.Contains(events, e => e.Stream == BuildProcessStream.StdOut && e.Message == "Step 1/1 : FROM scratch");
        var exit = Assert.Single(events, e => e.Stream == BuildProcessStream.Exit);
        Assert.Equal(0, exit.ExitCode);
        Assert.Equal(digest, exit.Digest);
    }

    [Fact]
    public async Task RunAsync_ShouldRejectBuildSecretsForDockerApiBuilds()
    {
        var imageConnector = new Mock<IImageConnector>(MockBehavior.Strict);
        var connectorFactory = new Mock<IConnectorFactory<IImageConnector>>(MockBehavior.Strict);
        var runner = new BuildProcessRunner(connectorFactory.Object);
        var command = CreateCommand(secrets: [new BuildProcessSecret("token", "super-secret")]);

        var events = await runner.RunAsync(command, TestContext.Current.CancellationToken)
            .ToListAsync(TestContext.Current.CancellationToken);

        Assert.Contains(events, e =>
            e.Stream == BuildProcessStream.StdErr &&
            e.Message!.Contains("Build secrets are not supported", StringComparison.Ordinal));
        var exit = Assert.Single(events, e => e.Stream == BuildProcessStream.Exit);
        Assert.Equal(1, exit.ExitCode);
        connectorFactory.Verify(x => x.GetConnector(It.IsAny<PlatformConnectorType>()), Times.Never);
        imageConnector.Verify(x => x.BuildImageProgressStreamAsync(It.IsAny<BuildImageCommand>(), It.IsAny<CancellationToken>()), Times.Never);
        imageConnector.Verify(x => x.PushImageProgressStreamAsync(It.IsAny<PushImageCommand>(), It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task RunAsync_ShouldApplyTimeoutAcrossBuildAndPush()
    {
        var imageConnector = new Mock<IImageConnector>(MockBehavior.Strict);
        var connectorFactory = new Mock<IConnectorFactory<IImageConnector>>(MockBehavior.Strict);
        var runner = new BuildProcessRunner(connectorFactory.Object);
        var command = CreateCommand(timeout: TimeSpan.FromMilliseconds(20));

        connectorFactory.Setup(x => x.GetConnector(command.PlatformConnectorType)).Returns(imageConnector.Object);
        imageConnector
            .Setup(x => x.BuildImageProgressStreamAsync(It.IsAny<BuildImageCommand>(), It.IsAny<CancellationToken>()))
            .Returns(Messages(new ImageBuildStreamItem(null, "Successfully built image", null, null, null, null, null)));
        imageConnector
            .Setup(x => x.PushImageProgressStreamAsync(It.IsAny<PushImageCommand>(), It.IsAny<CancellationToken>()))
            .Returns((PushImageCommand _, CancellationToken ct) => WaitForCancellation(ct));

        await Assert.ThrowsAnyAsync<OperationCanceledException>(async () =>
            await runner.RunAsync(command, TestContext.Current.CancellationToken).ToListAsync(TestContext.Current.CancellationToken));
    }

    [Fact]
    public async Task RunAsync_ShouldRemoveNullBytesFromDockerOutput()
    {
        var imageConnector = new Mock<IImageConnector>(MockBehavior.Strict);
        var connectorFactory = new Mock<IConnectorFactory<IImageConnector>>(MockBehavior.Strict);
        var runner = new BuildProcessRunner(connectorFactory.Object);
        var command = CreateCommand();

        connectorFactory.Setup(x => x.GetConnector(command.PlatformConnectorType)).Returns(imageConnector.Object);
        imageConnector
            .Setup(x => x.BuildImageProgressStreamAsync(It.IsAny<BuildImageCommand>(), It.IsAny<CancellationToken>()))
            .Returns(Messages(new ImageBuildStreamItem(null, "hello\0world", null, null, null, null, null)));
        imageConnector
            .Setup(x => x.PushImageProgressStreamAsync(It.IsAny<PushImageCommand>(), It.IsAny<CancellationToken>()))
            .Returns(Messages());

        var events = await runner.RunAsync(command, TestContext.Current.CancellationToken)
            .ToListAsync(TestContext.Current.CancellationToken);

        Assert.Contains(events, e => e.Stream == BuildProcessStream.StdOut && e.Message == "helloworld");
        Assert.DoesNotContain(events, e => e.Message?.Contains('\0', StringComparison.Ordinal) == true);
    }

    [Fact]
    public async Task RunAsync_ShouldNotPackageContextForLocalConnector()
    {
        var imageConnector = new Mock<IImageConnector>(MockBehavior.Strict);
        var connectorFactory = new Mock<IConnectorFactory<IImageConnector>>(MockBehavior.Strict);
        var runner = new BuildProcessRunner(connectorFactory.Object);
        var command = CreateCommand(connectorType: PlatformConnectorType.Local);
        BuildImageCommand? buildCommand = null;

        connectorFactory.Setup(x => x.GetConnector(PlatformConnectorType.Local)).Returns(imageConnector.Object);
        imageConnector
            .Setup(x => x.BuildImageProgressStreamAsync(It.IsAny<BuildImageCommand>(), It.IsAny<CancellationToken>()))
            .Callback<BuildImageCommand, CancellationToken>((cmd, _) => buildCommand = cmd)
            .Returns(Messages(new ImageBuildStreamItem(null, "Successfully built image", null, null, null, null, null)));
        imageConnector
            .Setup(x => x.PushImageProgressStreamAsync(It.IsAny<PushImageCommand>(), It.IsAny<CancellationToken>()))
            .Returns(Messages());

        await runner.RunAsync(command, TestContext.Current.CancellationToken)
            .ToListAsync(TestContext.Current.CancellationToken);

        Assert.NotNull(buildCommand);
        Assert.Null(buildCommand.ContextArchive);
        Assert.Null(buildCommand.DockerfileArchivePath);
    }

    private static BuildProcessCommand CreateCommand(
        IReadOnlyList<BuildProcessSecret>? secrets = null,
        TimeSpan? timeout = null,
        PlatformConnectorType connectorType = PlatformConnectorType.Local)
    {
        var root = Path.Combine(Path.GetTempPath(), "citadel-build-runner-tests", Guid.NewGuid().ToString("N"));
        var contextPath = Path.Combine(root, "src");
        Directory.CreateDirectory(contextPath);
        File.WriteAllText(Path.Combine(contextPath, "Dockerfile"), "FROM scratch");

        return new(
            PlatformAddress: "http://builder.example.com:8001",
            PlatformConnectorType: connectorType,
            WorkingDirectory: root,
            ContextPath: contextPath,
            DockerfilePath: Path.Combine(contextPath, "Dockerfile"),
            Target: null,
            ImageReferences: ["registry.example.com/citadel/demo:latest"],
            BuildArgs: [new BuildProcessBuildArg("CONFIGURATION", "Release")],
            Secrets: secrets ?? [],
            RegistryCredential: new BuildProcessRegistryCredential("registry.example.com", "registry-auth-token"),
            Timeout: timeout ?? TimeSpan.FromMinutes(5));
    }

    private static async IAsyncEnumerable<ImageBuildStreamItem> Messages(params ImageBuildStreamItem[] messages)
    {
        await Task.Yield();
        foreach (var message in messages)
            yield return message;
    }

    private static async IAsyncEnumerable<ImageBuildStreamItem> WaitForCancellation([EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await Task.Delay(TimeSpan.FromSeconds(30), cancellationToken);
        yield return new ImageBuildStreamItem(null, null, "unexpected", null, null, null, null);
    }
}
