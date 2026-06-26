using System.Threading.Channels;
using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Domain.Entities.Activities;
using Domain.Entities.Identity;
using Domain.Entities.Stacks;
using Microsoft.Extensions.DependencyInjection;
using Moq;

namespace Tests.Unit.Application.Features.Stacks;

public class ApplyStackServiceTests
{
    [Fact]
    public async Task StackSucceededWorkItem_creates_and_associates_containers_when_sync_has_not_seen_them_yet()
    {
        var platformId = Guid.CreateVersion7();
        var actorId = Guid.CreateVersion7();
        var stack = Stack.Create(
            name: "beszel",
            createdByActorId: actorId,
            StackSource: StackSource.WebEditor,
            platformId: platformId,
            spec: new ManualStack(
                ComposeFile: "services:\n  beszel:\n    image: henrygd/beszel\n  beszel-agent:\n    image: henrygd/beszel-agent\n",
                UpdateBehavior: StackUpdateBehavior.Disabled));
        stack.MarkProcessing(actorId);

        var dockerContainers = new[]
        {
            new DockerContainer(
                Name: "/beszel-beszel-1",
                Image: "henrygd/beszel:latest",
                Id: "beszel-container-id",
                ImageId: "sha256:beszel",
                State: ContainerStateStatus.Running,
                Created: 123,
                Stack: "beszel"),
            new DockerContainer(
                Name: "/beszel-beszel-agent-1",
                Image: "henrygd/beszel-agent:latest",
                Id: "beszel-agent-container-id",
                ImageId: "sha256:beszel-agent",
                State: ContainerStateStatus.Running,
                Created: 124,
                Stack: "beszel")
        };

        List<Container> upsertedContainers = [];
        ActivityEvent? activity = null;
        var stacks = new Mock<IStackRepository>();
        stacks
            .Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stack);
        stacks
            .Setup(x => x.UpdateAsync(stack, It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var containers = new Mock<IContainerRepository>();
        containers
            .Setup(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);
        containers
            .Setup(x => x.BulkUpsertAsync(It.IsAny<IEnumerable<Container>>(), It.IsAny<CancellationToken>()))
            .Callback<IEnumerable<Container>, CancellationToken>((items, _) => upsertedContainers = items.ToList())
            .ReturnsAsync(2);

        var images = new Mock<IImageRepository>();
        images
            .Setup(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);

        var activityEvents = new Mock<IActivityEventRepository>();
        activityEvents
            .Setup(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()))
            .Callback<ActivityEvent, CancellationToken>((item, _) => activity = item)
            .ReturnsAsync(1);

        var actors = new Mock<IActorRepository>();
        actors
            .Setup(x => x.GetById(actorId, It.IsAny<CancellationToken>()))
            .ReturnsAsync((Actor?)null);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Stacks).Returns(stacks.Object);
        unitOfWork.Setup(x => x.Containers).Returns(containers.Object);
        unitOfWork.Setup(x => x.Images).Returns(images.Object);
        unitOfWork.Setup(x => x.ActivityEventRepository).Returns(activityEvents.Object);
        unitOfWork.Setup(x => x.Actors).Returns(actors.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        var notificationQueue = new TestNotificationQueue();
        var workItem = new StackSucceededWorkItem(
            stack.Id,
            actorId,
            dockerContainers,
            Mock.Of<IStackStreamManager>(),
            Mock.Of<IActivityStreamManager>(),
            notificationQueue,
            StackApplyOperation.Apply);

        await workItem.ExecuteAsync(unitOfWork.Object, CancellationToken.None);

        Assert.Equal(StackReleaseStatus.Healthy, stack.CurrentStackRelease?.Status);
        Assert.Equal(ResourceControlState.Idle, stack.ControlState);
        Assert.Equal(2, upsertedContainers.Count);
        Assert.All(upsertedContainers, container => Assert.Equal(stack.Id, container.StackId));
        Assert.Contains(upsertedContainers, container => container.DockerContainerId == "beszel-container-id");
        Assert.Contains(upsertedContainers, container => container.DockerContainerId == "beszel-agent-container-id");

        var applied = Assert.IsType<StackApplied>(activity?.Info);
        Assert.Equal(["beszel-container-id", "beszel-agent-container-id"], applied.Result.ContainerIds);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
        Assert.Equal(2, notificationQueue.Items.Count);
    }

