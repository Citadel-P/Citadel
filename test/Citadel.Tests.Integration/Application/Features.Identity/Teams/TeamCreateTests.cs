using System.Text;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Domain.Entities.Identity;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Identity.Teams;

public class TeamCreateTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
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
    public async Task Create_Team_With_Assignments_Should_Assign_Users_Roles_And_Resource_Accesses()
    {
        var userId = await SeedUserAsync("team-create-member", "team-create-member@citadel.local");
        var roleId = await SeedRoleAsync("team-create-role");
        var deploymentVisible = await SeedDeploymentAsync("deployment-visible-create-team");
        var deploymentHidden = await SeedDeploymentAsync("deployment-hidden-create-team");

        var createJson = $$"""
        {
          "name": "team-with-assignments",
          "userIds": ["{{userId}}"],
          "roleIds": ["{{roleId}}"],
          "resourceAccesses": [
            {
              "resourceType": "Deployment",
              "resourceId": "{{deploymentVisible}}",
              "permissionLevel": "Read"
            }
          ]
        }
        """;

        var response = await Client.PostAsync(
            "/api/v1/teams",
            new StringContent(createJson, Encoding.UTF8, "application/json"),
            cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var team = (await uow.Teams.GetAllAsync(TestContext.Current.CancellationToken)).Single(x => x.Name == "team-with-assignments");

        var memberIds = (await uow.Teams.GetUserIdsAsync(team.Id, TestContext.Current.CancellationToken)).ToArray();
        var roleIds = (await uow.Roles.GetActorRoleIdsAsync(team.ActorId, TestContext.Current.CancellationToken)).ToArray();

        Assert.Contains(userId, memberIds);
        Assert.Contains(roleId, roleIds);

        var user = await uow.Users.GetAsync(userId, TestContext.Current.CancellationToken);
        Assert.NotNull(user);

        var canViewVisible = await uow.Users.HasPermissionAsync(
            userId,
            ResourceType.Deployment,
            PermissionLevel.Read,
            SpecificPermission.None,
            deploymentVisible,
            TestContext.Current.CancellationToken);

        var canViewHidden = await uow.Users.HasPermissionAsync(
            userId,
            ResourceType.Deployment,
            PermissionLevel.Read,
            SpecificPermission.None,
            deploymentHidden,
            TestContext.Current.CancellationToken);

        var hasGlobalView = await uow.Users.HasPermissionAsync(
            userId,
            ResourceType.Deployment,
            PermissionLevel.Read,
            SpecificPermission.None,
            null,
            TestContext.Current.CancellationToken);

        Assert.True(canViewVisible);
        Assert.False(canViewHidden);
        Assert.False(hasGlobalView);
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

    private async Task<Guid> SeedUserAsync(string name, string email)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var actor = Actor.Create(ActorType.User, new ActorMetadata(name));
        var user = new User(name, email, "password123", actor.Id, Constants.SystemId);

        await uow.Actors.AddAsync(actor, TestContext.Current.CancellationToken);
        await uow.Users.AddAsync(user, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        return user.Id;
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
}
