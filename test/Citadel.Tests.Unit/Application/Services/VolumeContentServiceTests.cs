using System.Text;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Images;
using Domain.Contracts.Resources.Volumes;
using Domain.Entities;
using Domain.Entities.Platforms;
using LightResults;
using Microsoft.Extensions.Logging.Abstractions;
using Moq;

namespace Tests.Unit.Application.Services;

public sealed class VolumeContentServiceTests
{
    [Fact]
    public async Task ListDirectoryAsync_ShouldCreateExecAndDeleteHelperOnOwningSwarmNode()
    {
        var platformId = Guid.CreateVersion7();
        const string dockerNodeId = "worker-1";
        var platform = Platform.FromPersistence(
            platformId,
            "swarm",
            "local://docker",
            0, 0, 0, 2, 2048,
            PlatformStatus.Online,
            PlatformConnectorType.Local,
            new DockerSwarmPlatformDescriptor(
                "manager-1", "10.0.0.1", "Active", true, 2, 1,
                "daemon-1", 0, 0, 0, 0, "cluster-1"),
            clusterId: "cluster-1");
        var directConnector = new Mock<IContainerConnector>();
        var containerConnectorFactory = new Mock<IConnectorFactory<IContainerConnector>>();
        containerConnectorFactory
            .Setup(factory => factory.GetConnector(PlatformConnectorType.Local))
            .Returns(directConnector.Object);
        var nodeConnector = new Mock<ISwarmNodeRuntimeConnector>();
        nodeConnector
            .Setup(connector => connector.CreateContainerAsync(
                platform,
                dockerNodeId,
                It.Is<CreateContainerCommand>(command =>
                    command.Labels != null
                    && command.Labels["com.citadel.system"] == "true"
                    && command.Labels["citadel.volume-browser"] == "true"
                    && command.Mounts != null
                    && command.Mounts.Single().Source == "app-data"
                    && command.EntryPoint != null
                    && command.EntryPoint.SequenceEqual(new[] { "/app/Citadel.Agent.VolumeHelper" })
                    && command.Command != null
                    && command.Command.SequenceEqual(new[] { "volume-helper", "idle" })),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success("helper-1"));
        nodeConnector
            .Setup(connector => connector.PatchContainersAsync(
                platform,
                dockerNodeId,
                ContainerAction.START,
                It.Is<IReadOnlyCollection<string>>(ids => ids.Count == 1 && ids.Single() == "helper-1"),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
        nodeConnector
            .Setup(connector => connector.InspectContainerAsync(
                platform, dockerNodeId, "helper-1", It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(RunningContainer("helper-1")));
        nodeConnector
            .Setup(connector => connector.ExecBinaryAsync(
                platform,
                dockerNodeId,
                It.Is<ContainerBinaryExecRequest>(request =>
                    request.ContainerId == "helper-1"
                    && request.Command.Count > 2
                    && request.Command[0] == "/app/Citadel.Agent.VolumeHelper"
                    && request.Command[1] == "volume-helper"
                    && request.Command[2] == "list"),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new ContainerBinaryExecResult
            {
                Output = ReadChunksAsync("""{"Path":"/","Entries":[],"IsTruncated":false}"""),
                GetExitCodeAsync = _ => Task.FromResult<int?>(0),
                CleanupAsync = () => ValueTask.CompletedTask
            }));
        nodeConnector
            .Setup(connector => connector.DeleteContainersAsync(
                platform,
                dockerNodeId,
                It.Is<IReadOnlyCollection<string>>(ids => ids.Count == 1 && ids.Single() == "helper-1"),
                false,
                true,
                false,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
        var helperImageResolver = new Mock<IVolumeHelperImageResolver>();
        helperImageResolver
            .Setup(resolver => resolver.Resolve(PlatformConnectorType.Agent))
            .Returns("citadel-agent:1.0");
        var service = new VolumeContentService(
            containerConnectorFactory.Object,
            Mock.Of<IConnectorFactory<IImageConnector>>(),
            nodeConnector.Object,
            helperImageResolver.Object,
            Mock.Of<IAgentRuntimeImageResolver>(),
            NullLogger<VolumeContentService>.Instance);

        var result = await service.ListDirectoryAsync(
            new ListVolumeDirectoryCommand(
                platform.Address,
                platformId,
                platform.ConnectorType,
                "app-data",
                new NormalizedVolumePath("/", []),
                platform,
                dockerNodeId),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out _, out var error), error?.Message);
        nodeConnector.VerifyAll();
        helperImageResolver.VerifyAll();
        directConnector.VerifyNoOtherCalls();
    }

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
            .Setup(resolver => resolver.Resolve(PlatformConnectorType.EdgeAgent))
            .Returns(VolumeContentService.DevelopmentHelperImage);
        var agentRuntimeImageResolver = new Mock<IAgentRuntimeImageResolver>();
        agentRuntimeImageResolver
            .Setup(resolver => resolver.TryResolveAsync(
                containerConnector.Object,
                "edge://platform-1",
                It.IsAny<Guid>(),
                PlatformConnectorType.EdgeAgent,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync((string?)null);

        var service = new VolumeContentService(
            containerConnectorFactory.Object,
            Mock.Of<IConnectorFactory<IImageConnector>>(),
            Mock.Of<ISwarmNodeRuntimeConnector>(),
            helperImageResolver.Object,
            agentRuntimeImageResolver.Object,
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

        Assert.NotNull(createCommand.EntryPoint);
        Assert.NotNull(createCommand.Command);

        var createEntryPoint = createCommand.EntryPoint!;
        var createArgs = createCommand.Command!;

        Assert.Equal(["/bin/sh"], createEntryPoint);
        Assert.Equal("-c", createArgs[0]);
        Assert.DoesNotContain('\r', createArgs[1]);
        Assert.Contains("/app/Citadel.VolumeHelper", createArgs[1]);
        Assert.Contains("Citadel.VolumeHelper.dll", createArgs[1]);
        Assert.Contains("/app/Citadel.Agent.VolumeHelper", createArgs[1]);
        Assert.Contains("Citadel.Agent.VolumeHelper.dll", createArgs[1]);
        Assert.Equal("citadel-volume-helper", createArgs[2]);
        Assert.Equal(["volume-helper", "idle"], createArgs.Skip(3));

        Assert.Equal("/bin/sh", execRequest.Command[0]);
        Assert.Equal("-c", execRequest.Command[1]);
        Assert.DoesNotContain('\r', execRequest.Command[2]);
        Assert.Contains("/app/Citadel.VolumeHelper", execRequest.Command[2]);
        Assert.Contains("Citadel.VolumeHelper.dll", execRequest.Command[2]);
        Assert.Contains("/app/Citadel.Agent.VolumeHelper", execRequest.Command[2]);
        Assert.Contains("Citadel.Agent.VolumeHelper.dll", execRequest.Command[2]);
        Assert.Equal("citadel-volume-helper", execRequest.Command[3]);
        Assert.Equal("volume-helper", execRequest.Command[4]);
        Assert.Equal("list", execRequest.Command[5]);
    }

    [Fact]
    public async Task ListDirectoryAsync_ShouldUseEdgeAgentRuntimeImageWhenAvailable()
    {
        CreateContainerCommand? createCommand = null;

        var platformId = Guid.CreateVersion7();
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
            .Setup(resolver => resolver.Resolve(PlatformConnectorType.EdgeAgent))
            .Returns("ghcr.io/citadel-p/citadel.agent:1.0");
        var agentRuntimeImageResolver = new Mock<IAgentRuntimeImageResolver>();
        agentRuntimeImageResolver
            .Setup(resolver => resolver.TryResolveAsync(
                containerConnector.Object,
                "edge://platform-1",
                platformId,
                PlatformConnectorType.EdgeAgent,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync("citadel-agent:dev");

        var service = new VolumeContentService(
            containerConnectorFactory.Object,
            Mock.Of<IConnectorFactory<IImageConnector>>(),
            Mock.Of<ISwarmNodeRuntimeConnector>(),
            helperImageResolver.Object,
            agentRuntimeImageResolver.Object,
            NullLogger<VolumeContentService>.Instance);

        var result = await service.ListDirectoryAsync(
            new ListVolumeDirectoryCommand(
                "edge://platform-1",
                platformId,
                PlatformConnectorType.EdgeAgent,
                "app-data",
                new NormalizedVolumePath("/", [])),
            CancellationToken.None);

        Assert.True(result.IsSuccess(out _, out var error), error?.Message);
        Assert.NotNull(createCommand);
        Assert.Equal("citadel-agent:dev", createCommand.ImageId);
    }

    [Fact]
    public async Task ListDirectoryAsync_ShouldUseAgentRuntimeImageWhenAvailable()
    {
        CreateContainerCommand? createCommand = null;
        var platformId = Guid.CreateVersion7();

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
                "agent://platform-1",
                It.IsAny<ContainerBinaryExecRequest>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new ContainerBinaryExecResult
            {
                Output = ReadChunksAsync("""{"Path":"/","Entries":[],"IsTruncated":false}"""),
                GetExitCodeAsync = _ => Task.FromResult<int?>(0),
                CleanupAsync = () => ValueTask.CompletedTask
            }));

        var containerConnectorFactory = new Mock<IConnectorFactory<IContainerConnector>>();
        containerConnectorFactory
            .Setup(factory => factory.GetConnector(PlatformConnectorType.Agent))
            .Returns(containerConnector.Object);

        var helperImageResolver = new Mock<IVolumeHelperImageResolver>();
        helperImageResolver
            .Setup(resolver => resolver.Resolve(PlatformConnectorType.Agent))
            .Returns("ghcr.io/citadel-p/citadel.agent:1.0");
        var agentRuntimeImageResolver = new Mock<IAgentRuntimeImageResolver>();
        agentRuntimeImageResolver
            .Setup(resolver => resolver.TryResolveAsync(
                containerConnector.Object,
                "agent://platform-1",
                platformId,
                PlatformConnectorType.Agent,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync("citadel-agent:dev");

        var service = new VolumeContentService(
            containerConnectorFactory.Object,
            Mock.Of<IConnectorFactory<IImageConnector>>(),
            Mock.Of<ISwarmNodeRuntimeConnector>(),
            helperImageResolver.Object,
            agentRuntimeImageResolver.Object,
            NullLogger<VolumeContentService>.Instance);

        var result = await service.ListDirectoryAsync(
            new ListVolumeDirectoryCommand(
                "agent://platform-1",
                platformId,
                PlatformConnectorType.Agent,
                "app-data",
                new NormalizedVolumePath("/", [])),
            CancellationToken.None);

        Assert.True(result.IsSuccess(out _, out var error), error?.Message);
        Assert.NotNull(createCommand);
        Assert.Equal("citadel-agent:dev", createCommand.ImageId);
    }

    [Fact]
    public async Task ListDirectoryAsync_ShouldIncludeHelperLogsWhenHelperExitsBeforeReady()
    {
        var containerConnector = new Mock<IContainerConnector>();
        containerConnector
            .Setup(connector => connector.CreateAsync(It.IsAny<CreateContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success("helper-1"));
        containerConnector
            .Setup(connector => connector.PatchAsync(It.IsAny<PatchContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
        containerConnector
            .Setup(connector => connector.InspectAsync(It.IsAny<InspectContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(ExitedContainer("helper-1", 127)));
        containerConnector
            .Setup(connector => connector.StreamLogsAsync(It.IsAny<StreamContainerLogsCommand>(), It.IsAny<CancellationToken>()))
            .Returns((StreamContainerLogsCommand _, CancellationToken _) => ReadLogChunksAsync("helper executable was not found"));
        containerConnector
            .Setup(connector => connector.DeleteAsync(It.IsAny<DeleteContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());

        var containerConnectorFactory = new Mock<IConnectorFactory<IContainerConnector>>();
        containerConnectorFactory
            .Setup(factory => factory.GetConnector(PlatformConnectorType.EdgeAgent))
            .Returns(containerConnector.Object);

        var helperImageResolver = new Mock<IVolumeHelperImageResolver>();
        helperImageResolver
            .Setup(resolver => resolver.Resolve(PlatformConnectorType.EdgeAgent))
            .Returns(VolumeContentService.DevelopmentHelperImage);
        var agentRuntimeImageResolver = new Mock<IAgentRuntimeImageResolver>();
        agentRuntimeImageResolver
            .Setup(resolver => resolver.TryResolveAsync(
                containerConnector.Object,
                "edge://platform-1",
                It.IsAny<Guid>(),
                PlatformConnectorType.EdgeAgent,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync((string?)null);

        var service = new VolumeContentService(
            containerConnectorFactory.Object,
            Mock.Of<IConnectorFactory<IImageConnector>>(),
            Mock.Of<ISwarmNodeRuntimeConnector>(),
            helperImageResolver.Object,
            agentRuntimeImageResolver.Object,
            NullLogger<VolumeContentService>.Instance);

        var result = await service.ListDirectoryAsync(
            new ListVolumeDirectoryCommand(
                "edge://platform-1",
                Guid.CreateVersion7(),
                PlatformConnectorType.EdgeAgent,
                "app-data",
                new NormalizedVolumePath("/", [])),
            CancellationToken.None);

        Assert.False(result.IsSuccess(out _, out var error));
        Assert.NotNull(error);
        Assert.Contains("exited before it was ready", error.Message);
        Assert.Contains("Exit code: 127", error.Message);
        Assert.Contains("helper executable was not found", error.Message);
    }

    [Fact]
    public async Task ListDirectoryAsync_OnSwarmManager_ShouldUseCurrentContainerImageAndSourceMount()
    {
        var platformId = Guid.CreateVersion7();
        const string managerNodeId = "manager-1";
        var platform = Platform.FromPersistence(
            platformId,
            "swarm",
            "local://docker",
            0, 0, 0, 1, 2048,
            PlatformStatus.Online,
            PlatformConnectorType.Local,
            new DockerSwarmPlatformDescriptor(
                managerNodeId, "10.0.0.1", "Active", true, 1, 1,
                "daemon-1", 0, 0, 0, 0, "cluster-1"),
            clusterId: "cluster-1");
        CreateContainerCommand? createCommand = null;
        ContainerBinaryExecRequest? execRequest = null;
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
            .ReturnsAsync(Result.Success("helper-1"));
        containerConnector
            .Setup(connector => connector.PatchAsync(It.IsAny<PatchContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
        containerConnector
            .Setup(connector => connector.InspectAsync(
                It.Is<InspectContainerCommand>(command => command.ContainerId == "helper-1"),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(RunningContainer("helper-1")));
        containerConnector
            .Setup(connector => connector.DeleteAsync(It.IsAny<DeleteContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
        containerConnector
            .Setup(connector => connector.ExecBinaryAsync(
                "local://docker",
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
            .Setup(factory => factory.GetConnector(PlatformConnectorType.Local))
            .Returns(containerConnector.Object);

        var helperImageResolver = new Mock<IVolumeHelperImageResolver>();
        helperImageResolver
            .SetupGet(resolver => resolver.IsExplicitlyConfigured)
            .Returns(false);
        helperImageResolver
            .Setup(resolver => resolver.Resolve(PlatformConnectorType.Local))
            .Returns("ghcr.io/citadel-p/citadel:1.0");

        var nodeConnector = new Mock<ISwarmNodeRuntimeConnector>();
        var service = new VolumeContentService(
            containerConnectorFactory.Object,
            Mock.Of<IConnectorFactory<IImageConnector>>(),
            nodeConnector.Object,
            helperImageResolver.Object,
            Mock.Of<IAgentRuntimeImageResolver>(),
            NullLogger<VolumeContentService>.Instance);

        var result = await service.ListDirectoryAsync(
            new ListVolumeDirectoryCommand(
                "local://docker",
                platformId,
                PlatformConnectorType.Local,
                "app-data",
                new NormalizedVolumePath("/", []),
                platform,
                managerNodeId),
            CancellationToken.None);

        Assert.True(result.IsSuccess(out _, out var error), error?.Message);
        Assert.NotNull(createCommand);
        Assert.NotNull(execRequest);

        Assert.Equal("citadel.dev:dev", createCommand.ImageId);
        Assert.Contains(createCommand.Mounts ?? [], mount =>
            mount.Type == "volume"
            && mount.Source == "app-data"
            && mount.Target == "/data");
        Assert.Contains(createCommand.Mounts ?? [], mount =>
            mount.Type == "bind"
            && mount.Source == "/run/desktop/mnt/host/d/Projects/Citadel"
            && mount.Target == "/src"
            && mount.ReadOnly == true);

        Assert.Contains("/src/src/Citadel.VolumeHelper/bin/Debug/net*/Citadel.VolumeHelper.dll", execRequest.Command[2]);
        nodeConnector.VerifyNoOtherCalls();
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

    private static ContainerInspectionInfo CurrentDevelopmentContainer(string id)
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
            Image: "sha256:debug-image",
            ResolvConfPath: null,
            HostnamePath: null,
            HostsPath: null,
            LogPath: null,
            Name: "citadel-server-1",
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
            Mounts:
            [
                new MountPointInfo(
                    Type: "bind",
                    Name: null,
                    Source: "/run/desktop/mnt/host/d/Projects/Citadel",
                    Destination: "/src",
                    Driver: null,
                    Mode: "rw",
                    RW: true,
                    Propagation: "rprivate")
            ],
            Config: new ContainerConfiguration(
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
                Labels: new Dictionary<string, string>()),
            NetworkSettings: null);

    private static ContainerInspectionInfo ExitedContainer(string id, int exitCode)
        => new(
            Id: id,
            Created: string.Empty,
            Path: null,
            Args: [],
            State: new ContainerRuntimeState(
                Status: ContainerStateStatus.Exited,
                Running: false,
                Paused: false,
                Restarting: false,
                OOMKilled: false,
                Dead: false,
                Pid: 0,
                ExitCode: exitCode,
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

    private static async IAsyncEnumerable<ReadOnlyMemory<byte>> ReadLogChunksAsync(string payload)
    {
        await Task.Yield();
        yield return Encoding.UTF8.GetBytes(payload);
    }
}
