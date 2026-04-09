using System.Text.Json;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.Identity.Teams;

public class TeamViewTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task List_Teams_Should_Return_Paged_Teams_For_Admin()
    {
        var first = await SeedTeamAsync("team-list-1");
        var second = await SeedTeamAsync("team-list-2");

        var response = await Client.GetAsync("/api/v1/teams?page=1&pageSize=10", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var pagedResult = document.RootElement.GetProperty("pagedResult");
        var items = pagedResult.GetProperty("items");
        Assert.Contains(items.EnumerateArray(), x => x.GetProperty("id").GetGuid() == first.TeamId);
        Assert.Contains(items.EnumerateArray(), x => x.GetProperty("id").GetGuid() == second.TeamId);
        Assert.Equal(1, pagedResult.GetProperty("page").GetInt32());
        Assert.Equal(10, pagedResult.GetProperty("pageSize").GetInt32());
    }

    [Fact]
    public async Task Get_Team_Should_Return_Team_For_Admin()
    {
        var seeded = await SeedTeamAsync("team-view", isEnabled: false);

        var response = await Client.GetAsync($"/api/v1/teams/{seeded.TeamId}", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(seeded.TeamId, document.RootElement.GetProperty("id").GetGuid());
        Assert.Equal("team-view", document.RootElement.GetProperty("name").GetString());
        Assert.False(document.RootElement.GetProperty("isEnabled").GetBoolean());
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
