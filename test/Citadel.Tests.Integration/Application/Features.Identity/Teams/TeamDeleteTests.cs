using System.Text;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.Identity.Teams;

public class TeamDeleteTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task Delete_Team_Should_Remove_Team()
    {
        var seeded = await SeedTeamAsync("team-delete");
        var content = $$"""
        {
            "ids": ["{{seeded.TeamId}}"]
        }
        """;

        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/teams")
        {
            Content = new StringContent(content, Encoding.UTF8, "application/json")
        };

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var team = await uow.Teams.GetAsync(seeded.TeamId, TestContext.Current.CancellationToken);

        Assert.Null(team);
    }

    private async Task<(Guid TeamId, Guid ActorId)> SeedTeamAsync(string name, bool isEnabled = true)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var actor = Actor.Create(ActorType.Team, new ActorMetadata(name), isEnabled);
        var team = Team.Create(name, actor.Id);

        await uow.Actors.AddAsync(actor, TestContext.Current.CancellationToken);
        await uow.Teams.AddAsync(team, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        return (team.Id, actor.Id);
    }
}
