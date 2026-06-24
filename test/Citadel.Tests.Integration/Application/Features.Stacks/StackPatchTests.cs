using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Domain.Entities.Platforms;
using Domain.Entities.Stacks;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Net;
using System.Text;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Stacks;

public class StackPatchTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private Guid stackId;
    private Guid platformId;
    private Guid otherPlatformId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();
        var otherPlatform = new Platform(
            name: "Docker-P-02",
            address: "https://other.address",
            networkCount: 1,
            volumeCount: 2,
            imageCount: 3,
            cpuCount: 4,
            memTotal: 500,
            serverVersion: "1.0.0",
            agentVersion: "1.0.0",
            status: PlatformStatus.Online,
            connectorType: PlatformConnectorType.Agent,
            platformDescriptor: new DockerPlatformDescriptor(
                DaemonId: "654321",
                ContainerCount: 5,
                ContainersRunning: 2,
                ContainersPaused: 2,
                ContainersStopped: 1));

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Platforms.AddAsync(otherPlatform, TestContext.Current.CancellationToken);

        var stack = Stack.Create(
            "stack-1",
            Constants.SystemId,
            StackSource.WebEditor,
            platform.Id,
            new ManualStack("docker-compose.yml", StackUpdateBehavior.ServiceAutoDeploy),
            description: "original-description");
        stack.ReleaseProcessing(StackReleaseStatus.Healthy);

        await uow.Stacks.AddAsync(stack, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        stackId = stack.Id;
        platformId = platform.Id;
        otherPlatformId = otherPlatform.Id;
    }

    [Fact]
    public async Task Patch_Stack_Should_Update_Current_Release_Definition_Without_Creating_New_Release()
    {
        var patchJson = $$"""
        {
          "platformId": "{{otherPlatformId}}",
          "spec": {
            "$type": "WebEditor",
            "composeFile": "compose.updated.yml"
          }
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/stacks/{stackId}", content, cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stack = await uow.Stacks.GetAsync(stackId, TestContext.Current.CancellationToken);
        var releases = (await uow.Stacks.GetReleasesByStackIdAsync(stackId, TestContext.Current.CancellationToken)).ToList();

        Assert.NotNull(stack);
        Assert.Equal("stack-1", stack.Name);
        Assert.Equal("original-description", stack.Description);
        Assert.Single(releases);
        Assert.Equal("1", stack.CurrentStackRelease!.Version);
        Assert.Equal(StackReleaseStatus.Healthy, stack.CurrentStackRelease.Status);
        Assert.Equal(otherPlatformId, stack.CurrentStackRelease.PlatformId);
        Assert.Equal(stack.CurrentStackReleaseId, stack.CurrentStackRelease.Id);
        Assert.Equal("compose.updated.yml", Assert.IsType<ManualStack>(stack.CurrentStackRelease.Spec).ComposeFile);
    }

    [Fact]
    public async Task Patch_Stack_DriftPolicy_Should_Not_Create_New_Release_Or_Reset_Status()
    {
        var patchJson = """
        {
          "driftPolicy": {
            "mode": "AutoFix",
            "alertOnDrift": true,
            "markDegraded": true,
            "autoStartStoppedContainers": true,
            "autoResumePausedContainers": false,
            "removeExtraContainers": false
          }
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/stacks/{stackId}", content, cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stack = await uow.Stacks.GetAsync(stackId, TestContext.Current.CancellationToken);
        var releases = (await uow.Stacks.GetReleasesByStackIdAsync(stackId, TestContext.Current.CancellationToken)).ToList();

        Assert.NotNull(stack);
        Assert.Single(releases);
        Assert.Equal("1", stack.CurrentStackRelease!.Version);
        Assert.Equal(StackReleaseStatus.Healthy, stack.CurrentStackRelease.Status);
        Assert.Equal(StackDriftMode.AutoFix, stack.DriftPolicy.Mode);
        Assert.True(stack.DriftPolicy.AutoStartStoppedContainers);
    }

    [Fact]
    public async Task Patch_Stack_With_Partial_Spec_Should_Not_Require_Type_Discriminator()
    {
        var patchJson = """
        {
          "spec": {
            "envFilePath": ".env"
          }
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/stacks/{stackId}", content, cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stack = await uow.Stacks.GetAsync(stackId, TestContext.Current.CancellationToken);
        var releases = (await uow.Stacks.GetReleasesByStackIdAsync(stackId, TestContext.Current.CancellationToken)).ToList();

        Assert.NotNull(stack);
        Assert.Single(releases);
        Assert.Equal("1", stack.CurrentStackRelease!.Version);
        Assert.Equal(StackReleaseStatus.Healthy, stack.CurrentStackRelease.Status);
        Assert.Equal(".env", Assert.IsType<ManualStack>(stack.CurrentStackRelease.Spec).EnvFilePath);
    }

    [Fact]
    public async Task Patch_Stack_Should_Record_Distinct_Old_And_New_Snapshots_In_Update_Activity()
    {
        var patchJson = $$"""
        {
          "platformId": "{{otherPlatformId}}",
          "spec": {
            "$type": "WebEditor",
            "composeFile": "compose.updated.yml"
          }
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/stacks/{stackId}", content, cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var activities = await uow.ActivityEventRepository.GetPagedAsync(
            stackId,
            ActivityResourceType.Stack,
            ActivityEventType.StackUpdated,
            1,
            10,
            TestContext.Current.CancellationToken);

        var activitySummary = Assert.Single(activities.Items);
        var activity = await uow.ActivityEventRepository.GetByIdAsync(activitySummary.Id, TestContext.Current.CancellationToken);
        var update = Assert.IsType<StackUpdated>(activity?.Info);

        Assert.Equal(platformId, update.OldStack.StackRelease!.PlatformId);
        Assert.Equal(otherPlatformId, update.NewStack.StackRelease!.PlatformId);
        Assert.Equal("docker-compose.yml", Assert.IsType<ManualStack>(update.OldStack.StackRelease.Spec).ComposeFile);
        Assert.Equal("compose.updated.yml", Assert.IsType<ManualStack>(update.NewStack.StackRelease.Spec).ComposeFile);
    }

    [Fact]
    public async Task Patch_Stack_With_Invalid_Data_Should_Return_BadRequest()
    {
        var patchJson = """
        {
          "platformId": "00000000-0000-0000-0000-000000000000"
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/stacks/{stackId}", content, cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task Patch_NonExistent_Stack_Should_Return_NotFound()
    {
        var patchJson = $$"""
        {
          "platformId": "{{platformId}}",
          "spec": {
            "$type": "WebEditor",
            "composeFile": "docker-compose.yml"
          }
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/stacks/{Guid.NewGuid()}", content, cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task Patch_Stack_With_Duplicate_Name_Should_Return_Conflict()
    {
        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var otherStack = Stack.Create(
                "other-stack",
                Constants.SystemId,
                StackSource.WebEditor,
                platformId,
                new ManualStack("docker-compose.other.yml", StackUpdateBehavior.StackAutoDeploy));

            await uow.Stacks.AddAsync(otherStack, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var renameJson = $$"""
        {
          "id": "{{stackId}}",
          "name": "other-stack"
        }
        """;
        var content = new StringContent(renameJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/stacks/rename", content, TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.Conflict, response.StatusCode);
    }

    [Fact]
    public async Task Patch_Stack_Metadata_Should_Update_Description()
    {
        var patchJson = """
        {
          "description": "updated-description"
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/stacks/{stackId}/_metadata", content, cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stack = await uow.Stacks.GetAsync(stackId, TestContext.Current.CancellationToken);

        Assert.Equal("updated-description", stack?.Description);
    }

    [Fact]
    public async Task Rename_Stack_Should_Update_Name()
    {
        var renameJson = $$"""
        {
          "id": "{{stackId}}",
          "name": "stack-1-updated"
        }
        """;
        var content = new StringContent(renameJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/stacks/rename", content, TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stack = await uow.Stacks.GetAsync(stackId, TestContext.Current.CancellationToken);

        Assert.Equal("stack-1-updated", stack?.Name);
    }

    [Fact]
    public async Task Rename_NonExistent_Stack_Should_Return_NotFound()
    {
        var renameJson = $$"""
        {
          "id": "{{Guid.NewGuid()}}",
          "name": "missing-stack"
        }
        """;
        var content = new StringContent(renameJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/stacks/rename", content, TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }
}
