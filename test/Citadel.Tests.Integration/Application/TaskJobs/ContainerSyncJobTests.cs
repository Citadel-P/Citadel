using Application.Configs;
using Application.Services;
using Application.Services.Abstractions;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Options;
using Moq;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.TaskJobs;

public class ContainerSyncJobTests : IntegrationTestBase
{
    private readonly Mock<IConnectorFactory<IContainerConnector>> containerFactoryMock = new();
    private readonly Mock<IContainerConnector> containerConnector = new();

    private readonly Mock<IOptions<JobConfiguration>> configMock = new();
    private readonly TestPlatformHealthBroadCaster broadcaster = new();
    private readonly Mock<IContainersStreamManager> streamManagerMock = new();

    private Guid platformId;
    private const int batchSize = 2;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        // Remove all existing hosted services
        services.RemoveAll<IHostedService>();

        services.AddHostedService<ContainerSyncJob>();

        services.AddSingleton(streamManagerMock.Object);
        services.AddSingleton(configMock.Object);
        services.AddSingleton(containerConnector.Object);
        services.AddSingleton(containerFactoryMock.Object);
        services.AddSingleton<IPlatformHealthBroadCaster>(broadcaster);

        configMock.Setup(x => x.Value).Returns(new JobConfiguration()
        {
            BatchSize = batchSize
        });
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();

        await uow.Platforms.AddPlatformAsync(platform, TestContext.Current.CancellationToken);
        foreach (var container in Fakes.GetDummyContainers())
        {
            await uow.Containers.AddAsync(new Container(
                    name: container.Name,
                    image: container.Image,
                    platformId: platform.Id,
                    ports: [],
                    containerId: container.ContainerId,
                    state: ContainerStateStatus.Offline)
                , TestContext.Current.CancellationToken);
        }

        await uow.CommitAsync();

