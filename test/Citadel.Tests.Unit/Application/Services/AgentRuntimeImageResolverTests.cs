using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Platforms;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging.Abstractions;
using Moq;

namespace Tests.Unit.Application.Services;

public sealed class AgentRuntimeImageResolverTests
{
    [Fact]
    public async Task TryResolveAsync_ShouldReturnAgentRuntimeImageFromPlatformInfo()
    {
        var platformId = Guid.CreateVersion7();
        var platformConnector = new Mock<IPlatformConnector>(MockBehavior.Strict);
        platformConnector
            .Setup(connector => connector.GetPlatformAsync(
                It.Is<GetPlatformCommand>(command => command.PlatformAddress == "agent://platform-01"),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new PlatformResult(
                Name: string.Empty,
                Address: "agent://platform-01",
                NetworkCount: 0,
                VolumeCount: 0,
                ImageCount: 0,
                CpuCount: 0,
                MemTotal: 0,
                ServerVersion: null,
                AgentVersion: "1.0",
                Descriptor: null,
                AgentRuntimeImage: "citadel-agent:dev")));

        var platformConnectorFactory = new Mock<IConnectorFactory<IPlatformConnector>>(MockBehavior.Strict);
        platformConnectorFactory
            .Setup(factory => factory.GetConnector(PlatformConnectorType.Agent))
            .Returns(platformConnector.Object);

        var resolver = new AgentRuntimeImageResolver(
            Mock.Of<IServiceScopeFactory>(),
            platformConnectorFactory.Object,
            NullLogger<AgentRuntimeImageResolver>.Instance);

        var image = await resolver.TryResolveAsync(
            Mock.Of<IContainerConnector>(),
            "agent://platform-01",
            platformId,
            PlatformConnectorType.Agent,
            CancellationToken.None);

        Assert.Equal("citadel-agent:dev", image);
    }

