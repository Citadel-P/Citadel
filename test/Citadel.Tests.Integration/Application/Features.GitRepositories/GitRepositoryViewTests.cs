using Application.TaskJobs;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using System.Text;
using System.Text.Json;
using System.Threading.Channels;

namespace Tests.Integration.Application.Features.GitRepositories;

public class GitRepositoryViewTests : IntegrationTestBase
{
    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<IHostedService>();
        services
            .AddSingleton(Channel.CreateBounded<GitRepoSyncRequest>(Hosting.Common.Helpers.ChannelDefaultOptions()))
            .AddSingleton(s => s.GetRequiredService<Channel<GitRepoSyncRequest>>().Writer)
            .AddSingleton(s => s.GetRequiredService<Channel<GitRepoSyncRequest>>().Reader);
    }

    [Fact]
    public async Task List_GitRepositories_Should_Return_Only_Repositories_User_Is_Permitted_To_View()
    {
        var visibleRepositoryId = await CreateGitRepositoryAsync("gr-visible");
        await CreateGitRepositoryAsync("gr-hidden");

        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.GitRepository, visibleRepositoryId, ResourceAction.View)]);

        Client.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync("/api/v1/gitRepositories", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement.GetProperty("gitRepositories");
        Assert.Equal(1, items.GetArrayLength());
        Assert.Equal(visibleRepositoryId, items[0].GetProperty("id").GetGuid());
        Assert.Equal("gr-visible", items[0].GetProperty("name").GetString());
    }

    private async Task<Guid> CreateGitRepositoryAsync(string name)
    {
        var createJson = $$"""
        {
          "name": "{{name}}",
          "description": "A git repository",
          "url": "https://github.com/citadel-p/citadel.git",
          "defaultBranch": "main",
          "webHookEnabled": true,
          "gitAccountId": null
        }
        """;

        var response = await Client.PostAsync(
            "/api/v1/gitRepositories",
            new StringContent(createJson, Encoding.UTF8, "application/json"),
            cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return (await uow.GitRepositories.GetAllAsync(TestContext.Current.CancellationToken))
            .Single(x => x.Name == name)
            .Id;
    }
}
