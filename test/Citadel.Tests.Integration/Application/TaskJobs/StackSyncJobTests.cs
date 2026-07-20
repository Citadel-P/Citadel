using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Domain.Entities.Platforms;
using Domain.Entities.Stacks;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Moq;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.TaskJobs;

public class StackSyncJobTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<INotificationQueue> notificationQueue = new();
    private readonly Mock<IStackStreamManager> stackStreamManager = new();
    private Guid platformId;
    private Guid healthyStackId;
    private Guid degradedStackId;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<IHostedService>();
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = new Platform(
            name: "stack-sync-platform",
            address: "https://stack-sync-platform",
            networkCount: 1,
            volumeCount: 1,
            imageCount: 1,
            cpuCount: 2,
            memTotal: 512,
            serverVersion: "1.0.0",
            agentVersion: "1.0.0",
            status: PlatformStatus.Online,
            connectorType: PlatformConnectorType.Agent,
            platformDescriptor: new DockerPlatformDescriptor(
                DaemonId: "stack-sync-platform",
                ContainerCount: 2,
                ContainersRunning: 2,
                ContainersPaused: 0,
                ContainersStopped: 0));

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        platformId = platform.Id;

        var healthyStack = CreateStack("stack-sync-healthy", platformId, StackReleaseStatus.Healthy);
        var degradedStack = CreateStack("stack-sync-degraded", platformId, StackReleaseStatus.Degraded);

        await uow.Stacks.AddAsync(healthyStack, TestContext.Current.CancellationToken);
        await uow.Stacks.AddAsync(degradedStack, TestContext.Current.CancellationToken);

        healthyStackId = healthyStack.Id;
        degradedStackId = degradedStack.Id;

        await uow.Containers.AddAsync(CreateContainer("stack-sync-healthy-1", healthyStackId, ContainerStateStatus.Running), TestContext.Current.CancellationToken);
        await uow.Containers.AddAsync(CreateContainer("stack-sync-degraded-1", degradedStackId, ContainerStateStatus.Running), TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    [Fact]
    public async Task Offline_Platform_Should_Mark_Healthy_Stack_Degraded()
    {
        await ExecuteStackSync(platformIsOnline: false);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stack = await uow.Stacks.GetInfoAsync(healthyStackId, TestContext.Current.CancellationToken);

        Assert.Equal(StackReleaseStatus.Degraded, stack?.CurrentStackRelease?.Status);
        notificationQueue.Verify(
            q => q.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task Online_Platform_Should_Recalculate_Stack_Status_From_Containers()
    {
        await ExecuteStackSync(platformIsOnline: true);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stack = await uow.Stacks.GetInfoAsync(degradedStackId, TestContext.Current.CancellationToken);

        Assert.Equal(StackReleaseStatus.Healthy, stack?.CurrentStackRelease?.Status);
        notificationQueue.Verify(
            q => q.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task Online_Platform_Should_Not_Recalculate_Processing_Stack()
    {
        await using (var arrangeScope = Services.CreateAsyncScope())
        {
            var arrangeUow = arrangeScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var stack = await arrangeUow.Stacks.GetAsync(healthyStackId, TestContext.Current.CancellationToken);
            Assert.NotNull(stack);

            stack.MarkProcessing(Constants.SystemId);
            stack.PartialUpdate(StackReleaseStatus.Applying);
            await arrangeUow.Stacks.UpdateAsync(stack, TestContext.Current.CancellationToken);
            await arrangeUow.CommitAsync(TestContext.Current.CancellationToken);
        }

        await ExecuteStackSync(platformIsOnline: true);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var persisted = await uow.Stacks.GetInfoAsync(healthyStackId, TestContext.Current.CancellationToken);

        Assert.Equal(ResourceControlState.Processing, persisted?.ControlState);
        Assert.Equal(StackReleaseStatus.Applying, persisted?.CurrentStackRelease?.Status);
        stackStreamManager.Verify(
            manager => manager.SendStackInfo(
                It.Is<Stack>(stack => stack.Id == healthyStackId),
                It.IsAny<string>()),
            Times.Never);
    }

    [Fact]
    public async Task Online_Platform_Should_Release_Stale_Processing_Stack_With_Final_Status()
    {
        await using (var arrangeScope = Services.CreateAsyncScope())
        {
            var arrangeUow = arrangeScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var stack = await arrangeUow.Stacks.GetAsync(healthyStackId, TestContext.Current.CancellationToken);
            Assert.NotNull(stack);

            stack.MarkProcessing(Constants.SystemId);
            stack.PartialUpdate(StackReleaseStatus.Degraded);
            await arrangeUow.Stacks.UpdateAsync(stack, TestContext.Current.CancellationToken);
            await arrangeUow.CommitAsync(TestContext.Current.CancellationToken);
        }

        await ExecuteStackSync(platformIsOnline: true);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var persisted = await uow.Stacks.GetInfoAsync(healthyStackId, TestContext.Current.CancellationToken);

        Assert.Equal(ResourceControlState.Idle, persisted?.ControlState);
        Assert.Equal(StackReleaseStatus.Degraded, persisted?.CurrentStackRelease?.Status);
        stackStreamManager.Verify(
            manager => manager.SendStackInfo(
                It.Is<Stack>(stack => stack.Id == healthyStackId),
                It.IsAny<string>()),
            Times.Once);
    }

    private async Task ExecuteStackSync(bool platformIsOnline)
    {
        notificationQueue.Reset();
        notificationQueue
            .Setup(q => q.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()))
            .Returns<INotificationWorkItem, CancellationToken>((workItem, cancellationToken) =>
                new ValueTask(workItem.ExecuteAsync(cancellationToken)));

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var workItem = new StackSyncWorkItem(
            stackStreamManager.Object,
            notificationQueue.Object,
            platformId,
            platformIsOnline);

        await workItem.ExecuteAsync(uow, TestContext.Current.CancellationToken);
    }

    private Container CreateContainer(string dockerContainerId, Guid stackId, ContainerStateStatus state)
        => new(
            name: dockerContainerId,
            dockerImageId: "image-id",
            platformId: platformId,
            dockerContainerId: dockerContainerId,
            state: state,
            stackId: stackId,
            ports: new Dictionary<string, IReadOnlyList<HostPortBinding>>());

    private static Stack CreateStack(string name, Guid platformId, StackReleaseStatus status)
    {
        var stack = Stack.Create(
            name: name,
            createdByActorId: Constants.SystemId,
            StackSource: StackSource.WebEditor,
            platformId: platformId,
            spec: new ManualStack("docker-compose.yml", StackUpdateBehavior.Disabled));

        stack.ReleaseProcessing(status);
        return stack;
    }
}
