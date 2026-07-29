using Application.Services.Identity;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Services.Identity;

public class ActorRoleServiceIntegrationTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task AssignAndRemoveRole_ShouldEvictRoleCacheEntries()
    {
        // Arrange: create a user and a role, then assign/remove role and assert cache eviction

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        // Insert actor, user and a role using repositories
        var actor = Actor.Create(ActorType.User, new ActorMetadata("test-user"));
        var user = new User(
            "test-user",
            "test-user@citadel.test",
            HashTestPassword("password"),
            actor.Id,
            Constants.SystemId);
        var role = Role.Create("integration-role", RoleType.Custom, [Permission.Create(Guid.Empty, ResourceType.Registry, PermissionLevel.Read)]);

        await uow.Actors.AddAsync(actor, TestContext.Current.CancellationToken);
        await uow.Users.AddAsync(user, TestContext.Current.CancellationToken);
        await uow.Roles.AddAsync(role, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        var roleCache = Services.GetRequiredService<IRoleCache>();
        var actorRoleService = Services.GetRequiredService<IActorRoleService>();

        // Seed role cache for the created user
        roleCache.SetRoles(user.Id, new[] { "integration-role" });
        Assert.NotNull(roleCache.GetRoles(user.Id));

        // Assign role
        var assignResult = await actorRoleService.AssignRoleAsync(actor.Id, role.Id, TestContext.Current.CancellationToken);
        Assert.True(assignResult.IsSuccess());

        // After assign, actor scope evictor should have removed cached roles for user (commit + eviction)
        var rolesAfterAssign = roleCache.GetRoles(user.Id);
        Assert.Null(rolesAfterAssign);

        // Re-seed and then remove role
        roleCache.SetRoles(user.Id, new[] { "integration-role" });
        var removeResult = await actorRoleService.RemoveRoleAsync(actor.Id, role.Id, TestContext.Current.CancellationToken);
        Assert.True(removeResult.IsSuccess());

        var rolesAfterRemove = roleCache.GetRoles(user.Id);
        Assert.Null(rolesAfterRemove);
    }
}
