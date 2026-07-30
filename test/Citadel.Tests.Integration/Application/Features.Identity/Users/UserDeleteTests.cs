using System.Text;
using System.Net;
using Application.Services.Identity;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.Identity.Users;

public class UserDeleteTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private static readonly Guid SeededAdminUserId = Guid.Parse("10000000-0000-0000-0000-000000000001");
    private static readonly Guid AdminRoleId = Guid.Parse("30000000-0000-0000-0000-000000000001");

    [Fact]
    public async Task Delete_User_Should_Remove_User()
    {
        var seeded = await SeedUserAsync("user-delete", "user-delete@citadel.local");
        var content = $$"""
        {
            "ids": ["{{seeded.UserId}}"]
        }
        """;

        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/users")
        {
            Content = new StringContent(content, Encoding.UTF8, "application/json")
        };

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var user = await uow.Users.GetAsync(seeded.UserId, TestContext.Current.CancellationToken);

        Assert.Null(user);
    }

    [Fact]
    public async Task Delete_Last_Administrator_Should_Return_Conflict_And_Keep_User()
    {
        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/users")
        {
            Content = new StringContent(
                $$"""{"ids":["{{SeededAdminUserId}}"]}""",
                Encoding.UTF8,
                "application/json")
        };

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.Conflict, response.StatusCode);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        Assert.NotNull(await uow.Users.GetAsync(SeededAdminUserId, TestContext.Current.CancellationToken));
    }

    [Fact]
    public async Task Concurrent_Administrator_Deletes_Should_Keep_One_Administrator()
    {
        var first = await SeedUserAsync("admin-delete-first", "admin-delete-first@citadel.local");
        var second = await SeedUserAsync("admin-delete-second", "admin-delete-second@citadel.local");

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Roles.AddActorRoleAsync(first.ActorId, AdminRoleId, TestContext.Current.CancellationToken);
            await uow.Roles.AddActorRoleAsync(second.ActorId, AdminRoleId, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var deleteSeededAdmin = await Client.SendAsync(
            CreateDeleteRequest(SeededAdminUserId),
            TestContext.Current.CancellationToken);
        deleteSeededAdmin.EnsureSuccessStatusCode();

        var results = await Task.WhenAll(
            DeleteAdministratorInIndependentTransactionAsync(first.UserId),
            DeleteAdministratorInIndependentTransactionAsync(second.UserId));

        Assert.Single(results, static result => result);
        Assert.Single(results, static result => !result);

        await using var verificationScope = Services.CreateAsyncScope();
        var verificationUow = verificationScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        Assert.True(await verificationUow.Users.HasEnabledAdministratorAsync(TestContext.Current.CancellationToken));
    }

    private async Task<(Guid UserId, Guid ActorId)> SeedUserAsync(string name, string email, string password = "password123", bool isEnabled = true)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var actor = Actor.Create(ActorType.User, new ActorMetadata(name), isEnabled);
        var user = new User(
            name,
            email,
            HashTestPassword(password),
            actor.Id,
            Constants.SystemId);

        await uow.Actors.AddAsync(actor, TestContext.Current.CancellationToken);
        await uow.Users.AddAsync(user, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        return (user.Id, actor.Id);
    }

    private static HttpRequestMessage CreateDeleteRequest(Guid userId)
        => new(HttpMethod.Delete, "/api/v1/users")
        {
            Content = new StringContent(
                $$"""{"ids":["{{userId}}"]}""",
                Encoding.UTF8,
                "application/json")
        };

    private async Task<bool> DeleteAdministratorInIndependentTransactionAsync(Guid userId)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var guard = scope.ServiceProvider.GetRequiredService<IAdministratorGuard>();

        await uow.Users.RemoveRangeAsync([userId], TestContext.Current.CancellationToken);
        var result = await guard.EnsureAdministratorRemainsAsync(TestContext.Current.CancellationToken);
        if (result.IsFailure())
            return false;

        await uow.CommitAsync(TestContext.Current.CancellationToken);
        return true;
    }
}
