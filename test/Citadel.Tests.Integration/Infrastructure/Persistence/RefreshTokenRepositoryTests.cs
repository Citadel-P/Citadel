using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Infrastructure.Persistence;

public sealed class RefreshTokenRepositoryTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task DeleteExpiredAsync_ShouldRemoveOnlyExpiredRefreshTokens()
    {
        var now = DateTime.UtcNow;
        var actor = Actor.Create(ActorType.User, new ActorMetadata("session-cleanup-user"));
        var user = new User(
            "session-cleanup-user",
            $"session-cleanup-{Guid.CreateVersion7():N}@citadel.test",
            HashTestPassword("password"),
            actor.Id,
            Constants.SystemId);

        var expiredToken = RefreshToken.Create(Guid.CreateVersion7(), user.Id, now.AddSeconds(-2), "expired", "127.0.0.1");
        var recentlyExpiredToken = RefreshToken.Create(Guid.CreateVersion7(), user.Id, now.AddSeconds(-1), "recently-expired", "127.0.0.1");
        var activeToken = RefreshToken.Create(Guid.CreateVersion7(), user.Id, now.AddSeconds(10), "active", "127.0.0.1");

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Actors.AddAsync(actor, TestContext.Current.CancellationToken);
            await uow.Users.AddAsync(user, TestContext.Current.CancellationToken);
            await uow.RefreshTokens.AddAsync(expiredToken, TestContext.Current.CancellationToken);
            await uow.RefreshTokens.AddAsync(recentlyExpiredToken, TestContext.Current.CancellationToken);
            await uow.RefreshTokens.AddAsync(activeToken, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var removed = await uow.RefreshTokens.DeleteExpiredAsync(now, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);

            Assert.Equal(2, removed);
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            Assert.Equal(1, await uow.RefreshTokens.CountAsync(user.Id, TestContext.Current.CancellationToken));
            Assert.Null(await uow.RefreshTokens.GetActiveTokenIdAsync(expiredToken.Id, user.Id, now, TestContext.Current.CancellationToken));
            Assert.Null(await uow.RefreshTokens.GetActiveTokenIdAsync(recentlyExpiredToken.Id, user.Id, now, TestContext.Current.CancellationToken));
            Assert.Equal(activeToken.Id, await uow.RefreshTokens.GetActiveTokenIdAsync(activeToken.Id, user.Id, now, TestContext.Current.CancellationToken));
        }
    }
}
