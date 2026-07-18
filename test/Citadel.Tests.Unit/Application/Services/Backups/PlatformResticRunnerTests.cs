using Application.Services;
using Application.Services.Backups;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Images;
using LightResults;
using Moq;
using System.Runtime.CompilerServices;
using System.Text;

namespace Tests.Unit.Application.Services.Backups;

public sealed class PlatformResticRunnerTests
{
    [Fact]
    public async Task RunAsync_ShouldCreatePlatformRepositoryPathBeforeStartingResticHelper()
    {
        var createCommands = new List<CreateContainerCommand>();
        var execRequests = new List<ContainerBinaryExecRequest>();
        var containerIds = new Queue<string>(["path-helper", "restic-helper"]);

        var containerConnector = new Mock<IContainerConnector>();
        containerConnector
            .Setup(connector => connector.CreateAsync(It.IsAny<CreateContainerCommand>(), It.IsAny<CancellationToken>()))
            .Callback<CreateContainerCommand, CancellationToken>((command, _) => createCommands.Add(command))
            .ReturnsAsync(() => Result.Success(containerIds.Dequeue()));
        containerConnector
            .Setup(connector => connector.PatchAsync(It.IsAny<PatchContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
        containerConnector
            .Setup(connector => connector.InspectAsync(It.IsAny<InspectContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((InspectContainerCommand command, CancellationToken _) => Result.Success(RunningContainer(command.ContainerId)));
        containerConnector
            .Setup(connector => connector.ExecBinaryAsync(
                "agent://platform-01",
                It.IsAny<ContainerBinaryExecRequest>(),
                It.IsAny<CancellationToken>()))
            .Callback<string, ContainerBinaryExecRequest, CancellationToken>((_, request, _) => execRequests.Add(request))
            .ReturnsAsync(() => Result.Success(new ContainerBinaryExecResult
            {
                Output = ReadChunksAsync(execRequests.Count == 1 ? null : "[]"),
                GetExitCodeAsync = _ => Task.FromResult<int?>(0),
                CleanupAsync = () => ValueTask.CompletedTask
            }));
        containerConnector
            .Setup(connector => connector.DeleteAsync(It.IsAny<DeleteContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());

        var containerFactory = new Mock<IConnectorFactory<IContainerConnector>>();
        containerFactory
            .Setup(factory => factory.GetConnector(PlatformConnectorType.Agent))
            .Returns(containerConnector.Object);

        var imageFactory = new Mock<IConnectorFactory<IImageConnector>>();
        imageFactory
            .Setup(factory => factory.GetConnector(PlatformConnectorType.Agent))
            .Returns(Mock.Of<IImageConnector>());

        var helperImageResolver = new Mock<IVolumeHelperImageResolver>();
        helperImageResolver
            .Setup(resolver => resolver.Resolve(PlatformConnectorType.Agent))
            .Returns(VolumeContentService.DevelopmentHelperImage);

        var runner = new PlatformResticRunner(
            containerFactory.Object,
            imageFactory.Object,
            helperImageResolver.Object,
            Mock.Of<IAgentRuntimeImageResolver>());

        var events = new List<ResticProcessEvent>();
        await foreach (var item in runner.RunAsync(
                           new PlatformResticCommand(
                               Guid.CreateVersion7(),
                               "agent://platform-01",
                               PlatformConnectorType.Agent,
                               "restic",
                               ["snapshots", "--json"],
                               new Dictionary<string, string> { ["RESTIC_REPOSITORY"] = "/repository" },
                               TimeSpan.FromSeconds(30),
                               [],
                               4096,
                               SourceVolumeName: null,
                               TargetVolumeName: null,
                               RepositoryHostPath: "/srv/backup-01",
                               NetworkMode: "none"),
                           CancellationToken.None))
        {
            events.Add(item);
        }

        Assert.Equal(2, createCommands.Count);
        Assert.Equal(2, execRequests.Count);
        Assert.Contains(events, item => item.ExitCode == 0);

        var pathHelper = createCommands[0];
        var pathMount = Assert.Single(pathHelper.Mounts ?? []);
        Assert.Equal("/srv", pathMount.Source);
        Assert.Equal("/host-parent", pathMount.Target);
        Assert.Equal("bind", pathMount.Type);
        Assert.Equal("none", pathHelper.NetworkMode);

        Assert.Equal(["/bin/sh", "-c", "mkdir -p -- '/host-parent/backup-01'"], execRequests[0].Command);

        var resticHelper = createCommands[1];
        var repositoryMount = Assert.Single(resticHelper.Mounts ?? []);
        Assert.Equal("/srv/backup-01", repositoryMount.Source);
        Assert.Equal("/repository", repositoryMount.Target);
        Assert.Equal("bind", repositoryMount.Type);
        Assert.Equal(["restic", "snapshots", "--json"], execRequests[1].Command);
    }

    [Fact]
    public async Task RunAsync_ShouldReturnHelpfulError_WhenHelperImageDoesNotIncludeRestic()
    {
        var containerConnector = new Mock<IContainerConnector>();
        containerConnector
            .Setup(connector => connector.CreateAsync(It.IsAny<CreateContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success("restic-helper"));
        containerConnector
            .Setup(connector => connector.PatchAsync(It.IsAny<PatchContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
        containerConnector
            .Setup(connector => connector.InspectAsync(It.IsAny<InspectContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((InspectContainerCommand command, CancellationToken _) => Result.Success(RunningContainer(command.ContainerId)));
        containerConnector
            .Setup(connector => connector.ExecBinaryAsync(
                "agent://platform-01",
                It.IsAny<ContainerBinaryExecRequest>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Failure<ContainerBinaryExecResult>(
                "OCI runtime exec failed: exec failed: unable to start container process: exec: \"restic\": executable file not found in $PATH"));
        containerConnector
            .Setup(connector => connector.DeleteAsync(It.IsAny<DeleteContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());

        var containerFactory = new Mock<IConnectorFactory<IContainerConnector>>();
        containerFactory
            .Setup(factory => factory.GetConnector(PlatformConnectorType.Agent))
            .Returns(containerConnector.Object);

        var imageFactory = new Mock<IConnectorFactory<IImageConnector>>();
        imageFactory
            .Setup(factory => factory.GetConnector(PlatformConnectorType.Agent))
            .Returns(Mock.Of<IImageConnector>());

        var helperImageResolver = new Mock<IVolumeHelperImageResolver>();
        helperImageResolver
            .Setup(resolver => resolver.Resolve(PlatformConnectorType.Agent))
            .Returns(VolumeContentService.DevelopmentHelperImage);

        var runner = new PlatformResticRunner(
            containerFactory.Object,
            imageFactory.Object,
            helperImageResolver.Object,
            Mock.Of<IAgentRuntimeImageResolver>());

        var events = new List<ResticProcessEvent>();
        await foreach (var item in runner.RunAsync(
                           new PlatformResticCommand(
                               Guid.CreateVersion7(),
                               "agent://platform-01",
                               PlatformConnectorType.Agent,
                               "restic",
                               ["snapshots", "--json"],
                               new Dictionary<string, string> { ["RESTIC_REPOSITORY"] = "/repository" },
                               TimeSpan.FromSeconds(30),
                               [],
                               4096,
                               SourceVolumeName: null,
                               TargetVolumeName: null,
                               RepositoryHostPath: null,
                               NetworkMode: "none"),
                           CancellationToken.None))
        {
            events.Add(item);
        }

        var error = Assert.Single(events, item => item.Stream == ResticProcessStream.StdErr);
        Assert.Contains($"Backup helper image '{VolumeContentService.DevelopmentHelperImage}' does not include 'restic'", error.Message);
        Assert.Contains("Citadel helper image", error.Message);
        Assert.Contains(events, item => item.Stream == ResticProcessStream.Exit && item.ExitCode == 1);
    }

    [Fact]
    public async Task RunAsync_ShouldUseCurrentContainerImageForLocalDevelopment()
    {
        CreateContainerCommand? createCommand = null;
        var currentContainerId = Environment.MachineName;

        var containerConnector = new Mock<IContainerConnector>();
        containerConnector
            .Setup(connector => connector.InspectAsync(
                It.Is<InspectContainerCommand>(command => command.ContainerId == currentContainerId),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(CurrentDevelopmentContainer(currentContainerId)));
        containerConnector
            .Setup(connector => connector.CreateAsync(It.IsAny<CreateContainerCommand>(), It.IsAny<CancellationToken>()))
            .Callback<CreateContainerCommand, CancellationToken>((command, _) => createCommand = command)
            .ReturnsAsync(Result.Success("restic-helper"));
        containerConnector
            .Setup(connector => connector.PatchAsync(It.IsAny<PatchContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
        containerConnector
            .Setup(connector => connector.InspectAsync(
                It.Is<InspectContainerCommand>(command => command.ContainerId == "restic-helper"),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(RunningContainer("restic-helper")));
        containerConnector
            .Setup(connector => connector.ExecBinaryAsync(
                "local://docker",
                It.IsAny<ContainerBinaryExecRequest>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new ContainerBinaryExecResult
            {
                Output = ReadChunksAsync("[]", TestContext.Current.CancellationToken),
                GetExitCodeAsync = _ => Task.FromResult<int?>(0),
                CleanupAsync = () => ValueTask.CompletedTask
            }));
        containerConnector
            .Setup(connector => connector.DeleteAsync(It.IsAny<DeleteContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());

        var containerFactory = new Mock<IConnectorFactory<IContainerConnector>>();
        containerFactory
            .Setup(factory => factory.GetConnector(PlatformConnectorType.Local))
            .Returns(containerConnector.Object);

        var imageFactory = new Mock<IConnectorFactory<IImageConnector>>();
        imageFactory
            .Setup(factory => factory.GetConnector(PlatformConnectorType.Local))
            .Returns(Mock.Of<IImageConnector>());

        var helperImageResolver = new Mock<IVolumeHelperImageResolver>();
        helperImageResolver
            .SetupGet(resolver => resolver.IsExplicitlyConfigured)
            .Returns(false);
        helperImageResolver
            .Setup(resolver => resolver.Resolve(PlatformConnectorType.Local))
            .Returns("ghcr.io/citadel-p/citadel:1.0");

        var runner = new PlatformResticRunner(
            containerFactory.Object,
            imageFactory.Object,
            helperImageResolver.Object,
            Mock.Of<IAgentRuntimeImageResolver>());

        var events = new List<ResticProcessEvent>();
        await foreach (var item in runner.RunAsync(
                           new PlatformResticCommand(
                               Guid.CreateVersion7(),
                               "local://docker",
                               PlatformConnectorType.Local,
                               "restic",
                               ["snapshots", "--json"],
                               new Dictionary<string, string> { ["RESTIC_REPOSITORY"] = "/repository" },
                               TimeSpan.FromSeconds(30),
                               [],
                               4096,
                               SourceVolumeName: null,
                               TargetVolumeName: null,
                               RepositoryHostPath: null,
                               NetworkMode: "none"),
                           CancellationToken.None))
        {
            events.Add(item);
        }

        Assert.NotNull(createCommand);
        Assert.Equal("citadel.dev:dev", createCommand.ImageId);
        Assert.Contains(events, item => item.Stream == ResticProcessStream.Exit && item.ExitCode == 0);
    }

    [Fact]
    public async Task RunAsync_ShouldUseEdgeAgentRuntimeImageWhenAvailable()
    {
        CreateContainerCommand? createCommand = null;
        var platformId = Guid.CreateVersion7();

        var containerConnector = new Mock<IContainerConnector>();
        containerConnector
            .Setup(connector => connector.CreateAsync(It.IsAny<CreateContainerCommand>(), It.IsAny<CancellationToken>()))
            .Callback<CreateContainerCommand, CancellationToken>((command, _) => createCommand = command)
            .ReturnsAsync(Result.Success("restic-helper"));
        containerConnector
            .Setup(connector => connector.PatchAsync(It.IsAny<PatchContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
        containerConnector
            .Setup(connector => connector.InspectAsync(It.IsAny<InspectContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((InspectContainerCommand command, CancellationToken _) => Result.Success(RunningContainer(command.ContainerId)));
        containerConnector
            .Setup(connector => connector.ExecBinaryAsync(
                "edge://platform-01",
                It.IsAny<ContainerBinaryExecRequest>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new ContainerBinaryExecResult
            {
                Output = ReadChunksAsync("[]", TestContext.Current.CancellationToken),
                GetExitCodeAsync = _ => Task.FromResult<int?>(0),
                CleanupAsync = () => ValueTask.CompletedTask
            }));
        containerConnector
            .Setup(connector => connector.DeleteAsync(It.IsAny<DeleteContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());

        var containerFactory = new Mock<IConnectorFactory<IContainerConnector>>();
        containerFactory
            .Setup(factory => factory.GetConnector(PlatformConnectorType.EdgeAgent))
            .Returns(containerConnector.Object);

        var imageFactory = new Mock<IConnectorFactory<IImageConnector>>();
        imageFactory
            .Setup(factory => factory.GetConnector(PlatformConnectorType.EdgeAgent))
            .Returns(Mock.Of<IImageConnector>());

        var helperImageResolver = new Mock<IVolumeHelperImageResolver>();
        helperImageResolver
            .Setup(resolver => resolver.Resolve(PlatformConnectorType.EdgeAgent))
            .Returns("ghcr.io/citadel-p/citadel.agent:1.0");
        var agentRuntimeImageResolver = new Mock<IAgentRuntimeImageResolver>();
        agentRuntimeImageResolver
            .Setup(resolver => resolver.TryResolveAsync(
                containerConnector.Object,
                "edge://platform-01",
                platformId,
                PlatformConnectorType.EdgeAgent,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync("citadel-agent:dev");

        var runner = new PlatformResticRunner(
            containerFactory.Object,
            imageFactory.Object,
            helperImageResolver.Object,
            agentRuntimeImageResolver.Object);

        var events = new List<ResticProcessEvent>();
        await foreach (var item in runner.RunAsync(
                           new PlatformResticCommand(
                               platformId,
                               "edge://platform-01",
                               PlatformConnectorType.EdgeAgent,
                               "restic",
                               ["snapshots", "--json"],
                               new Dictionary<string, string> { ["RESTIC_REPOSITORY"] = "/repository" },
                               TimeSpan.FromSeconds(30),
                               [],
                               4096,
                               SourceVolumeName: null,
                               TargetVolumeName: null,
                               RepositoryHostPath: null,
                               NetworkMode: "none"),
                           CancellationToken.None))
        {
            events.Add(item);
        }

        Assert.NotNull(createCommand);
        Assert.Equal("citadel-agent:dev", createCommand.ImageId);
        Assert.Contains(events, item => item.Stream == ResticProcessStream.Exit && item.ExitCode == 0);
    }

    [Fact]
    public async Task RunAsync_ShouldUseAgentRuntimeImageWhenAvailable()
    {
        CreateContainerCommand? createCommand = null;
        var platformId = Guid.CreateVersion7();

        var containerConnector = new Mock<IContainerConnector>();
        containerConnector
            .Setup(connector => connector.CreateAsync(It.IsAny<CreateContainerCommand>(), It.IsAny<CancellationToken>()))
            .Callback<CreateContainerCommand, CancellationToken>((command, _) => createCommand = command)
            .ReturnsAsync(Result.Success("restic-helper"));
        containerConnector
            .Setup(connector => connector.PatchAsync(It.IsAny<PatchContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
        containerConnector
            .Setup(connector => connector.InspectAsync(It.IsAny<InspectContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((InspectContainerCommand command, CancellationToken _) => Result.Success(RunningContainer(command.ContainerId)));
        containerConnector
            .Setup(connector => connector.ExecBinaryAsync(
                "agent://platform-01",
                It.IsAny<ContainerBinaryExecRequest>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new ContainerBinaryExecResult
            {
                Output = ReadChunksAsync("[]", TestContext.Current.CancellationToken),
                GetExitCodeAsync = _ => Task.FromResult<int?>(0),
                CleanupAsync = () => ValueTask.CompletedTask
            }));
        containerConnector
            .Setup(connector => connector.DeleteAsync(It.IsAny<DeleteContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());

        var containerFactory = new Mock<IConnectorFactory<IContainerConnector>>();
        containerFactory
            .Setup(factory => factory.GetConnector(PlatformConnectorType.Agent))
            .Returns(containerConnector.Object);

        var imageFactory = new Mock<IConnectorFactory<IImageConnector>>();
        imageFactory
            .Setup(factory => factory.GetConnector(PlatformConnectorType.Agent))
            .Returns(Mock.Of<IImageConnector>());

        var helperImageResolver = new Mock<IVolumeHelperImageResolver>();
        helperImageResolver
            .Setup(resolver => resolver.Resolve(PlatformConnectorType.Agent))
            .Returns("ghcr.io/citadel-p/citadel.agent:1.0");
        var agentRuntimeImageResolver = new Mock<IAgentRuntimeImageResolver>();
        agentRuntimeImageResolver
            .Setup(resolver => resolver.TryResolveAsync(
                containerConnector.Object,
                "agent://platform-01",
                platformId,
                PlatformConnectorType.Agent,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync("citadel-agent:dev");

        var runner = new PlatformResticRunner(
            containerFactory.Object,
            imageFactory.Object,
            helperImageResolver.Object,
            agentRuntimeImageResolver.Object);

        var events = new List<ResticProcessEvent>();
        await foreach (var item in runner.RunAsync(
                           new PlatformResticCommand(
                               platformId,
                               "agent://platform-01",
                               PlatformConnectorType.Agent,
                               "restic",
                               ["snapshots", "--json"],
                               new Dictionary<string, string> { ["RESTIC_REPOSITORY"] = "/repository" },
                               TimeSpan.FromSeconds(30),
                               [],
                               4096,
                               SourceVolumeName: null,
                               TargetVolumeName: null,
                               RepositoryHostPath: null,
                               NetworkMode: "none"),
                           CancellationToken.None))
        {
            events.Add(item);
        }

        Assert.NotNull(createCommand);
        Assert.Equal("citadel-agent:dev", createCommand.ImageId);
        Assert.Contains(events, item => item.Stream == ResticProcessStream.Exit && item.ExitCode == 0);
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
            Name: id,
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

    private static ContainerInspectionInfo CurrentDevelopmentContainer(string id)
        => RunningContainer(id) with
        {
            Config = new ContainerConfiguration(
                Hostname: id,
                Domainname: null,
                User: null,
                AttachStdin: null,
                AttachStdout: null,
                AttachStderr: null,
                ExposedPorts: null,
                Tty: null,
                OpenStdin: null,
                StdinOnce: null,
                Env: [],
                Cmd: [],
                Image: "citadel.dev:dev",
                Volumes: null,
                WorkingDir: null,
                Entrypoint: [],
                NetworkDisabled: null,
                MacAddress: null,
                OnBuild: [],
                Labels: new Dictionary<string, string>())
        };

    private static async IAsyncEnumerable<ContainerBinaryExecChunk> ReadChunksAsync(
        string? stdout,
        [EnumeratorCancellation] CancellationToken cancellationToken = default)
    {
        await Task.Yield();
        cancellationToken.ThrowIfCancellationRequested();

        if (stdout is not null)
            yield return new ContainerBinaryExecChunk(ContainerExecStream.Stdout, Encoding.UTF8.GetBytes(stdout));
    }
}
