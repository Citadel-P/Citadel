using Application.Services.Identity;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Microsoft.Extensions.Caching.Memory;
using Moq;

namespace Tests.Unit.Application.Services.Identity;

public class ActorScopeEvictorBatchTests
{
    [Fact]
    public async Task EvictUsers_UsesBatchedRoleLookup_And_SetsRoleCacheOncePerUser()
    {
        var uow = new Mock<IUnitOfWork>();

        // Prepare users
        var userA = Guid.NewGuid();
        var userB = Guid.NewGuid();
        var userC = Guid.NewGuid();

        var actorA = Guid.NewGuid();
        var actorB = Guid.NewGuid();

        var users = new[]
        {
            User.FromPersistence(userA, "a", "a@x", "p", actorA, Guid.Empty, DateTime.UtcNow),
            User.FromPersistence(userB, "b", "b@x", "p", actorA, Guid.Empty, DateTime.UtcNow),
            User.FromPersistence(userC, "c", "c@x", "p", actorB, Guid.Empty, DateTime.UtcNow),
        };

        uow.Setup(x => x.Users.GetAllAsync(It.IsAny<IEnumerable<Guid>>(), It.IsAny<CancellationToken>()))
            .Returns((IEnumerable<Guid> ids, CancellationToken ct) => Task.FromResult((IEnumerable<User>?)[.. users.Where(u => ids.Contains(u.Id))]));

        var roleRepo = new Mock<IRoleRepository>();
        uow.SetupGet(x => x.Roles).Returns(roleRepo.Object);

        // actorA has two roles, actorB has none
        var role1 = Guid.NewGuid();
        var role2 = Guid.NewGuid();

        var actorRoleMap = new Dictionary<Guid, Guid[]>
        {
            [actorA] = new[] { role1, role2 },
            [actorB] = Array.Empty<Guid>()
        };

        roleRepo.Setup(r => r.GetActorRoleIdsAsync(It.IsAny<IEnumerable<Guid>>(), It.IsAny<CancellationToken>()))
            .Returns((IEnumerable<Guid> ids, CancellationToken ct) => Task.FromResult((IDictionary<Guid, Guid[]>)actorRoleMap.Where(kv => ids.Contains(kv.Key)).ToDictionary(kv => kv.Key, kv => kv.Value)));

        roleRepo.Setup(r => r.GetAllAsync(It.Is<IEnumerable<Guid>>(ids => ids.Contains(role1) && ids.Contains(role2)), It.IsAny<CancellationToken>()))
            .ReturnsAsync(new[] { Role.FromPersistence(role1, "role1", global::Domain.RoleType.Custom, null), Role.FromPersistence(role2, "role2", global::Domain.RoleType.Custom, null) });

        var memoryCache = new Mock<IMemoryCache>();
        var roleCache = new Mock<IRoleCache>();

        var actorScopeProvider = new Mock<IActorScopeProvider>();
        var permissionCache = new Mock<IPermissionCache>();
        var evictor = new ActorScopeEvictor(uow.Object, memoryCache.Object, roleCache.Object, actorScopeProvider.Object, permissionCache.Object);

        await evictor.EvictUsers(new[] { userA, userB, userC }, CancellationToken.None);

        // actor-scope removed for all
        memoryCache.Verify(m => m.Remove(Hosting.Common.Constants.CacheKeys.ActorScope(userA)), Times.Once);
        memoryCache.Verify(m => m.Remove(Hosting.Common.Constants.CacheKeys.ActorScope(userB)), Times.Once);
        memoryCache.Verify(m => m.Remove(Hosting.Common.Constants.CacheKeys.ActorScope(userC)), Times.Once);

        // role cache set for users A and B (actorA)
        roleCache.Verify(rc => rc.SetRoles(userA, It.Is<string[]>(arr => arr.Contains("role1") && arr.Contains("role2"))), Times.Once);
        roleCache.Verify(rc => rc.SetRoles(userB, It.Is<string[]>(arr => arr.Contains("role1") && arr.Contains("role2"))), Times.Once);

        // role cache removed for userC (actorB has no roles)
        roleCache.Verify(rc => rc.RemoveRoles(userC), Times.Once);
    }
}
