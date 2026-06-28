using Domain;
using Domain.Entities.Stacks;

namespace Tests.Unit.Domain.Entities.Stacks;

public class StackTests
{
    [Fact]
    public void PrepareReleaseForApply_Should_Reuse_Failed_Release()
    {
        var stack = CreateManualStack();
        stack.ReleaseProcessing(StackReleaseStatus.Failed);
        var releaseId = stack.CurrentStackReleaseId;
        var version = stack.CurrentStackRelease!.Version;

        var prepared = stack.PrepareReleaseForApply(Guid.CreateVersion7());

        Assert.True(prepared);
        Assert.Equal(releaseId, stack.CurrentStackReleaseId);
        Assert.Equal(version, stack.CurrentStackRelease!.Version);
    }

    [Fact]
    public void PrepareReleaseForApply_Should_Create_Next_Release_After_Successful_Release()
    {
        var stack = CreateManualStack();
        stack.ReleaseProcessing(StackReleaseStatus.Healthy);
        var releaseId = stack.CurrentStackReleaseId;

        var prepared = stack.PrepareReleaseForApply(Guid.CreateVersion7());

        Assert.True(prepared);
        Assert.NotEqual(releaseId, stack.CurrentStackReleaseId);
        Assert.Equal("2", stack.CurrentStackRelease!.Version);
        Assert.Equal(StackReleaseStatus.Created, stack.CurrentStackRelease.Status);
    }

    [Fact]
    public void PrepareReleaseForApply_Should_Reuse_Healthy_Release_When_Next_Release_Is_Not_Requested()
    {
        var stack = CreateManualStack();
        stack.ReleaseProcessing(StackReleaseStatus.Healthy);
        var releaseId = stack.CurrentStackReleaseId;
        var version = stack.CurrentStackRelease!.Version;

        var prepared = stack.PrepareReleaseForApply(Guid.CreateVersion7(), createNextRelease: false);

        Assert.True(prepared);
        Assert.Equal(releaseId, stack.CurrentStackReleaseId);
        Assert.Equal(version, stack.CurrentStackRelease!.Version);
    }

    [Theory]
    [InlineData(StackReleaseStatus.Healthy, ContainerStateStatus.Running, ContainerStateStatus.Running)]
    [InlineData(StackReleaseStatus.Paused, ContainerStateStatus.Paused, ContainerStateStatus.Paused)]
    [InlineData(StackReleaseStatus.Stopped, ContainerStateStatus.Exited, ContainerStateStatus.Exited)]
    [InlineData(StackReleaseStatus.Stopped, ContainerStateStatus.Exited, ContainerStateStatus.Offline)]
    [InlineData(StackReleaseStatus.Pending, ContainerStateStatus.Running, ContainerStateStatus.Restarting)]
    [InlineData(StackReleaseStatus.Degraded, ContainerStateStatus.Running, ContainerStateStatus.Paused)]
    [InlineData(StackReleaseStatus.Degraded, ContainerStateStatus.Running, ContainerStateStatus.Exited)]
    [InlineData(StackReleaseStatus.Degraded, ContainerStateStatus.Paused, ContainerStateStatus.Exited)]
    [InlineData(StackReleaseStatus.Failed, ContainerStateStatus.Dead, ContainerStateStatus.Dead)]
    [InlineData(StackReleaseStatus.Unknown, ContainerStateStatus.Unknown, ContainerStateStatus.Unknown)]
    public void ToStackStatus_AggregatesContainerStates(StackReleaseStatus expectedStatus, params ContainerStateStatus[] states)
    {
        var status = Stack.ToStackStatus(states);

        Assert.Equal(expectedStatus, status);
    }

    private static Stack CreateManualStack()
        => Stack.Create(
            name: "stack",
            createdByActorId: Guid.CreateVersion7(),
            StackSource: StackSource.WebEditor,
            platformId: Guid.CreateVersion7(),
            spec: new ManualStack(
                ComposeFile: "services:\n  app:\n    image: nginx\n",
                UpdateBehavior: StackUpdateBehavior.Disabled));
}
