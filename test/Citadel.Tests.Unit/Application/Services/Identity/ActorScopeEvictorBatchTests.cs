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

        roleCache.Verify(rc => rc.RemoveRoles(userA), Times.Once);
        roleCache.Verify(rc => rc.RemoveRoles(userB), Times.Once);
        roleCache.Verify(rc => rc.RemoveRoles(userC), Times.Once);
        permissionCache.Verify(cache => cache.InvalidateUser(userA), Times.Once);
        permissionCache.Verify(cache => cache.InvalidateUser(userB), Times.Once);
        permissionCache.Verify(cache => cache.InvalidateUser(userC), Times.Once);
    }
}
