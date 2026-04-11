using Application.Services;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Deployments;
using Domain.Entities.Deployments;
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

    private Func<CancellationToken, Task>? _runAutoUpdateJob;
    private Guid _deploymentId;
    private Guid _platformId;

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

        _delayWithJitter
            .Setup(x => x.DelayWithJitterForAsync(It.IsAny<Func<CancellationToken, Task>>(), It.IsAny<TimeSpan>(), It.IsAny<CancellationToken>()))
            .Returns<Func<CancellationToken, Task>, TimeSpan, CancellationToken>((func, _, __) =>
            {
                _runAutoUpdateJob = func;
                return Task.CompletedTask;
            });
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();
        var deployment = CreateDeployment(platform.Id, UpdateBehavior.Notify, "old-digest");

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Deployments.AddAsync(deployment, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        _deploymentId = deployment.Id;
        _platformId = platform.Id;
    }

    [Fact]
    public async Task AutoUpdateJob_Should_Mark_Deployment_UpToDate_When_Cache_Digest_Matches()
    {
        var cache = Services.GetRequiredService<ImageDigestCache>();
        cache.Set(new ImageKey(Constants.DefaultRegistryId, "nginx", "latest"), "old-digest");

        await RunAutoUpdateJobOnceAsync(TestContext.Current.CancellationToken);
        await Task.Delay(500, TestContext.Current.CancellationToken);

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

        await RunAutoUpdateJobOnceAsync(TestContext.Current.CancellationToken);
        await Task.Delay(500, TestContext.Current.CancellationToken);

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

        await RunAutoUpdateJobOnceAsync(TestContext.Current.CancellationToken);
        await Task.Delay(500, TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var deployment = await uow.Deployments.GetAsync(_deploymentId, TestContext.Current.CancellationToken);

        Assert.NotNull(deployment?.AutoUpdateState);
        Assert.Equal(AutoUpdateStatus.Failed, deployment.AutoUpdateState!.Status);
        Assert.Equal("auto-deploy-error", deployment.AutoUpdateState.LastError);
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
        if (_runAutoUpdateJob is null)
            throw new InvalidOperationException("Deployment auto-update job was not initialized.");

        Services.GetRequiredService<ISyncBarrier>().MarkSynced<DeploymentImageScannerJob>(_platformId);

        using var cts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        cts.CancelAfter(TimeSpan.FromSeconds(1));

        try
        {
            await _runAutoUpdateJob(cts.Token);
        }
        catch (OperationCanceledException) when (cts.IsCancellationRequested)
        {
        }
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
}
