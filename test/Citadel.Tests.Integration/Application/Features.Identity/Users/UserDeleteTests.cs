using System.Text;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.Identity.Users;

public class UserDeleteTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
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
