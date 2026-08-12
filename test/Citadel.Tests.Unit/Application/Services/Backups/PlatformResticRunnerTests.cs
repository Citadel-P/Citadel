using Application.Services;
using Application.Services.Backups;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Images;
using Domain.Entities.Platforms;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
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
            Mock.Of<IAgentRuntimeImageResolver>(),
            Mock.Of<ISwarmNodeRuntimeConnector>(),
            Mock.Of<IServiceScopeFactory>());

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
        Assert.True(pathHelper.AutoRemove);
        Assert.Equal(["-c", "trap 'exit 0' TERM INT; sleep 330"], pathHelper.Command);

        Assert.Equal(["/bin/sh", "-c", "mkdir -p -- '/host-parent/backup-01'"], execRequests[0].Command);

        var resticHelper = createCommands[1];
        var repositoryMount = Assert.Single(resticHelper.Mounts ?? []);
        Assert.Equal("/srv/backup-01", repositoryMount.Source);
        Assert.Equal("/repository", repositoryMount.Target);
        Assert.Equal("bind", repositoryMount.Type);
        Assert.True(resticHelper.AutoRemove);
        Assert.Equal(["-c", "trap 'exit 0' TERM INT; sleep 330"], resticHelper.Command);
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
            Mock.Of<IAgentRuntimeImageResolver>(),
            Mock.Of<ISwarmNodeRuntimeConnector>(),
            Mock.Of<IServiceScopeFactory>());

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
            Mock.Of<IAgentRuntimeImageResolver>(),
            Mock.Of<ISwarmNodeRuntimeConnector>(),
            Mock.Of<IServiceScopeFactory>());

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
            agentRuntimeImageResolver.Object,
            Mock.Of<ISwarmNodeRuntimeConnector>(),
            Mock.Of<IServiceScopeFactory>());

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
            agentRuntimeImageResolver.Object,
            Mock.Of<ISwarmNodeRuntimeConnector>(),
            Mock.Of<IServiceScopeFactory>());

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

    [Fact]
    public async Task RunAsync_ShouldReusePinnedNodeAgentImageForWorkerBackupHelper()
    {
        var platform = new Platform(
            "swarm",
            "http://manager:2375",
            0,
            0,
            0,
            3,
            1024,
            "29.0",
            null,
            PlatformStatus.Online,
            PlatformConnectorType.Local,
            new DockerSwarmPlatformDescriptor(
                "manager-node", "10.0.0.1", "Active", true, 1, 1, "manager-daemon", 0, 0, 0, 0),
            clusterId: "cluster-1");
        const string serviceId = "node-agent-service";
        const string pinnedAgentImage = "registry:5000/citadel-agent@sha256:test";
        var now = DateTimeOffset.UtcNow;
        var installation = new SwarmNodeAgentInstallation(
            platform.Id,
            platform.ClusterId!,
            "manager-node",
            "manager-daemon",
            serviceId,
            $"citadel-node-agent-{platform.Id:N}",
            "registry:5000/citadel-agent:candidate",
            "sha256:test",
            null,
            null,
            SwarmNodeAgentDesiredState.Installed,
            null,
            null,
            null,
            null,
            null,
            null,
            now.UtcDateTime,
            now.UtcDateTime);
        var service = new SwarmServiceProjection(
            platform.Id,
            serviceId,
            1,
            installation.DockerServiceName,
            "Global",
            pinnedAgentImage,
            2,
            2,
            "Completed",
            null,
            [],
            [],
            [],
            [],
            new Dictionary<string, string>
            {
                ["com.citadel.system"] = "true",
                ["com.citadel.system-role"] = "swarm-node-agent",
                ["com.citadel.platform-id"] = platform.Id.ToString("D"),
                ["com.citadel.swarm-cluster-id"] = platform.ClusterId!
            },
            now,
            now,
            now,
            false,
            SwarmServiceOwnership.System);

        var platforms = new Mock<IPlatformRepository>();
        platforms
            .Setup(repository => repository.GetByIdAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(platform);
        var edgeAgents = new Mock<IEdgeAgentRepository>();
        edgeAgents
            .Setup(repository => repository.GetNodeAgentInstallationAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(installation);
        var swarm = new Mock<ISwarmProjectionRepository>();
        swarm
            .Setup(repository => repository.GetServiceAsync(platform.Id, serviceId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(service);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(value => value.Platforms).Returns(platforms.Object);
        unitOfWork.SetupGet(value => value.EdgeAgents).Returns(edgeAgents.Object);
        unitOfWork.SetupGet(value => value.Swarm).Returns(swarm.Object);
        await using var services = new ServiceCollection()
            .AddSingleton(unitOfWork.Object)
            .BuildServiceProvider();

        CreateContainerCommand? createCommand = null;
        var nodeRuntime = new Mock<ISwarmNodeRuntimeConnector>();
        nodeRuntime
            .Setup(connector => connector.CreateContainerAsync(
                platform,
                "worker-node",
                It.IsAny<CreateContainerCommand>(),
                It.IsAny<CancellationToken>()))
            .Callback<Platform, string, CreateContainerCommand, CancellationToken>((_, _, command, _) => createCommand = command)
            .ReturnsAsync(Result.Success("helper-id"));
        nodeRuntime
            .Setup(connector => connector.PatchContainersAsync(
                platform, "worker-node", ContainerAction.START, It.IsAny<IReadOnlyCollection<string>>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
        nodeRuntime
            .Setup(connector => connector.InspectContainerAsync(
                platform, "worker-node", "helper-id", It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(RunningContainer("helper-id")));
        nodeRuntime
            .Setup(connector => connector.ExecBinaryAsync(
                platform, "worker-node", It.IsAny<ContainerBinaryExecRequest>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new ContainerBinaryExecResult
            {
                Output = ReadChunksAsync("{}", TestContext.Current.CancellationToken),
                GetExitCodeAsync = _ => Task.FromResult<int?>(0),
                CleanupAsync = () => ValueTask.CompletedTask
            }));
        nodeRuntime
            .Setup(connector => connector.DeleteContainersAsync(
                platform,
                "worker-node",
                It.IsAny<IReadOnlyCollection<string>>(),
                false,
                true,
                false,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());

        var containerFactory = new Mock<IConnectorFactory<IContainerConnector>>();
        containerFactory
            .Setup(factory => factory.GetConnector(PlatformConnectorType.Local))
            .Returns(Mock.Of<IContainerConnector>());
        var imageFactory = new Mock<IConnectorFactory<IImageConnector>>();
        imageFactory
            .Setup(factory => factory.GetConnector(PlatformConnectorType.Local))
            .Returns(Mock.Of<IImageConnector>());
        var helperImageResolver = new Mock<IVolumeHelperImageResolver>();
        helperImageResolver
            .Setup(resolver => resolver.Resolve(PlatformConnectorType.Agent))
            .Returns("registry:5000/unavailable-core-helper:candidate");
        var runner = new PlatformResticRunner(
            containerFactory.Object,
            imageFactory.Object,
            helperImageResolver.Object,
            Mock.Of<IAgentRuntimeImageResolver>(),
            nodeRuntime.Object,
            services.GetRequiredService<IServiceScopeFactory>());

        var events = new List<ResticProcessEvent>();
        await foreach (var item in runner.RunAsync(
                           new PlatformResticCommand(
                               platform.Id,
                               platform.Address,
                               platform.ConnectorType,
                               "restic",
                               ["backup", "--json", "/source"],
                               new Dictionary<string, string>(),
                               TimeSpan.FromSeconds(60),
                               [],
                               4096,
                               SourceVolumeName: "data",
                               TargetVolumeName: null,
                               RepositoryHostPath: null,
                               NetworkMode: null,
                               DockerNodeId: "worker-node"),
                           TestContext.Current.CancellationToken))
        {
            events.Add(item);
        }

        Assert.NotNull(createCommand);
        Assert.Equal(pinnedAgentImage, createCommand.ImageId);
        Assert.Contains(events, item => item.Stream == ResticProcessStream.Exit && item.ExitCode == 0);
        helperImageResolver.Verify(
            resolver => resolver.Resolve(PlatformConnectorType.Agent),
            Times.Never);
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
