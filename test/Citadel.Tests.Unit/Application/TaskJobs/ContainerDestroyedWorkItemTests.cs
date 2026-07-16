using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Stacks;
using Moq;

namespace Tests.Unit.Application.TaskJobs;

public sealed class ContainerDestroyedWorkItemTests
{
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
