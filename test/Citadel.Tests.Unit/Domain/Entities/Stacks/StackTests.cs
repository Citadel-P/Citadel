using Domain;
using Domain.Entities.Stacks;

namespace Tests.Unit.Domain.Entities.Stacks;

public class StackTests
{
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
}
