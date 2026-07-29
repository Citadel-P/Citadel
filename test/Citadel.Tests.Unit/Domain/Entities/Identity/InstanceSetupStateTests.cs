using Domain.Entities.Identity;

namespace Tests.Unit.Domain.Entities.Identity;

public sealed class InstanceSetupStateTests
{
    [Fact]
    public void TryComplete_Should_Be_OneWay()
    {
        var createdAt = DateTimeOffset.UtcNow.AddMinutes(-1);
        var completedAt = DateTimeOffset.UtcNow;
        var firstActorId = Guid.CreateVersion7();
        var secondActorId = Guid.CreateVersion7();
        var state = InstanceSetupState.FromPersistence(
            InstanceSetupState.SingletonId,
            initializedAt: null,
            initialAdministratorActorId: null,
            createdAt,
            createdAt);

        Assert.True(state.TryComplete(firstActorId, completedAt));
        Assert.False(state.RequiresSetup);
        Assert.Equal(firstActorId, state.InitialAdministratorActorId);
        Assert.Equal(completedAt, state.InitializedAt);

        Assert.False(state.TryComplete(secondActorId, completedAt.AddMinutes(1)));
        Assert.Equal(firstActorId, state.InitialAdministratorActorId);
        Assert.Equal(completedAt, state.InitializedAt);
    }
}
