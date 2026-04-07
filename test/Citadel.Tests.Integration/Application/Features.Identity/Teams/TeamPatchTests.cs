using System.Text;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.Identity.Teams;

public class TeamPatchTests : IntegrationTestBase
{
    [Fact]
    public async Task Patch_Team_Should_Update_IsEnabled()
    {
        var seeded = await SeedTeamAsync("team-patch");
        var patchJson = """
        {
          "isEnabled": false
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/teams/{seeded.TeamId}", content, cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var actor = await uow.Actors.GetById(seeded.ActorId, TestContext.Current.CancellationToken);

        Assert.NotNull(actor);
        Assert.False(actor!.IsEnabled);
    }

    [Fact]
    public async Task Rename_Team_Should_Update_Name()
    {
        var seeded = await SeedTeamAsync("team-rename");
        var renameJson = $$"""
        {
          "id": "{{seeded.TeamId}}",
          "name": "team-renamed"
        }
        """;
        var content = new StringContent(renameJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/teams/rename", content, TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var team = await uow.Teams.GetAsync(seeded.TeamId, TestContext.Current.CancellationToken);

        Assert.Equal("team-renamed", team?.Name);
    }

    [Fact]
    public async Task Add_Team_Role_Should_Assign_Role()
    {
        var seeded = await SeedTeamAsync("team-role-add");
        var addRoleJson = $$"""
        {
          "roleId": "{{ViewerRoleId}}"
        }
        """;
        var content = new StringContent(addRoleJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync($"/api/v1/teams/{seeded.TeamId}/roles", content, TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var roleIds = await uow.Roles.GetActorRoleIdsAsync(seeded.ActorId, TestContext.Current.CancellationToken);

        Assert.Contains(ViewerRoleId, roleIds);
    }

    [Fact]
    public async Task Remove_Team_Role_Should_Unassign_Role()
    {
        var seeded = await SeedTeamAsync("team-role-remove");

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Roles.AddActorRoleAsync(seeded.ActorId, ViewerRoleId, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var response = await Client.DeleteAsync($"/api/v1/teams/{seeded.TeamId}/roles/{ViewerRoleId}", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var verificationScope = Services.CreateAsyncScope();
        var verificationUow = verificationScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var roleIds = await verificationUow.Roles.GetActorRoleIdsAsync(seeded.ActorId, TestContext.Current.CancellationToken);

        Assert.DoesNotContain(ViewerRoleId, roleIds);
    }

    [Fact]
    public async Task Add_Team_Member_Should_Assign_User()
    {
        var seeded = await SeedTeamAsync("team-member-add");
        var user = await SeedUserAsync("team-member-user", "team-member-user@citadel.local");
        var addMemberJson = $$"""
        {
          "userId": "{{user.UserId}}"
        }
        """;
        var content = new StringContent(addMemberJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync($"/api/v1/teams/{seeded.TeamId}/members", content, TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var userIds = await uow.Teams.GetUserIdsAsync(seeded.TeamId, TestContext.Current.CancellationToken);

        Assert.Contains(user.UserId, userIds);
    }

    [Fact]
    public async Task Remove_Team_Member_Should_Unassign_User()
    {
        var seeded = await SeedTeamAsync("team-member-remove");
        var user = await SeedUserAsync("team-member-user-remove", "team-member-user-remove@citadel.local");

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Teams.AddMemberAsync(seeded.TeamId, user.UserId, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var response = await Client.DeleteAsync($"/api/v1/teams/{seeded.TeamId}/members/{user.UserId}", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var verificationScope = Services.CreateAsyncScope();
        var verificationUow = verificationScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var userIds = await verificationUow.Teams.GetUserIdsAsync(seeded.TeamId, TestContext.Current.CancellationToken);

        Assert.DoesNotContain(user.UserId, userIds);
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
