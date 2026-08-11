using System.Collections.Immutable;
using System.Net;
using System.Net.Http.Json;
using System.Threading.Channels;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Domain.Entities.Platforms;
using Hosting.Common;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Moq;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Containers;

public sealed class ContainerCommandEndpointTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private static readonly Guid PlatformId = Guid.Parse("019c2534-5d6b-722c-9f10-2b534ba1eed3");
    private static readonly Guid PersistedContainerId = Guid.Parse("019c2534-d2f9-71d0-b68a-b57e9cc53f16");
    private const string DockerContainerId = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    private readonly Mock<IConnectorFactory<IContainerConnector>> connectorFactory = new();
    private readonly Mock<IContainerConnector> connector = new();

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        connectorFactory
            .Setup(value => value.GetConnector(PlatformConnectorType.Agent))
            .Returns(connector.Object);
        services.ReplaceService<IConnectorFactory<IContainerConnector>>(connectorFactory.Object);
        services.RemoveService<IDbWorkQueue>();
        services.AddSingleton<IDbWorkQueue, InlineDbWorkQueue>();
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Platform.FromPersistence(
            PlatformId,
            "container-command-platform",
            "docker.test:2375",
            networkCount: 0,
            volumeCount: 0,
            imageCount: 0,
            cpuCount: 2,
            memTotal: 2048,
            status: PlatformStatus.Online,
            connectorType: PlatformConnectorType.Agent,
            platformDescriptor: new DockerPlatformDescriptor("container-command-daemon", 0, 0, 0, 1));
        var container = Container.FromPersistence(
            PersistedContainerId,
            PlatformId,
            DockerContainerId,
            "sha256:nginx",
            "nginx",
            DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            rowVersion: 0,
            controlStartedAt: null,
            controlTriggeredBy: null,
            controlState: ResourceControlState.Idle,
            state: ContainerStateStatus.Exited,
            ports: new Dictionary<string, IReadOnlyList<HostPortBinding>>());

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Containers.AddAsync(container, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    [Theory]
    [InlineData("start", ContainerAction.START, ContainerStateStatus.Running)]
    [InlineData("stop", ContainerAction.STOP, ContainerStateStatus.Exited)]
    [InlineData("pause", ContainerAction.PAUSE, ContainerStateStatus.Paused)]
    [InlineData("unpause", ContainerAction.UNPAUSE, ContainerStateStatus.Running)]
    [InlineData("restart", ContainerAction.RESTART, ContainerStateStatus.Running)]
    public async Task ContainerCommandEndpoint_ShouldCompleteAndPersistRuntimeState(
        string route,
        ContainerAction expectedAction,
        ContainerStateStatus expectedState)
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        Services.GetRequiredService<IPlatformContainerCache>().ReplacePlatformContainers(
            PlatformId,
            new PlatformCacheEntry(
                PlatformId,
                "docker.test:2375",
                PlatformConnectorType.Agent,
                new Dictionary<string, Guid> { [DockerContainerId] = PersistedContainerId }.ToImmutableDictionary()));
        connector
            .Setup(value => value.PatchAsync(
                It.Is<PatchContainerCommand>(command =>
                    command.PlatformAddress == "docker.test:2375"
                    && command.Action == expectedAction
                    && command.ContainerIds.SequenceEqual(new[] { DockerContainerId })),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
        connector
            .Setup(value => value.InspectAsync(
                It.Is<InspectContainerCommand>(command =>
                    command.PlatformAddress == "docker.test:2375"
                    && command.ContainerId == DockerContainerId),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(Inspection(expectedState)));

        using var response = await Client.PatchAsJsonAsync(
            $"/api/v1/containers/{route}",
            new[] { PersistedContainerId.ToString("D") },
            cancellationToken);
        Assert.Equal(HttpStatusCode.NoContent, response.StatusCode);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var persisted = await uow.Containers.GetByIdAsync(DockerContainerId, cancellationToken);
        Assert.NotNull(persisted);
        Assert.Equal(expectedState, persisted.State);
        Assert.Equal(ResourceControlState.Idle, persisted.ControlState);
        Assert.Null(persisted.ControlTriggeredBy);

        connector.VerifyAll();
    }

    [Fact]
    public async Task ContainerCommandEndpoint_ShouldRejectDirectSwarmTaskMutation()
    {
        const string taskContainerId = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        var cancellationToken = TestContext.Current.CancellationToken;
        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Containers.AddAsync(
                Container.FromPersistence(
                    Guid.NewGuid(),
                    PlatformId,
                    taskContainerId,
                    "sha256:redis",
                    "redis-test_web.1",
                    DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
                    DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
                    rowVersion: 0,
                    controlStartedAt: null,
                    controlTriggeredBy: null,
                    controlState: ResourceControlState.Idle,
                    state: ContainerStateStatus.Running,
                    dockerStack: "redis-test",
                    ports: new Dictionary<string, IReadOnlyList<HostPortBinding>>(),
                    isSwarmTask: true),
                cancellationToken);
            await uow.CommitAsync(cancellationToken);
        }

        using var response = await Client.PatchAsJsonAsync(
            "/api/v1/containers/restart",
            new[] { taskContainerId[..12] },
            cancellationToken);

        Assert.Equal(HttpStatusCode.Conflict, response.StatusCode);
        connector.Verify(
            value => value.PatchAsync(It.IsAny<PatchContainerCommand>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }

    private static ContainerInspectionInfo Inspection(ContainerStateStatus state)
        => new(
            Id: DockerContainerId,
            Created: DateTimeOffset.UtcNow.ToString("O"),
            Path: null,
            Args: [],
            State: new ContainerRuntimeState(
                state,
                Running: state == ContainerStateStatus.Running,
                Paused: state == ContainerStateStatus.Paused,
                Restarting: false,
                OOMKilled: false,
                Dead: false,
                Pid: 1,
                ExitCode: null,
                Error: null,
                StartedAt: DateTimeOffset.UtcNow.ToString("O"),
                FinishedAt: null,
                Health: null),
            Image: "nginx",
            ResolvConfPath: null,
            HostnamePath: null,
            HostsPath: null,
            LogPath: null,
            Name: "nginx",
            RestartCount: 0,
            Driver: null,
            Platform: "linux",
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

    private sealed class InlineDbWorkQueue(IServiceScopeFactory scopeFactory) : IDbWorkQueue
    {
        private readonly Channel<IDbWorkItem> channel = Channel.CreateUnbounded<IDbWorkItem>();

        public ChannelReader<IDbWorkItem> Reader => channel.Reader;

        public ValueTask EnqueueAsync(IDbWorkItem item, CancellationToken cancellationToken)
            => EnqueueAndWaitAsync(item, cancellationToken);

        public async ValueTask EnqueueAndWaitAsync(IDbWorkItem item, CancellationToken cancellationToken)
        {
            await using var scope = scopeFactory.CreateAsyncScope();
            await item.ExecuteAsync(scope.ServiceProvider.GetRequiredService<IUnitOfWork>(), cancellationToken);
        }
    }
}