    [Fact]
    public async Task StackSucceededWorkItem_Should_Record_Rollback_Activity_For_Rollback_Operation()
    {
        var platformId = Guid.CreateVersion7();
        var actorId = Guid.CreateVersion7();
        var stack = Stack.Create(
            name: "beszel",
            createdByActorId: actorId,
            StackSource: StackSource.WebEditor,
            platformId: platformId,
            spec: new ManualStack(
                ComposeFile: "services:\n  beszel:\n    image: henrygd/beszel\n",
                UpdateBehavior: StackUpdateBehavior.Notify));
        var previousStackSnapshot = stack.ToSnapshot();
        stack.UpdateCurrentStackReleaseDefinition(
            platformId,
            new ManualStack(
                ComposeFile: "services:\n  beszel:\n    image: henrygd/beszel\n",
                UpdateBehavior: StackUpdateBehavior.ServiceAutoDeploy));
        stack.MarkProcessing(actorId);

        ActivityEvent? activity = null;
        var stacks = new Mock<IStackRepository>();
        stacks
            .Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stack);
        stacks
            .Setup(x => x.UpdateAsync(stack, It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var containers = new Mock<IContainerRepository>();
        containers
            .Setup(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);
        containers
            .Setup(x => x.BulkUpsertAsync(It.IsAny<IEnumerable<Container>>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(0);

        var images = new Mock<IImageRepository>();
        images
            .Setup(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);

        var activityEvents = new Mock<IActivityEventRepository>();
        activityEvents
            .Setup(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()))
            .Callback<ActivityEvent, CancellationToken>((item, _) => activity = item)
            .ReturnsAsync(1);

        var actors = new Mock<IActorRepository>();
        actors
            .Setup(x => x.GetById(actorId, It.IsAny<CancellationToken>()))
            .ReturnsAsync((Actor?)null);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Stacks).Returns(stacks.Object);
        unitOfWork.Setup(x => x.Containers).Returns(containers.Object);
        unitOfWork.Setup(x => x.Images).Returns(images.Object);
        unitOfWork.Setup(x => x.ActivityEventRepository).Returns(activityEvents.Object);
        unitOfWork.Setup(x => x.Actors).Returns(actors.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        var workItem = new StackSucceededWorkItem(
            stack.Id,
            actorId,
            [],
            Mock.Of<IStackStreamManager>(),
            Mock.Of<IActivityStreamManager>(),
            new TestNotificationQueue(),
            StackApplyOperation.Rollback,
            previousStackSnapshot);

        await workItem.ExecuteAsync(unitOfWork.Object, CancellationToken.None);

        Assert.Equal(ActivityEventType.StackRollback, activity?.EventType);
        var rollback = Assert.IsType<StackRollback>(activity?.Info);
        Assert.Equal(StackUpdateBehavior.Notify, Assert.IsType<ManualStack>(rollback.OldStack!.StackRelease!.Spec).UpdateBehavior);
        Assert.Equal(StackUpdateBehavior.ServiceAutoDeploy, Assert.IsType<ManualStack>(rollback.NewStack!.StackRelease!.Spec).UpdateBehavior);
    }

    private sealed class TestNotificationQueue : INotificationQueue
    {
        public List<INotificationWorkItem> Items { get; } = [];

        public ChannelReader<INotificationWorkItem> Reader { get; } = Channel.CreateUnbounded<INotificationWorkItem>().Reader;

        public ValueTask EnqueueAsync(INotificationWorkItem item, CancellationToken ct)
        {
            Items.Add(item);
            return ValueTask.CompletedTask;
        }
    }
}
