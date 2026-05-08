using Domain.Contracts.Interfaces;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Text;
using System.Text.Json;

namespace Tests.Integration.Application.Features.GitAccounts;

public class GitAccountViewTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task List_GitAccounts_Should_Return_Only_Accounts_User_Is_Permitted_To_View()
    {
        var visibleAccountId = await CreateGitAccountAsync("ga-visible");
        await CreateGitAccountAsync("ga-hidden");

        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.GitAccount, visibleAccountId, PermissionLevel.Read)]);

        Client.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync("/api/v1/gitAccounts", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement.GetProperty("gitAccounts");
        Assert.Equal(1, items.GetArrayLength());
        Assert.Equal(visibleAccountId, items[0].GetProperty("id").GetGuid());
        Assert.Equal("ga-visible", items[0].GetProperty("name").GetString());
    }

    private async Task<Guid> CreateGitAccountAsync(string name)
    {
        var createJson = $$"""
        {
          "name": "{{name}}",
          "domain": "github.com",
          "transport": "Https",
          "authType": "Basic",
          "configuration": {
            "$type": "Basic",
            "username": "dummy-user",
            "password": "dummy-password123"
          }
        }
        """;

        var response = await Client.PostAsync(
            "/api/v1/gitAccounts",
            new StringContent(createJson, Encoding.UTF8, "application/json"),
            cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return (await uow.GitAccounts.GetAllAsync(TestContext.Current.CancellationToken))
            .Single(x => x.Name == name)
            .Id;
    }
}
