using Application.Configs;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Domain.Entities.Automation;
using Domain.Entities.Tags;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Options;
using System.Runtime.CompilerServices;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Threading.Channels;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Automation;

public sealed class AutomationActionIntegrationTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly FakeAutomationProcessRunner processRunner = new();
    private readonly string workDir = Path.Combine(Path.GetTempPath(), $"citadel-automation-test-{Guid.NewGuid():N}");

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<IDbWorkQueue>();
        services.ReplaceService<IAutomationProcessRunner>(processRunner);
        services.AddSingleton<IDbWorkQueue, InlineDbWorkQueue>();
        services.Configure<AutomationOptions>(options =>
        {
            options.Enabled = true;
            options.DenoPath = Environment.ProcessPath ?? "dotnet";
            options.WorkDir = Path.Combine(workDir, "runs");
            options.DenoCacheDir = Path.Combine(workDir, "deno-cache");
            options.InternalBaseUrl = "http://127.0.0.1:8000";
            options.AllowNet = "127.0.0.1:8000";
            options.DefaultTimeoutSeconds = 30;
            options.MaxTimeoutSeconds = 300;
            options.MaxLogBytes = 4096;
        });
    }

    [Fact]
    public void EndpointCatalog_ShouldExposeAutomationSafeResourceEndpoints()
    {
        var catalog = Services.GetRequiredService<IAutomationApiEndpointCatalog>();
        using var document = JsonDocument.Parse(catalog.Json);
        var endpoints = document.RootElement.EnumerateArray().ToArray();

        Assert.Contains(endpoints, endpoint =>
            endpoint.GetProperty("key").GetString() == "listPlatforms"
            && endpoint.GetProperty("method").GetString() == "GET"
            && endpoint.GetProperty("path").GetString() == "/api/v1/platforms"
            && endpoint.GetProperty("group").GetString() == "platforms");
        Assert.Contains(endpoints, endpoint =>
            endpoint.GetProperty("key").GetString() == "listGitRepositories"
            && endpoint.GetProperty("group").GetString() == "gitRepositories");
        Assert.Contains(endpoints, endpoint =>
            endpoint.GetProperty("key").GetString() == "applyDeployment"
            && endpoint.GetProperty("method").GetString() == "POST");
        Assert.DoesNotContain(endpoints, endpoint => endpoint.GetProperty("key").GetString() == "login");
        Assert.DoesNotContain(endpoints, endpoint => endpoint.GetProperty("group").GetString() == "automationActions");
        Assert.DoesNotContain(endpoints, endpoint => endpoint.GetProperty("path").GetString()?.Contains("/exec", StringComparison.OrdinalIgnoreCase) == true);
    }

    [Fact]
    public async Task CreateAutomationAction_ShouldPersistActionAndActivity()
    {
        var response = await CreateActionAsync("action-create", "console.log('create');");
        var body = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        var actionId = ReadId(body);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var action = await uow.AutomationActions.GetAsync(actionId, TestContext.Current.CancellationToken);
        var activities = await GetActivitiesAsync(uow, actionId);

        Assert.NotNull(action);
        Assert.Equal("action-create", action.Name);
        Assert.Equal("automation test", action.Description);
        AssertJsonEqual("""{"source":"default"}""", action.DefaultArgsJson);
        Assert.Equal(Constants.DefaultAdminId, action.RunAsActorId);
        Assert.Contains(activities, activity => activity.EventType == ActivityEventType.ActionCreated);
    }

    [Fact]
    public async Task SeededDefaultAutomationActions_ShouldBeDisabledAndTagged()
    {
        var response = await Client.GetAsync(
            $"/api/v1/automation/actions?tags={Uri.EscapeDataString("Automation Examples")}",
            TestContext.Current.CancellationToken);
        var body = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        Assert.True(response.IsSuccessStatusCode, body);

        using var document = JsonDocument.Parse(body);
        var actions = document.RootElement.GetProperty("actions").EnumerateArray().ToArray();
        var prune = Assert.Single(actions, action => action.GetProperty("name").GetString() == "Daily unused image prune");
        var restart = Assert.Single(actions, action => action.GetProperty("name").GetString() == "Restart unhealthy Prod stacks");

        Assert.False(prune.GetProperty("enabled").GetBoolean());
        Assert.True(prune.GetProperty("scheduleEnabled").GetBoolean());
        Assert.Equal("0 12 * * *", prune.GetProperty("scheduleCron").GetString());
        AssertContainsTag(prune.GetProperty("tags"), "Automation Examples");

        Assert.False(restart.GetProperty("enabled").GetBoolean());
        Assert.True(restart.GetProperty("scheduleEnabled").GetBoolean());
        Assert.Equal("*/15 * * * *", restart.GetProperty("scheduleCron").GetString());
        AssertContainsTag(restart.GetProperty("tags"), "Automation Examples");
        AssertContainsTag(restart.GetProperty("tags"), "Prod");
    }

    [Fact]
    public async Task AutomationActionTags_ShouldCreateFilterByTagNameAndReplace()
    {
        Tag blueTag;
        Tag greenTag;

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            blueTag = await CreateTagAsync(uow, "automation-blue", "#3366FF");
            greenTag = await CreateTagAsync(uow, "automation-green", "#33AA66");
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var createResponse = await CreateActionAsync("action-tags", "console.log('tags');", tagIds: [blueTag.Id]);
        var createBody = await createResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        Assert.True(createResponse.IsSuccessStatusCode, createBody);
        var actionId = ReadId(createBody);

        using (var document = JsonDocument.Parse(createBody))
        {
            AssertContainsTag(document.RootElement.GetProperty("tags"), blueTag.Name);
        }

        var getTagsResponse = await Client.GetAsync($"/api/v1/automation/actions/{actionId}/tags", TestContext.Current.CancellationToken);
        getTagsResponse.EnsureSuccessStatusCode();

        using (var document = JsonDocument.Parse(await getTagsResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken)))
        {
            AssertContainsTag(document.RootElement.GetProperty("tags"), blueTag.Name);
        }

        var blueListResponse = await Client.GetAsync(
            $"/api/v1/automation/actions?tags={Uri.EscapeDataString(blueTag.Name)}",
            TestContext.Current.CancellationToken);
        blueListResponse.EnsureSuccessStatusCode();

        using (var document = JsonDocument.Parse(await blueListResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken)))
        {
            var action = Assert.Single(document.RootElement.GetProperty("actions").EnumerateArray());
            Assert.Equal(actionId, action.GetProperty("id").GetGuid());
            AssertContainsTag(action.GetProperty("tags"), blueTag.Name);
        }

        var replaceResponse = await Client.PutAsync(
            $"/api/v1/automation/actions/{actionId}/tags",
            JsonContent($$"""{ "tagIds": ["{{greenTag.Id}}"] }"""),
            TestContext.Current.CancellationToken);
        replaceResponse.EnsureSuccessStatusCode();

        var oldTagResponse = await Client.GetAsync(
            $"/api/v1/automation/actions?tags={Uri.EscapeDataString(blueTag.Name)}",
            TestContext.Current.CancellationToken);
        oldTagResponse.EnsureSuccessStatusCode();

        using (var document = JsonDocument.Parse(await oldTagResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken)))
        {
            Assert.Empty(document.RootElement.GetProperty("actions").EnumerateArray());
        }

        var newTagResponse = await Client.GetAsync(
            $"/api/v1/automation/actions?tags={Uri.EscapeDataString(greenTag.Name)}",
            TestContext.Current.CancellationToken);
        newTagResponse.EnsureSuccessStatusCode();

        using (var document = JsonDocument.Parse(await newTagResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken)))
        {
            var action = Assert.Single(document.RootElement.GetProperty("actions").EnumerateArray());
            Assert.Equal(actionId, action.GetProperty("id").GetGuid());
            AssertContainsTag(action.GetProperty("tags"), greenTag.Name);
        }
    }

    [Fact]
    public async Task RunAutomationAction_ShouldStreamProcessAndPersistSucceededRun()
    {
        var createResponse = await CreateActionAsync("action-run", "console.log('persisted');");
        var createBody = await createResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        createResponse.EnsureSuccessStatusCode();
        var actionId = ReadId(createBody);

        var runInput = """
        {
          "argsJson": "{\"mode\":\"manual\"}",
          "timeoutSeconds": 30
        }
        """;
        var runResponse = await Client.PostAsync(
            $"/api/v1/automation/actions/{actionId}/run",
            new StringContent(runInput, Encoding.UTF8, "application/json"),
            TestContext.Current.CancellationToken);
        var runBody = await runResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        Assert.True(runResponse.IsSuccessStatusCode, runBody);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var action = await uow.AutomationActions.GetAsync(actionId, TestContext.Current.CancellationToken);
        var run = await uow.ActionRuns.GetLatestByActionAsync(actionId, TestContext.Current.CancellationToken);
        var activities = await GetActivitiesAsync(uow, actionId);

        Assert.NotNull(action);
        Assert.NotNull(run);
        Assert.Equal(ResourceControlState.Idle, action.ControlState);
        Assert.Null(action.CurrentRunId);
        Assert.Equal(ActionRunTrigger.Manual, run.Trigger);
        Assert.Equal(ActionRunStatus.Succeeded, run.Status);
        Assert.Equal(Constants.DefaultAdminId, run.RunAsActorId);
        Assert.Equal(Constants.SystemId, run.TriggeredByActorId);
        AssertJsonEqual("""{"mode":"manual"}""", run.ArgsJson);
        Assert.Equal("console.log('persisted');", run.CodeSnapshot);
        Assert.Equal(0, run.ExitCode);
        Assert.Contains("automation stdout", run.Logs);
        Assert.Contains(activities, activity => activity.EventType == ActivityEventType.ActionRunStarted);
        Assert.Contains(activities, activity => activity.EventType == ActivityEventType.ActionRunSucceeded);

        var call = Assert.Single(processRunner.Calls);
        Assert.Equal(Environment.ProcessPath, call.FileName);
        Assert.Contains("run", call.Arguments);
        Assert.Contains("--allow-net=127.0.0.1:8000", call.Arguments);
        Assert.StartsWith(Path.Combine(workDir, "runs"), call.WorkingDirectory, StringComparison.OrdinalIgnoreCase);

        var runsResponse = await Client.GetAsync($"/api/v1/automation/actions/{actionId}/runs?limit=1", TestContext.Current.CancellationToken);
        var persistedRunResponse = await Client.GetAsync($"/api/v1/automation/actions/{actionId}/runs/{run.Id}", TestContext.Current.CancellationToken);
        var logsResponse = await Client.GetAsync($"/api/v1/automation/actions/{actionId}/runs/{run.Id}/logs", TestContext.Current.CancellationToken);

        runsResponse.EnsureSuccessStatusCode();
        persistedRunResponse.EnsureSuccessStatusCode();
        logsResponse.EnsureSuccessStatusCode();

        Assert.Contains(run.Id.ToString(), await runsResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
        Assert.Contains("console.log('persisted');", await persistedRunResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
        Assert.Contains("automation stdout", await logsResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
    }

    [Fact]
    public async Task UpdateAutomationAction_ShouldApplyMergePatchAndRecordActivity()
    {
        var createResponse = await CreateActionAsync("action-update", "console.log('old');");
        var actionId = ReadId(await createResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
        createResponse.EnsureSuccessStatusCode();

        var patchJson = """
        {
          "description": null,
          "code": "console.log('updated');",
          "defaultArgsJson": "{\"updated\":true}",
          "enabled": false,
          "scheduleEnabled": true,
          "scheduleCron": "*/5 * * * *",
          "scheduleTimeZone": "Europe/Paris",
          "webhook": {
            "enabled": true,
            "provider": "GitLab",
            "authScheme": "GitLabSignedToken",
            "secret": "secret-123",
            "branchFilter": "main"
          },
          "timeoutSeconds": 45,
          "alertOnFailure": false
        }
        """;

        var response = await Client.PatchAsync(
            $"/api/v1/automation/actions/{actionId}",
            MergePatchContent(patchJson),
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var action = await uow.AutomationActions.GetAsync(actionId, TestContext.Current.CancellationToken);
        var activities = await GetActivitiesAsync(uow, actionId, ActivityEventType.ActionUpdated);

        Assert.NotNull(action);
        Assert.Null(action.Description);
        Assert.Equal("console.log('updated');", action.Code);
        AssertJsonEqual("""{"updated":true}""", action.DefaultArgsJson);
        Assert.False(action.Enabled);
        Assert.True(action.ScheduleEnabled);
        Assert.Equal("*/5 * * * *", action.ScheduleCron);
        Assert.Equal("Europe/Paris", action.ScheduleTimeZone);
        Assert.True(action.Webhook?.Enabled);
        Assert.Equal(WebhookProvider.GitLab, action.Webhook?.Provider);
        Assert.Equal(WebhookAuthScheme.GitLabSignedToken, action.Webhook?.AuthScheme);
        Assert.Equal("secret-123", action.Webhook?.Secret);
        Assert.Equal("main", action.Webhook?.BranchFilter);
        Assert.Equal(45, action.TimeoutSeconds);
        Assert.False(action.AlertOnFailure);
        Assert.Single(activities);
    }

    [Fact]
    public async Task PatchAutomationActionMetadata_ShouldUpdateDescriptionWithoutActivity()
    {
        var createResponse = await CreateActionAsync("action-metadata", "console.log('metadata');");
        var actionId = ReadId(await createResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
        createResponse.EnsureSuccessStatusCode();

        var patchJson = """
        {
          "description": "metadata description"
        }
        """;

        var response = await Client.PatchAsync(
            $"/api/v1/automation/actions/{actionId}/_metadata",
            MergePatchContent(patchJson),
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var action = await uow.AutomationActions.GetAsync(actionId, TestContext.Current.CancellationToken);
        var activities = await GetActivitiesAsync(uow, actionId);

        Assert.Equal("metadata description", action?.Description);
        Assert.DoesNotContain(activities, activity => activity.EventType == ActivityEventType.ActionUpdated);
    }

    [Fact]
    public async Task RenameAutomationAction_ShouldPersistNameAndRecordActivity()
    {
        var createResponse = await CreateActionAsync("action-rename", "console.log('rename');");
        var actionId = ReadId(await createResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
        createResponse.EnsureSuccessStatusCode();

        var renameJson = $$"""
        {
          "id": "{{actionId}}",
          "name": "action-renamed"
        }
        """;

        var response = await Client.PostAsync(
            "/api/v1/automation/actions/rename",
            JsonContent(renameJson),
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var action = await uow.AutomationActions.GetAsync(actionId, TestContext.Current.CancellationToken);
        var activities = await GetActivitiesAsync(uow, actionId, ActivityEventType.ActionRenamed);

        Assert.Equal("action-renamed", action?.Name);
        Assert.Single(activities);
    }

    [Fact]
    public async Task DeleteAutomationAction_ShouldRemoveActionAndRecordActivity()
    {
        var createResponse = await CreateActionAsync("action-delete", "console.log('delete');");
        var actionId = ReadId(await createResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
        createResponse.EnsureSuccessStatusCode();

        var response = await Client.DeleteAsync($"/api/v1/automation/actions/{actionId}", TestContext.Current.CancellationToken);

        Assert.Equal(204, (int)response.StatusCode);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var action = await uow.AutomationActions.GetAsync(actionId, TestContext.Current.CancellationToken);
        var activities = await GetActivitiesAsync(uow, actionId, ActivityEventType.ActionDeleted);

        Assert.Null(action);
        Assert.Single(activities);
    }

    [Fact]
    public async Task TestAutomationAction_ShouldExecuteDraftCodeWithoutPersistingIt()
    {
        var createResponse = await CreateActionAsync("action-test", "console.log('persisted');");
        var actionId = ReadId(await createResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
        createResponse.EnsureSuccessStatusCode();

        var testJson = $$"""
        {
          "code": "console.log('draft');",
          "argsJson": "{\"draft\":true}",
          "defaultArgsJson": "{\"default\":true}",
          "timeoutSeconds": 30,
          "runAsActorId": "{{Constants.DefaultAdminId}}"
        }
        """;

        var response = await Client.PostAsync(
            $"/api/v1/automation/actions/{actionId}/test",
            JsonContent(testJson),
            TestContext.Current.CancellationToken);
        var body = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        Assert.True(response.IsSuccessStatusCode, body);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var action = await uow.AutomationActions.GetAsync(actionId, TestContext.Current.CancellationToken);
        var run = await uow.ActionRuns.GetLatestByActionAsync(actionId, TestContext.Current.CancellationToken);

        Assert.NotNull(action);
        Assert.NotNull(run);
        Assert.Equal("console.log('persisted');", action.Code);
        Assert.Equal(ActionRunTrigger.Test, run.Trigger);
        Assert.Equal("console.log('draft');", run.CodeSnapshot);
        AssertJsonEqual("""{"draft":true}""", run.ArgsJson);
        Assert.Equal(ActionRunStatus.Succeeded, run.Status);
    }

    [Fact]
    public async Task RunAutomationAction_WhenDisabled_ShouldReturnStreamErrorWithoutCreatingRun()
    {
        var createResponse = await CreateActionAsync("action-disabled", "console.log('disabled');", enabled: false);
        var actionId = ReadId(await createResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
        createResponse.EnsureSuccessStatusCode();

        var response = await Client.PostAsync(
            $"/api/v1/automation/actions/{actionId}/run",
            JsonContent("""{"argsJson":"{}","timeoutSeconds":30}"""),
            TestContext.Current.CancellationToken);
        var body = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        Assert.True(response.IsSuccessStatusCode, body);
        Assert.Contains("Automation action is disabled.", body);
        Assert.Empty(processRunner.Calls);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        Assert.Null(await uow.ActionRuns.GetLatestByActionAsync(actionId, TestContext.Current.CancellationToken));
    }

    [Fact]
    public async Task RunAutomationAction_WhenActiveRunExists_ShouldRejectAndRecordRejectedRun()
    {
        var createResponse = await CreateActionAsync("action-conflict", "console.log('conflict');");
        var actionId = ReadId(await createResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
        createResponse.EnsureSuccessStatusCode();
        await SeedQueuedRunAsync(actionId);

        var response = await Client.PostAsync(
            $"/api/v1/automation/actions/{actionId}/run",
            JsonContent("""{"argsJson":"{}","timeoutSeconds":30}"""),
            TestContext.Current.CancellationToken);
        var body = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        Assert.True(response.IsSuccessStatusCode, body);
        Assert.Contains("Another run for this action is already queued or running.", body);
        Assert.Empty(processRunner.Calls);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var runs = (await uow.ActionRuns.GetByActionAsync(actionId, 10, TestContext.Current.CancellationToken)).ToArray();
        var activities = await GetActivitiesAsync(uow, actionId, ActivityEventType.ActionRunRejected);

        Assert.Contains(runs, run => run.Status == ActionRunStatus.Queued);
        Assert.Contains(runs, run => run.Status == ActionRunStatus.Rejected);
        Assert.Single(activities);
    }

    [Fact]
    public async Task CancelAutomationActionRun_ShouldCancelQueuedRunAndRecordActivity()
    {
        var createResponse = await CreateActionAsync("action-cancel", "console.log('cancel');");
        var actionId = ReadId(await createResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
        createResponse.EnsureSuccessStatusCode();
        var runId = await SeedQueuedRunAsync(actionId);

        var response = await Client.PostAsync(
            $"/api/v1/automation/actions/{actionId}/runs/{runId}/cancel",
            content: null,
            TestContext.Current.CancellationToken);

        Assert.Equal(204, (int)response.StatusCode);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var run = await uow.ActionRuns.GetAsync(runId, TestContext.Current.CancellationToken);
        var activities = await GetActivitiesAsync(uow, actionId, ActivityEventType.ActionRunCancelled);

        Assert.Equal(ActionRunStatus.Cancelled, run?.Status);
        Assert.Equal("Run cancelled.", run?.ErrorMessage);
        Assert.Single(activities);
    }

    [Fact]
    public async Task RunAutomationAction_WhenProcessExitsNonZero_ShouldPersistFailure()
    {
        processRunner.ExitCode = 7;
        processRunner.StdErr = "deno failed";

        var createResponse = await CreateActionAsync("action-fail", "console.log('fail');");
        var actionId = ReadId(await createResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
        createResponse.EnsureSuccessStatusCode();

        var response = await Client.PostAsync(
            $"/api/v1/automation/actions/{actionId}/run",
            JsonContent("""{"argsJson":"{}","timeoutSeconds":30}"""),
            TestContext.Current.CancellationToken);
        var body = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        Assert.True(response.IsSuccessStatusCode, body);
        Assert.Contains("Deno exited with code 7.", body);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var run = await uow.ActionRuns.GetLatestByActionAsync(actionId, TestContext.Current.CancellationToken);
        var activities = await GetActivitiesAsync(uow, actionId, ActivityEventType.ActionRunFailed);

        Assert.NotNull(run);
        Assert.Equal(ActionRunStatus.Failed, run.Status);
        Assert.Equal(7, run.ExitCode);
        Assert.Equal("Deno exited with code 7.", run.ErrorMessage);
        Assert.Contains("[stderr] deno failed", run.Logs);
        Assert.Single(activities);
    }

    [Fact]
    public async Task AutomationActionRepository_ShouldFilterScheduledActionsAndMarkScheduledOnce()
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var scheduled = CreateDomainAction("repo-scheduled", enabled: true, scheduleEnabled: true, scheduleCron: "* * * * *");
        var disabled = CreateDomainAction("repo-disabled", enabled: false, scheduleEnabled: true, scheduleCron: "* * * * *");
        var unscheduled = CreateDomainAction("repo-unscheduled", enabled: true, scheduleEnabled: false, scheduleCron: "* * * * *");

        await uow.AutomationActions.AddAsync(scheduled, TestContext.Current.CancellationToken);
        await uow.AutomationActions.AddAsync(disabled, TestContext.Current.CancellationToken);
        await uow.AutomationActions.AddAsync(unscheduled, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        var scheduledActions = (await uow.AutomationActions.GetScheduledAsync(TestContext.Current.CancellationToken)).ToArray();
        var now = DateTime.UtcNow;
        var scheduledMinuteUtc = now.AddTicks(-(now.Ticks % TimeSpan.TicksPerMinute));

        Assert.Contains(scheduledActions, action => action.Id == scheduled.Id);
        Assert.DoesNotContain(scheduledActions, action => action.Id == disabled.Id);
        Assert.DoesNotContain(scheduledActions, action => action.Id == unscheduled.Id);

        Assert.True(await uow.AutomationActions.TryMarkScheduledAsync(scheduled.Id, scheduledMinuteUtc, TestContext.Current.CancellationToken));
        Assert.False(await uow.AutomationActions.TryMarkScheduledAsync(scheduled.Id, scheduledMinuteUtc, TestContext.Current.CancellationToken));
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        var persisted = await uow.AutomationActions.GetAsync(scheduled.Id, TestContext.Current.CancellationToken);
        Assert.Equal(scheduledMinuteUtc, persisted?.LastScheduledRunAt);
    }

    private Task<HttpResponseMessage> CreateActionAsync(
        string name,
        string code,
        bool enabled = true,
        IReadOnlyCollection<Guid>? tagIds = null)
    {
        var tagIdValues = tagIds is null ? null : string.Join(",", tagIds.Select(id => $"\"{id}\""));
        var tagIdsJson = tagIdValues is null ? string.Empty : $",\n  \"tagIds\": [{tagIdValues}]";
        var createJson = $$"""
        {
          "name": "{{name}}",
          "description": "automation test",
          "code": "{{code}}",
          "defaultArgsJson": "{\"source\":\"default\"}",
          "enabled": {{enabled.ToString().ToLowerInvariant()}},
          "scheduleEnabled": false,
          "scheduleCron": null,
          "scheduleTimeZone": "UTC",
          "webhook": null,
          "timeoutSeconds": 30,
          "alertOnFailure": true,
          "runAsActorId": "{{Constants.DefaultAdminId}}"{{tagIdsJson}}
        }
        """;

        return Client.PostAsync(
            "/api/v1/automation/actions",
            new StringContent(createJson, Encoding.UTF8, "application/json"),
            TestContext.Current.CancellationToken);
    }

    private async Task<Guid> SeedQueuedRunAsync(Guid actionId)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var action = await uow.AutomationActions.GetAsync(actionId, TestContext.Current.CancellationToken);
        Assert.NotNull(action);

        var run = new ActionRun(
            action.Id,
            action.Name,
            ActionRunTrigger.Manual,
            action.RunAsActorId,
            Constants.SystemId,
            "{}",
            action.Code,
            action.TimeoutSeconds);

        await uow.ActionRuns.AddAsync(run, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        return run.Id;
    }

    private static AutomationAction CreateDomainAction(
        string name,
        bool enabled,
        bool scheduleEnabled,
        string? scheduleCron)
        => new(
            name,
            description: null,
            code: "console.log('repository');",
            defaultArgsJson: "{}",
            enabled,
            scheduleEnabled,
            scheduleCron,
            scheduleTimeZone: "UTC",
            webhook: null,
            timeoutSeconds: 30,
            alertOnFailure: false,
            runAsActorId: Constants.DefaultAdminId,
            createdByActorId: Constants.SystemId);

    private static StringContent JsonContent(string json) => new(json, Encoding.UTF8, "application/json");

    private static StringContent MergePatchContent(string json) => new(json, Encoding.UTF8, "application/merge-patch+json");

    private static Guid ReadId(string json)
    {
        using var document = JsonDocument.Parse(json);
        return document.RootElement.GetProperty("id").GetGuid();
    }

    private static void AssertJsonEqual(string expectedJson, string? actualJson)
    {
        Assert.NotNull(actualJson);
        Assert.True(
            JsonNode.DeepEquals(JsonNode.Parse(expectedJson), JsonNode.Parse(actualJson)),
            actualJson);
    }

    private static async Task<Tag> CreateTagAsync(IUnitOfWork uow, string name, string color)
    {
        var tag = Tag.Create(name, color, Constants.SystemId);
        await uow.Tags.AddAsync(tag, TestContext.Current.CancellationToken);
        return tag;
    }

    private static void AssertContainsTag(JsonElement tags, string name)
    {
        Assert.Contains(
            tags.EnumerateArray(),
            tag => tag.GetProperty("name").GetString() == name);
    }

    private static async Task<IReadOnlyList<ActivityEvent>> GetActivitiesAsync(
        IUnitOfWork uow,
        Guid actionId,
        ActivityEventType? eventType = null)
    {
        var page = await uow.ActivityEventRepository.GetPagedAsync(
            actionId,
            ActivityResourceType.AutomationAction,
            eventType,
            page: 1,
            pageSize: 50,
            TestContext.Current.CancellationToken);

        return page.Items.ToArray();
    }

    private sealed class FakeAutomationProcessRunner : IAutomationProcessRunner
    {
        public List<ProcessCall> Calls { get; } = [];
        public string? StdOut { get; set; } = "automation stdout";
        public string? StdErr { get; set; }
        public int ExitCode { get; set; }

        public async IAsyncEnumerable<AutomationProcessOutput> StreamAsync(
            string fileName,
            IEnumerable<string> arguments,
            IDictionary<string, string>? environmentVariables,
            string workingDirectory,
            [EnumeratorCancellation] CancellationToken cancellationToken)
        {
            try
            {
                Calls.Add(new ProcessCall(
                    fileName,
                    [.. arguments],
                    environmentVariables is null ? null : new Dictionary<string, string>(environmentVariables),
                    workingDirectory));

                await Task.Yield();
                if (StdOut is not null)
                    yield return new AutomationProcessOutput(StdOut, null);

                if (StdErr is not null)
                    yield return new AutomationProcessOutput(null, StdErr);

                yield return new AutomationProcessOutput(null, null, ExitCode);
            }
            finally
            {
                TryDeleteDirectory(workingDirectory);
                if (environmentVariables?.TryGetValue("DENO_DIR", out var denoCacheDir) == true)
                    TryDeleteDirectory(denoCacheDir);
            }
        }

        private static void TryDeleteDirectory(string path)
        {
            if (Directory.Exists(path))
                Directory.Delete(path, recursive: true);
        }
    }

    private sealed record ProcessCall(
        string FileName,
        IReadOnlyList<string> Arguments,
        IReadOnlyDictionary<string, string>? EnvironmentVariables,
        string WorkingDirectory);

    private sealed class InlineDbWorkQueue(IServiceScopeFactory scopeFactory) : IDbWorkQueue
    {
        private readonly Channel<IDbWorkItem> channel = Channel.CreateUnbounded<IDbWorkItem>();

        public ChannelReader<IDbWorkItem> Reader => channel.Reader;

        public ValueTask EnqueueAsync(IDbWorkItem item, CancellationToken cancellationToken)
            => EnqueueAndWaitAsync(item, cancellationToken);

        public async ValueTask EnqueueAndWaitAsync(IDbWorkItem item, CancellationToken cancellationToken)
        {
            await using var scope = scopeFactory.CreateAsyncScope();
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await item.ExecuteAsync(uow, cancellationToken);
        }
    }
}
