using System.Net;
using System.Net.Http.Json;
using System.Text.Json;
using Application.Services.Identity;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.Identity.Setup;

public sealed class SetupEndpointTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    protected override bool SeedDefaultAdministrator => false;

    [Fact]
    public async Task Pending_Instance_Should_Expose_Status_And_Gate_Normal_Api()
    {
        var status = await Client.GetAsync(
            "/api/v1/setup/status",
            TestContext.Current.CancellationToken);
        var gated = await Client.GetAsync(
            "/api/v1/application/info",
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.OK, status.StatusCode);
        Assert.Equal("no-store", status.Headers.CacheControl?.ToString());
        Assert.True((await ReadJsonAsync(status)).GetProperty("requiresSetup").GetBoolean());

        Assert.Equal(HttpStatusCode.Conflict, gated.StatusCode);
        Assert.Equal("setup_required", (await ReadJsonAsync(gated)).GetProperty("type").GetString());
    }

    [Fact]
    public async Task Pending_Instance_Should_Not_Accept_Former_Default_Credentials()
    {
        var response = await Client.PostAsJsonAsync(
            "/api/v1/authentication/login",
            new
            {
                emailOrName = "admin@citadel.local",
                password = "admin123"
            },
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.Conflict, response.StatusCode);
        Assert.Equal(
            "setup_required",
            (await ReadJsonAsync(response)).GetProperty("type").GetString());
    }

    [Fact]
    public async Task Initialize_Should_Create_Admin_Complete_Setup_And_Start_Session()
    {
        var response = await InitializeAsync("owner", "owner@example.test");

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        var body = await ReadJsonAsync(response);
        Assert.Equal("Completed", body.GetProperty("nextStep").GetString());
        Assert.False(string.IsNullOrWhiteSpace(body.GetProperty("accessToken").GetString()));

        await using var scope = Services.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var user = Assert.Single(await unitOfWork.Users.GetAllAsync(TestContext.Current.CancellationToken));
        var roleIds = await unitOfWork.Roles.GetActorRoleIdsAsync(
            user.ActorId,
            TestContext.Current.CancellationToken);
        var actor = await unitOfWork.Actors.GetById(
            user.ActorId,
            TestContext.Current.CancellationToken);
        var setupState = await unitOfWork.InstanceSetupState.GetAsync(TestContext.Current.CancellationToken);
        var activities = await unitOfWork.ActivityEventRepository.GetPagedAsync(
            user.Id,
            ActivityResourceType.User,
            ActivityEventType.InitialAdministratorCreated,
            1,
            10,
            TestContext.Current.CancellationToken);
        var automationActions = (await unitOfWork.AutomationActions.GetAllAsync(
            TestContext.Current.CancellationToken)).ToList();

        Assert.Equal("owner", user.Name);
        Assert.Equal(Constants.SystemId, user.CreatedByActorId);
        Assert.True(actor?.IsEnabled);
        Assert.Contains(
            Guid.Parse("30000000-0000-0000-0000-000000000001"),
            roleIds);
        Assert.False(setupState?.RequiresSetup);
        Assert.Equal(user.ActorId, setupState?.InitialAdministratorActorId);
        Assert.False(
            Services.GetRequiredService<ISetupStateCache>()
                .TryGetRequiresSetup(out var requiresSetup)
            && requiresSetup);
        var activity = Assert.Single(activities.Items);
        Assert.Equal(Constants.SystemId, activity.CreatedByActorId);
        Assert.IsType<InitialAdministratorCreated>(activity.Info);

        Assert.Collection(
            automationActions.OrderBy(action => action.Name),
            action =>
            {
                Assert.Equal("Prune images", action.Name);
                Assert.Equal(user.ActorId, action.RunAsActorId);
                Assert.Equal(Constants.SystemId, action.CreatedByActorId);
                Assert.False(action.Enabled);
                Assert.False(action.ScheduleEnabled);
                Assert.Equal("0 12 * * *", action.ScheduleCron);
                Assert.Collection(
                    action.Tags,
                    tag => Assert.Equal("System", tag.Name));
            },
            action =>
            {
                Assert.Equal("Restart unhealthy stacks", action.Name);
                Assert.Equal(user.ActorId, action.RunAsActorId);
                Assert.Equal(Constants.SystemId, action.CreatedByActorId);
                Assert.False(action.Enabled);
                Assert.False(action.ScheduleEnabled);
                Assert.Equal("*/15 * * * *", action.ScheduleCron);
                Assert.Equal(
                    ["Prod", "System"],
                    action.Tags.Select(tag => tag.Name).Order());
            });
    }

    [Fact]
    public async Task Initialize_Should_Reject_Subsequent_Request()
    {
        var first = await InitializeAsync("owner", "owner@example.test");
        var second = await InitializeAsync("other-owner", "other-owner@example.test");

        Assert.Equal(HttpStatusCode.OK, first.StatusCode);
        Assert.Equal(HttpStatusCode.Conflict, second.StatusCode);
        Assert.Equal(
            "setup_already_complete",
            (await ReadJsonAsync(second)).GetProperty("type").GetString());
    }

    [Fact]
    public async Task Deleting_Initial_User_Should_Not_Reopen_Setup()
    {
        var initialize = await InitializeAsync("owner", "owner@example.test");
        Assert.Equal(HttpStatusCode.OK, initialize.StatusCode);

        await using (var scope = Services.CreateAsyncScope())
        {
            var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var user = Assert.Single(
                await unitOfWork.Users.GetAllAsync(TestContext.Current.CancellationToken));
            await unitOfWork.Users.RemoveRangeAsync(
                [user.Id],
                TestContext.Current.CancellationToken);
            await unitOfWork.CommitAsync(TestContext.Current.CancellationToken);
        }

        Services.GetRequiredService<ISetupStateCache>().Reset();
        var status = await Client.GetAsync(
            "/api/v1/setup/status",
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.OK, status.StatusCode);
        Assert.False((await ReadJsonAsync(status)).GetProperty("requiresSetup").GetBoolean());
    }

    [Fact]
    public async Task Concurrent_Initialization_Should_Create_Exactly_One_Admin()
    {
        var firstTask = InitializeAsync("first-owner", "first-owner@example.test");
        var secondTask = InitializeAsync("second-owner", "second-owner@example.test");

        var responses = await Task.WhenAll(firstTask, secondTask);

        Assert.Single(responses, response => response.StatusCode == HttpStatusCode.OK);
        Assert.Single(responses, response => response.StatusCode == HttpStatusCode.Conflict);

        await using var scope = Services.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        Assert.Single(await unitOfWork.Users.GetAllAsync(TestContext.Current.CancellationToken));
        Assert.Equal(
            2,
            (await unitOfWork.AutomationActions.GetAllAsync(
                TestContext.Current.CancellationToken)).Count());
    }

    private Task<HttpResponseMessage> InitializeAsync(string name, string email)
        => Client.PostAsJsonAsync(
            "/api/v1/setup/initialize",
            new
            {
                name,
                email,
                password = "correct-horse-battery-staple"
            },
            TestContext.Current.CancellationToken);

    private static async Task<JsonElement> ReadJsonAsync(HttpResponseMessage response)
    {
        await using var stream = await response.Content.ReadAsStreamAsync(
            TestContext.Current.CancellationToken);
        using var document = await JsonDocument.ParseAsync(
            stream,
            cancellationToken: TestContext.Current.CancellationToken);
        return document.RootElement.Clone();
    }
}
