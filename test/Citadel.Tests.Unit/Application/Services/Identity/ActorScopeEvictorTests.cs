using Application.Services.Identity;
using Domain.Contracts.Interfaces;
using Microsoft.Extensions.Caching.Memory;
using Moq;

namespace Tests.Unit.Application.Services.Identity;

public class ActorScopeEvictorTests
{
    [Fact]
    public async Task EvictForActorAsync_RemovesNamespacedActorScopeKeys()
    {
        var teams = new Mock<ITeamRepository>();
        var userA = Guid.NewGuid();
        var userB = Guid.NewGuid();

        // include a duplicate to ensure Distinct() is applied
        var userIds = new[] { userA, userB, userA };

        var actorId = Guid.NewGuid();

        teams
            .Setup(x => x.GetUserIdsByActorIdAsync(actorId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(userIds);

        var uow = new Mock<IUnitOfWork>();
        uow.SetupGet(x => x.Teams).Returns(teams.Object);

        var memoryCache = new Mock<IMemoryCache>();

        var evictor = new ActorScopeEvictor(uow.Object, memoryCache.Object);

        await evictor.EvictPermissionsForActorAsync(actorId, TestContext.Current.CancellationToken);

        memoryCache.Verify(m => m.Remove($"actor-scope:{userA}"), Times.Once);
        memoryCache.Verify(m => m.Remove($"actor-scope:{userB}"), Times.Once);
    }

    [Fact]
    public void EvictUsersAsync_RemovesNamespacedActorScopeKeys()
    {
        var uow = new Mock<IUnitOfWork>();
        var memoryCache = new Mock<IMemoryCache>();

        var evictor = new ActorScopeEvictor(uow.Object, memoryCache.Object);

        var userA = Guid.NewGuid();
        var userB = Guid.NewGuid();

        evictor.EvictUsers(new[] { userA, userB }, TestContext.Current.CancellationToken);

        memoryCache.Verify(m => m.Remove($"actor-scope:{userA}"), Times.Once);
        memoryCache.Verify(m => m.Remove($"actor-scope:{userB}"), Times.Once);
    }
}
