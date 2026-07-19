using Domain.Contracts.Interfaces;
using Hosting.DockerClient.HttpClient;
using Hosting.DockerClient.Models.Images;
using Hosting.DockerClient.Services;
using Infrastructure.Repositories;
using Moq;
using System.Runtime.CompilerServices;

namespace Tests.Unit.Infrastructure.Repositories;

public sealed class BuildProcessRunnerTests
{
    [Fact]
    public async Task RunAsync_ShouldBuildAndPushThroughDockerApiStreams()
    {
        var imageService = new Mock<IImageService>(MockBehavior.Strict);
        BuildImageStreamCommand? buildCommand = null;
        PushImageStreamCommand? pushCommand = null;
        var digest = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        var runner = new BuildProcessRunner(imageService.Object);
        var command = CreateCommand();

        imageService
            .Setup(x => x.StreamBuildImage(It.IsAny<BuildImageStreamCommand>(), It.IsAny<CancellationToken>()))
            .Callback<BuildImageStreamCommand, CancellationToken>((cmd, _) => buildCommand = cmd)
            .Returns(Messages(new JSONMessage { Stream = "Step 1/1 : FROM scratch" }));
        imageService
            .Setup(x => x.StreamPushImage(It.IsAny<PushImageStreamCommand>(), It.IsAny<CancellationToken>()))
            .Callback<PushImageStreamCommand, CancellationToken>((cmd, _) => pushCommand = cmd)
            .Returns(Messages(new JSONMessage { Status = $"latest: digest: {digest} size: 123" }));

        var events = await runner.RunAsync(command, TestContext.Current.CancellationToken)
            .ToListAsync(TestContext.Current.CancellationToken);

        Assert.NotNull(buildCommand);
        Assert.Equal(command.ContextPath, buildCommand.ContextDirectory);
        Assert.Equal(command.DockerfilePath, buildCommand.DockerfilePath);
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
        var imageService = new Mock<IImageService>(MockBehavior.Strict);
        var runner = new BuildProcessRunner(imageService.Object);
        var command = CreateCommand(secrets: [new BuildProcessSecret("token", "super-secret")]);

        var events = await runner.RunAsync(command, TestContext.Current.CancellationToken)
            .ToListAsync(TestContext.Current.CancellationToken);

        Assert.Contains(events, e =>
            e.Stream == BuildProcessStream.StdErr &&
            e.Message!.Contains("Build secrets are not supported", StringComparison.Ordinal));
        var exit = Assert.Single(events, e => e.Stream == BuildProcessStream.Exit);
        Assert.Equal(1, exit.ExitCode);
        imageService.Verify(x => x.StreamBuildImage(It.IsAny<BuildImageStreamCommand>(), It.IsAny<CancellationToken>()), Times.Never);
        imageService.Verify(x => x.StreamPushImage(It.IsAny<PushImageStreamCommand>(), It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task RunAsync_ShouldApplyTimeoutAcrossBuildAndPush()
    {
        var imageService = new Mock<IImageService>(MockBehavior.Strict);
        var runner = new BuildProcessRunner(imageService.Object);
        var command = CreateCommand(timeout: TimeSpan.FromMilliseconds(20));

        imageService
            .Setup(x => x.StreamBuildImage(It.IsAny<BuildImageStreamCommand>(), It.IsAny<CancellationToken>()))
            .Returns(Messages(new JSONMessage { Stream = "Successfully built image" }));
        imageService
            .Setup(x => x.StreamPushImage(It.IsAny<PushImageStreamCommand>(), It.IsAny<CancellationToken>()))
            .Returns((PushImageStreamCommand _, CancellationToken ct) => WaitForCancellation(ct));

        await Assert.ThrowsAnyAsync<OperationCanceledException>(async () =>
            await runner.RunAsync(command, TestContext.Current.CancellationToken).ToListAsync(TestContext.Current.CancellationToken));
    }

    [Fact]
    public async Task RunAsync_ShouldRemoveNullBytesFromDockerOutput()
    {
        var imageService = new Mock<IImageService>(MockBehavior.Strict);
        var runner = new BuildProcessRunner(imageService.Object);
        var command = CreateCommand();

        imageService
            .Setup(x => x.StreamBuildImage(It.IsAny<BuildImageStreamCommand>(), It.IsAny<CancellationToken>()))
            .Returns(Messages(new JSONMessage { Stream = "hello\0world" }));
        imageService
            .Setup(x => x.StreamPushImage(It.IsAny<PushImageStreamCommand>(), It.IsAny<CancellationToken>()))
            .Returns(Messages());

        var events = await runner.RunAsync(command, TestContext.Current.CancellationToken)
            .ToListAsync(TestContext.Current.CancellationToken);

        Assert.Contains(events, e => e.Stream == BuildProcessStream.StdOut && e.Message == "helloworld");
        Assert.DoesNotContain(events, e => e.Message?.Contains('\0', StringComparison.Ordinal) == true);
    }

    private static BuildProcessCommand CreateCommand(
        IReadOnlyList<BuildProcessSecret>? secrets = null,
        TimeSpan? timeout = null)
        => new(
            WorkingDirectory: @"C:\repo",
            ContextPath: @"C:\repo\src",
            DockerfilePath: @"C:\repo\src\Dockerfile",
            Target: null,
            ImageReferences: ["registry.example.com/citadel/demo:latest"],
            BuildArgs: [new BuildProcessBuildArg("CONFIGURATION", "Release")],
            Secrets: secrets ?? [],
            RegistryCredential: new BuildProcessRegistryCredential("registry.example.com", "registry-auth-token"),
            Timeout: timeout ?? TimeSpan.FromMinutes(5));

    private static async IAsyncEnumerable<JSONMessage> Messages(params JSONMessage[] messages)
    {
        await Task.Yield();
        foreach (var message in messages)
            yield return message;
    }

    private static async IAsyncEnumerable<JSONMessage> WaitForCancellation([EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await Task.Delay(TimeSpan.FromSeconds(30), cancellationToken);
        yield return new JSONMessage { Status = "unexpected" };
    }
}
