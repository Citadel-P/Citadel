using System.Text.Json;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.Identity.Users;

public class UserViewTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task List_Users_Should_Return_Paged_Users_For_Admin()
    {
        var first = await SeedUserAsync("user-list-1", "user-list-1@citadel.local");
        var second = await SeedUserAsync("user-list-2", "user-list-2@citadel.local");

        var response = await Client.GetAsync("/api/v1/users?page=1&pageSize=10", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var pagedResult = document.RootElement.GetProperty("pagedResult");
        var items = pagedResult.GetProperty("items");
        Assert.Contains(items.EnumerateArray(), x => x.GetProperty("id").GetGuid() == first.UserId);
        Assert.Contains(items.EnumerateArray(), x => x.GetProperty("id").GetGuid() == second.UserId);
        Assert.Equal(1, pagedResult.GetProperty("page").GetInt32());
        Assert.Equal(10, pagedResult.GetProperty("pageSize").GetInt32());
    }

    [Fact]
    public async Task Get_User_Should_Return_User_For_Admin()
    {
        var seeded = await SeedUserAsync("user-view", "user-view@citadel.local", isEnabled: false);

        var response = await Client.GetAsync($"/api/v1/users/{seeded.UserId}", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(seeded.UserId, document.RootElement.GetProperty("id").GetGuid());
        Assert.Equal("user-view", document.RootElement.GetProperty("name").GetString());
        Assert.Equal("user-view@citadel.local", document.RootElement.GetProperty("email").GetString());
        Assert.False(document.RootElement.GetProperty("isEnabled").GetBoolean());
    }

    private async Task<(Guid UserId, Guid ActorId)> SeedUserAsync(string name, string email, string password = "password123", bool isEnabled = true)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var actor = Actor.Create(ActorType.User, new ActorMetadata(name), isEnabled);
        var user = new User(name, email, password, actor.Id, Constants.SystemId);

        await uow.Actors.AddAsync(actor, TestContext.Current.CancellationToken);
        await uow.Users.AddAsync(user, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        return (user.Id, actor.Id);
    }
}
