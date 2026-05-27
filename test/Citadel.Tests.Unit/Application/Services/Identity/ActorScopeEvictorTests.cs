using System;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;
using Application.Services.Identity;
using Moq;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Microsoft.Extensions.Caching.Memory;

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

        // provide minimal user/role repository mocks used by the evictor to avoid NREs
        var usersRepo = new Mock<IUserRepository>();
        usersRepo.Setup(r => r.GetAllAsync(It.IsAny<IEnumerable<Guid>>(), It.IsAny<CancellationToken>()))
            .Returns((IEnumerable<Guid> ids, CancellationToken ct) => Task.FromResult((IEnumerable<User>?)Array.Empty<User>()));
        uow.SetupGet(x => x.Users).Returns(usersRepo.Object);

        var rolesRepo = new Mock<IRoleRepository>();
        rolesRepo.Setup(r => r.GetActorRoleIdsAsync(It.IsAny<IEnumerable<Guid>>(), It.IsAny<CancellationToken>()))
            .Returns((IEnumerable<Guid> ids, CancellationToken ct) => Task.FromResult((IDictionary<Guid, Guid[]>)new Dictionary<Guid, Guid[]>()));
        rolesRepo.Setup(r => r.GetAllAsync(It.IsAny<IEnumerable<Guid>>(), It.IsAny<CancellationToken>()))
            .Returns((IEnumerable<Guid> ids, CancellationToken ct) => Task.FromResult((IEnumerable<Role>?)Array.Empty<Role>()));
        uow.SetupGet(x => x.Roles).Returns(rolesRepo.Object);

        var memoryCache = new Mock<IMemoryCache>();
        var roleCache = new Mock<IRoleCache>();

        var evictor = new ActorScopeEvictor(uow.Object, memoryCache.Object, roleCache.Object);

        await evictor.EvictPermissionsForActorAsync(actorId, TestContext.Current.CancellationToken);

        memoryCache.Verify(m => m.Remove($"actor-scope:{userA}"), Times.Once);
        memoryCache.Verify(m => m.Remove($"actor-scope:{userB}"), Times.Once);
    }

    [Fact]
    public async Task EvictUsersAsync_RemovesNamespacedActorScopeKeys()
    {
        var uow = new Mock<IUnitOfWork>();
        var usersRepo = new Mock<IUserRepository>();
        usersRepo.Setup(r => r.GetAllAsync(It.IsAny<IEnumerable<Guid>>(), It.IsAny<CancellationToken>()))
            .Returns((IEnumerable<Guid> ids, CancellationToken ct) => Task.FromResult((IEnumerable<User>?)Array.Empty<User>()));
        uow.SetupGet(x => x.Users).Returns(usersRepo.Object);

        var rolesRepo = new Mock<IRoleRepository>();
        rolesRepo.Setup(r => r.GetActorRoleIdsAsync(It.IsAny<IEnumerable<Guid>>(), It.IsAny<CancellationToken>()))
            .Returns((IEnumerable<Guid> ids, CancellationToken ct) => Task.FromResult((IDictionary<Guid, Guid[]>)new Dictionary<Guid, Guid[]>()));
        rolesRepo.Setup(r => r.GetAllAsync(It.IsAny<IEnumerable<Guid>>(), It.IsAny<CancellationToken>()))
            .Returns((IEnumerable<Guid> ids, CancellationToken ct) => Task.FromResult((IEnumerable<Role>)Array.Empty<Role>()));
        uow.SetupGet(x => x.Roles).Returns(rolesRepo.Object);
        var memoryCache = new Mock<IMemoryCache>();
        var roleCache = new Mock<IRoleCache>();

        var evictor = new ActorScopeEvictor(uow.Object, memoryCache.Object, roleCache.Object);

        var userA = Guid.NewGuid();
        var userB = Guid.NewGuid();

        await evictor.EvictUsers(new[] { userA, userB }, TestContext.Current.CancellationToken);

        memoryCache.Verify(m => m.Remove($"actor-scope:{userA}"), Times.Once);
        memoryCache.Verify(m => m.Remove($"actor-scope:{userB}"), Times.Once);
    }
}