    [Fact]
    public async Task TryResolveAsync_ShouldCacheAgentRuntimeImage()
    {
        var platformId = Guid.CreateVersion7();
        var platformConnector = new Mock<IPlatformConnector>(MockBehavior.Strict);
        platformConnector
            .Setup(connector => connector.GetPlatformAsync(
                It.Is<GetPlatformCommand>(command => command.PlatformAddress == "agent://platform-01"),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new PlatformResult(
                Name: string.Empty,
                Address: "agent://platform-01",
                NetworkCount: 0,
                VolumeCount: 0,
                ImageCount: 0,
                CpuCount: 0,
                MemTotal: 0,
                ServerVersion: null,
                AgentVersion: "1.0",
                Descriptor: null,
                AgentRuntimeImage: "citadel-agent:dev")));

        var platformConnectorFactory = new Mock<IConnectorFactory<IPlatformConnector>>(MockBehavior.Strict);
        platformConnectorFactory
            .Setup(factory => factory.GetConnector(PlatformConnectorType.Agent))
            .Returns(platformConnector.Object);

        var resolver = new AgentRuntimeImageResolver(
            Mock.Of<IServiceScopeFactory>(),
            platformConnectorFactory.Object,
            NullLogger<AgentRuntimeImageResolver>.Instance);

        var first = await resolver.TryResolveAsync(
            Mock.Of<IContainerConnector>(),
            "agent://platform-01",
            platformId,
            PlatformConnectorType.Agent,
            CancellationToken.None);
        var second = await resolver.TryResolveAsync(
            Mock.Of<IContainerConnector>(),
            "agent://platform-01",
            platformId,
            PlatformConnectorType.Agent,
            CancellationToken.None);

        Assert.Equal("citadel-agent:dev", first);
        Assert.Equal("citadel-agent:dev", second);
        platformConnector.Verify(
            connector => connector.GetPlatformAsync(It.IsAny<GetPlatformCommand>(), It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task TryResolveAsync_ShouldInspectLastSeenEdgeAgentContainerAndReturnItsImageWhenPlatformInfoHasNoImage()
    {
        var platformId = Guid.CreateVersion7();
        var binding = CreateBinding(platformId, "edge-agent-container");

        var edgeAgents = new Mock<IEdgeAgentRepository>(MockBehavior.Strict);
        edgeAgents
            .Setup(repository => repository.GetBindingByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(binding);

        var unitOfWork = new Mock<IUnitOfWork>(MockBehavior.Strict);
        unitOfWork.SetupGet(uow => uow.EdgeAgents).Returns(edgeAgents.Object);
        unitOfWork.Setup(uow => uow.DisposeAsync()).Returns(ValueTask.CompletedTask);

        await using var provider = new ServiceCollection()
            .AddScoped(_ => unitOfWork.Object)
            .BuildServiceProvider();

        var containerConnector = new Mock<IContainerConnector>(MockBehavior.Strict);
        containerConnector
            .Setup(connector => connector.InspectAsync(
                It.Is<InspectContainerCommand>(command => command.ContainerId == "edge-agent-container"),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(InspectedContainer("edge-agent-container", "citadel-agent:dev")));

        var platformConnector = new Mock<IPlatformConnector>(MockBehavior.Strict);
        platformConnector
            .Setup(connector => connector.GetPlatformAsync(It.IsAny<GetPlatformCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new PlatformResult(
                Name: string.Empty,
                Address: "edge://platform-01",
                NetworkCount: 0,
                VolumeCount: 0,
                ImageCount: 0,
                CpuCount: 0,
                MemTotal: 0,
                ServerVersion: null,
                AgentVersion: "1.0",
                Descriptor: null)));

        var platformConnectorFactory = new Mock<IConnectorFactory<IPlatformConnector>>(MockBehavior.Strict);
        platformConnectorFactory
            .Setup(factory => factory.GetConnector(PlatformConnectorType.EdgeAgent))
            .Returns(platformConnector.Object);

        var resolver = new AgentRuntimeImageResolver(
            provider.GetRequiredService<IServiceScopeFactory>(),
            platformConnectorFactory.Object,
            NullLogger<AgentRuntimeImageResolver>.Instance);

        var image = await resolver.TryResolveAsync(
            containerConnector.Object,
            "edge://platform-01",
            platformId,
            PlatformConnectorType.EdgeAgent,
            CancellationToken.None);

        Assert.Equal("citadel-agent:dev", image);
    }

    [Fact]
    public async Task TryResolveAsync_ShouldReturnNullWhenBindingHasNoHostname()
    {
        var platformId = Guid.CreateVersion7();
        var binding = CreateBinding(platformId, null);

        var edgeAgents = new Mock<IEdgeAgentRepository>(MockBehavior.Strict);
        edgeAgents
            .Setup(repository => repository.GetBindingByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(binding);

        var unitOfWork = new Mock<IUnitOfWork>(MockBehavior.Strict);
        unitOfWork.SetupGet(uow => uow.EdgeAgents).Returns(edgeAgents.Object);
        unitOfWork.Setup(uow => uow.DisposeAsync()).Returns(ValueTask.CompletedTask);

        await using var provider = new ServiceCollection()
            .AddScoped(_ => unitOfWork.Object)
            .BuildServiceProvider();

        var platformConnector = new Mock<IPlatformConnector>(MockBehavior.Strict);
        platformConnector
            .Setup(connector => connector.GetPlatformAsync(It.IsAny<GetPlatformCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new PlatformResult(
                Name: string.Empty,
                Address: "edge://platform-01",
                NetworkCount: 0,
                VolumeCount: 0,
                ImageCount: 0,
                CpuCount: 0,
                MemTotal: 0,
                ServerVersion: null,
                AgentVersion: "1.0",
                Descriptor: null)));

        var platformConnectorFactory = new Mock<IConnectorFactory<IPlatformConnector>>(MockBehavior.Strict);
        platformConnectorFactory
            .Setup(factory => factory.GetConnector(PlatformConnectorType.EdgeAgent))
            .Returns(platformConnector.Object);

        var resolver = new AgentRuntimeImageResolver(
            provider.GetRequiredService<IServiceScopeFactory>(),
            platformConnectorFactory.Object,
            NullLogger<AgentRuntimeImageResolver>.Instance);

        var image = await resolver.TryResolveAsync(
            Mock.Of<IContainerConnector>(),
            "edge://platform-01",
            platformId,
            PlatformConnectorType.EdgeAgent,
            CancellationToken.None);

        Assert.Null(image);
    }

    private static EdgeAgentBinding CreateBinding(Guid platformId, string? hostname)
        => new(
            Id: Guid.CreateVersion7(),
            PlatformId: platformId,
            AgentId: Guid.CreateVersion7(),
            AgentPublicKey: "public-key",
            AgentFingerprint: "SHA256:fingerprint",
            ConnectionStatus: EdgeAgentConnectionStatus.Connected,
            LastConnectedAtUtc: DateTime.UtcNow,
            LastDisconnectedAtUtc: null,
            LastHeartbeatAtUtc: DateTime.UtcNow,
            LastSeenVersion: "1.0",
            LastSeenHostname: hostname,
            CapabilitiesJson: "{}",
            ProtocolVersion: 1,
            RevokedAtUtc: null,
            CreatedAtUtc: DateTime.UtcNow,
            UpdatedAtUtc: DateTime.UtcNow);

    private static ContainerInspectionInfo InspectedContainer(string id, string image)
        => new(
            Id: id,
            Created: string.Empty,
            Path: null,
            Args: [],
            State: null,
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
                Image: image,
                Volumes: null,
                WorkingDir: null,
                Entrypoint: [],
                NetworkDisabled: null,
                MacAddress: null,
                OnBuild: [],
                Labels: new Dictionary<string, string>()),
            NetworkSettings: null);
}
