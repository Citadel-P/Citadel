
using System.Threading.Channels;
using Application.Configs;
using Application.Services;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Options;
using Moq;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.TaskJobs;

public class ContainerStatsStreamerJobTests : IntegrationTestBase
{
    private readonly Channel<ContainersStatBatch> _channel = Channel.CreateUnbounded<ContainersStatBatch>();
    private readonly Mock<IConnectorFactory<IContainerConnector>> _containerFactoryMock = new();
    private readonly Mock<IContainerConnector> _containerConnectorMock = new();
    private readonly Mock<IOptions<JobConfiguration>> _configMock = new();
    private readonly TestPlatformHealthBroadCaster _broadcaster = new();

    private Guid _platformId;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<IHostedService>();
        services.AddHostedService<ContainerStatsStreamerJob>();

        services.AddSingleton(_channel);
        services.AddSingleton(_configMock.Object);
        services.AddSingleton(_containerFactoryMock.Object);
        services.AddSingleton(_containerConnectorMock.Object);
        services.AddSingleton<IPlatformHealthBroadCaster>(_broadcaster);

        _configMock.Setup(x => x.Value).Returns(new JobConfiguration()
        {
            BatchSize = 100,
            ContainersInfoInterval = 10
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
                state: ContainerStateStatus.Running), TestContext.Current.CancellationToken);
        }

        await uow.CommitAsync();
        _platformId = platform.Id;
    }

    [Fact]
    public async Task StreamerJob_ProcessesStats_And_ReturnsToPool()
    {
        // Arrange
        var containerStats = new DockerContainerStats();
       
        _containerFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>())).Returns(_containerConnectorMock.Object);
        _containerConnectorMock
            .Setup(x => x.StreamContainerStatsAsync(It.IsAny<StreamContainerStatsCommand>(), It.IsAny<CancellationToken>()))
            .Returns(GetDockerContainerStats(containerStats));

        var objectPoolManager = Services.GetRequiredService<IObjectPoolManager>();

        // Act
        await _broadcaster.BroadcastAsync(new PlatformHealth(_platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true),
            cancellationToken: TestContext.Current.CancellationToken);
        await Task.Delay(500, TestContext.Current.CancellationToken);

        // Assert: Verify that the objects were returned to the pool
        var pool = objectPoolManager.GetPool<DockerContainerStats>();
        var statsFromPool = pool.Get();
        Assert.NotSame(containerStats, statsFromPool);
        pool.Return(statsFromPool);
    }

    private static async IAsyncEnumerable<DockerContainerStats> GetDockerContainerStats(DockerContainerStats containerStats)
    {
        containerStats.Add("container-id-0", new DockerContainerStat(1, 1, 1, 1, 1));
        yield return containerStats;
        containerStats.Add("container-id-1", new DockerContainerStat(2, 2, 2, 2, 2));
        yield return containerStats;
        await Task.CompletedTask;
    }
}
