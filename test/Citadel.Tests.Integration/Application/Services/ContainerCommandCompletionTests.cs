using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Domain.Entities.Deployments;
using Domain.Entities.Platforms;
using Domain.Entities.Stacks;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging.Abstractions;
using Moq;

namespace Tests.Integration.Application.Services;

public sealed class ContainerCommandCompletionTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task Completion_ShouldPersistRuntimeStateAndReleaseOwnedContainerDeploymentAndStackClaims()
    {
        var ct = TestContext.Current.CancellationToken;
        var actorId = Constants.SystemId;
        var platform = CreatePlatform();
        var deployment = new Deployment(
            $"deployment-{Guid.CreateVersion7():N}",
            Constants.SystemId,
            platform.Id);
        var stack = Stack.Create(
            $"stack-{Guid.CreateVersion7():N}",
            Constants.SystemId,
            StackSource.WebEditor,
            platform.Id,
            new ManualStack("services:\n  app:\n    image: nginx\n", StackUpdateBehavior.Disabled));
        var directContainer = CreateContainer(platform.Id);
        var deploymentContainer = CreateContainer(platform.Id, deploymentId: deployment.Id);
        var stackContainer = CreateContainer(platform.Id, stackId: stack.Id);

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Platforms.AddAsync(platform, ct);
            await uow.Deployments.AddAsync(deployment, ct);
            await uow.Stacks.AddAsync(stack, ct);
            await uow.Containers.AddAsync(directContainer, ct);
            await uow.Containers.AddAsync(deploymentContainer, ct);
            await uow.Containers.AddAsync(stackContainer, ct);
            await uow.CommitAsync(ct);
        }

        ProcessedResources claims;
        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var storedDirect = Assert.Single(await uow.Containers.GetByIdAsync([directContainer.Id], ct));
            var storedDeployment = (await uow.Deployments.GetAsync(deployment.Id, ct))!;
            var storedStack = (await uow.Stacks.GetAsync(stack.Id, ct))!;
            var startedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds();

            Assert.Equal(1, await uow.Containers.UpdateProcessingAsync(
                storedDirect.Id, ResourceControlState.Processing, startedAt,
                storedDirect.RowVersion, true, actorId, ct));
            Assert.Equal(1, await uow.Deployments.UpdateProcessingAsync(
                storedDeployment.Id, DeploymentStatus.Pending, ResourceControlState.Processing, startedAt,
                storedDeployment.RowVersion, true, actorId, ct));
            Assert.True(await uow.Stacks.UpdateProcessingAsync(
                storedStack.Id, StackReleaseStatus.Pending, ResourceControlState.Processing, startedAt,
                storedStack.RowVersion, true, actorId, ct));
            await uow.CommitAsync(ct);

            claims = new ProcessedResources([storedDirect], [storedDeployment], [storedStack]);
        }

        var notificationQueue = new Mock<INotificationQueue>();
        notificationQueue
            .Setup(queue => queue.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()))
            .Returns(ValueTask.CompletedTask);

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var workItem = new CompleteContainerCommandWorkItem(
                claims,
                new Dictionary<Guid, ContainerStateStatus>
                {
                    [directContainer.Id] = ContainerStateStatus.Running,
                    [deploymentContainer.Id] = ContainerStateStatus.Running,
                    [stackContainer.Id] = ContainerStateStatus.Running
                },
                actorId,
                notificationQueue.Object,
                Mock.Of<IDockerDaemonStreamManager>(),
                Mock.Of<IContainerEventBroadcaster>(),
                Mock.Of<IDeploymentStreamManager>(),
            Mock.Of<IStackStreamManager>(),
            NullLogger<ContainerProcessingService>.Instance);

            await workItem.ExecuteAsync(uow, ct);
        }

        await using var assertScope = Services.CreateAsyncScope();
        var assertUow = assertScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var storedContainers = (await assertUow.Containers.GetByIdAsync(
            [directContainer.Id, deploymentContainer.Id, stackContainer.Id], ct)).ToArray();
        var storedDeploymentAfter = (await assertUow.Deployments.GetAsync(deployment.Id, ct))!;
        var storedStackAfter = (await assertUow.Stacks.GetAsync(stack.Id, ct))!;

        Assert.All(storedContainers, container => Assert.Equal(ContainerStateStatus.Running, container.State));
        Assert.Equal(ResourceControlState.Idle, Assert.Single(storedContainers, container => container.Id == directContainer.Id).ControlState);
        Assert.Equal(DeploymentStatus.Healthy, storedDeploymentAfter.Status);
        Assert.Equal(ResourceControlState.Idle, storedDeploymentAfter.ControlState);
        Assert.Equal(StackReleaseStatus.Healthy, storedStackAfter.CurrentStackRelease!.Status);
        Assert.Equal(ResourceControlState.Idle, storedStackAfter.ControlState);
    }

    [Fact]
    public async Task StaleCompletion_ShouldNotOverwriteOrReleaseANewerClaimByTheSameActor()
    {
        var ct = TestContext.Current.CancellationToken;
        var actorId = Constants.SystemId;
        var platform = CreatePlatform();
        var container = CreateContainer(platform.Id);

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Platforms.AddAsync(platform, ct);
            await uow.Containers.AddAsync(container, ct);
            await uow.CommitAsync(ct);
        }

        Container originalClaim;
        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            originalClaim = Assert.Single(await uow.Containers.GetByIdAsync([container.Id], ct));
            Assert.Equal(1, await uow.Containers.UpdateProcessingAsync(
                originalClaim.Id,
                ResourceControlState.Processing,
                DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
                originalClaim.RowVersion,
                true,
                actorId,
                ct));
            await uow.CommitAsync(ct);
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var firstClaim = Assert.Single(await uow.Containers.GetByIdAsync([container.Id], ct));
            Assert.Equal(1, await uow.Containers.UpdateProcessingAsync(
                firstClaim.Id,
                ResourceControlState.Idle,
                null,
                firstClaim.RowVersion,
                true,
                null,
                ct));
            await uow.CommitAsync(ct);
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var idle = Assert.Single(await uow.Containers.GetByIdAsync([container.Id], ct));
            Assert.Equal(1, await uow.Containers.UpdateProcessingAsync(
                idle.Id,
                ResourceControlState.Processing,
                DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
                idle.RowVersion,
                true,
                actorId,
                ct));
            await uow.CommitAsync(ct);
        }

        var notificationQueue = new Mock<INotificationQueue>();
        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var workItem = new CompleteContainerCommandWorkItem(
                new ProcessedResources([originalClaim], [], []),
                new Dictionary<Guid, ContainerStateStatus>
                {
                    [originalClaim.Id] = ContainerStateStatus.Running
                },
                actorId,
                notificationQueue.Object,
                Mock.Of<IDockerDaemonStreamManager>(),
                Mock.Of<IContainerEventBroadcaster>(),
                Mock.Of<IDeploymentStreamManager>(),
            Mock.Of<IStackStreamManager>(),
            NullLogger<ContainerProcessingService>.Instance);

            await workItem.ExecuteAsync(uow, ct);
        }

        await using var assertScope = Services.CreateAsyncScope();
        var assertUow = assertScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stored = Assert.Single(await assertUow.Containers.GetByIdAsync([container.Id], ct));
        Assert.Equal(ContainerStateStatus.Exited, stored.State);
        Assert.Equal(ResourceControlState.Processing, stored.ControlState);
        Assert.Equal(actorId, stored.ControlTriggeredBy);
        notificationQueue.Verify(
            queue => queue.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }

    private static Container CreateContainer(Guid platformId, Guid? deploymentId = null, Guid? stackId = null)
        => new(
            "container",
            "sha256:nginx",
            platformId,
            dockerContainerId: $"container-{Guid.CreateVersion7():N}",
            state: ContainerStateStatus.Exited,
            deploymentId: deploymentId,
            stackId: stackId);

    private static Platform CreatePlatform()
        => new(
            $"platform-{Guid.CreateVersion7():N}",
            $"edge://{Guid.CreateVersion7():D}",
            0,
            0,
            0,
            1,
            1,
            null,
            null,
            PlatformStatus.Online,
            PlatformConnectorType.EdgeAgent,
            new DockerPlatformDescriptor("daemon", 0, 0, 0, 0));
}
