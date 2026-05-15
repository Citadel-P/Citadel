using System.Text;
using System.Text.Json;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Domain.Entities.Identity;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Identity.Teams;

public class TeamPatchTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
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

    [Fact]
    public async Task Add_Team_Resource_Access_Should_Allow_Team_Member_To_View_Only_Granted_Deployment()
    {
        var deploymentId = await SeedDeploymentAsync("deployment-visible-by-team-access");
        await SeedDeploymentAsync("deployment-hidden-by-team-access");

        var seededTeam = await SeedTeamAsync("team-resource-access-add");
        var teamUser = await SeedUserAsync("team-resource-access-user", "team-resource-access-user@citadel.local");

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Teams.AddMemberAsync(seededTeam.TeamId, teamUser.UserId, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var payload = $$"""
        {
          "resourceType": "Deployment",
          "resourceId": "{{deploymentId}}",
          "permissionLevel": "Read"
        }
        """;

        var addResponse = await Client.PostAsync(
            $"/api/v1/teams/{seededTeam.TeamId}/resource-accesses",
            new StringContent(payload, Encoding.UTF8, "application/json"),
            TestContext.Current.CancellationToken);
        addResponse.EnsureSuccessStatusCode();

        Client.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(teamUser.UserId, teamUser.ActorId));

        var listResponse = await Client.GetAsync("/api/v1/deployments", TestContext.Current.CancellationToken);
        listResponse.EnsureSuccessStatusCode();

        using var doc = await JsonDocument.ParseAsync(
            await listResponse.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var deployments = doc.RootElement.GetProperty("deployments");
        Assert.Equal(1, deployments.GetArrayLength());
        Assert.Equal(deploymentId, deployments[0].GetProperty("id").GetGuid());
    }

    [Fact]
    public async Task Remove_Team_Resource_Access_Should_Revoke_Team_Member_Deployment_View()
    {
        var deploymentId = await SeedDeploymentAsync("deployment-revoked-team-access");

        var seededTeam = await SeedTeamAsync("team-resource-access-remove");
        var teamUser = await SeedUserAsync("team-resource-access-user-remove", "team-resource-access-user-remove@citadel.local");

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Teams.AddMemberAsync(seededTeam.TeamId, teamUser.UserId, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var payload = $$"""
        {
          "resourceType": "Deployment",
          "resourceId": "{{deploymentId}}",
          "permissionLevel": "Read"
        }
        """;

        var addResponse = await Client.PostAsync(
            $"/api/v1/teams/{seededTeam.TeamId}/resource-accesses",
            new StringContent(payload, Encoding.UTF8, "application/json"),
            TestContext.Current.CancellationToken);
        addResponse.EnsureSuccessStatusCode();

        var removeRequest = new HttpRequestMessage(HttpMethod.Delete, $"/api/v1/teams/{seededTeam.TeamId}/resource-accesses")
        {
            Content = new StringContent(payload, Encoding.UTF8, "application/json")
        };

        var removeResponse = await Client.SendAsync(removeRequest, TestContext.Current.CancellationToken);
        removeResponse.EnsureSuccessStatusCode();

        Client.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(teamUser.UserId, teamUser.ActorId));

        var listResponse = await Client.GetAsync("/api/v1/deployments", TestContext.Current.CancellationToken);
        listResponse.EnsureSuccessStatusCode();

        using var doc = await JsonDocument.ParseAsync(
            await listResponse.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var deployments = doc.RootElement.GetProperty("deployments");
        Assert.Equal(0, deployments.GetArrayLength());
    }

    [Fact]
    public async Task Patch_Team_Should_Replace_Users_Roles_And_Resource_Accesses()
    {
        var seededTeam = await SeedTeamAsync("team-patch-assignments");

        var oldUser = await SeedUserAsync("team-patch-old-user", "team-patch-old-user@citadel.local");
        var newUser = await SeedUserAsync("team-patch-new-user", "team-patch-new-user@citadel.local");
        var oldRoleId = await SeedRoleAsync("team-patch-old-role");
        var newRoleId = await SeedRoleAsync("team-patch-new-role");
        var oldDeploymentId = await SeedDeploymentAsync("deployment-old-team-patch");
        var newDeploymentId = await SeedDeploymentAsync("deployment-new-team-patch");

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Teams.ReplaceMembersAsync(seededTeam.TeamId, [oldUser.UserId], TestContext.Current.CancellationToken);
            await uow.Roles.ReplaceActorRolesAsync(seededTeam.ActorId, [oldRoleId], TestContext.Current.CancellationToken);
            await uow.ResourceAccesses.ReplaceAsync(
                seededTeam.ActorId,
                [ResourceAccess.Create(ResourceType.Deployment, oldDeploymentId, seededTeam.ActorId, PermissionLevel.Read)],
                TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var patchJson = $$"""
        {
          "userIds": ["{{newUser.UserId}}"],
          "roleIds": ["{{newRoleId}}"],
          "resourceAccesses": [
            {
              "resourceType": "Deployment",
              "resourceId": "{{newDeploymentId}}",
              "permissionLevel": "Read"
            }
          ]
        }
        """;

        var response = await Client.PatchAsync(
            $"/api/v1/teams/{seededTeam.TeamId}",
            new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json"),
            cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var verificationScope = Services.CreateAsyncScope();
        var uowVerify = verificationScope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var memberIds = (await uowVerify.Teams.GetUserIdsAsync(seededTeam.TeamId, TestContext.Current.CancellationToken)).ToArray();
        var roleIds = (await uowVerify.Roles.GetActorRoleIdsAsync(seededTeam.ActorId, TestContext.Current.CancellationToken)).ToArray();

        Assert.Single(memberIds);
        Assert.Contains(newUser.UserId, memberIds);
        Assert.DoesNotContain(oldUser.UserId, memberIds);

        Assert.Single(roleIds);
        Assert.Contains(newRoleId, roleIds);
        Assert.DoesNotContain(oldRoleId, roleIds);

        var actorIdsOld = await uowVerify.Users.GetActorScopeAsync(oldUser.UserId, TestContext.Current.CancellationToken);
        var actorIdsNew = await uowVerify.Users.GetActorScopeAsync(newUser.UserId, TestContext.Current.CancellationToken);

        var canOldUserViewNewDeployment = await uowVerify.Users.HasPermissionAsync(
            oldUser.UserId,
            ResourceType.Deployment,
            PermissionLevel.Read,
            SpecificPermission.None,
            newDeploymentId,
            actorIdsOld,
            TestContext.Current.CancellationToken);

        var canNewUserViewNewDeployment = await uowVerify.Users.HasPermissionAsync(
            newUser.UserId,
            ResourceType.Deployment,
            PermissionLevel.Read,
            SpecificPermission.None,
            newDeploymentId,
            actorIdsNew,
            TestContext.Current.CancellationToken);

        var canNewUserViewOldDeployment = await uowVerify.Users.HasPermissionAsync(
            newUser.UserId,
            ResourceType.Deployment,
            PermissionLevel.Read,
            SpecificPermission.None,
            oldDeploymentId,
            actorIdsNew,
            TestContext.Current.CancellationToken);

        Assert.False(canOldUserViewNewDeployment);
        Assert.True(canNewUserViewNewDeployment);
        Assert.False(canNewUserViewOldDeployment);
    }

    [Fact]
    public async Task Patch_Team_With_Empty_Assignment_Collections_Should_Clear_Users_Roles_And_Resource_Accesses()
    {
        var seededTeam = await SeedTeamAsync("team-patch-clear-assignments");
        var member = await SeedUserAsync("team-patch-clear-user", "team-patch-clear-user@citadel.local");
        var roleId = await SeedRoleAsync("team-patch-clear-role");
        var deploymentId = await SeedDeploymentAsync("deployment-clear-team-patch");

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Teams.ReplaceMembersAsync(seededTeam.TeamId, [member.UserId], TestContext.Current.CancellationToken);
            await uow.Roles.ReplaceActorRolesAsync(seededTeam.ActorId, [roleId], TestContext.Current.CancellationToken);
            await uow.ResourceAccesses.ReplaceAsync(
                seededTeam.ActorId,
                [ResourceAccess.Create(ResourceType.Deployment, deploymentId, seededTeam.ActorId, PermissionLevel.Read)],
                TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var patchJson = """
        {
          "userIds": [],
          "roleIds": [],
          "resourceAccesses": []
        }
        """;

        var response = await Client.PatchAsync(
            $"/api/v1/teams/{seededTeam.TeamId}",
            new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json"),
            cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var verificationScope = Services.CreateAsyncScope();
        var uowVerify = verificationScope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var memberIds = (await uowVerify.Teams.GetUserIdsAsync(seededTeam.TeamId, TestContext.Current.CancellationToken)).ToArray();
        var roleIds = (await uowVerify.Roles.GetActorRoleIdsAsync(seededTeam.ActorId, TestContext.Current.CancellationToken)).ToArray();

        Assert.Empty(memberIds);
        Assert.Empty(roleIds);

        var actorIdsMember = await uowVerify.Users.GetActorScopeAsync(member.UserId, TestContext.Current.CancellationToken);

        var hasAccess = await uowVerify.Users.HasPermissionAsync(
            member.UserId,
            ResourceType.Deployment,
            PermissionLevel.Read,
            SpecificPermission.None,
            deploymentId,
            actorIdsMember,
            TestContext.Current.CancellationToken);

        Assert.False(hasAccess);
    }

    private async Task<Guid> SeedRoleAsync(string name)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var role = Role.Create(name, RoleType.Custom, [Permission.Create(Guid.Empty, ResourceType.Registry, PermissionLevel.Read)]);

        await uow.Roles.AddAsync(role, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        return role.Id;
    }

    private async Task<Guid> SeedDeploymentAsync(string name)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var platform = Fakes.GetDummyPlatform();
        platform.PartialUpdate(address: $"https://{Guid.CreateVersion7():N}.address");
        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);

        var deployment = new Deployment(name, Constants.SystemId, platform.Id);
        await uow.Deployments.AddAsync(deployment, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        return deployment.Id;
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
