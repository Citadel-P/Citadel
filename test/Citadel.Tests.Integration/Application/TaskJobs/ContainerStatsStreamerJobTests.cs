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
                dockerImageId: container.ImageId,
                platformId: platform.Id,
                ports: new Dictionary<string, IReadOnlyList<HostPortBinding>>(),
                dockerContainerId: container.Id,
                state: ContainerStateStatus.Running), TestContext.Current.CancellationToken);
        }

        await uow.CommitAsync(TestContext.Current.CancellationToken);
        _platformId = platform.Id;
    }

}
