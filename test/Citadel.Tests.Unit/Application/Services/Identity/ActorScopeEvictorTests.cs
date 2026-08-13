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
        var actorA = Guid.NewGuid();
        var actorB = Guid.NewGuid();

        // include a duplicate to ensure Distinct() is applied
        var actorIds = new[] { actorA, actorB, actorA };

        var actorId = Guid.NewGuid();

        teams
            .Setup(x => x.GetAffectedPrincipalActorIdsAsync(actorId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(actorIds);

        var users = new Mock<IUserRepository>();
        users.Setup(x => x.GetUserIdsByActorIdsAsync(It.IsAny<IEnumerable<Guid>>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);

        var uow = new Mock<IUnitOfWork>();
        uow.SetupGet(x => x.Teams).Returns(teams.Object);
        uow.SetupGet(x => x.Users).Returns(users.Object);

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
                It.Is<IEnumerable<Guid>>(ids => ids.Count() == 2 && ids.Contains(actorA) && ids.Contains(actorB)),
                It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task EvictUsersAsync_RemovesNamespacedActorScopeKeys()
    {
        var uow = new Mock<IUnitOfWork>();
        var users = new Mock<IUserRepository>();
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
        users.Setup(x => x.GetActorIdsAsync(It.IsAny<IEnumerable<Guid>>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync([userA, userB]);
        uow.SetupGet(x => x.Users).Returns(users.Object);

        await evictor.EvictUsers(new[] { userA, userB }, TestContext.Current.CancellationToken);

        roleCache.Verify(cache => cache.RemoveRoles(userA), Times.Once);
        roleCache.Verify(cache => cache.RemoveRoles(userB), Times.Once);
        permissionCache.Verify(cache => cache.InvalidateActor(userA), Times.Once);
        permissionCache.Verify(cache => cache.InvalidateActor(userB), Times.Once);
        connectionRevoker.Verify(
            revoker => revoker.RevokeUsers(
                It.Is<IEnumerable<Guid>>(ids => ids.Order().SequenceEqual(new[] { userA, userB }.Order()))),
            Times.Once);
    }

    [Fact]
    public async Task EvictUsersAsync_RevokesConnectionsBeforeDistributedInvalidationFails()
    {
        var userId = Guid.NewGuid();
        var users = new Mock<IUserRepository>();
        users.Setup(x => x.GetActorIdsAsync(It.IsAny<IEnumerable<Guid>>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync([userId]);
        var uow = new Mock<IUnitOfWork>();
        uow.SetupGet(x => x.Users).Returns(users.Object);
        var actorScopeProvider = new Mock<IActorScopeProvider>();
        actorScopeProvider
            .Setup(provider => provider.InvalidateManyAsync(
                It.IsAny<IEnumerable<Guid>>(),
                It.IsAny<CancellationToken>()))
            .ThrowsAsync(new InvalidOperationException("cache unavailable"));
        var connectionRevoker = new Mock<IUserConnectionRevoker>();
        var evictor = new ActorScopeEvictor(
            uow.Object,
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
