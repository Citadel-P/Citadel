using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using System.Net;
using System.Text;
using System.Threading.Channels;

namespace Tests.Integration.WebApi;

public sealed class WebhookListenerTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Channel<GitRepoSyncRequest> _gitSyncChannel = Channel.CreateUnbounded<GitRepoSyncRequest>();
    private Guid _repoId;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<IHostedService>();
        services.AddSingleton(_gitSyncChannel);
        services.AddSingleton(s => s.GetRequiredService<Channel<GitRepoSyncRequest>>().Reader);
        services.AddSingleton(s => s.GetRequiredService<Channel<GitRepoSyncRequest>>().Writer);
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var repo = new GitRepository(
            name: "webhook-repo",
            description: "Repository with unsigned webhook enabled",
            url: "https://github.com/octocat/Hello-World.git",
            defaultBranch: "main",
            gitAccountId: null,
            createdByActorId: Constants.SystemId,
            webhook: new RepoWebhookConfig(Enabled: true));

        await uow.GitRepositories.AddAsync(repo, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        _repoId = repo.Id;
    }

    [Fact]
    public async Task Listener_RepoPull_With_EmptySecret_Accepts_And_Queues_Sync()
    {
        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var repo = await uow.GitRepositories.GetAsync(_repoId, TestContext.Current.CancellationToken);
            Assert.NotNull(repo);
            Assert.True(repo.Webhook?.Enabled);
            Assert.True(string.IsNullOrWhiteSpace(repo.Webhook?.Secret));
        }

        using (var routeProbe = new HttpRequestMessage(HttpMethod.Post, $"/listener/unsupported/repo/{_repoId}/pull"))
        {
            routeProbe.Content = new StringContent("{}", Encoding.UTF8, "application/json");
            var routeProbeResponse = await Client.SendAsync(routeProbe, TestContext.Current.CancellationToken);
            Assert.Equal(HttpStatusCode.BadRequest, routeProbeResponse.StatusCode);
        }

        using var request = new HttpRequestMessage(HttpMethod.Post, $"/listener/github/repo/{_repoId}/pull");
        request.Headers.Add("X-GitHub-Event", "push");
        request.Content = new StringContent(
            """
            {
              "ref": "refs/heads/main",
              "after": "2f1f6a0c5f6ed3c9b8b1fb8099c2c2f05bb42f5d",
              "repository": {
                "html_url": "https://github.com/octocat/Hello-World",
                "clone_url": "https://github.com/octocat/Hello-World.git",
                "full_name": "octocat/Hello-World"
              }
            }
            """,
            Encoding.UTF8,
            "application/json");

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.Accepted, response.StatusCode);
        Assert.True(_gitSyncChannel.Reader.TryRead(out var syncRequest));
        Assert.Equal(_repoId, syncRequest.RepoId);
        Assert.Equal("main", syncRequest.Branch);
        Assert.Equal(GitRepoSyncTrigger.Webhook, syncRequest.Trigger);
    }
}
