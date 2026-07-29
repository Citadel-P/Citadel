using System.Text;
using System.Text.Json;
using Application.Services.Identity;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Domain.Entities.Identity;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Identity.Users;

public class UserPatchTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task Patch_User_Should_Update_Email_Password_And_IsEnabled()
    {
        var seeded = await SeedUserAsync("user-patch", "user-patch@citadel.local");
        var patchJson = """
        {
          "email": "user-patched@citadel.local",
          "password": "newPassword12345",
          "isEnabled": false
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/users/{seeded.UserId}", content, cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var user = await uow.Users.GetAsync(seeded.UserId, TestContext.Current.CancellationToken);
        var actor = await uow.Actors.GetById(seeded.ActorId, TestContext.Current.CancellationToken);

        Assert.NotNull(user);
        Assert.Equal("user-patched@citadel.local", user!.Email);
        var passwordHasher = scope.ServiceProvider.GetRequiredService<ICitadelPasswordHasher>();
        Assert.True(passwordHasher.Verify("newPassword12345", user.Password));
        Assert.NotNull(actor);
        Assert.False(actor!.IsEnabled);
    }

    [Fact]
    public async Task Rename_User_Should_Update_Name()
    {
        var seeded = await SeedUserAsync("user-rename", "user-rename@citadel.local");
        var renameJson = $$"""
        {
          "id": "{{seeded.UserId}}",
          "name": "user-renamed"
        }
        """;
        var content = new StringContent(renameJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/users/rename", content, TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var user = await uow.Users.GetAsync(seeded.UserId, TestContext.Current.CancellationToken);

        Assert.Equal("user-renamed", user?.Name);
    }

    [Fact]
    public async Task Add_User_Role_Should_Assign_Role()
    {
        var seeded = await SeedUserAsync("user-role-add", "user-role-add@citadel.local");
        var addRoleJson = $$"""
        {
          "roleId": "{{ViewerRoleId}}"
        }
        """;
        var content = new StringContent(addRoleJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync($"/api/v1/users/{seeded.UserId}/roles", content, TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var roleIds = await uow.Roles.GetActorRoleIdsAsync(seeded.ActorId, TestContext.Current.CancellationToken);

        Assert.Contains(ViewerRoleId, roleIds);
    }

    [Fact]
    public async Task Remove_User_Role_Should_Unassign_Role()
    {
        var seeded = await SeedUserAsync("user-role-remove", "user-role-remove@citadel.local");

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Roles.AddActorRoleAsync(seeded.ActorId, ViewerRoleId, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var response = await Client.DeleteAsync($"/api/v1/users/{seeded.UserId}/roles/{ViewerRoleId}", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var verificationScope = Services.CreateAsyncScope();
        var verificationUow = verificationScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var roleIds = await verificationUow.Roles.GetActorRoleIdsAsync(seeded.ActorId, TestContext.Current.CancellationToken);

        Assert.DoesNotContain(ViewerRoleId, roleIds);
    }

    [Fact]
    public async Task Add_User_Resource_Access_Should_Allow_Viewing_Only_Granted_Deployment()
    {
        var deploymentId = await SeedDeploymentAsync("deployment-visible-by-user-access");
        await SeedDeploymentAsync("deployment-hidden-by-user-access");

        var seeded = await SeedUserAsync("user-resource-access-add", "user-resource-access-add@citadel.local");
        var payload = $$"""
        {
          "resourceType": "Deployment",
          "resourceId": "{{deploymentId}}",
          "permissionLevel": "Read"
        }
        """;

        var addResponse = await Client.PostAsync(
            $"/api/v1/users/{seeded.UserId}/resource-accesses",
            new StringContent(payload, Encoding.UTF8, "application/json"),
            TestContext.Current.CancellationToken);
        addResponse.EnsureSuccessStatusCode();

        Client.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(seeded.UserId, seeded.ActorId));

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
    public async Task Remove_User_Resource_Access_Should_Revoke_Granted_Deployment_View()
    {
        var deploymentId = await SeedDeploymentAsync("deployment-revoked-user-access");
        var seeded = await SeedUserAsync("user-resource-access-remove", "user-resource-access-remove@citadel.local");
        var payload = $$"""
        {
          "resourceType": "Deployment",
          "resourceId": "{{deploymentId}}",
          "permissionLevel": "Read"
        }
        """;

        var addResponse = await Client.PostAsync(
            $"/api/v1/users/{seeded.UserId}/resource-accesses",
            new StringContent(payload, Encoding.UTF8, "application/json"),
            TestContext.Current.CancellationToken);
        addResponse.EnsureSuccessStatusCode();

        var removeRequest = new HttpRequestMessage(HttpMethod.Delete, $"/api/v1/users/{seeded.UserId}/resource-accesses")
        {
            Content = new StringContent(payload, Encoding.UTF8, "application/json")
        };

        var removeResponse = await Client.SendAsync(removeRequest, TestContext.Current.CancellationToken);
        removeResponse.EnsureSuccessStatusCode();

        Client.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(seeded.UserId, seeded.ActorId));

        var listResponse = await Client.GetAsync("/api/v1/deployments", TestContext.Current.CancellationToken);
        listResponse.EnsureSuccessStatusCode();

        using var doc = await JsonDocument.ParseAsync(
            await listResponse.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var deployments = doc.RootElement.GetProperty("deployments");
        Assert.Equal(0, deployments.GetArrayLength());
    }

    [Fact]
    public async Task Patch_User_Should_Replace_Teams_Roles_And_Resource_Accesses()
    {
        var seeded = await SeedUserAsync("user-patch-assignments", "user-patch-assignments@citadel.local");

        var oldTeamId = await SeedTeamAsync("user-patch-old-team");
        var newTeamId = await SeedTeamAsync("user-patch-new-team");
        var oldRoleId = await SeedRoleAsync("user-patch-old-role");
        var newRoleId = await SeedRoleAsync("user-patch-new-role");
        var oldDeploymentId = await SeedDeploymentAsync("deployment-old-patch-user");
        var newDeploymentId = await SeedDeploymentAsync("deployment-new-patch-user");

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Users.ReplaceTeamsAsync(seeded.UserId, [oldTeamId], TestContext.Current.CancellationToken);
            await uow.Roles.ReplaceActorRolesAsync(seeded.ActorId, [oldRoleId], TestContext.Current.CancellationToken);
            await uow.ResourceAccesses.ReplaceAsync(
                seeded.ActorId,
                [ResourceAccess.Create(ResourceType.Deployment, oldDeploymentId, seeded.ActorId, PermissionLevel.Read)],
                TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var patchJson = $$"""
        {
          "teamIds": ["{{newTeamId}}"],
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
            $"/api/v1/users/{seeded.UserId}",
            new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json"),
            cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var verificationScope = Services.CreateAsyncScope();
        var verificationUow = verificationScope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var teamIds = (await verificationUow.Users.GetTeamIdsAsync(seeded.UserId, TestContext.Current.CancellationToken)).ToArray();
        var roleIds = (await verificationUow.Roles.GetActorRoleIdsAsync(seeded.ActorId, TestContext.Current.CancellationToken)).ToArray();

        Assert.Single(teamIds);
        Assert.Contains(newTeamId, teamIds);
        Assert.DoesNotContain(oldTeamId, teamIds);

        Assert.Single(roleIds);
        Assert.Contains(newRoleId, roleIds);
        Assert.DoesNotContain(oldRoleId, roleIds);

        var actorIds = await verificationUow.Users.GetActorScopeAsync(seeded.UserId, TestContext.Current.CancellationToken);

        var oldMetadata = await verificationUow.Users.GetEffectivePermissionsAsync(
            actorIds,
            ResourceType.Deployment,
            oldDeploymentId,
            TestContext.Current.CancellationToken);

        var newMetadata = await verificationUow.Users.GetEffectivePermissionsAsync(
            actorIds,
            ResourceType.Deployment,
            newDeploymentId,
            TestContext.Current.CancellationToken);

        Assert.False(oldMetadata.Has(PermissionLevel.Read, SpecificPermission.None));
        Assert.True(newMetadata.Has(PermissionLevel.Read, SpecificPermission.None));
    }

    [Fact]
    public async Task Patch_User_With_Empty_Assignment_Collections_Should_Clear_Teams_Roles_And_Resource_Accesses()
    {
        var seeded = await SeedUserAsync("user-patch-clear-assignments", "user-patch-clear-assignments@citadel.local");

        var teamId = await SeedTeamAsync("user-patch-clear-team");
        var roleId = await SeedRoleAsync("user-patch-clear-role");
        var deploymentId = await SeedDeploymentAsync("deployment-clear-patch-user");

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Users.ReplaceTeamsAsync(seeded.UserId, [teamId], TestContext.Current.CancellationToken);
            await uow.Roles.ReplaceActorRolesAsync(seeded.ActorId, [roleId], TestContext.Current.CancellationToken);
            await uow.ResourceAccesses.ReplaceAsync(
                seeded.ActorId,
                [ResourceAccess.Create(ResourceType.Deployment, deploymentId, seeded.ActorId, PermissionLevel.Read)],
                TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var patchJson = """
        {
          "teamIds": [],
          "roleIds": [],
          "resourceAccesses": []
        }
        """;

        var response = await Client.PatchAsync(
            $"/api/v1/users/{seeded.UserId}",
            new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json"),
            cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var verificationScope = Services.CreateAsyncScope();
        var verificationUow = verificationScope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var teamIds = (await verificationUow.Users.GetTeamIdsAsync(seeded.UserId, TestContext.Current.CancellationToken)).ToArray();
        var roleIds = (await verificationUow.Roles.GetActorRoleIdsAsync(seeded.ActorId, TestContext.Current.CancellationToken)).ToArray();

        Assert.Empty(teamIds);
        Assert.Empty(roleIds);

        var actorIds2 = await verificationUow.Users.GetActorScopeAsync(seeded.UserId, TestContext.Current.CancellationToken);

        var metadata = await verificationUow.Users.GetEffectivePermissionsAsync(
            actorIds2,
            ResourceType.Deployment,
            deploymentId,
            TestContext.Current.CancellationToken);

        Assert.False(metadata.Has(PermissionLevel.Read, SpecificPermission.None));
    }

    private async Task<Guid> SeedTeamAsync(string name)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var actor = Actor.Create(ActorType.Team, new ActorMetadata(name));
        var team = Team.Create(name, actor.Id);

        await uow.Actors.AddAsync(actor, TestContext.Current.CancellationToken);
        await uow.Teams.AddAsync(team, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        return team.Id;
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
}
