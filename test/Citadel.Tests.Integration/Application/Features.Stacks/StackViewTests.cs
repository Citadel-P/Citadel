using Domain.Contracts.Interfaces;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Text;
using System.Text.Json;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Stacks;

public class StackViewTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private Guid? platformId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();
        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        platformId = platform.Id;
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

    private async Task<Guid> CreateStackAsync(string name)
    {
        var createJson = $$"""
            {
                "name":"{{name}}",
                "platformId":"{{platformId}}",
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
}
