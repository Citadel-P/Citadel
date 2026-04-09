using Domain;
using Domain.Contracts.Interfaces;
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
            StackSource.Manual,
            platform.Id,
            new ManualStack("docker-compose.yml", StackUpdateBehavior.ServiceAutoDeploy),
            description: "original-description");

        await uow.Stacks.AddAsync(stack, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        stackId = stack.Id;
        platformId = platform.Id;
        otherPlatformId = otherPlatform.Id;
    }

    [Fact]
    public async Task Patch_Stack_Should_Update_Details_And_Create_New_Release()
    {
        var patchJson = $$"""
        {
          "platformId": "{{otherPlatformId}}",
          "spec": {
            "$type": "Manual",
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
        Assert.Equal(2, releases.Count);
        Assert.Equal("2", stack.CurrentStackRelease!.Version);
        Assert.Equal(otherPlatformId, stack.CurrentStackRelease.PlatformId);
        Assert.Equal(stack.CurrentStackReleaseId, stack.CurrentStackRelease.Id);
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
            "$type": "Manual",
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
                StackSource.Manual,
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
