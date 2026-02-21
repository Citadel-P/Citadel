using Application.Services;
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
    private readonly Mock<IDelayWithJitterService> _delayWithJitter = new();
    private readonly Mock<INotificationQueue> notificationMock = new();
    private Guid platformId;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<IHostedService>();
        services.RemoveAll<IDelayWithJitterService>();

        services
            .AddHostedService<DbWriteWorker>()
            .AddHostedService<ReconcilableResourceJob>();

        services.AddSingleton(streamManagerMock.Object);
        services.AddSingleton(notificationMock.Object);
        services.AddSingleton(_ => _delayWithJitter.Object);

        _delayWithJitter
         .Setup(x => x.DelayWithJitterForAsync(It.IsAny<Func<CancellationToken, Task>>(),
                                               It.IsAny<TimeSpan>(),
                                               It.IsAny<CancellationToken>()))
         .Returns<Func<CancellationToken, Task>, TimeSpan, CancellationToken>(async (func, _, ct) =>
         {
             await Task.Delay(TimeSpan.FromMilliseconds(500), ct);
             await func(ct);
         });
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();
        platformId = platform.Id;

        var deployment = new Deployment
        (
            name: "Test Deployment",
            description: "A deployment for testing",
            platformId: platform.Id,
            createdByActorId: Constants.SystemId,
            spec: new DeploymentSpec
            (
                UpdateBehavior: UpdateBehavior.AutoDeploy,
                Image: new ExternalImage
                (
                    RegistryId: Constants.DefaultRegistryId,
                    ImageTag: "nginx:latest"
                ),
                Ports: new List<string> { "80:80" },
                EnvVars: new List<string> { "ENV=production" }
            )

        );
        var container = new Container(
            platformId: platformId,
            dockerContainerId: "container-123",
            dockerImageId: "image",
            name: "deployment-container",
            created: 999999,
            state: ContainerStateStatus.Running,
            deploymentId: deployment.Id
        );
        var image = new Image(
            platformId: platformId,
            dockerImageId: "image",
            name: "deployment-image",
            tags: new List<string> { "nginx:latest" },
            containers: 1,
            size: 123456,
            createdAt: DateTime.UtcNow
        );

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Deployments.AddAsync(deployment, TestContext.Current.CancellationToken);
        await uow.Containers.AddAsync(container, TestContext.Current.CancellationToken);
        await uow.Images.AddOrUpdateAsync(image, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    [Fact]
    public async Task RunPeriodicJanitor_Clean_Stuck_Deployment()
    {
        // Arrange
        await MarkDeploymentAsync(ResourceControlState.Processing, DateTimeOffset.UtcNow.ToUnixTimeSeconds() - 90);

        await Task.Delay(TimeSpan.FromSeconds(1), TestContext.Current.CancellationToken);

        // Assert
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var deployment = (await uow.Deployments.GetInfoAsync(TestContext.Current.CancellationToken)).First();
        Assert.Equal(ResourceControlState.Idle, deployment.ControlState);

        notificationMock.Verify(
           nq => nq.EnqueueAsync(It.IsAny<DeploymentNotificationWorkItem>(), It.IsAny<CancellationToken>()),
           Times.Once);
    }

    [Fact]
    public async Task RunPeriodicJanitor_Clean_Stuck_Container()
    {
        // Arrange
        await MarkContainerAsync(ResourceControlState.Processing, DateTimeOffset.UtcNow.ToUnixTimeSeconds() - 90);
        await Task.Delay(TimeSpan.FromSeconds(1), TestContext.Current.CancellationToken);

        // Assert
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var container = (await uow.Containers.GetByPlatformIdAsync(platformId, TestContext.Current.CancellationToken)).First();
        Assert.Equal(ResourceControlState.Idle, container.ControlState);

        notificationMock.Verify(
           nq => nq.EnqueueAsync(It.IsAny<ContainerNotificationWorkItem>(), It.IsAny<CancellationToken>()),
           Times.Once);
    }

    [Fact]
    public async Task RunPeriodicJanitor_Clean_Stuck_Image()
    {
        // Arrange
        await MarkImageAsync(ResourceControlState.Processing, DateTimeOffset.UtcNow.ToUnixTimeSeconds() - 90);

        await Task.Delay(TimeSpan.FromSeconds(1), TestContext.Current.CancellationToken);

        // Assert
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var image = (await uow.Images.GetByPlatformIdAsync(platformId, TestContext.Current.CancellationToken)).First();
        Assert.Equal(ResourceControlState.Idle, image.ControlState);

        notificationMock.Verify(
           nq => nq.EnqueueAsync(It.IsAny<ImageNotificationWorkItem>(), It.IsAny<CancellationToken>()),
           Times.Once);
    }

    [Fact]
    public async Task RunPeriodicJanitor_DoesNotEnqueue_WhenNoStuckDeployments()
    {
        // Arrange
        await MarkDeploymentAsync(ResourceControlState.Idle, null);

        await Task.Delay(1000, TestContext.Current.CancellationToken); 

        // Assert
        notificationMock.Verify(
           nq => nq.EnqueueAsync(It.IsAny<DeploymentNotificationWorkItem>(), It.IsAny<CancellationToken>()),
           Times.Never);
    }

    [Fact]
    public async Task RunPeriodicJanitor_DoesNotEnqueue_WhenNoStuckContainers()
    {
        // Arrange
        await MarkContainerAsync(ResourceControlState.Idle, null);

        await Task.Delay(1000, TestContext.Current.CancellationToken); 

        // Assert
        notificationMock.Verify(
           nq => nq.EnqueueAsync(It.IsAny<ContainerNotificationWorkItem>(), It.IsAny<CancellationToken>()),
           Times.Never);
    }

    [Fact]
    public async Task RunPeriodicJanitor_DoesNotEnqueue_WhenNoStuckImages()
    {
        // Arrange
        await MarkImageAsync(ResourceControlState.Idle, null);

        await Task.Delay(1000, TestContext.Current.CancellationToken);

        // Assert
        notificationMock.Verify(
           nq => nq.EnqueueAsync(It.IsAny<ImageNotificationWorkItem>(), It.IsAny<CancellationToken>()),
           Times.Never);
    }

    private async Task MarkDeploymentAsync(ResourceControlState state, long? startedAt)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var deployment = (await uow.Deployments.GetInfoAsync(TestContext.Current.CancellationToken)).First();

        await uow.Deployments.UpdateProcessingAsync(
            id: deployment.Id,
            status: deployment.Status,
            state: state,
            startedAt: startedAt,
            rowVersion: deployment.RowVersion,
            checkRowVersion: false,
            controlTriggeredBy: deployment.ControlTriggeredBy,
            TestContext.Current.CancellationToken);

        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    private async Task MarkContainerAsync(ResourceControlState state, long? startedAt)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var container = (await uow.Containers.GetByPlatformIdAsync(platformId, TestContext.Current.CancellationToken)).First();

        await uow.Containers.UpdateProcessingAsync(
            id: container.Id,
            state: state,
            startedAt: startedAt,
            rowVersion: container.RowVersion,
            checkRowVersion: false,
            controlTriggeredBy: container.ControlTriggeredBy,
            TestContext.Current.CancellationToken);

        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    private async Task MarkImageAsync(ResourceControlState state, long? startedAt)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var image = (await uow.Images.GetByPlatformIdAsync(platformId, TestContext.Current.CancellationToken)).First();

        await uow.Images.UpdateProcessingAsync(
            id: image.Id,
            state: state,
            startedAt: startedAt,
            rowVersion: image.RowVersion,
            checkRowVersion: false,
            TestContext.Current.CancellationToken);

        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

}
