using Application.Services.Identity;
using Moq;
using Domain.Contracts.Interfaces;

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

        var roleCache = new Mock<IRoleCache>();
        var actorScopeProvider = new Mock<IActorScopeProvider>();
        var permissionCache = new Mock<IPermissionCache>();
        var evictor = new ActorScopeEvictor(uow.Object, roleCache.Object, actorScopeProvider.Object, permissionCache.Object);

        await evictor.EvictPermissionsForActorAsync(actorId, TestContext.Current.CancellationToken);

        actorScopeProvider.Verify(
            provider => provider.InvalidateManyAsync(
                It.Is<IEnumerable<Guid>>(ids => ids.Count() == 2 && ids.Contains(userA) && ids.Contains(userB)),
                It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task EvictUsersAsync_RemovesNamespacedActorScopeKeys()
    {
        var uow = new Mock<IUnitOfWork>();
        var roleCache = new Mock<IRoleCache>();
        var actorScopeProvider = new Mock<IActorScopeProvider>();
        var permissionCache = new Mock<IPermissionCache>();
        var evictor = new ActorScopeEvictor(uow.Object, roleCache.Object, actorScopeProvider.Object, permissionCache.Object);

        var userA = Guid.NewGuid();
        var userB = Guid.NewGuid();

        await evictor.EvictUsers(new[] { userA, userB }, TestContext.Current.CancellationToken);

        roleCache.Verify(cache => cache.RemoveRoles(userA), Times.Once);
        roleCache.Verify(cache => cache.RemoveRoles(userB), Times.Once);
        permissionCache.Verify(cache => cache.InvalidateUser(userA), Times.Once);
        permissionCache.Verify(cache => cache.InvalidateUser(userB), Times.Once);
    }
}