        platformId = platform.Id;
    }

    [Fact]
    public async Task SynchronizesContainers_WhenPlatformBecomesOnline()
    {
        // Arrange
        containerFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>())).Returns(containerConnector.Object);

        containerConnector.Setup(x => x.ListContainersAsync(It.IsAny<ContainerFilterCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(Fakes.GetDummyContainers().ToDictionary(c => c.ContainerId) as IReadOnlyDictionary<string, DockerContainer>));

        // Act
        await broadcaster.BroadcastAsync(new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true),
            cancellationToken: TestContext.Current.CancellationToken);

        await Task.Delay(500, TestContext.Current.CancellationToken); // wait for jobs to process

        // Assert
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var cache = scope.ServiceProvider.GetRequiredService<IPlatformContainerCache>();

        var dbContainers = await uow.Containers.GetByPlatformIdAsync(platformId, cancellationToken: TestContext.Current.CancellationToken);

        Assert.All(dbContainers, s =>
            Assert.True(
                s.State == ContainerStateStatus.Running,
                $"Container {s.ContainerId} is not running. Actual state: {s.State}"
            ));
        Assert.True(cache.TryGetContainers(platformId, out var cacheContainers));
        Assert.Equal(3, cacheContainers.Count);
        streamManagerMock.Verify(x => x.SendContainersInfo(It.IsAny<Guid>(), It.IsAny<IEnumerable<Container>>()), Times.Once);
    }

    [Fact]
    public async Task RemovesStaleContainers_WhenNotInFreshList()
    {
        // Arrange: Seed DB with a container that will be missing from the fresh list
        var staleContainer = new Container(
            name: "stale",
            image: "stale:latest",
            platformId: platformId,
            ports: [],
            containerId: "stale-id",
            state: ContainerStateStatus.Running);
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.Containers.AddAsync(staleContainer, TestContext.Current.CancellationToken);
        await uow.CommitAsync();

        // Only return fresh containers that do NOT include the stale one
        containerFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>())).Returns(containerConnector.Object);
        containerConnector.Setup(x => x.ListContainersAsync(It.IsAny<ContainerFilterCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(Fakes.GetDummyContainers().ToDictionary(c => c.ContainerId) as IReadOnlyDictionary<string, DockerContainer>));

        // Act
        await broadcaster.BroadcastAsync(new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true),
            cancellationToken: TestContext.Current.CancellationToken);
        await Task.Delay(500, TestContext.Current.CancellationToken);

        // Assert: Stale container should be removed
        await using var assertScope = Services.CreateAsyncScope();
        var freshUow = assertScope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var dbContainers = await freshUow.Containers.GetByPlatformIdAsync(platformId, TestContext.Current.CancellationToken);
        Assert.DoesNotContain(dbContainers, c => c.ContainerId == "stale-id");
    }

    [Fact]
    public async Task AddsNewContainers_WhenNotInDatabase()
    {
        // Arrange: DB has no containers, but fresh list has one
        var newDockerContainer = new DockerContainer(
            name: "new",
            image: "new:latest",
            containerId: "new-id",
            state: ContainerStateStatus.Running,
            ports: [],
            created: 123456,
            stack: null
        );
        containerFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>())).Returns(containerConnector.Object);
        containerConnector.Setup(x => x.ListContainersAsync(It.IsAny<ContainerFilterCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new Dictionary<string, DockerContainer> { { "new-id", newDockerContainer } } as IReadOnlyDictionary<string, DockerContainer>));

        // Act
        await broadcaster.BroadcastAsync(new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true),
            cancellationToken: TestContext.Current.CancellationToken);
        await Task.Delay(500, TestContext.Current.CancellationToken);

        // Assert: New container should be added
        await using var scope = Services.CreateAsyncScope();
        var uow2 = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var dbContainers = await uow2.Containers.GetByPlatformIdAsync(platformId, TestContext.Current.CancellationToken);
        Assert.Contains(dbContainers, c => c.ContainerId == "new-id");
    }

    [Fact]
    public async Task UpdatesExistingContainers_WhenPropertiesChange()
    {
        // Arrange: Seed DB with a container, then fresh list has same container with different properties
        var containerId = "update-id";
        var oldContainer = new Container(
            name: "old",
            image: "old:latest",
            platformId: platformId,
            ports: [],
            containerId: containerId,
            state: ContainerStateStatus.Paused);
        await using (var uow = Services.GetRequiredService<IUnitOfWork>())
        {
            await uow.Containers.AddAsync(oldContainer, TestContext.Current.CancellationToken);
            await uow.CommitAsync();
        }

        var updatedDockerContainer = new DockerContainer(
            name: "updated",
            image: "updated:latest",
            containerId: containerId,
            state: ContainerStateStatus.Running,
            ports: [],
            created: 123456,
            stack: null
        );
        containerFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>())).Returns(containerConnector.Object);
        containerConnector.Setup(x => x.ListContainersAsync(It.IsAny<ContainerFilterCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new Dictionary<string, DockerContainer> { { containerId, updatedDockerContainer } } as IReadOnlyDictionary<string, DockerContainer>));

        // Act
        await broadcaster.BroadcastAsync(new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true),
            cancellationToken: TestContext.Current.CancellationToken);
        await Task.Delay(500, TestContext.Current.CancellationToken);

        // Assert: Container should be updated
        await using var scope = Services.CreateAsyncScope();
        var uow2 = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var dbContainers = await uow2.Containers.GetByPlatformIdAsync(platformId, TestContext.Current.CancellationToken);
        var updated = dbContainers.First(c => c.ContainerId == containerId);
        Assert.Equal("updated", updated.Name);
        Assert.Equal("updated:latest", updated.Image);
        Assert.Equal(ContainerStateStatus.Running, updated.State);
    }

    [Fact]
    public async Task SetsAllContainersOffline_WhenPlatformGoesOffline()
    {
        // Act
        await broadcaster.BroadcastAsync(new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: false),
            cancellationToken: TestContext.Current.CancellationToken);
        await Task.Delay(500, TestContext.Current.CancellationToken);

        // Assert: All containers should be offline
        await using var scope = Services.CreateAsyncScope();
        var uow2 = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var dbContainers = await uow2.Containers.GetByPlatformIdAsync(platformId, TestContext.Current.CancellationToken);
        Assert.All(dbContainers, c => Assert.Equal(ContainerStateStatus.Offline, c.State));
    }
}

