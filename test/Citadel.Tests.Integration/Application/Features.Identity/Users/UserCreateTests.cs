using System.Text;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Domain.Entities.Identity;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Tests.Integration.Helpers;

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
    public async Task Create_User_With_Assignments_Should_Assign_Teams_Roles_And_Resource_Accesses()
    {
        var teamId = await SeedTeamAsync("team-for-create-user");
        var roleId = await SeedRoleAsync("role-for-create-user");
        var deploymentVisible = await SeedDeploymentAsync("deployment-visible-create-user");
        var deploymentHidden = await SeedDeploymentAsync("deployment-hidden-create-user");

        var createJson = $$"""
        {
          "name": "user-with-assignments",
          "email": "user-with-assignments@citadel.local",
          "password": "password123",
          "teamIds": ["{{teamId}}"],
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
            "/api/v1/users",
            new StringContent(createJson, Encoding.UTF8, "application/json"),
            cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var user = (await uow.Users.GetAllAsync(TestContext.Current.CancellationToken)).Single(x => x.Name == "user-with-assignments");

        var assignedTeamIds = (await uow.Users.GetTeamIdsAsync(user.Id, TestContext.Current.CancellationToken)).ToArray();
        var assignedRoleIds = (await uow.Roles.GetActorRoleIdsAsync(user.ActorId, TestContext.Current.CancellationToken)).ToArray();

        Assert.Contains(teamId, assignedTeamIds);
        Assert.Contains(roleId, assignedRoleIds);

        var canViewVisible = await uow.Users.HasPermissionAsync(
            user.Id,
            ResourceType.Deployment,
            PermissionLevel.Read,
            SpecificPermission.None,
            deploymentVisible,
            TestContext.Current.CancellationToken);

        var canViewHidden = await uow.Users.HasPermissionAsync(
            user.Id,
            ResourceType.Deployment,
            PermissionLevel.Read,
            SpecificPermission.None,
            deploymentHidden,
            TestContext.Current.CancellationToken);

        var hasGlobalView = await uow.Users.HasPermissionAsync(
            user.Id,
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
}
