using Application.Services;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Images;
using Domain.Entities.Deployments;
using Hosting.Common;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Moq;
using System.Collections.Immutable;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.TaskJobs;

public class DeploymentImageScannerJobTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<IConnectorFactory<IImageConnector>> _imageConnectorFactoryMock = new();
    private readonly Mock<IImageConnector> _imageConnectorMock = new();
    private readonly Mock<IDelayWithJitterService> _delayWithJitter = new();
    private readonly Mock<IPlatformContainerCache> _platformCache = new();

    private Func<CancellationToken, Task>? _runScannerJob;
    private Guid _platformId;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<IHostedService>();
        services.RemoveAll<IDelayWithJitterService>();
        services.RemoveAll<IPlatformContainerCache>();

        services.AddHostedService<DeploymentImageScannerJob>();
        services.AddSingleton(_ => _imageConnectorFactoryMock.Object);
        services.AddSingleton(_ => _imageConnectorMock.Object);
        services.AddSingleton(_ => _delayWithJitter.Object);
        services.AddSingleton(_ => _platformCache.Object);

        var cacheEntry = new PlatformCacheEntry(_platformId, "localhost", PlatformConnectorType.Local, new Dictionary<string, Guid>().ToImmutableDictionary());
        var emptyError = Error.Empty as Error;
        _platformCache.Setup(x => x.TryGetCacheEntry(It.IsAny<Guid>(), out cacheEntry, out emptyError)).Returns(true);

        _imageConnectorFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>()))
            .Returns(_imageConnectorMock.Object);

        var dist = new DistributionResult(new OCIDescriptorResult("zip", "new-digest", 2000000, new OCIPlatformResult("x86", "linux", "6.2"), "art-1"));
        _imageConnectorMock.Setup(x => x.DistributionInspectAsync(It.IsAny<DistributionInspectCommand>(), It.IsAny<CancellationToken>()))
            .Returns(Task.FromResult(Result.Success(dist)));

        _delayWithJitter
            .Setup(x => x.DelayWithJitterForAsync(It.IsAny<Func<CancellationToken, Task>>(), It.IsAny<TimeSpan>(), It.IsAny<CancellationToken>()))
            .Returns<Func<CancellationToken, Task>, TimeSpan, CancellationToken>((func, _, __) =>
            {
                _runScannerJob = func;
                return Task.CompletedTask;
            });
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();
        var deployment1 = CreateDeployment(platform.Id, "deployment-1", "nginx:latest");
        var deployment2 = CreateDeployment(platform.Id, "deployment-2", "nginx:latest");

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Deployments.AddAsync(deployment1, TestContext.Current.CancellationToken);
        await uow.Deployments.AddAsync(deployment2, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        _platformId = platform.Id;
    }

    [Fact]
    public async Task Scanner_Should_Store_Digest_In_Cache()
    {
        await RunScannerJobOnceAsync(TestContext.Current.CancellationToken);

        var cache = Services.GetRequiredService<ImageDigestCache>();
        var key = new ImageKey(Constants.DefaultRegistryId, "nginx", "latest");

        Assert.True(cache.TryGet(key, out var entry));
        Assert.Equal("new-digest", entry.Digest);
    }

    [Fact]
    public async Task Scanner_Should_Inspect_Unique_Image_Once_Per_Cycle()
    {
        await RunScannerJobOnceAsync(TestContext.Current.CancellationToken);

        _imageConnectorMock.Verify(
            x => x.DistributionInspectAsync(It.IsAny<DistributionInspectCommand>(), It.IsAny<CancellationToken>()),
            Times.Once);
    }

    private async Task RunScannerJobOnceAsync(CancellationToken cancellationToken)
    {
        if (_runScannerJob is null)
            throw new InvalidOperationException("Deployment image scanner job was not initialized.");

        using var cts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        cts.CancelAfter(TimeSpan.FromSeconds(1));

        try
        {
            await _runScannerJob(cts.Token);
        }
        catch (OperationCanceledException) when (cts.IsCancellationRequested)
        {
        }
    }

    private static Deployment CreateDeployment(Guid platformId, string name, string imageTag)
        => new(
            name: name,
            description: "scanner-test",
            platformId: platformId,
            createdByActorId: Constants.SystemId,
            spec: new DeploymentSpec(
                UpdateBehavior: UpdateBehavior.Notify,
                Image: new ExternalImage(Constants.DefaultRegistryId, imageTag)));
}
