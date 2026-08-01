using Application.Services.Identity;
using Application.Services.SignalR;
using Moq;
using Domain.Contracts.Interfaces;
using Microsoft.Extensions.Caching.Memory;
using Domain.Entities.Identity;

namespace Tests.Unit.Application.Services.Identity;

public class ActorScopeEvictorRoleTests
{
    [Fact]
    public async Task EvictPermissionsForRoleAsync_Invokes_UserLevelInvalidation()
    {
        var roleId = Guid.NewGuid();

        var actorA = Guid.NewGuid();
        var actorB = Guid.NewGuid();

        var userA = Guid.NewGuid();
        var userB = Guid.NewGuid();
        var userC = Guid.NewGuid();

        var roles = new Mock<IRoleRepository>();
        roles.Setup(r => r.GetActorIdsByRoleIdAsync(roleId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new[] { actorA, actorB });

        var teams = new Mock<ITeamRepository>();
        teams.Setup(t => t.GetUserIdsByActorIdAsync(actorA, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new[] { userA, userB });
        teams.Setup(t => t.GetUserIdsByActorIdAsync(actorB, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new[] { userC });

        var uow = new Mock<IUnitOfWork>();
        uow.SetupGet(x => x.Roles).Returns(roles.Object);
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

        await evictor.EvictPermissionsForRoleAsync(roleId, TestContext.Current.CancellationToken);

        // Ensure permission cache invalidation was attempted for each user
        permissionCache.Verify(p => p.InvalidateUser(userA), Times.Once);
        permissionCache.Verify(p => p.InvalidateUser(userB), Times.Once);
        permissionCache.Verify(p => p.InvalidateUser(userC), Times.Once);

        actorScopeProvider.Verify(
            provider => provider.InvalidateManyAsync(
                It.Is<IEnumerable<Guid>>(ids => ids.Order().SequenceEqual(new[] { userA, userB, userC }.Order())),
                It.IsAny<CancellationToken>()),
            Times.Once);
    }
}
