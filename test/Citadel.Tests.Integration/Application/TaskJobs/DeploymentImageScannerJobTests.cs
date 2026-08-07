using Application.Services;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Images;
using Domain.Entities.Deployments;
using Domain.Entities.Platforms;
using Domain.Entities.SwarmServices;
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
        var swarmPlatform = CreateSwarmPlatform();
        var deployment1 = CreateDeployment(platform.Id, "deployment-1", "nginx:latest");
        var deployment2 = CreateDeployment(platform.Id, "deployment-2", "nginx:latest");
        var swarmDeployment = CreateDeployment(
            swarmPlatform.Id,
            "swarm-deployment",
            "redis:latest");
        var managedSwarmService = CreateAppliedSwarmService(swarmPlatform.Id, "traefik:latest");

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Platforms.AddAsync(swarmPlatform, TestContext.Current.CancellationToken);
        await uow.Deployments.AddAsync(deployment1, TestContext.Current.CancellationToken);
        await uow.Deployments.AddAsync(deployment2, TestContext.Current.CancellationToken);
        await uow.Deployments.AddAsync(swarmDeployment, TestContext.Current.CancellationToken);
        await uow.SwarmServices.AddAsync(managedSwarmService, TestContext.Current.CancellationToken);
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
            x => x.DistributionInspectAsync(
                It.Is<DistributionInspectCommand>(command => command.ImageName == "nginx:latest"),
                It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task Scanner_Should_Not_Inspect_Unsupported_Swarm_Workloads()
    {
        await RunScannerJobOnceAsync(TestContext.Current.CancellationToken);

        _imageConnectorMock.Verify(
            x => x.DistributionInspectAsync(
                It.Is<DistributionInspectCommand>(command => command.ImageName == "redis:latest"),
                It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task Scanner_Should_Inspect_ManagedSwarmServiceImages()
    {
        await RunScannerJobOnceAsync(TestContext.Current.CancellationToken);

        _imageConnectorMock.Verify(
            x => x.DistributionInspectAsync(
                It.Is<DistributionInspectCommand>(command => command.ImageName == "traefik:latest"),
                It.IsAny<CancellationToken>()),
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

    private static Deployment CreateDeployment(
        Guid platformId,
        string name,
        string imageTag)
        => new(
            name: name,
            description: "scanner-test",
            platformId: platformId,
            createdByActorId: Constants.SystemId,
            spec: new DeploymentSpec(
                UpdateBehavior: UpdateBehavior.Notify,
                Image: new ExternalImage(Constants.DefaultRegistryId, imageTag)));

    private static Platform CreateSwarmPlatform() => new(
        name: "scanner-swarm",
        address: "https://scanner-swarm.test",
        networkCount: 0,
        volumeCount: 0,
        imageCount: 0,
        cpuCount: 1,
        memTotal: 1024,
        serverVersion: null,
        agentVersion: null,
        status: PlatformStatus.Online,
        connectorType: PlatformConnectorType.Agent,
        platformDescriptor: new DockerSwarmPlatformDescriptor(
            "node", "10.0.0.1", "Active", true, 1, 1,
            "scanner-swarm-daemon", 0, 0, 0, 0,
            ClusterId: "scanner-swarm-cluster"),
        clusterId: "scanner-swarm-cluster");

    private static SwarmService CreateAppliedSwarmService(Guid platformId, string imageTag)
    {
        var service = new SwarmService(
            "scanner-managed-service",
            platformId,
            Constants.SystemId,
            new SwarmServiceSpec
            {
                Image = new SwarmExternalImage(Constants.DefaultRegistryId, imageTag),
                UpdateBehavior = UpdateBehavior.Notify,
                SchedulingMode = SwarmServiceSchedulingMode.Replicated,
                Replicas = 1
            });
        service.TryPrepareOperation(
            SwarmServiceOperationKind.Apply,
            Guid.CreateVersion7(),
            Constants.SystemId,
            targetRuntimeHash: "runtime-hash");
        service.MarkOperationAttempted();
        service.MarkOperationAccepted("docker-service", 1);
        service.CompleteOperation(SwarmServiceOperationState.Completed, "runtime-hash", "old-digest");
        return service;
    }
}
