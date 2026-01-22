using Application.Services.SignalR;
using Application.TaskJobs;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Hosting.Common;
using Infrastructure.Repositories.DbQueue;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Moq;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.TaskJobs;

public class ReconcilableResourceJobTests : IntegrationTestBase
{
    private readonly Mock<IDeploymentStreamManager> streamManagerMock = new();
    private readonly Mock<INotificationQueue> notificationMock = new();

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<IHostedService>();

        services
            .AddHostedService<DbWriteWorker>()
            .AddHostedService<ReconcilableResourceJob>();

        services.AddSingleton(streamManagerMock.Object);
        services.AddSingleton(notificationMock.Object);
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();

        var deployment = new Deployment
        (
            name: "Test Deployment",
            description: "A deployment for testing",
            platformId: platform.Id,
            status: DeploymentStatus.Created,
            createdByActorId: Constants.SystemId,
            updateBehavior: UpdateBehavior.AutoDeploy,
            spec: new DeploymentSpec
            (
                Image: new ExternalImage
                (
                    RegistryId: Constants.DefaultRegistryId,
                    ImageTag: "nginx:latest"
                ),
                Ports: new List<string> { "80:80" },
                EnvVars: new List<string> { "ENV=production" }
            )

        );

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Deployments.AddAsync(deployment, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    [Fact]
    public async Task RunPeriodicJanitor_Clean_Stuck_Deployment()
    {
        // Arrange
        await MarkDeploymentAsync(ResourceControlState.Processing, DateTimeOffset.UtcNow.ToUnixTimeSeconds() - 90);

        await Task.Delay(TimeSpan.FromSeconds(30), TestContext.Current.CancellationToken);

        // Assert
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var deployment = (await uow.Deployments.GetAllAsync(TestContext.Current.CancellationToken)).First();
        Assert.Equal(ResourceControlState.Idle, deployment.ControlState);

        notificationMock.Verify(
           nq => nq.EnqueueAsync(It.IsAny<DeploymentNotificationWorkItem>(), It.IsAny<CancellationToken>()),
           Times.Once);
    }

    [Fact]
    public async Task RunPeriodicJanitor_DoesNotEnqueue_WhenNoStuckDeployments()
    {
        // Arrange
        await MarkDeploymentAsync(ResourceControlState.Idle, null);

        await Task.Delay(5000, TestContext.Current.CancellationToken); // wait for jobs to process

        // Assert
        notificationMock.Verify(
           nq => nq.EnqueueAsync(It.IsAny<DeploymentNotificationWorkItem>(), It.IsAny<CancellationToken>()),
           Times.Never);
    }

    private async Task MarkDeploymentAsync(ResourceControlState state, long? startedAt)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var deployment = (await uow.Deployments.GetAllAsync(TestContext.Current.CancellationToken)).First();

        await uow.Deployments.UpdateProcessingAsync(
            id: deployment.Id,
            status: deployment.Status,
            state: state,
            startedAt: startedAt,
            rowVersion: deployment.RowVersion,
            checkRowVersion: false,
            TestContext.Current.CancellationToken);

        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

}
