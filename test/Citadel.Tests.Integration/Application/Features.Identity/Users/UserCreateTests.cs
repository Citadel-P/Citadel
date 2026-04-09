using System.Text;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.Identity.Users;

public class UserCreateTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task Create_User_ReturnsSuccess()
    {
        var createJson = """
        {
          "name": "user-api",
          "email": "user-api@citadel.local",
          "password": "password123"
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/users", content, cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var user = (await uow.Users.GetAllAsync(TestContext.Current.CancellationToken)).Single(x => x.Name == "user-api");
        var actor = await uow.Actors.GetById(user.ActorId, TestContext.Current.CancellationToken);

        Assert.Equal("user-api@citadel.local", user.Email);
        Assert.NotNull(actor);
        Assert.True(actor!.IsEnabled);
    }

    [Fact]
    public async Task Create_User_With_Duplicate_Name_Returns_Conflict()
    {
        await SeedUserAsync("user-dup", "user-dup@citadel.local");

        var createJson = """
        {
          "name": "user-dup",
          "email": "user-dup-2@citadel.local",
          "password": "password123"
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/users", content, cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.Conflict, response.StatusCode);
    }

    [Fact]
    public async Task Create_User_With_Duplicate_Email_Returns_Conflict()
    {
        await SeedUserAsync("user-mail-1", "user-mail@citadel.local");

        var createJson = """
        {
          "name": "user-mail-2",
          "email": "user-mail@citadel.local",
          "password": "password123"
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/users", content, cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.Conflict, response.StatusCode);
    }

    private async Task SeedUserAsync(string name, string email, string password = "password123", bool isEnabled = true)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var actor = Actor.Create(ActorType.User, new ActorMetadata(name), isEnabled);
        var user = new User(name, email, password, actor.Id, Constants.SystemId);

        await uow.Actors.AddAsync(actor, TestContext.Current.CancellationToken);
        await uow.Users.AddAsync(user, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }
}
