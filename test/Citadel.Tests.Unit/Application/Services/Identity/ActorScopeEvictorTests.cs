using Application.Services.Identity;
using Application.Services.SignalR;
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
        var evictor = new ActorScopeEvictor(
            uow.Object,
            roleCache.Object,
            actorScopeProvider.Object,
            permissionCache.Object,
            Mock.Of<IUserConnectionRevoker>());

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
        var connectionRevoker = new Mock<IUserConnectionRevoker>();
        var evictor = new ActorScopeEvictor(
            uow.Object,
            roleCache.Object,
            actorScopeProvider.Object,
            permissionCache.Object,
            connectionRevoker.Object);

        var userA = Guid.NewGuid();
        var userB = Guid.NewGuid();

        await evictor.EvictUsers(new[] { userA, userB }, TestContext.Current.CancellationToken);

        roleCache.Verify(cache => cache.RemoveRoles(userA), Times.Once);
        roleCache.Verify(cache => cache.RemoveRoles(userB), Times.Once);
        permissionCache.Verify(cache => cache.InvalidateUser(userA), Times.Once);
        permissionCache.Verify(cache => cache.InvalidateUser(userB), Times.Once);
        connectionRevoker.Verify(
            revoker => revoker.RevokeUsers(
                It.Is<IEnumerable<Guid>>(ids => ids.Order().SequenceEqual(new[] { userA, userB }.Order()))),
            Times.Once);
    }

    [Fact]
    public async Task EvictUsersAsync_RevokesConnectionsBeforeDistributedInvalidationFails()
    {
        var userId = Guid.NewGuid();
        var actorScopeProvider = new Mock<IActorScopeProvider>();
        actorScopeProvider
            .Setup(provider => provider.InvalidateManyAsync(
                It.IsAny<IEnumerable<Guid>>(),
                It.IsAny<CancellationToken>()))
            .ThrowsAsync(new InvalidOperationException("cache unavailable"));
        var connectionRevoker = new Mock<IUserConnectionRevoker>();
        var evictor = new ActorScopeEvictor(
            Mock.Of<IUnitOfWork>(),
            Mock.Of<IRoleCache>(),
            actorScopeProvider.Object,
            Mock.Of<IPermissionCache>(),
            connectionRevoker.Object);

        await Assert.ThrowsAsync<InvalidOperationException>(() =>
            evictor.EvictUsers([userId], TestContext.Current.CancellationToken));

        connectionRevoker.Verify(
            revoker => revoker.RevokeUsers(
                It.Is<IEnumerable<Guid>>(ids => ids.SequenceEqual(new[] { userId }))),
            Times.Once);
    }
}
