using Microsoft.Extensions.Caching.Memory;
using Microsoft.Extensions.DependencyInjection;
using Application.Services.Identity;
using Hosting.Common;
using Hosting.Common.Attributes;

namespace Tests.Integration.Application.Features.Identity;

public class ActorScopeEvictorIntegrationTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task EvictForActorAsync_RemovesCachedActorScopeEntries()
    {
        // Create a user with a team (teamActorId will be set)
        var subject = await CreateAuthorizationSubjectAsync(createTeam: true);
        var userId = subject.UserId;
        var teamActorId = subject.TeamActorId!.Value;

        await using var scope = Services.CreateAsyncScope();
        var memoryCache = scope.ServiceProvider.GetRequiredService<IMemoryCache>();
        var evictor = scope.ServiceProvider.GetRequiredService<IActorScopeEvictor>();
        var permissionService = scope.ServiceProvider.GetRequiredService<Hosting.Common.Pipelines.Interfaces.IPermissionService>();

        var uow = scope.ServiceProvider.GetRequiredService<Domain.Contracts.Interfaces.IUnitOfWork>();
        var actorIdsFromDb = await uow.Users.GetActorScopeAsync(userId, TestContext.Current.CancellationToken);
        Assert.True(actorIdsFromDb.Length > 0, "Precondition failed: User actor scope not found in DB. Seed may have failed.");

        // Ensure we exercise the DB path (not a stale perm cache entry) so actor-scope gets populated
        foreach (ResourceType rt in Enum.GetValues<ResourceType>())
        {
            var permKey = Constants.CacheKeys.Permission(userId, (int)rt, string.Empty);
            memoryCache.Remove(permKey);
        }

        // Populate actor-scope via permission service
        var permMeta = await permissionService.ResolvePermissionsAsync(userId, ResourceType.Deployment, null, TestContext.Current.CancellationToken);
        Console.WriteLine($"DEBUG: permMeta={permMeta}");

        var actorCacheKey = Constants.CacheKeys.ActorScope(userId);
        var permCacheKey = Constants.CacheKeys.Permission(userId, (int)ResourceType.Deployment, string.Empty);

        var hasActorCache = memoryCache.TryGetValue(actorCacheKey, out Guid[]? cachedAfter);
        var hasPermCache = memoryCache.TryGetValue(permCacheKey, out PermissionMetadata permCached);

        Assert.True(cachedAfter?.Length > 0);

        // Act
        await evictor.EvictPermissionsForActorAsync(teamActorId, TestContext.Current.CancellationToken);

        // Assert removed
        Assert.False(memoryCache.TryGetValue(Constants.CacheKeys.ActorScope(userId), out _));

        // Simulate a new request: create a fresh scope so IPermissionService request cache is empty
        await using var newScope = Services.CreateAsyncScope();
        var newPermissionService = newScope.ServiceProvider.GetRequiredService<Hosting.Common.Pipelines.Interfaces.IPermissionService>();

        // Remove any perm cache entries so ResolvePermissionsAsync will call GetActorScopeAsync
        foreach (ResourceType rt in Enum.GetValues<ResourceType>())
        {
            var permKey = Constants.CacheKeys.Permission(userId, (int)rt, string.Empty);
            memoryCache.Remove(permKey);
        }

        var repop = await newPermissionService.ResolvePermissionsAsync(userId, ResourceType.Deployment, null, TestContext.Current.CancellationToken);
        Assert.True(memoryCache.TryGetValue(Constants.CacheKeys.ActorScope(userId), out Guid[]? _));
    }

    [Fact]
    public async Task EvictUsersAsync_RemovesCachedActorScopeEntries()
    {
        var subject = await CreateAuthorizationSubjectAsync();
        var userId = subject.UserId;

        await using var scope = Services.CreateAsyncScope();
        var memoryCache = scope.ServiceProvider.GetRequiredService<IMemoryCache>();
        var evictor = scope.ServiceProvider.GetRequiredService<IActorScopeEvictor>();
        var permissionService = scope.ServiceProvider.GetRequiredService<Hosting.Common.Pipelines.Interfaces.IPermissionService>();

        // Ensure no stale permission cache entry for the tested resource prevents actor-scope population
        var singlePermKey = Constants.CacheKeys.Permission(userId, (int)ResourceType.Deployment, string.Empty);
        memoryCache.Remove(singlePermKey);

        var _ = await permissionService.ResolvePermissionsAsync(userId, ResourceType.Deployment, null, TestContext.Current.CancellationToken);
        Assert.True(memoryCache.TryGetValue(Constants.CacheKeys.ActorScope(userId), out Guid[]? _));

        await evictor.EvictUsers(new[] { userId }, TestContext.Current.CancellationToken);

        Assert.False(memoryCache.TryGetValue(Constants.CacheKeys.ActorScope(userId), out _));

        // Simulate a new request for repopulation
        await using var newScope2 = Services.CreateAsyncScope();
        var newPermissionService2 = newScope2.ServiceProvider.GetRequiredService<Hosting.Common.Pipelines.Interfaces.IPermissionService>();

        // Remove any perm cache entries so ResolvePermissionsAsync will call GetActorScopeAsync
        foreach (ResourceType rt in Enum.GetValues<ResourceType>())
        {
            var permKey = Constants.CacheKeys.Permission(userId, (int)rt, string.Empty);
            memoryCache.Remove(permKey);
        }

        var repop2 = await newPermissionService2.ResolvePermissionsAsync(userId, ResourceType.Deployment, null, TestContext.Current.CancellationToken);
        Assert.True(memoryCache.TryGetValue(Constants.CacheKeys.ActorScope(userId), out Guid[]? _));
    }
}
