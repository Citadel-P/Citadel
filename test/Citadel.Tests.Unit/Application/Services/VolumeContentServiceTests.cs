using System.Text;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Images;
using Domain.Contracts.Resources.Volumes;
using Domain.Entities;
using LightResults;
using Microsoft.Extensions.Logging.Abstractions;
using Moq;

namespace Tests.Unit.Application.Services;

public sealed class VolumeContentServiceTests
{
    [Fact]
    public async Task ListDirectoryAsync_ShouldLaunchHelperThroughShellWrapper()
    {
        CreateContainerCommand? createCommand = null;
        ContainerBinaryExecRequest? execRequest = null;

        var containerConnector = new Mock<IContainerConnector>();
        containerConnector
            .Setup(connector => connector.CreateAsync(It.IsAny<CreateContainerCommand>(), It.IsAny<CancellationToken>()))
            .Callback<CreateContainerCommand, CancellationToken>((command, _) => createCommand = command)
            .ReturnsAsync(Result.Success("helper-1"));
        containerConnector
            .Setup(connector => connector.PatchAsync(It.IsAny<PatchContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
        containerConnector
            .Setup(connector => connector.InspectAsync(It.IsAny<InspectContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(RunningContainer("helper-1")));
        containerConnector
            .Setup(connector => connector.DeleteAsync(It.IsAny<DeleteContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
        containerConnector
            .Setup(connector => connector.ExecBinaryAsync(
                "edge://platform-1",
                It.IsAny<ContainerBinaryExecRequest>(),
                It.IsAny<CancellationToken>()))
            .Callback<string, ContainerBinaryExecRequest, CancellationToken>((_, request, _) => execRequest = request)
            .ReturnsAsync(Result.Success(new ContainerBinaryExecResult
            {
                Output = ReadChunksAsync("""{"Path":"/","Entries":[],"IsTruncated":false}"""),
                GetExitCodeAsync = _ => Task.FromResult<int?>(0),
                CleanupAsync = () => ValueTask.CompletedTask
            }));

        var containerConnectorFactory = new Mock<IConnectorFactory<IContainerConnector>>();
        containerConnectorFactory
            .Setup(factory => factory.GetConnector(PlatformConnectorType.EdgeAgent))
            .Returns(containerConnector.Object);

        var helperImageResolver = new Mock<IVolumeHelperImageResolver>();
        helperImageResolver
            .Setup(resolver => resolver.Resolve())
            .Returns("citadel-agent:dev");

        var service = new VolumeContentService(
            containerConnectorFactory.Object,
            Mock.Of<IConnectorFactory<IImageConnector>>(),
            helperImageResolver.Object,
            NullLogger<VolumeContentService>.Instance);

        var result = await service.ListDirectoryAsync(
            new ListVolumeDirectoryCommand(
                "edge://platform-1",
                Guid.CreateVersion7(),
                PlatformConnectorType.EdgeAgent,
                "app-data",
                new NormalizedVolumePath("/", [])),
            CancellationToken.None);

        Assert.True(result.IsSuccess(out _, out var error), error?.Message);
        Assert.NotNull(createCommand);
        Assert.NotNull(execRequest);

        var createEntryPoint = Assert.NotNull(createCommand.EntryPoint);
        var createArgs = Assert.NotNull(createCommand.Command);

        Assert.Equal(["/bin/sh"], createEntryPoint);
        Assert.Equal("-c", createArgs[0]);
        Assert.Contains("/app/Citadel.Agent.VolumeHelper", createArgs[1]);
        Assert.Contains("Citadel.Agent.VolumeHelper.dll", createArgs[1]);
        Assert.Equal("citadel-volume-helper", createArgs[2]);
        Assert.Equal(["volume-helper", "idle"], createArgs.Skip(3));

        Assert.Equal("/bin/sh", execRequest.Command[0]);
        Assert.Equal("-c", execRequest.Command[1]);
        Assert.Contains("/app/Citadel.Agent.VolumeHelper", execRequest.Command[2]);
        Assert.Contains("Citadel.Agent.VolumeHelper.dll", execRequest.Command[2]);
        Assert.Equal("citadel-volume-helper", execRequest.Command[3]);
        Assert.Equal("volume-helper", execRequest.Command[4]);
        Assert.Equal("list", execRequest.Command[5]);
    }

    private static ContainerInspectionInfo RunningContainer(string id)
        => new(
            Id: id,
            Created: string.Empty,
            Path: null,
            Args: [],
            State: new ContainerRuntimeState(
                Status: ContainerStateStatus.Running,
                Running: true,
                Paused: false,
                Restarting: false,
                OOMKilled: false,
                Dead: false,
                Pid: 1,
                ExitCode: null,
                Error: null,
                StartedAt: null,
                FinishedAt: null,
                Health: null),
            Image: null,
            ResolvConfPath: null,
            HostnamePath: null,
            HostsPath: null,
            LogPath: null,
            Name: null,
            RestartCount: 0,
            Driver: null,
            Platform: null,
            MountLabel: null,
            ProcessLabel: null,
            AppArmorProfile: null,
            ExecIDs: [],
            HostConfig: null,
            GraphDriver: null,
            SizeRw: null,
            SizeRootFs: null,
            Mounts: [],
            Config: null,
            NetworkSettings: null);

    private static async IAsyncEnumerable<ContainerBinaryExecChunk> ReadChunksAsync(string payload)
    {
        await Task.Yield();
        yield return new ContainerBinaryExecChunk(
            ContainerExecStream.Stdout,
            Encoding.UTF8.GetBytes(payload));
    }
}
