using Application.Services.Identity;
using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Moq;

namespace Tests.Unit.Application.Services.Identity;

public class ActorScopeEvictorBatchTests
{
    [Fact]
    public async Task EvictUsers_InvalidatesEachDistinctUserOnce()
    {
        var uow = new Mock<IUnitOfWork>();

        // Prepare users
        var userA = Guid.NewGuid();
        var userB = Guid.NewGuid();
        var userC = Guid.NewGuid();
        var actorA = Guid.NewGuid();
        var actorB = Guid.NewGuid();
        var actorC = Guid.NewGuid();
        uow.Setup(x => x.Users.GetActorIdsAsync(
                It.IsAny<IEnumerable<Guid>>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync([actorA, actorB, actorC]);

        var roleCache = new Mock<IRoleCache>();
        var actorScopeProvider = new Mock<IActorScopeProvider>();
        var permissionCache = new Mock<IPermissionCache>();
        var evictor = new ActorScopeEvictor(
            uow.Object,
            roleCache.Object,
            actorScopeProvider.Object,
            permissionCache.Object,
            Mock.Of<IUserConnectionRevoker>());

        await evictor.EvictUsers(
            new[] { userA, userB, userA, userC },
            TestContext.Current.CancellationToken);

        roleCache.Verify(rc => rc.RemoveRoles(actorA), Times.Once);
        roleCache.Verify(rc => rc.RemoveRoles(actorB), Times.Once);
        roleCache.Verify(rc => rc.RemoveRoles(actorC), Times.Once);
        permissionCache.Verify(cache => cache.InvalidateActor(actorA), Times.Once);
        permissionCache.Verify(cache => cache.InvalidateActor(actorB), Times.Once);
        permissionCache.Verify(cache => cache.InvalidateActor(actorC), Times.Once);
    }
}
