using Application.Services;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Hosting.Common;
using Infrastructure.Repositories.DbQueue;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.TaskJobs;

public class DeploymentSyncJobTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly TestPlatformHealthBroadCaster broadcaster = new();
    private readonly ObservableDbWorkQueue dbWorkQueue = new();
    private Guid platformId;
    private Guid deploymentId;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<IHostedService>();

        services.AddHostedService<DeploymentSyncJob>();
        services.AddHostedService<DbWriteWorker>();
        services.AddSingleton<IPlatformHealthBroadCaster>(broadcaster);
        services.RemoveAll<IDbWorkQueue>();
        services.AddSingleton<IDbWorkQueue>(dbWorkQueue);
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();
        var deployment = new Deployment("deployment-sync-offline", Constants.SystemId, platform.Id);
        deployment.ReleaseProcessing(DeploymentStatus.Healthy);

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Deployments.AddAsync(deployment, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        platformId = platform.Id;
        deploymentId = deployment.Id;
    }

    [Fact]
    public async Task Platform_Offline_Event_Should_Mark_Deployment_Degraded()
    {
        var checkpoint = dbWorkQueue.CreateCheckpoint();
        await broadcaster.PublishAsync(
            new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: false),
            TestContext.Current.CancellationToken);
        await dbWorkQueue.WaitForIdleAfterAsync(
            checkpoint,
            TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var deployment = await uow.Deployments.GetInfoAsync(deploymentId, TestContext.Current.CancellationToken);

        Assert.Equal(DeploymentStatus.Degraded, deployment?.Status);
    }
}
