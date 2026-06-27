using Application.Services;
using Application.Services.Alerts;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Activities;
using Domain.Entities.Git;
using Domain.Entities.Stacks;
using Hosting.Common;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Moq;
using System.Net;
using System.Net.Http.Json;
using System.Text;
using System.Threading.Channels;
using Tests.Integration.Helpers;

namespace Tests.Integration.WebApi;

public sealed class WebhookListenerTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Channel<GitRepoSyncRequest> _gitSyncChannel = Channel.CreateUnbounded<GitRepoSyncRequest>();
    private readonly Mock<IRepoCacheManager> _repoCacheManagerMock = new();
    private readonly Mock<IGitCliRepository> _gitCliRepositoryMock = new();
    private readonly Mock<IApplyStackService> _applyStackServiceMock = new();
    private readonly Mock<IAlertService> _alertServiceMock = new();
    private Guid _repoId;
    private Guid _stackId;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<IHostedService>();
        services.AddSingleton(_gitSyncChannel);
        services.AddSingleton(s => s.GetRequiredService<Channel<GitRepoSyncRequest>>().Reader);
        services.AddSingleton(s => s.GetRequiredService<Channel<GitRepoSyncRequest>>().Writer);
        services.ReplaceService(_repoCacheManagerMock.Object);
        services.ReplaceService(_gitCliRepositoryMock.Object);
        services.ReplaceService(_applyStackServiceMock.Object);
        services.ReplaceService(_alertServiceMock.Object);
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

        var platform = Fakes.GetDummyPlatform();
        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);

        var stack = Stack.Create(
            name: "webhook-git-stack",
            createdByActorId: Constants.SystemId,
            StackSource: StackSource.Git,
            platformId: platform.Id,
            spec: new GitStack(
                GitRepoId: repo.Id,
                Branch: "main",
                CommitSha: null,
                UpdateBehavior: StackUpdateBehavior.Notify,
                Webhook: new StackWebhookConfig(Enabled: true),
                ComposePaths: ["stacks/app/compose.yml"],
                WorkingDirectory: "stacks/app"));

        stack.ReleaseProcessing(StackReleaseStatus.Healthy);
        stack.CurrentStackRelease!.UpdateSource(new StackReleaseSource(
            SourceType: StackSource.Git,
            GitRepositoryId: repo.Id,
            GitRepositoryName: repo.Name,
            Branch: "main",
            RequestedCommitSha: null,
            ResolvedCommitSha: "old-commit",
            ComposePaths: ["stacks/app/compose.yml"],
            EnvFilePaths: [],
            WorkingDirectory: "stacks/app"));

        await uow.Stacks.AddAsync(stack, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        _repoId = repo.Id;
        _stackId = stack.Id;
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

        await using var activityScope = Services.CreateAsyncScope();
        var activityUow = activityScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var activities = await activityUow.ActivityEventRepository.GetPagedAsync(
            _repoId,
            ActivityResourceType.GitRepository,
            ActivityEventType.GitRepoWebhookReceived,
            page: 1,
            pageSize: 10,
            TestContext.Current.CancellationToken);

        var activityItems = activities.Items.ToList();
        Assert.Equal(2, activities.TotalCount);
        Assert.Contains(activityItems, activity =>
            activity.Status == ActivityStatus.Failure
            && activity.EventType == ActivityEventType.GitRepoWebhookReceived);
        Assert.Contains(activityItems, activity =>
            activity.Status == ActivityStatus.Success
            && activity.EventType == ActivityEventType.GitRepoWebhookReceived);

        var rejected = Assert.Single(activityItems, activity => activity.Status == ActivityStatus.Failure);
        var queued = Assert.Single(activityItems, activity => activity.Status == ActivityStatus.Success);
        var rejectedDetails = await activityUow.ActivityEventRepository.GetByIdAsync(rejected.Id, TestContext.Current.CancellationToken);
        var queuedDetails = await activityUow.ActivityEventRepository.GetByIdAsync(queued.Id, TestContext.Current.CancellationToken);

        Assert.IsType<GitRepoWebhookReceived>(rejectedDetails?.Info);
        Assert.IsType<GitRepoWebhookReceived>(queuedDetails?.Info);
    }

    [Fact]
    public async Task Listener_StackDeploy_WithoutPayloadPaths_DiffsRepositoryBeforeDeploying()
    {
        _repoCacheManagerMock
            .Setup(x => x.SynchronizeAsync(It.IsAny<GitRepository>(), It.IsAny<GitAccount?>(), "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, Hash: "new-commit", Success: true));
        _gitCliRepositoryMock
            .Setup(x => x.GetChangedPathsAsync(It.IsAny<string>(), "old-commit", "new-commit", It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<string>>(["stacks/other/compose.yml", "README.md"]));

        using var request = new HttpRequestMessage(HttpMethod.Post, $"/listener/github/stack/{_stackId}/deploy");
        request.Headers.Add("X-GitHub-Event", "push");
        request.Content = new StringContent(PushPayloadWithoutChangedPaths(), Encoding.UTF8, "application/json");

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.Accepted, response.StatusCode);
        var result = await response.Content.ReadFromJsonAsync<WebhookResponse>(cancellationToken: TestContext.Current.CancellationToken);
        Assert.NotNull(result);
        Assert.Equal("noop", result!.Status);
        Assert.Equal("No relevant path changes", result.Reason);

        await using var activityScope = Services.CreateAsyncScope();
        var activityUow = activityScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var activities = await activityUow.ActivityEventRepository.GetPagedAsync(
            _stackId,
            ActivityResourceType.Stack,
            ActivityEventType.StackWebhookReceived,
            page: 1,
            pageSize: 10,
            TestContext.Current.CancellationToken);

        var activity = Assert.Single(activities.Items);
        Assert.Equal(ActivityStatus.Information, activity.Status);
        var details = await activityUow.ActivityEventRepository.GetByIdAsync(activity.Id, TestContext.Current.CancellationToken);
        var info = Assert.IsType<StackWebhookReceived>(details?.Info);
        Assert.Equal("noop", info.Status);
        Assert.Equal("No relevant path changes", info.Reason);

        _applyStackServiceMock.Verify(
            x => x.ApplyAsync(
                It.IsAny<Guid>(),
                It.IsAny<Guid>(),
                It.IsAny<IReadOnlyList<string>?>(),
                It.IsAny<bool>(),
                It.IsAny<StackApplyOperation>(),
                It.IsAny<StackSnapshot?>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
        _alertServiceMock.Verify(
            x => x.ProcessAsync(AlertType.WebhookDispatchFailed, It.IsAny<AlertEvaluationContext>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task Listener_StackDeploy_WithNotifyOnlyPolicy_QueuesSyncForRelevantRepositoryDiff()
    {
        _repoCacheManagerMock
            .Setup(x => x.SynchronizeAsync(It.IsAny<GitRepository>(), It.IsAny<GitAccount?>(), "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, Hash: "new-commit", Success: true));
        _gitCliRepositoryMock
            .Setup(x => x.GetChangedPathsAsync(It.IsAny<string>(), "old-commit", "new-commit", It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<string>>(["stacks/app/compose.yml"]));
        _applyStackServiceMock
            .Setup(x => x.ApplyAsync(
                _stackId,
                Constants.SystemId,
                It.Is<IReadOnlyList<string>?>(services => services == null),
                true,
                StackApplyOperation.Apply,
                null,
                It.IsAny<CancellationToken>()))
            .Returns(EmptyStackStream());

        using var request = new HttpRequestMessage(HttpMethod.Post, $"/listener/github/stack/{_stackId}/deploy");
        request.Headers.Add("X-GitHub-Event", "push");
        request.Content = new StringContent(PushPayloadWithoutChangedPaths(), Encoding.UTF8, "application/json");

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.Accepted, response.StatusCode);
        var result = await response.Content.ReadFromJsonAsync<WebhookResponse>(cancellationToken: TestContext.Current.CancellationToken);
        Assert.NotNull(result);
        Assert.Equal("queued", result!.Status);
        Assert.Equal("Stack update notification queued", result.Reason);
        Assert.True(_gitSyncChannel.Reader.TryRead(out var syncRequest));
        Assert.Equal(_repoId, syncRequest.RepoId);
        Assert.Equal("main", syncRequest.Branch);
        Assert.Equal(GitRepoSyncTrigger.Webhook, syncRequest.Trigger);
        _applyStackServiceMock.Verify(
            x => x.ApplyAsync(
                It.IsAny<Guid>(),
                It.IsAny<Guid>(),
                It.IsAny<IReadOnlyList<string>?>(),
                It.IsAny<bool>(),
                It.IsAny<StackApplyOperation>(),
                It.IsAny<StackSnapshot?>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
    }

    private static string PushPayloadWithoutChangedPaths()
        => """
           {
             "ref": "refs/heads/main",
             "after": "2f1f6a0c5f6ed3c9b8b1fb8099c2c2f05bb42f5d",
             "repository": {
               "html_url": "https://github.com/octocat/Hello-World",
               "clone_url": "https://github.com/octocat/Hello-World.git",
               "full_name": "octocat/Hello-World"
             }
           }
           """;

    private static async IAsyncEnumerable<StackStreamItem> EmptyStackStream()
    {
        await Task.CompletedTask;
        yield break;
    }

    private sealed record WebhookResponse(bool Accepted, string Status, Guid RequestId, string? Reason);
}
