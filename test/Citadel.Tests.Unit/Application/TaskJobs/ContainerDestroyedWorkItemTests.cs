using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Stacks;
using Moq;

namespace Tests.Unit.Application.TaskJobs;

public sealed class ContainerDestroyedWorkItemTests
{
    [Fact]
    public async Task UpdateStackStatus_Should_Release_Direct_Container_Operation_As_Degraded()
    {
        var platformId = Guid.CreateVersion7();
        var actorId = Guid.CreateVersion7();
        var stack = Stack.Create(
            name: "beszel-copy",
            createdByActorId: actorId,
            StackSource: StackSource.WebEditor,
            platformId: platformId,
            spec: new ManualStack(
                ComposeFile: "services:\n  beszel:\n    image: example/beszel\n  copy:\n    image: example/copy\n",
                UpdateBehavior: StackUpdateBehavior.Disabled));

        stack.PartialUpdate(StackReleaseStatus.Healthy);
        stack.MarkProcessing(actorId);

        var stacks = new Mock<IStackRepository>();
        stacks
            .Setup(x => x.GetInfoAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stack);

        var activities = new Mock<IActivityEventRepository>();
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Stacks).Returns(stacks.Object);
        unitOfWork.Setup(x => x.ActivityEventRepository).Returns(activities.Object);

        var (updatedStack, activity) = await ContainerDestroyedWorkItem.UpdateStackStatus(
            unitOfWork.Object,
            stack.Id,
            [
                new StackContainerState("beszel-container-id", ContainerStateStatus.Exited),
                new StackContainerState("copy-container-id", ContainerStateStatus.Running)
            ],
            ContainerStateStatus.Exited,
            "beszel-container-id",
            forcedStatus: null,
            cancellationToken: TestContext.Current.CancellationToken,
            allowDegradedWhileProcessing: true);

        Assert.Same(stack, updatedStack);
        Assert.NotNull(activity);
        Assert.Equal(ResourceControlState.Idle, stack.ControlState);
        Assert.Equal(StackReleaseStatus.Degraded, stack.CurrentStackRelease?.Status);
        stacks.Verify(
            x => x.UpdateProcessingAsync(
                stack.Id,
                StackReleaseStatus.Degraded,
                ResourceControlState.Idle,
                null,
                It.IsAny<long>(),
                false,
                null,
                It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task UpdateStackStatus_Should_Not_Return_Processing_Stack_When_Degraded_Event_Is_Skipped_During_Apply()
    {
        var platformId = Guid.CreateVersion7();
        var actorId = Guid.CreateVersion7();
        var stack = Stack.Create(
            name: "minio-stack",
            createdByActorId: actorId,
            StackSource: StackSource.WebEditor,
            platformId: platformId,
            spec: new ManualStack(
                ComposeFile: "services:\n  minio:\n    image: minio/minio\n  minio-init:\n    image: minio/mc\n",
                UpdateBehavior: StackUpdateBehavior.Disabled));

        stack.ReleaseProcessing(StackReleaseStatus.Degraded);
        stack.MarkProcessing(actorId);
        stack.PartialUpdate(StackReleaseStatus.Applying);

        var stacks = new Mock<IStackRepository>();
        stacks
            .Setup(x => x.GetInfoAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stack);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Stacks).Returns(stacks.Object);

        var (updatedStack, activity) = await ContainerDestroyedWorkItem.UpdateStackStatus(
            unitOfWork.Object,
            stack.Id,
            [new StackContainerState("minio-init-container-id", ContainerStateStatus.Exited)],
            ContainerStateStatus.Exited,
            "minio-init-container-id",
            StackReleaseStatus.Degraded,
            TestContext.Current.CancellationToken);

        Assert.Null(updatedStack);
        Assert.Null(activity);
        Assert.Equal(ResourceControlState.Processing, stack.ControlState);
        Assert.Equal(StackReleaseStatus.Applying, stack.CurrentStackRelease?.Status);
        stacks.Verify(
            x => x.UpdateProcessingAsync(
                It.IsAny<Guid>(),
                It.IsAny<StackReleaseStatus>(),
                It.IsAny<ResourceControlState>(),
                It.IsAny<long?>(),
                It.IsAny<long>(),
                It.IsAny<bool?>(),
                It.IsAny<Guid?>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
    }
}
