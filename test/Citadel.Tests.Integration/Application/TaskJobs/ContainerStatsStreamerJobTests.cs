using System.Threading.Channels;
using Application.Configs;
using Application.Services;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Hosting.Common.ObjectPoolManager;
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
        var objectPoolManager = Services.GetRequiredService<IObjectPoolManager>();
        var pooledDict = objectPoolManager.GetPooled<Dictionary<string, DockerContainerStat>>();

        _containerFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>())).Returns(_containerConnectorMock.Object);
        _containerConnectorMock
            .Setup(x => x.StreamContainersStatsAsync(It.IsAny<StreamContainersStatsCommand>(), It.IsAny<CancellationToken>()))
            .Returns(GetDockerContainerStats(pooledDict));

        // Act
        await _broadcaster.BroadcastAsync(new PlatformHealth(_platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true),
            cancellationToken: TestContext.Current.CancellationToken);
        await Task.Delay(500, TestContext.Current.CancellationToken);

        // Assert: Verify that the objects were returned to the pool
        var pooledDict2 = objectPoolManager.GetPooled<Dictionary<string, DockerContainerStat>>();
        Assert.Same(pooledDict.Value, pooledDict2.Value);
        Assert.Empty(pooledDict2.Value);
        pooledDict2.Dispose();
    }

    private static async IAsyncEnumerable<PooledHandle<Dictionary<string, DockerContainerStat>>> GetDockerContainerStats(PooledHandle<Dictionary<string, DockerContainerStat>> pooledDict)
    {
        pooledDict.Value.Add("container-id-0", new DockerContainerStat(1, 1, 1, 1, 1));
        yield return pooledDict;
        pooledDict.Value.Add("container-id-1", new DockerContainerStat(2, 2, 2, 2, 2));
        yield return pooledDict;
        await Task.CompletedTask;
    }
}
