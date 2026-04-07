using System.Text;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.Identity.Teams;

public class TeamCreateTests : IntegrationTestBase
{
    [Fact]
    public async Task Create_Team_ReturnsSuccess()
    {
        var createJson = """
        {
          "name": "team-api"
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/teams", content, cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var team = (await uow.Teams.GetAllAsync(TestContext.Current.CancellationToken)).Single(x => x.Name == "team-api");
        var actor = await uow.Actors.GetById(team.ActorId, TestContext.Current.CancellationToken);

        Assert.NotNull(actor);
        Assert.True(actor!.IsEnabled);
    }

    [Fact]
    public async Task Create_Team_With_Duplicate_Name_Returns_Conflict()
    {
        await SeedTeamAsync("team-dup");

        var createJson = """
        {
          "name": "team-dup"
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/teams", content, cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.Conflict, response.StatusCode);
    }

    private async Task SeedTeamAsync(string name, bool isEnabled = true)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var actor = Actor.Create(ActorType.Team, new ActorMetadata(name), isEnabled);
        var team = Team.Create(name, actor.Id);

        await uow.Actors.AddAsync(actor, TestContext.Current.CancellationToken);
        await uow.Teams.AddAsync(team, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }
}
