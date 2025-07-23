
using System.Threading.Channels;
using Application.Configs;
using Application.Services;
using Application.Services.Abstractions;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Options;
using Moq;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.TaskJobs;

public class ContainerStatsWriterJobTests : IntegrationTestBase
{
    private readonly Mock<IObjectPoolManager> _objectPoolManagerMock = new();
    private readonly Mock<ISignalRConnectionTracker> _connectionTrackerMock = new();
    private readonly Mock<IContainerHubDispatcher> _containerHubDispatcherMock = new();
    private readonly Channel<ContainersStatBatch> _channel = Channel.CreateUnbounded<ContainersStatBatch>();

    private readonly Mock<IConnectorFactory<IContainerConnector>> _containerFactoryMock = new();
    private readonly Mock<IContainerConnector> _containerConnectorMock = new();
    private readonly Mock<IOptions<JobConfiguration>> _configMock = new();
    private readonly TestPlatformHealthBroadCaster _broadcaster = new();

    private Guid _platformId;
    private Guid _containerId;
    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<IHostedService>();
        services.AddHostedService<ContainerStatsWriterJob>();

        services.AddSingleton(_channel);
        services.AddSingleton(_configMock.Object);
        services.AddSingleton(_containerFactoryMock.Object);
        services.AddSingleton(_objectPoolManagerMock.Object);
        services.AddSingleton(_connectionTrackerMock.Object);
        services.AddSingleton(_containerConnectorMock.Object);
        services.AddSingleton(_containerHubDispatcherMock.Object);
        services.AddSingleton<IPlatformHealthBroadCaster>(_broadcaster);

        _configMock.Setup(x => x.Value).Returns(new JobConfiguration() { BatchSize = 2 });
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();
        await uow.Platforms.AddPlatformAsync(platform, TestContext.Current.CancellationToken);

        var container = new Container(
                name: "container-1",
                image: "image-1",
                platformId: platform.Id,
                ports: [],
                containerId: "container-id-1",
                state: ContainerStateStatus.Running);
        await uow.Containers.AddAsync(container, TestContext.Current.CancellationToken);
        
        await uow.CommitAsync();
        _platformId = platform.Id;
        _containerId = container.Id;
    }

    [Fact]
    public async Task ExecuteAsync_ShouldFlushWhenBatchSizeIsReached()
    {
        // Arrange
        var time = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        var stats = new List<ContainerStat>
        {
            new(_containerId, 100, 5, 300, 100, 200, time),
            new(_containerId, 200, 2, 600, 200, 400, time - 60),
        };

        _connectionTrackerMock.Setup(c => c.HasUsersInGroup(It.IsAny<string>())).Returns(true);
        var batch = new ContainersStatBatch(_platformId, stats);

        // Act
        await _channel.Writer.WriteAsync(batch, TestContext.Current.CancellationToken);
        await Task.Delay(500, TestContext.Current.CancellationToken);

        // Assert
        _containerHubDispatcherMock.Verify(d => d.SendContainersStats(_platformId, It.IsAny<IEnumerable<ContainerStat>>()), Times.Once);
        _objectPoolManagerMock.Verify(p => p.Return(It.IsAny<List<ContainerStat>>()), Times.Exactly(2));
        _objectPoolManagerMock.Verify(p => p.Return(It.IsAny<ContainerStat>()), Times.Exactly(2));

        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var containers = await db.ContainerStats.GetStatsAggregatedLast24HoursAsync("container-id-1", TestContext.Current.CancellationToken);
        Assert.Equal(2, containers.Count());
    }

    [Fact]
    public async Task ExecuteAsync_ShouldFlushWhenFlushIntervalIsReached()
    {
        // Arrange
        _configMock.Setup(c => c.Value).Returns(new JobConfiguration { BatchSize = 100, FlashInterval = 1 });
        var time = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        var stats = new List<ContainerStat>
        {
            new(_containerId, 100, 5, 300, 100, 200, time),
            new(_containerId, 200, 2, 600, 200, 400, time - 60),
        };

        var batch = new ContainersStatBatch(_platformId, stats);
        _connectionTrackerMock.Setup(c => c.HasUsersInGroup(It.IsAny<string>())).Returns(true);

        // Act
        await _channel.Writer.WriteAsync(batch, TestContext.Current.CancellationToken);
        await Task.Delay(1500, TestContext.Current.CancellationToken);

        // Assert
        _containerHubDispatcherMock.Verify(d => d.SendContainersStats(_platformId, It.IsAny<IEnumerable<ContainerStat>>()), Times.Once);
        _objectPoolManagerMock.Verify(p => p.Return(It.IsAny<List<ContainerStat>>()), Times.Exactly(2));
        _objectPoolManagerMock.Verify(p => p.Return(It.IsAny<ContainerStat>()), Times.Exactly(2));

        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var containers = await db.ContainerStats.GetStatsAggregatedLast24HoursAsync("container-id-1", TestContext.Current.CancellationToken);
        Assert.Equal(2, containers.Count());
    }

    [Fact]
    public async Task ExecuteAsync_ShouldNotNotifyClients_WhenNoUsersInGroup()
    {
        // Arrange
        var time = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        var stats = new List<ContainerStat>
        {
            new(_containerId, 100, 5, 300, 100, 200, time),
            new(_containerId, 200, 2, 600, 200, 400, time - 60),
        };

        var batch = new ContainersStatBatch(_platformId, stats);
        _connectionTrackerMock.Setup(c => c.HasUsersInGroup(It.IsAny<string>())).Returns(false);

        // Act
        await _channel.Writer.WriteAsync(batch, TestContext.Current.CancellationToken);
        await Task.Delay(1500, TestContext.Current.CancellationToken);

        // Assert
        _containerHubDispatcherMock.Verify(d => d.SendContainersStats(It.IsAny<Guid>(), It.IsAny<List<ContainerStat>>()), Times.Never);
        _objectPoolManagerMock.Verify(p => p.Return(It.IsAny<List<ContainerStat>>()), Times.Exactly(2));
        _objectPoolManagerMock.Verify(p => p.Return(It.IsAny<ContainerStat>()), Times.Exactly(2));

        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var containers = await db.ContainerStats.GetStatsAggregatedLast24HoursAsync("container-id-1", TestContext.Current.CancellationToken);
        Assert.Equal(2, containers.Count());
    }
}
