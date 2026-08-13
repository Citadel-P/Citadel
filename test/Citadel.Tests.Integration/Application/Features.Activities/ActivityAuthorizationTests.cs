using System.Net;
using System.Net.Http.Headers;
using System.Text.Json;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Hosting.Common;

namespace Tests.Integration.Application.Features.Activities;

public sealed class ActivityAuthorizationTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private Guid visibleActivityId;
    private Guid sensitiveActivityId;
    private Guid platformId;
    private Guid serviceAccountId;
    private Guid serviceAccountActivityId;
    private Guid teamActivityId;
    private Guid roleActivityId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        platformId = Guid.CreateVersion7();
        var visibleActivity = new ActivityEvent(
            platformId: null,
            resourceId: platformId,
            actorId: Constants.SystemId,
            resourceName: "visible-platform",
            eventType: ActivityEventType.PlatformRenamed,
            status: ActivityStatus.Success,
            info: new PlatformRenamed("old", "new"));
        var sensitiveActivity = new ActivityEvent(
            platformId: null,
            resourceId: Guid.CreateVersion7(),
            actorId: Constants.SystemId,
            resourceName: "sensitive-user",
            eventType: ActivityEventType.UserProfileUpdated,
            status: ActivityStatus.Information,
            info: new UserProfileUpdated([]));
        serviceAccountId = Guid.CreateVersion7();
        var serviceAccountActivity = new ActivityEvent(
            platformId: null,
            resourceId: serviceAccountId,
            actorId: Constants.SystemId,
            resourceName: "build-runner",
            eventType: ActivityEventType.ServiceAccountRenamed,
            status: ActivityStatus.Success,
            info: new ServiceAccountRenamed("old-name", "build-runner"));
        var teamActivity = new ActivityEvent(
            platformId: null,
            resourceId: Guid.CreateVersion7(),
            actorId: Constants.SystemId,
            resourceName: "sensitive-team",
            eventType: ActivityEventType.TeamCreated,
            status: ActivityStatus.Success,
            info: new TeamCreated(new TeamActivitySnapshot(true, [], [], [])));
        var roleActivity = new ActivityEvent(
            platformId: null,
            resourceId: Guid.CreateVersion7(),
            actorId: Constants.SystemId,
            resourceName: "sensitive-role",
            eventType: ActivityEventType.RoleCreated,
            status: ActivityStatus.Success,
            info: new RoleCreated(new RoleActivitySnapshot(RoleType.Custom, [])));

        await uow.ActivityEventRepository.AddAsync(visibleActivity, TestContext.Current.CancellationToken);
        await uow.ActivityEventRepository.AddAsync(sensitiveActivity, TestContext.Current.CancellationToken);
        await uow.ActivityEventRepository.AddAsync(serviceAccountActivity, TestContext.Current.CancellationToken);
        await uow.ActivityEventRepository.AddAsync(teamActivity, TestContext.Current.CancellationToken);
        await uow.ActivityEventRepository.AddAsync(roleActivity, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        visibleActivityId = visibleActivity.Id;
        sensitiveActivityId = sensitiveActivity.Id;
        serviceAccountActivityId = serviceAccountActivity.Id;
        teamActivityId = teamActivity.Id;
        roleActivityId = roleActivity.Id;
    }

    [Fact]
    public async Task List_Returns_Only_Activities_For_Authorized_Resources()
    {
        await AuthenticatePlatformReaderAsync();

        var response = await Client.GetAsync(
            "/api/v1/activities?page=1&pageSize=10",
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        using var document = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);
        var pagedResult = document.RootElement.GetProperty("pagedResult");
        var items = pagedResult.GetProperty("items");

        Assert.Equal(1, pagedResult.GetProperty("totalCount").GetInt32());
        Assert.Equal(1, items.GetArrayLength());
        Assert.Equal(visibleActivityId, items[0].GetProperty("id").GetGuid());
    }

    [Fact]
    public async Task Get_Conceals_Administrator_Only_Activity_From_NonAdministrator()
    {
        await AuthenticatePlatformReaderAsync();

        var response = await Client.GetAsync(
            $"/api/v1/activities/{sensitiveActivityId}",
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task Get_Conceals_Team_And_Role_Activities_From_NonAdministrator()
    {
        await AuthenticatePlatformReaderAsync();

        foreach (var activityId in new[] { teamActivityId, roleActivityId })
        {
            var response = await Client.GetAsync(
                $"/api/v1/activities/{activityId}",
                TestContext.Current.CancellationToken);
            Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
        }
    }

    [Fact]
    public async Task Get_Returns_Activity_For_Authorized_Resource()
    {
        await AuthenticatePlatformReaderAsync();

        var response = await Client.GetAsync(
            $"/api/v1/activities/{visibleActivityId}",
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
    }

    [Fact]
    public async Task List_Returns_ServiceAccount_Activity_For_Authorized_Resource()
    {
        await AuthenticateServiceAccountReaderAsync();

        var response = await Client.GetAsync(
            $"/api/v1/activities?resourceId={serviceAccountId}&resourceType=ServiceAccount&page=1&pageSize=10",
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        using var document = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);
        var pagedResult = document.RootElement.GetProperty("pagedResult");
        var items = pagedResult.GetProperty("items");

        Assert.Equal(1, pagedResult.GetProperty("totalCount").GetInt32());
        Assert.Equal(serviceAccountActivityId, items[0].GetProperty("id").GetGuid());
    }

    [Fact]
    public async Task Get_Returns_ServiceAccount_Activity_For_Authorized_Resource()
    {
        await AuthenticateServiceAccountReaderAsync();

        var response = await Client.GetAsync(
            $"/api/v1/activities/{serviceAccountActivityId}",
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
    }

    private async Task AuthenticatePlatformReaderAsync()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.Platform, platformId, PermissionLevel.Read)
            ]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));
    }

    private async Task AuthenticateServiceAccountReaderAsync()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.ServiceAccount, serviceAccountId, PermissionLevel.Read)
            ]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));
    }
}
