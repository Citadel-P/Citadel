using System.Text;
using System.Text.Json;
using System.Net;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.Identity;

public class IdentityActivityEndpointTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task UserLifecycle_ShouldPersistSafeActivities()
    {
        const string password = "password12345678";
        var createResponse = await Client.PostAsync(
            "/api/v1/users",
            Json("""
                 {
                   "name": "activity-user",
                   "email": "activity-user@citadel.local",
                   "password": "password12345678"
                 }
                 """),
            TestContext.Current.CancellationToken);
        createResponse.EnsureSuccessStatusCode();
        var userId = await FindUserIdAsync("activity-user");

        var patchResponse = await Client.PatchAsync(
            $"/api/v1/users/{userId}",
            MergePatch("""
                       {
                         "email": "activity-user-updated@citadel.local",
                         "password": "updatedPassword12345"
                       }
                       """),
            TestContext.Current.CancellationToken);
        patchResponse.EnsureSuccessStatusCode();

        var renameResponse = await Client.PostAsync(
            "/api/v1/users/rename",
            Json($$"""{ "id": "{{userId}}", "name": "activity-user-renamed" }"""),
            TestContext.Current.CancellationToken);
        renameResponse.EnsureSuccessStatusCode();

        var deleteResponse = await Client.SendAsync(
            Delete("/api/v1/users", $$"""{ "ids": ["{{userId}}"] }"""),
            TestContext.Current.CancellationToken);
        deleteResponse.EnsureSuccessStatusCode();

        var activities = await GetActivitiesAsync(userId, ActivityResourceType.User);
        Assert.Equal(
            [
                ActivityEventType.UserCreated,
                ActivityEventType.UserUpdated,
                ActivityEventType.UserRenamed,
                ActivityEventType.UserDeleted
            ],
            activities.Select(static activity => activity.EventType).OrderBy(EventOrder));

        var serialized = string.Join(
            '\n',
            activities.Select(static activity => JsonSerializer.Serialize(
                activity.Info,
                EventInfoJsonContext.Default.ActivityEventInfo)));
        Assert.DoesNotContain(password, serialized, StringComparison.Ordinal);
        Assert.DoesNotContain("updatedPassword12345", serialized, StringComparison.Ordinal);
        Assert.DoesNotContain("passwordHash", serialized, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public async Task TeamLifecycle_ShouldPersistActivities()
    {
        var createResponse = await Client.PostAsync(
            "/api/v1/teams",
            Json("""{ "name": "activity-team" }"""),
            TestContext.Current.CancellationToken);
        createResponse.EnsureSuccessStatusCode();
        var teamId = await FindTeamIdAsync("activity-team");

        var patchResponse = await Client.PatchAsync(
            $"/api/v1/teams/{teamId}",
            MergePatch("""{ "isEnabled": false }"""),
            TestContext.Current.CancellationToken);
        patchResponse.EnsureSuccessStatusCode();

        var renameResponse = await Client.PostAsync(
            "/api/v1/teams/rename",
            Json($$"""{ "id": "{{teamId}}", "name": "activity-team-renamed" }"""),
            TestContext.Current.CancellationToken);
        renameResponse.EnsureSuccessStatusCode();

        var deleteResponse = await Client.SendAsync(
            Delete("/api/v1/teams", $$"""{ "ids": ["{{teamId}}"] }"""),
            TestContext.Current.CancellationToken);
        deleteResponse.EnsureSuccessStatusCode();

        var activities = await GetActivitiesAsync(teamId, ActivityResourceType.Team);
        Assert.Equal(
            [
                ActivityEventType.TeamCreated,
                ActivityEventType.TeamUpdated,
                ActivityEventType.TeamRenamed,
                ActivityEventType.TeamDeleted
            ],
            activities.Select(static activity => activity.EventType).OrderBy(EventOrder));
    }

    [Fact]
    public async Task RoleLifecycle_ShouldPersistActivities()
    {
        var createResponse = await Client.PostAsync(
            "/api/v1/roles",
            Json("""
                 {
                   "name": "activity-role",
                   "permissions": [
                     {
                       "resourceType": "Registry",
                       "permissionLevel": "Read",
                       "specificPermissions": []
                     }
                   ]
                 }
                 """),
            TestContext.Current.CancellationToken);
        createResponse.EnsureSuccessStatusCode();
        var roleId = await FindRoleIdAsync("activity-role");

        var patchResponse = await Client.PatchAsync(
            $"/api/v1/roles/{roleId}/permissions",
            MergePatch("""
                       {
                         "permissions": [
                           {
                             "resourceType": "Registry",
                             "permissionLevel": "Write",
                             "specificPermissions": []
                           }
                         ]
                       }
                       """),
            TestContext.Current.CancellationToken);
        patchResponse.EnsureSuccessStatusCode();

        var renameResponse = await Client.PostAsync(
            "/api/v1/roles/rename",
            Json($$"""{ "id": "{{roleId}}", "name": "activity-role-renamed" }"""),
            TestContext.Current.CancellationToken);
        renameResponse.EnsureSuccessStatusCode();

        var deleteResponse = await Client.SendAsync(
            Delete("/api/v1/roles", $$"""{ "ids": ["{{roleId}}"] }"""),
            TestContext.Current.CancellationToken);
        deleteResponse.EnsureSuccessStatusCode();

        var activities = await GetActivitiesAsync(roleId, ActivityResourceType.Role);
        Assert.Equal(
            [
                ActivityEventType.RoleCreated,
                ActivityEventType.RoleUpdated,
                ActivityEventType.RoleRenamed,
                ActivityEventType.RoleDeleted
            ],
            activities.Select(static activity => activity.EventType).OrderBy(EventOrder));
    }

    [Fact]
    public async Task AssignmentEndpoints_ShouldAuditSuccessfulChangesWithoutGhostEvents()
    {
        var createUserResponse = await Client.PostAsync(
            "/api/v1/users",
            Json("""
                 {
                   "name": "activity-assignment-user",
                   "email": "activity-assignment-user@citadel.local",
                   "password": "password12345678"
                 }
                 """),
            TestContext.Current.CancellationToken);
        createUserResponse.EnsureSuccessStatusCode();
        var userId = await FindUserIdAsync("activity-assignment-user");
        var userActorId = await FindUserActorIdAsync(userId);

        var addRoleResponse = await Client.PostAsync(
            $"/api/v1/users/{userId}/roles",
            Json($$"""{ "roleId": "{{ViewerRoleId}}" }"""),
            TestContext.Current.CancellationToken);
        addRoleResponse.EnsureSuccessStatusCode();

        var duplicateRoleResponse = await Client.PostAsync(
            $"/api/v1/users/{userId}/roles",
            Json($$"""{ "roleId": "{{ViewerRoleId}}" }"""),
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.Conflict, duplicateRoleResponse.StatusCode);

        var removeRoleResponse = await Client.DeleteAsync(
            $"/api/v1/users/{userId}/roles/{ViewerRoleId}",
            TestContext.Current.CancellationToken);
        removeRoleResponse.EnsureSuccessStatusCode();

        var userActivities = await GetActivitiesAsync(userId, ActivityResourceType.User);
        var userUpdates = userActivities
            .Where(activity => activity.EventType == ActivityEventType.UserUpdated)
            .Select(activity => Assert.IsType<UserUpdated>(activity.Info))
            .ToArray();
        Assert.Equal(2, userUpdates.Length);
        Assert.Contains(userUpdates, update => !update.OldUser.RoleIds.Contains(ViewerRoleId) && update.NewUser.RoleIds.Contains(ViewerRoleId));
        Assert.Contains(userUpdates, update => update.OldUser.RoleIds.Contains(ViewerRoleId) && !update.NewUser.RoleIds.Contains(ViewerRoleId));

        var createTeamResponse = await Client.PostAsync(
            "/api/v1/teams",
            Json("""{ "name": "activity-assignment-team" }"""),
            TestContext.Current.CancellationToken);
        createTeamResponse.EnsureSuccessStatusCode();
        var teamId = await FindTeamIdAsync("activity-assignment-team");

        var addMemberResponse = await Client.PostAsync(
            $"/api/v1/teams/{teamId}/members",
            Json($$"""{ "memberActorId": "{{userActorId}}" }"""),
            TestContext.Current.CancellationToken);
        addMemberResponse.EnsureSuccessStatusCode();

        var removeMemberResponse = await Client.DeleteAsync(
            $"/api/v1/teams/{teamId}/members/{userActorId}",
            TestContext.Current.CancellationToken);
        removeMemberResponse.EnsureSuccessStatusCode();

        var teamActivities = await GetActivitiesAsync(teamId, ActivityResourceType.Team);
        var teamUpdates = teamActivities
            .Where(activity => activity.EventType == ActivityEventType.TeamUpdated)
            .Select(activity => Assert.IsType<TeamUpdated>(activity.Info))
            .ToArray();
        Assert.Equal(2, teamUpdates.Length);
        Assert.Contains(teamUpdates, update => !update.OldTeam.MemberActorIds.Contains(userActorId) && update.NewTeam.MemberActorIds.Contains(userActorId));
        Assert.Contains(teamUpdates, update => update.OldTeam.MemberActorIds.Contains(userActorId) && !update.NewTeam.MemberActorIds.Contains(userActorId));
    }

    private async Task<Guid> FindUserIdAsync(string name)
    {
        await using var scope = Services.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return (await unitOfWork.Users.GetAllAsync(TestContext.Current.CancellationToken)).Single(user => user.Name == name).Id;
    }

    private async Task<Guid> FindUserActorIdAsync(Guid userId)
    {
        await using var scope = Services.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return (await unitOfWork.Users.GetAsync(userId, TestContext.Current.CancellationToken))!.ActorId;
    }

    private async Task<Guid> FindTeamIdAsync(string name)
    {
        await using var scope = Services.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return (await unitOfWork.Teams.GetAllAsync(TestContext.Current.CancellationToken)).Single(team => team.Name == name).Id;
    }

    private async Task<Guid> FindRoleIdAsync(string name)
    {
        await using var scope = Services.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return (await unitOfWork.Roles.GetAllAsync(TestContext.Current.CancellationToken)).Single(role => role.Name == name).Id;
    }

    private async Task<ActivityEvent[]> GetActivitiesAsync(Guid resourceId, ActivityResourceType resourceType)
    {
        await using var scope = Services.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var result = await unitOfWork.ActivityEventRepository.GetPagedAsync(
            resourceId,
            resourceType,
            eventType: null,
            page: 1,
            pageSize: 20,
            TestContext.Current.CancellationToken);
        return result.Items.ToArray();
    }

    private static int EventOrder(ActivityEventType eventType)
        => eventType switch
        {
            ActivityEventType.UserCreated or ActivityEventType.TeamCreated or ActivityEventType.RoleCreated => 0,
            ActivityEventType.UserUpdated or ActivityEventType.TeamUpdated or ActivityEventType.RoleUpdated => 1,
            ActivityEventType.UserRenamed or ActivityEventType.TeamRenamed or ActivityEventType.RoleRenamed => 2,
            _ => 3
        };

    private static StringContent Json(string value) => new(value, Encoding.UTF8, "application/json");

    private static StringContent MergePatch(string value) => new(value, Encoding.UTF8, "application/merge-patch+json");

    private static HttpRequestMessage Delete(string path, string body)
        => new(HttpMethod.Delete, path) { Content = Json(body) };
}
