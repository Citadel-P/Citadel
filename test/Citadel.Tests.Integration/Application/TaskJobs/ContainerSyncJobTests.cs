using Application.Configs;
using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Domain.Entities.Stacks;
using Hosting.Common;
using Infrastructure.Repositories.DbQueue;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using Moq;
using System.Collections.Immutable;
using System.Threading.Channels;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.TaskJobs;

public class ContainerSyncJobTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<IConnectorFactory<IContainerConnector>> containerFactoryMock = new();
    private readonly Mock<IContainerConnector> containerConnector = new();
    private readonly Mock<ISyncBarrier> syncBarrierMock = new();

    private readonly Mock<IOptions<JobConfiguration>> configMock = new();
    private readonly TestPlatformHealthBroadCaster broadcaster = new();
    private readonly Mock<IContainerStreamManager> streamManagerMock = new();
    private readonly ObservableDbWorkQueue dbWorkQueue = new();

    private Guid platformId;
    private const int batchSize = 2;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        // Remove all existing hosted services
        services.RemoveAll<IHostedService>();

        services.AddHostedService<ContainerSyncJob>();
        services.AddHostedService<DbWriteWorker>();
        services.AddHostedService<NotificationWorker>();

        services.AddSingleton(syncBarrierMock.Object);
        services.AddSingleton(configMock.Object);
        services.AddSingleton(streamManagerMock.Object);
        services.AddSingleton(containerConnector.Object);
        services.AddSingleton(containerFactoryMock.Object);
        services.AddSingleton<IPlatformHealthBroadCaster>(broadcaster);
        services.RemoveAll<IDbWorkQueue>();
        services.AddSingleton<IDbWorkQueue>(dbWorkQueue);

        configMock.Setup(x => x.Value).Returns(new JobConfiguration()
        {
            BatchSize = batchSize
        });
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        foreach (var container in Fakes.GetDummyContainers())
        {
            await uow.Containers.AddAsync(new Container(
                    name: container.Name,
                    dockerImageId: container.Image,
                    platformId: platform.Id,
                    ports: new Dictionary<string, IReadOnlyList<HostPortBinding>>(),
                    dockerContainerId: container.Id,
                    state: ContainerStateStatus.Offline)
                , TestContext.Current.CancellationToken);
        }

        await uow.CommitAsync(TestContext.Current.CancellationToken);

        platformId = platform.Id;
    }

    [Fact]
    public async Task SynchronizesContainers_WhenPlatformBecomesOnline()
    {
        // Arrange
        syncBarrierMock.Setup(x => x.WaitForAsync<ImageSyncJob>(It.IsAny<Guid>(), It.IsAny<CancellationToken>())).Returns(ValueTask.CompletedTask);
        containerFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>())).Returns(containerConnector.Object);
        containerConnector.Setup(x => x.ListContainersAsync(It.IsAny<ContainerFilterCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(Fakes.GetDummyContainers().ToDictionary(c => c.Id) as IReadOnlyDictionary<string, DockerContainer>));

        // Act
        var checkpoint = dbWorkQueue.CreateCheckpoint();
        await broadcaster.PublishAsync(new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true, IsValidated: true),
            cancellationToken: TestContext.Current.CancellationToken);
        await dbWorkQueue.WaitForIdleAfterAsync(
            checkpoint,
            TestContext.Current.CancellationToken);

        // Assert
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var cache = scope.ServiceProvider.GetRequiredService<IPlatformContainerCache>();

        var dbContainers = await uow.Containers.GetByPlatformIdAsync(platformId, cancellationToken: TestContext.Current.CancellationToken);

        Assert.All(dbContainers, s =>
            Assert.True(
                s.State == ContainerStateStatus.Running,
                $"Container {s.DockerContainerId} is not running. Actual state: {s.State}"
            ));
        Assert.True(cache.TryGetContainers(platformId, out var cacheContainers));
        Assert.Equal(3, cacheContainers?.Count);
        streamManagerMock.Verify(x => x.SendContainersInfo(It.IsAny<Guid>(), It.IsAny<IEnumerable<Container>>()), Times.Once);
    }

    [Fact]
    public async Task RemovesStaleContainers_WhenNotInFreshList()
    {
        // Arrange: Seed DB with a container that will be missing from the fresh list
        var staleContainer = new Container(
            name: "stale",
            dockerImageId: "fake-id",
            platformId: platformId,
            ports: new Dictionary<string, IReadOnlyList<HostPortBinding>>(),
            dockerContainerId: "stale-id",
            state: ContainerStateStatus.Running);
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.Containers.AddAsync(staleContainer, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        // Return fresh containers
        syncBarrierMock.Setup(x => x.WaitForAsync<ImageSyncJob>(It.IsAny<Guid>(), It.IsAny<CancellationToken>())).Returns(ValueTask.CompletedTask);
        containerFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>())).Returns(containerConnector.Object);
        containerConnector.Setup(x => x.ListContainersAsync(It.IsAny<ContainerFilterCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(Fakes.GetDummyContainers().ToDictionary(c => c.Id) as IReadOnlyDictionary<string, DockerContainer>));

        // Act
        var checkpoint = dbWorkQueue.CreateCheckpoint();
        await broadcaster.PublishAsync(new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true, IsValidated: true),
            cancellationToken: TestContext.Current.CancellationToken);
        await dbWorkQueue.WaitForIdleAfterAsync(
            checkpoint,
            TestContext.Current.CancellationToken);

        // Assert: Stale container should be removed
        await using var assertScope = Services.CreateAsyncScope();
        var freshUow = assertScope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var dbContainers = await freshUow.Containers.GetByPlatformIdAsync(platformId, TestContext.Current.CancellationToken);
        Assert.DoesNotContain(dbContainers, c => c.DockerContainerId == "stale-id");
    }

    [Fact]
    public async Task AddsNewContainers_WhenNotInDatabase()
    {
        // Arrange: DB has no containers, but fresh list has one
        var newDockerContainer = new DockerContainer(
            Name: "new",
            Image: "new:latest",
            ImageId: "fake-id",
            Id: "new-id",
            State: ContainerStateStatus.Running,
            Ports: new Dictionary<string, IReadOnlyList<HostPortBinding>>(),
            Created: 123456,
            Stack: null
        );

        syncBarrierMock.Setup(x => x.WaitForAsync<ImageSyncJob>(It.IsAny<Guid>(), It.IsAny<CancellationToken>())).Returns(ValueTask.CompletedTask);
        containerFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>())).Returns(containerConnector.Object);
        containerConnector.Setup(x => x.ListContainersAsync(It.IsAny<ContainerFilterCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new Dictionary<string, DockerContainer> { { "new-id", newDockerContainer } } as IReadOnlyDictionary<string, DockerContainer>));

        // Act
        var checkpoint = dbWorkQueue.CreateCheckpoint();
        await broadcaster.PublishAsync(new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true, IsValidated: true),
            cancellationToken: TestContext.Current.CancellationToken);
        await dbWorkQueue.WaitForIdleAfterAsync(
            checkpoint,
            TestContext.Current.CancellationToken);

        // Assert: New container should be added
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var dbContainers = await uow.Containers.GetByPlatformIdAsync(platformId, TestContext.Current.CancellationToken);
        Assert.Contains(dbContainers, c => c.DockerContainerId == "new-id");
    }

    [Fact]
    public async Task OnlineSync_ShouldNotOverwriteOrDeleteContainersUpdatedAfterSnapshotStarted()
    {
        var ct = TestContext.Current.CancellationToken;
        var staleSnapshotContainer = Fakes.GetDummyContainers().First() with
        {
            State = ContainerStateStatus.Exited
        };
        var notificationQueue = new Mock<INotificationQueue>();
        notificationQueue
            .Setup(queue => queue.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()))
            .Returns(ValueTask.CompletedTask);

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var cache = scope.ServiceProvider.GetRequiredService<IPlatformContainerCache>();
            var workItem = new SyncOnlinePlatformContainersWorkItem(
                new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true, IsValidated: true),
                notificationQueue.Object,
                new Dictionary<string, DockerContainer>
                {
                    [staleSnapshotContainer.Id] = staleSnapshotContainer
                },
                cache,
                Mock.Of<IContainerStreamManager>(),
                Mock.Of<IDeploymentStreamManager>(),
                Mock.Of<IStackStreamManager>(),
                snapshotStartedAt: 0,
                cacheMutationVersion: cache.GetMutationVersion(platformId),
                logger: Mock.Of<ILogger<ContainerSyncJob>>());

            await workItem.ExecuteAsync(uow, ct);
        }

        await using var assertScope = Services.CreateAsyncScope();
        var assertUow = assertScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stored = (await assertUow.Containers.GetByPlatformIdAsync(platformId, ct)).ToArray();

        Assert.Equal(3, stored.Length);
        Assert.Equal(
            ContainerStateStatus.Offline,
            Assert.Single(stored, container => container.DockerContainerId == staleSnapshotContainer.Id).State);
    }

    [Fact]
    public async Task OnlineSync_ShouldDiscardSnapshotCapturedBeforeContainerCacheMutation()
    {
        var ct = TestContext.Current.CancellationToken;
        var staleSnapshotContainer = Fakes.GetDummyContainers().First() with
        {
            State = ContainerStateStatus.Running
        };
        var notificationQueue = new Mock<INotificationQueue>();

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var cache = scope.ServiceProvider.GetRequiredService<IPlatformContainerCache>();
            var stored = (await uow.Containers.GetByPlatformIdAsync(platformId, ct)).ToArray();
            cache.ReplacePlatformContainers(
                platformId,
                new PlatformCacheEntry(
                    platformId,
                    "https://original.address",
                    PlatformConnectorType.Agent,
                    stored.ToImmutableDictionary(container => container.DockerContainerId, container => container.Id)));
            var capturedVersion = cache.GetMutationVersion(platformId);
            Assert.True(cache.TryRemoveContainer(platformId, staleSnapshotContainer.Id));

            var workItem = new SyncOnlinePlatformContainersWorkItem(
                new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true, IsValidated: true),
                notificationQueue.Object,
                new Dictionary<string, DockerContainer>
                {
                    [staleSnapshotContainer.Id] = staleSnapshotContainer
                },
                cache,
                Mock.Of<IContainerStreamManager>(),
                Mock.Of<IDeploymentStreamManager>(),
                Mock.Of<IStackStreamManager>(),
                snapshotStartedAt: DateTimeOffset.UtcNow.ToUnixTimeSeconds() + 1,
                cacheMutationVersion: capturedVersion,
                logger: Mock.Of<ILogger<ContainerSyncJob>>());

            await workItem.ExecuteAsync(uow, ct);
        }

        await using var assertScope = Services.CreateAsyncScope();
        var assertUow = assertScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var storedAfter = Assert.Single(
            await assertUow.Containers.GetByPlatformIdAsync(platformId, ct),
            container => container.DockerContainerId == staleSnapshotContainer.Id);
        Assert.Equal(ContainerStateStatus.Offline, storedAfter.State);
        notificationQueue.Verify(
            queue => queue.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task OnlineSync_Should_Persist_Provided_StackId()
    {
        var stack = Stack.Create(
            name: "stack-sync-app",
            createdByActorId: Constants.SystemId,
            StackSource: StackSource.WebEditor,
            platformId: platformId,
            spec: new ManualStack(
                ComposeFile: "services:\n  app:\n    image: nginx\n",
                UpdateBehavior: StackUpdateBehavior.Disabled));

        await using (var seedScope = Services.CreateAsyncScope())
        {
            var seedUow = seedScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await seedUow.Stacks.AddAsync(stack, TestContext.Current.CancellationToken);
            await seedUow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var dockerContainer = new DockerContainer(
            Name: "/stack-sync-app-app-1",
            Image: "nginx:latest",
            ImageId: "sha256:nginx",
            Id: "stack-sync-app-container-id",
            State: ContainerStateStatus.Running,
            Ports: new Dictionary<string, IReadOnlyList<HostPortBinding>>(),
            Created: 123456,
            Stack: "stack-sync-app",
            StackId: stack.Id);

        var notificationQueue = new Mock<INotificationQueue>();
        notificationQueue
            .Setup(queue => queue.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()))
            .Returns(ValueTask.CompletedTask);

        await using (var syncScope = Services.CreateAsyncScope())
        {
            var syncUow = syncScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var cache = syncScope.ServiceProvider.GetRequiredService<IPlatformContainerCache>();
            var workItem = new SyncOnlinePlatformContainersWorkItem(
                new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true, IsValidated: true),
                notificationQueue.Object,
                new Dictionary<string, DockerContainer>
                {
                    [dockerContainer.Id] = dockerContainer
                },
                cache,
                Mock.Of<IContainerStreamManager>(),
                Mock.Of<IDeploymentStreamManager>(),
                Mock.Of<IStackStreamManager>(),
                DateTimeOffset.UtcNow.ToUnixTimeSeconds() + 1,
                cache.GetMutationVersion(platformId),
                Mock.Of<ILogger<ContainerSyncJob>>());

            await workItem.ExecuteAsync(syncUow, TestContext.Current.CancellationToken);
        }

        await using var assertScope = Services.CreateAsyncScope();
        var uow = assertScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var dbContainers = await uow.Containers.GetByPlatformIdAsync(platformId, TestContext.Current.CancellationToken);
        var linked = Assert.Single(dbContainers, container => container.DockerContainerId == dockerContainer.Id);
        Assert.Equal(stack.Id, linked.StackId);
    }

    [Fact]
    public async Task OnlineSync_ShouldPreserveRecoveredStackRelationshipWhileRuntimeHasOldOwnerLabel()
    {
        var dockerContainer = new DockerContainer(
            Name: "/beszel",
            Image: "henrygd/beszel:latest",
            ImageId: "sha256:beszel",
            Id: "orphaned-owned-container-id",
            State: ContainerStateStatus.Running,
            Ports: new Dictionary<string, IReadOnlyList<HostPortBinding>>(),
            Created: 123456,
            Stack: "beszel-git",
            StackId: Guid.CreateVersion7(),
            HasCitadelOwnershipLabels: true);
        var notificationQueue = new Mock<INotificationQueue>();
        notificationQueue
            .Setup(queue => queue.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()))
            .Returns(ValueTask.CompletedTask);

        await using (var seedScope = Services.CreateAsyncScope())
        {
            var seedUow = seedScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await seedUow.Containers.AddAsync(
                new Container(
                    name: dockerContainer.Name,
                    dockerImageId: dockerContainer.ImageId,
                    platformId: platformId,
                    dockerContainerId: dockerContainer.Id,
                    state: dockerContainer.State,
                    created: dockerContainer.Created,
                    dockerStack: dockerContainer.Stack),
                TestContext.Current.CancellationToken);
            await seedUow.CommitAsync(TestContext.Current.CancellationToken);
        }

        await using (var syncScope = Services.CreateAsyncScope())
        {
            var syncUow = syncScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var cache = syncScope.ServiceProvider.GetRequiredService<IPlatformContainerCache>();
            var workItem = new SyncOnlinePlatformContainersWorkItem(
                new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true, IsValidated: true),
                notificationQueue.Object,
                new Dictionary<string, DockerContainer>
                {
                    [dockerContainer.Id] = dockerContainer
                },
                cache,
                Mock.Of<IContainerStreamManager>(),
                Mock.Of<IDeploymentStreamManager>(),
                Mock.Of<IStackStreamManager>(),
                DateTimeOffset.UtcNow.ToUnixTimeSeconds() + 1,
                cache.GetMutationVersion(platformId),
                Mock.Of<ILogger<ContainerSyncJob>>());

            await workItem.ExecuteAsync(syncUow, TestContext.Current.CancellationToken);
        }

        var recoveredStack = Stack.Create(
            name: $"recovered-beszel-{Guid.CreateVersion7():N}",
            createdByActorId: Constants.SystemId,
            StackSource: StackSource.WebEditor,
            platformId: platformId,
            spec: new ManualStack(
                ComposeFile: "services:\n  beszel:\n    image: henrygd/beszel:latest\n",
                UpdateBehavior: StackUpdateBehavior.Disabled,
                ProjectName: dockerContainer.Stack));

        await using (var recoveryScope = Services.CreateAsyncScope())
        {
            var recoveryUow = recoveryScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var persisted = Assert.Single(
                await recoveryUow.Containers.GetByPlatformIdAsync(platformId, TestContext.Current.CancellationToken),
                container => container.DockerContainerId == dockerContainer.Id);

            Assert.Null(persisted.StackId);
            Assert.True(persisted.HasCitadelOwnershipLabels);
            await recoveryUow.Stacks.AddAsync(recoveredStack, TestContext.Current.CancellationToken);

            var blockedWithoutRecovery = await recoveryUow.Containers.TryAssignComposeProjectToStackAsync(
                platformId,
                dockerContainer.Stack!,
                [persisted.Id],
                [persisted.DockerContainerId],
                recoveredStack.Id,
                orphanedOwnerStackId: null,
                cancellationToken: TestContext.Current.CancellationToken);
            var assigned = await recoveryUow.Containers.TryAssignComposeProjectToStackAsync(
                platformId,
                dockerContainer.Stack!,
                [persisted.Id],
                [persisted.DockerContainerId],
                recoveredStack.Id,
                orphanedOwnerStackId: dockerContainer.StackId,
                cancellationToken: TestContext.Current.CancellationToken);
            await recoveryUow.CommitAsync(TestContext.Current.CancellationToken);

            Assert.Equal(0, blockedWithoutRecovery);
            Assert.Equal(1, assigned);
        }

        await using (var resyncScope = Services.CreateAsyncScope())
        {
            var resyncUow = resyncScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var cache = resyncScope.ServiceProvider.GetRequiredService<IPlatformContainerCache>();
            var workItem = new SyncOnlinePlatformContainersWorkItem(
                new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true, IsValidated: true),
                notificationQueue.Object,
                new Dictionary<string, DockerContainer> { [dockerContainer.Id] = dockerContainer },
                cache,
                Mock.Of<IContainerStreamManager>(),
                Mock.Of<IDeploymentStreamManager>(),
                Mock.Of<IStackStreamManager>(),
                DateTimeOffset.UtcNow.ToUnixTimeSeconds() + 1,
                cache.GetMutationVersion(platformId),
                Mock.Of<ILogger<ContainerSyncJob>>());

            await workItem.ExecuteAsync(resyncUow, TestContext.Current.CancellationToken);
        }

        await using var assertScope = Services.CreateAsyncScope();
        var assertUow = assertScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var recovered = Assert.Single(
            await assertUow.Containers.GetByPlatformIdAsync(platformId, TestContext.Current.CancellationToken),
            container => container.DockerContainerId == dockerContainer.Id);
        Assert.Equal(recoveredStack.Id, recovered.StackId);
    }

    [Fact]
    public async Task CreatedEvent_Should_Persist_Provided_StackId()
    {
        var stack = Stack.Create(
            name: "daemon-stack",
            createdByActorId: Constants.SystemId,
            StackSource: StackSource.WebEditor,
            platformId: platformId,
            spec: new ManualStack(
                ComposeFile: "services:\n  app:\n    image: nginx\n",
                UpdateBehavior: StackUpdateBehavior.Disabled));

        await using (var seedScope = Services.CreateAsyncScope())
        {
            var seedUow = seedScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await seedUow.Stacks.AddAsync(stack, TestContext.Current.CancellationToken);
            await seedUow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var dockerContainer = new DockerContainer(
            Name: "/daemon-stack-app-1",
            Image: "nginx:latest",
            ImageId: "sha256:nginx",
            Id: "daemon-stack-container-id",
            State: ContainerStateStatus.Running,
            Ports: new Dictionary<string, IReadOnlyList<HostPortBinding>>(),
            Created: 123456,
            Stack: "daemon-stack",
            StackId: stack.Id);

        var notificationQueue = new Mock<INotificationQueue>();
        notificationQueue
            .Setup(queue => queue.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()))
            .Returns(ValueTask.CompletedTask);
        var unmanagedAlerts = Channel.CreateUnbounded<UnmanagedContainerAlertRequest>();

        await using (var syncScope = Services.CreateAsyncScope())
        {
            var syncUow = syncScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var cache = syncScope.ServiceProvider.GetRequiredService<IPlatformContainerCache>();
            var workItem = new ContainerCreatedWorkItem(
                new DaemonContainerEventInfo("create", dockerContainer.Id, dockerContainer),
                platformId,
                notificationQueue.Object,
                unmanagedAlerts.Writer,
                Mock.Of<IDockerDaemonStreamManager>(),
                cache,
                Mock.Of<IContainerEventBroadcaster>(),
                Mock.Of<ILogger<ContainerCreatedWorkItem>>());

            await workItem.ExecuteAsync(syncUow, TestContext.Current.CancellationToken);
        }

        await using var assertScope = Services.CreateAsyncScope();
        var uow = assertScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var dbContainers = await uow.Containers.GetByPlatformIdAsync(platformId, TestContext.Current.CancellationToken);
        var linked = Assert.Single(dbContainers, container => container.DockerContainerId == dockerContainer.Id);
        Assert.Equal(stack.Id, linked.StackId);
    }

    [Fact]
    public async Task CreatedEvent_ShouldPersistSystemClassificationAndSkipUnmanagedAlert()
    {
        var dockerContainer = new DockerContainer(
            Name: "/citadel-server-1",
            Image: "citadel:latest",
            ImageId: "sha256:citadel",
            Id: "citadel-system-container-id",
            State: ContainerStateStatus.Running,
            Ports: new Dictionary<string, IReadOnlyList<HostPortBinding>>(),
            Created: 123456,
            Stack: "citadel",
            IsSystem: true,
            SystemRole: ContainerSystemRole.Core);
        var notificationQueue = new Mock<INotificationQueue>();
        notificationQueue
            .Setup(queue => queue.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()))
            .Returns(ValueTask.CompletedTask);
        var unmanagedAlerts = Channel.CreateUnbounded<UnmanagedContainerAlertRequest>();

        await using (var syncScope = Services.CreateAsyncScope())
        {
            var syncUow = syncScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var cache = syncScope.ServiceProvider.GetRequiredService<IPlatformContainerCache>();
            var workItem = new ContainerCreatedWorkItem(
                new DaemonContainerEventInfo("create", dockerContainer.Id, dockerContainer),
                platformId,
                notificationQueue.Object,
                unmanagedAlerts.Writer,
                Mock.Of<IDockerDaemonStreamManager>(),
                cache,
                Mock.Of<IContainerEventBroadcaster>(),
                Mock.Of<ILogger<ContainerCreatedWorkItem>>());

            await workItem.ExecuteAsync(syncUow, TestContext.Current.CancellationToken);
        }

        await using var assertScope = Services.CreateAsyncScope();
        var uow = assertScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var persisted = Assert.Single(
            await uow.Containers.GetByPlatformIdAsync(platformId, TestContext.Current.CancellationToken),
            container => container.DockerContainerId == dockerContainer.Id);
        Assert.True(persisted.IsSystem);
        Assert.Equal(ContainerSystemRole.Core, persisted.SystemRole);
        Assert.False(unmanagedAlerts.Reader.TryRead(out _));
    }

    [Fact]
    public async Task UpdatedEvent_ShouldRefreshSystemClassification()
    {
        const string containerId = "system-classification-update-id";
        var container = new Container(
            name: "citadel-server",
            dockerImageId: "sha256:citadel",
            platformId: platformId,
            dockerContainerId: containerId,
            state: ContainerStateStatus.Running);

        await using (var seedScope = Services.CreateAsyncScope())
        {
            var uow = seedScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Containers.AddAsync(container, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var eventContainer = new DockerContainer(
            Name: "/citadel-server",
            Image: "citadel:latest",
            ImageId: "sha256:citadel",
            Id: containerId,
            State: ContainerStateStatus.Running,
            IsSystem: true,
            SystemRole: ContainerSystemRole.Core);
        var notificationQueue = new Mock<INotificationQueue>();
        notificationQueue
            .Setup(queue => queue.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()))
            .Returns(ValueTask.CompletedTask);

        await using (var updateScope = Services.CreateAsyncScope())
        {
            var uow = updateScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var workItem = new ContainerUpdatedWorkItem(
                new DaemonContainerEventInfo("update", containerId, eventContainer),
                notificationQueue.Object,
                Mock.Of<IActivityStreamManager>(),
                Mock.Of<IDeploymentStreamManager>(),
                Mock.Of<IDockerDaemonStreamManager>(),
                Mock.Of<IContainerEventBroadcaster>(),
                Mock.Of<IStackStreamManager>(),
                Mock.Of<ILogger<ContainerUpdatedWorkItem>>());

            await workItem.ExecuteAsync(uow, TestContext.Current.CancellationToken);
        }

        await using var assertScope = Services.CreateAsyncScope();
        var assertUow = assertScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var persisted = await assertUow.Containers.GetByIdAsync(containerId, TestContext.Current.CancellationToken);
        Assert.NotNull(persisted);
        Assert.True(persisted.IsSystem);
        Assert.Equal(ContainerSystemRole.Core, persisted.SystemRole);
    }

    [Fact]
    public async Task UpdatesExistingContainers_WhenPropertiesChange()
    {
        // Arrange: Seed DB with a container, then fresh list has same container with different properties
        var containerId = "update-id";
        var oldContainer = new Container(
            name: "old",
            dockerImageId: "fake-id",
            platformId: platformId,
            ports: new Dictionary<string, IReadOnlyList<HostPortBinding>>(),
            dockerContainerId: containerId,
            state: ContainerStateStatus.Paused);
        await using (var uow = Services.GetRequiredService<IUnitOfWork>())
        {
            await uow.Containers.AddAsync(oldContainer, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var updatedDockerContainer = new DockerContainer(
            Name: "updated",
            Image: "updated:latest",
            ImageId: "fake-id",
            Id: containerId,
            State: ContainerStateStatus.Running,
            Ports: new Dictionary<string, IReadOnlyList<HostPortBinding>>(),
            Created: 123456,
            Stack: null
        );

        syncBarrierMock.Setup(x => x.WaitForAsync<ImageSyncJob>(It.IsAny<Guid>(), It.IsAny<CancellationToken>())).Returns(ValueTask.CompletedTask);
        containerFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>())).Returns(containerConnector.Object);
        containerConnector.Setup(x => x.ListContainersAsync(It.IsAny<ContainerFilterCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new Dictionary<string, DockerContainer> { { containerId, updatedDockerContainer } } as IReadOnlyDictionary<string, DockerContainer>));

        // Act
        var checkpoint = dbWorkQueue.CreateCheckpoint();
        await broadcaster.PublishAsync(new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true, IsValidated: true),
            cancellationToken: TestContext.Current.CancellationToken);
        await dbWorkQueue.WaitForIdleAfterAsync(
            checkpoint,
            TestContext.Current.CancellationToken);

        // Assert: Container should be updated
        await using var scope = Services.CreateAsyncScope();
        var uow2 = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var dbContainers = await uow2.Containers.GetByPlatformIdAsync(platformId, TestContext.Current.CancellationToken);
        var updated = dbContainers.First(c => c.DockerContainerId == containerId);
        Assert.Equal("updated", updated.Name);
        Assert.Equal(ContainerStateStatus.Running, updated.State);
    }

    [Fact]
    public async Task SetsAllContainersOffline_WhenPlatformGoesOffline()
    {
        // Act
        var checkpoint = dbWorkQueue.CreateCheckpoint();
        await broadcaster.PublishAsync(new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: false),
            cancellationToken: TestContext.Current.CancellationToken);
        await dbWorkQueue.WaitForIdleAfterAsync(
            checkpoint,
            TestContext.Current.CancellationToken);

        // Assert: All containers should be offline
        await using var scope = Services.CreateAsyncScope();
        var uow2 = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var dbContainers = await uow2.Containers.GetByPlatformIdAsync(platformId, TestContext.Current.CancellationToken);
        Assert.All(dbContainers, c => Assert.Equal(ContainerStateStatus.Offline, c.State));
    }
}

