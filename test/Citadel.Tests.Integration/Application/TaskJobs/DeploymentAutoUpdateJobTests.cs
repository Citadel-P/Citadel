using Application.Services;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Deployments;
using Domain.Entities.Deployments;
using Domain.Entities.Platforms;
using Domain.Entities.SwarmServices;
using Hosting.Common;
using Infrastructure.Repositories.DbQueue;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Moq;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.TaskJobs;

public class DeploymentAutoUpdateJobTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<IDelayWithJitterService> _delayWithJitter = new();
    private readonly Mock<IApplyDeploymentService> _applyDeploymentService = new();
    private readonly ObservableDbWorkQueue _dbWorkQueue = new();

    private Guid _deploymentId;
    private Guid _platformId;
    private Guid _swarmServiceId;
    private Guid _swarmPlatformId;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<IHostedService>();
        services.RemoveAll<IDelayWithJitterService>();
        services.RemoveAll<IApplyDeploymentService>();

        services
            .AddHostedService<DeploymentAutoUpdateJob>()
            .AddHostedService<DbWriteWorker>()
            .AddHostedService<NotificationWorker>()
            .AddHostedService<AlertRuleCacheWarmup>();

        services.AddSingleton(_ => _delayWithJitter.Object);
        services.AddScoped(_ => _applyDeploymentService.Object);
        services.RemoveAll<IDbWorkQueue>();
        services.AddSingleton<IDbWorkQueue>(_dbWorkQueue);

        _delayWithJitter
            .Setup(x => x.DelayWithJitterForAsync(It.IsAny<Func<CancellationToken, Task>>(), It.IsAny<TimeSpan>(), It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();
        var deployment = CreateDeployment(platform.Id, UpdateBehavior.Notify, "old-digest");
        var swarmPlatform = CreateSwarmPlatform();
        var swarmService = CreateAppliedSwarmService(swarmPlatform.Id);

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Platforms.AddAsync(swarmPlatform, TestContext.Current.CancellationToken);
        await uow.Deployments.AddAsync(deployment, TestContext.Current.CancellationToken);
        await uow.SwarmServices.AddAsync(swarmService, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        _deploymentId = deployment.Id;
        _platformId = platform.Id;
        _swarmServiceId = swarmService.Id;
        _swarmPlatformId = swarmPlatform.Id;
    }

    [Fact]
    public async Task AutoUpdateJob_Should_Mark_Deployment_UpToDate_When_Cache_Digest_Matches()
    {
        var cache = Services.GetRequiredService<ImageDigestCache>();
        cache.Set(new ImageKey(Constants.DefaultRegistryId, "nginx", "latest"), "old-digest");

        var checkpoint = _dbWorkQueue.CreateCheckpoint();
        await RunAutoUpdateJobOnceAsync(TestContext.Current.CancellationToken);
        await _dbWorkQueue.WaitForIdleAfterAsync(
            checkpoint,
            TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var deployment = await uow.Deployments.GetAsync(_deploymentId, TestContext.Current.CancellationToken);

        Assert.NotNull(deployment?.AutoUpdateState);
        Assert.Equal(AutoUpdateStatus.UpToDate, deployment.AutoUpdateState!.Status);
        Assert.Equal("old-digest", deployment.AutoUpdateState.CurrentDigest);
        Assert.Equal("old-digest", deployment.AutoUpdateState.RemoteDigest);
    }

    [Fact]
    public async Task AutoUpdateJob_Should_Mark_Deployment_UpdateAvailable_When_Notify_And_Digest_Differs()
    {
        var cache = Services.GetRequiredService<ImageDigestCache>();
        cache.Set(new ImageKey(Constants.DefaultRegistryId, "nginx", "latest"), "new-digest");

        var checkpoint = _dbWorkQueue.CreateCheckpoint();
        await RunAutoUpdateJobOnceAsync(TestContext.Current.CancellationToken);
        await _dbWorkQueue.WaitForIdleAfterAsync(
            checkpoint,
            TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var deployment = await uow.Deployments.GetAsync(_deploymentId, TestContext.Current.CancellationToken);

        Assert.NotNull(deployment?.AutoUpdateState);
        Assert.Equal(AutoUpdateStatus.UpdateAvailable, deployment.AutoUpdateState!.Status);
        Assert.Equal("old-digest", deployment.AutoUpdateState.CurrentDigest);
        Assert.Equal("new-digest", deployment.AutoUpdateState.RemoteDigest);
    }

    [Fact]
    public async Task AutoUpdateJob_Should_Mark_Deployment_Failed_When_AutoDeploy_Fails()
    {
        await UpdateDeploymentBehaviorAsync(UpdateBehavior.AutoDeploy, TestContext.Current.CancellationToken);
        var cache = Services.GetRequiredService<ImageDigestCache>();
        cache.Set(new ImageKey(Constants.DefaultRegistryId, "nginx", "latest"), "new-digest");

        _applyDeploymentService
            .Setup(x => x.ApplyAsync(_deploymentId, Constants.SystemId, true, It.IsAny<CancellationToken>()))
            .Returns(FailedApplyStream());

        var checkpoint = _dbWorkQueue.CreateCheckpoint();
        await RunAutoUpdateJobOnceAsync(TestContext.Current.CancellationToken);
        await _dbWorkQueue.WaitForIdleAfterAsync(
            checkpoint,
            TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var deployment = await uow.Deployments.GetAsync(_deploymentId, TestContext.Current.CancellationToken);

        Assert.NotNull(deployment?.AutoUpdateState);
        Assert.Equal(AutoUpdateStatus.Failed, deployment.AutoUpdateState!.Status);
        Assert.Equal("auto-deploy-error", deployment.AutoUpdateState.LastError);
    }

    [Fact]
    public async Task AutoUpdateJob_Should_Mark_ManagedSwarmService_UpdateAvailable_When_Notify()
    {
        var cache = Services.GetRequiredService<ImageDigestCache>();
        cache.Set(new ImageKey(Constants.DefaultRegistryId, "nginx", "latest"), "new-digest");

        var checkpoint = _dbWorkQueue.CreateCheckpoint();
        await RunAutoUpdateJobOnceAsync(TestContext.Current.CancellationToken);
        await _dbWorkQueue.WaitForIdleAfterAsync(checkpoint, TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var service = await uow.SwarmServices.GetAsync(_swarmServiceId, TestContext.Current.CancellationToken);

        Assert.NotNull(service);
        Assert.Equal(AutoUpdateStatus.UpdateAvailable, service.AutoUpdateState.Status);
        Assert.Equal("old-digest", service.AutoUpdateState.CurrentDigest);
        Assert.Equal("new-digest", service.AutoUpdateState.RemoteDigest);
    }

    private async Task UpdateDeploymentBehaviorAsync(UpdateBehavior updateBehavior, CancellationToken cancellationToken)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var deployment = await uow.Deployments.GetAsync(_deploymentId, cancellationToken);
        if (deployment?.Spec is null)
            throw new InvalidOperationException("Deployment spec was not found.");

        deployment.PartialUpdate(spec: deployment.Spec with { UpdateBehavior = updateBehavior });
        await uow.Deployments.UpdateAsync(deployment, cancellationToken);
        await uow.CommitAsync(cancellationToken);
    }

    private async Task RunAutoUpdateJobOnceAsync(CancellationToken cancellationToken)
    {
        Services.GetRequiredService<ISyncBarrier>().MarkSynced<DeploymentImageScannerJob>(_platformId);
        Services.GetRequiredService<ISyncBarrier>().MarkSynced<DeploymentImageScannerJob>(_swarmPlatformId);
        var job = Services.GetServices<IHostedService>()
            .OfType<DeploymentAutoUpdateJob>()
            .Single();
        await job.RunOnceAsync(cancellationToken);
    }

    private static async IAsyncEnumerable<DeploymentStreamItem> FailedApplyStream()
    {
        yield return new DeploymentStreamItem(ErrorMessage: "auto-deploy-error");
        await Task.CompletedTask;
    }

    private static Deployment CreateDeployment(Guid platformId, UpdateBehavior updateBehavior, string resolvedDigest)
        => new(
            name: "auto-update-test",
            description: "auto-update-test",
            platformId: platformId,
            createdByActorId: Constants.SystemId,
            spec: new DeploymentSpec(
                UpdateBehavior: updateBehavior,
                Image: new ExternalImage(Constants.DefaultRegistryId, "nginx:latest", resolvedDigest)));

    private static SwarmService CreateAppliedSwarmService(Guid platformId)
    {
        var service = new SwarmService(
            "managed-swarm-service",
            platformId,
            Constants.SystemId,
            new SwarmServiceSpec
            {
                Image = new SwarmExternalImage(Constants.DefaultRegistryId, "nginx:latest"),
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
        service.CompleteOperation(
            SwarmServiceOperationState.Completed,
            "runtime-hash",
            "old-digest");
        return service;
    }

    private static Platform CreateSwarmPlatform() => new(
        name: "auto-update-swarm",
        address: "https://auto-update-swarm.test",
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
            "auto-update-swarm-daemon", 0, 0, 0, 0,
            ClusterId: "auto-update-swarm-cluster"),
        clusterId: "auto-update-swarm-cluster");
}
