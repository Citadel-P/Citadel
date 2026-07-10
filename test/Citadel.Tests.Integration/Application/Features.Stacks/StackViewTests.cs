using Domain.Contracts.Interfaces;
using Domain;
using Domain.Entities.Platforms;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Text;
using System.Text.Json;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Stacks;

public class StackViewTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private Guid? platformId;
    private Guid? otherPlatformId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = CreatePlatform("stack-view-platform-a", "https://stack-view-a");
        var otherPlatform = CreatePlatform("stack-view-platform-b", "https://stack-view-b");
        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Platforms.AddAsync(otherPlatform, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        platformId = platform.Id;
        otherPlatformId = otherPlatform.Id;
    }

    [Fact]
    public async Task List_Stacks_Should_Return_Only_Stacks_User_Is_Permitted_To_View()
    {
        var visibleStackId = await CreateStackAsync("stack-visible");
        await CreateStackAsync("stack-hidden");

        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Stack, visibleStackId, PermissionLevel.Read)]);

        Client.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync("/api/v1/stacks", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement.GetProperty("stacks");
        Assert.Equal(1, items.GetArrayLength());
        Assert.Equal(visibleStackId, items[0].GetProperty("id").GetGuid());
        Assert.Equal("stack-visible", items[0].GetProperty("name").GetString());
    }

    [Fact]
    public async Task List_Stacks_Should_Filter_By_Platform()
    {
        var targetStackId = await CreateStackAsync("stack-platform-target", platformId!.Value);
        await CreateStackAsync("stack-platform-other", otherPlatformId!.Value);

        var response = await Client.GetAsync($"/api/v1/stacks?platformId={platformId!.Value:D}", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var stack = Assert.Single(document.RootElement.GetProperty("stacks").EnumerateArray());
        Assert.Equal(targetStackId, stack.GetProperty("id").GetGuid());
        Assert.Equal(platformId, stack.GetProperty("platformId").GetGuid());
    }

    private async Task<Guid> CreateStackAsync(string name)
        => await CreateStackAsync(name, platformId!.Value);

    private async Task<Guid> CreateStackAsync(string name, Guid selectedPlatformId)
    {
        var createJson = $$"""
            {
                "name":"{{name}}",
                "platformId":"{{selectedPlatformId}}",
                "stackSource":"WebEditor",
                "spec":{
                    "$type":"WebEditor",
                    "composeFile":"docker-compose.yml"
                }
            }
            """;

        var response = await Client.PostAsync(
            "/api/v1/stacks",
            new StringContent(createJson, Encoding.UTF8, "application/json"),
            cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return (await uow.Stacks.GetInfoAsync(TestContext.Current.CancellationToken))
            .Single(x => x.Name == name)
            .Id;
    }

    private static Platform CreatePlatform(string name, string address) => new(
        name: name,
        address: address,
        networkCount: 1,
        volumeCount: 1,
        imageCount: 1,
        cpuCount: 2,
        memTotal: 512,
        serverVersion: "1.0.0",
        agentVersion: "1.0.0",
        status: PlatformStatus.Online,
        connectorType: PlatformConnectorType.Agent,
        platformDescriptor: new DockerPlatformDescriptor(
            DaemonId: name,
            ContainerCount: 0,
            ContainersRunning: 0,
            ContainersPaused: 0,
            ContainersStopped: 0));
}
