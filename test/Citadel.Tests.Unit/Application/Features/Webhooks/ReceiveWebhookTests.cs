using Application.Features.Webhooks.Commands;
using Application.Services;
using Application.Services.Alerts;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Activities;
using Domain.Entities.Git;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Microsoft.Extensions.Logging.Abstractions;
using Moq;
using System.Text;
using System.Threading.Channels;

namespace Tests.Unit.Application.Features.Webhooks;

public sealed class ReceiveWebhookTests
{
    [Fact]
    public async Task RepoPull_WithMatchingPush_QueuesWebhookSyncAndMarksRepoProcessing()
    {
        var repo = CreateRepository();
        var channel = Channel.CreateUnbounded<GitRepoSyncRequest>();
        var gitRepos = new Mock<IGitReposRepository>();
        gitRepos.Setup(x => x.GetAsync(repo.Id, It.IsAny<CancellationToken>())).ReturnsAsync(repo);
        gitRepos.Setup(x => x.UpdateAsync(repo, It.IsAny<CancellationToken>())).ReturnsAsync(1);
        List<ActivityEvent> activities = [];

        var notificationQueue = new TestNotificationQueue();
        var handler = CreateHandler(gitRepos: gitRepos.Object, gitSyncWriter: channel.Writer, notificationQueue: notificationQueue, activities: activities);

        var result = await handler.Handle(
            CreateRepoPullCommand(repo.Id, branch: "main", repositoryUrl: "https://github.com/octocat/Hello-World.git"),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var response, out var error), error?.Message);
        Assert.Equal("queued", response.Status);
        Assert.True(channel.Reader.TryRead(out var syncRequest));
        Assert.Equal(repo.Id, syncRequest.RepoId);
        Assert.Equal("main", syncRequest.Branch);
        Assert.Equal(GitRepoSyncTrigger.Webhook, syncRequest.Trigger);
        Assert.Equal(GitReposStatus.Pending, repo.Status);
        Assert.Equal(ResourceControlState.Processing, repo.ControlState);
        Assert.Equal(2, notificationQueue.Items.Count);
        var activity = Assert.Single(activities);
        Assert.Equal(ActivityEventType.GitRepoWebhookReceived, activity.EventType);
        Assert.Equal(ActivityStatus.Success, activity.Status);
        var info = Assert.IsType<GitRepoWebhookReceived>(activity.Info);
        Assert.Equal("queued", info.Status);
        Assert.Equal("main", info.Branch);
        gitRepos.Verify(x => x.UpdateAsync(repo, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task RepoPull_WithBranchMismatch_ReturnsNoOpAndDoesNotQueueSync()
    {
        var repo = CreateRepository();
        var channel = Channel.CreateUnbounded<GitRepoSyncRequest>();
        var gitRepos = new Mock<IGitReposRepository>();
        gitRepos.Setup(x => x.GetAsync(repo.Id, It.IsAny<CancellationToken>())).ReturnsAsync(repo);
        List<ActivityEvent> activities = [];
        var alertService = new Mock<IAlertService>();

        var handler = CreateHandler(gitRepos: gitRepos.Object, gitSyncWriter: channel.Writer, activities: activities, alertService: alertService.Object);

        var result = await handler.Handle(
            CreateRepoPullCommand(repo.Id, branch: "develop", repositoryUrl: "https://github.com/octocat/Hello-World.git"),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var response, out var error), error?.Message);
        Assert.Equal("noop", response.Status);
        Assert.Equal("Branch mismatch", response.Reason);
        Assert.False(channel.Reader.TryRead(out _));
        Assert.Equal(GitReposStatus.Created, repo.Status);
        Assert.Equal(ResourceControlState.Idle, repo.ControlState);
        var activity = Assert.Single(activities);
        Assert.Equal(ActivityStatus.Information, activity.Status);
        var info = Assert.IsType<GitRepoWebhookReceived>(activity.Info);
        Assert.Equal("noop", info.Status);
        Assert.Equal("Branch mismatch", info.Reason);
        gitRepos.Verify(x => x.UpdateAsync(It.IsAny<GitRepository>(), It.IsAny<CancellationToken>()), Times.Never);
        alertService.Verify(
            x => x.ProcessAsync(It.IsAny<AlertType>(), It.IsAny<AlertEvaluationContext>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task RepoPull_WithRepositoryMismatch_ReturnsNoOpAndDoesNotQueueSync()
    {
        var repo = CreateRepository();
        var channel = Channel.CreateUnbounded<GitRepoSyncRequest>();
        var gitRepos = new Mock<IGitReposRepository>();
        gitRepos.Setup(x => x.GetAsync(repo.Id, It.IsAny<CancellationToken>())).ReturnsAsync(repo);
        List<ActivityEvent> activities = [];
        var alertService = new Mock<IAlertService>();

        var handler = CreateHandler(gitRepos: gitRepos.Object, gitSyncWriter: channel.Writer, activities: activities, alertService: alertService.Object);

        var result = await handler.Handle(
            CreateRepoPullCommand(
                repo.Id,
                branch: "main",
                repositoryUrl: "https://github.com/not-octocat/Other.git",
                repositoryFullName: "not-octocat/Other"),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var response, out var error), error?.Message);
        Assert.Equal("noop", response.Status);
        Assert.Equal("Repository identity mismatch", response.Reason);
        Assert.False(channel.Reader.TryRead(out _));
        var activity = Assert.Single(activities);
        var info = Assert.IsType<GitRepoWebhookReceived>(activity.Info);
        Assert.Equal("Repository identity mismatch", info.Reason);
        gitRepos.Verify(x => x.UpdateAsync(It.IsAny<GitRepository>(), It.IsAny<CancellationToken>()), Times.Never);
        alertService.Verify(
            x => x.ProcessAsync(
                AlertType.WebhookDispatchFailed,
                It.Is<AlertEvaluationContext>(context =>
                    context.Webhooks != null
                    && context.Webhooks.Single().ResourceId == repo.Id
                    && context.Webhooks.Single().Reason == "Repository identity mismatch"),
                It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task RepoPull_WithConfiguredSecretAndMissingSignature_ReturnsUnauthorized()
    {
        var repo = CreateRepository(new RepoWebhookConfig(Enabled: true, Secret: "configured-secret"));
        var gitRepos = new Mock<IGitReposRepository>();
        gitRepos.Setup(x => x.GetAsync(repo.Id, It.IsAny<CancellationToken>())).ReturnsAsync(repo);
        List<ActivityEvent> activities = [];
        var alertService = new Mock<IAlertService>();

        var handler = CreateHandler(gitRepos: gitRepos.Object, activities: activities, alertService: alertService.Object);

        var result = await handler.Handle(
            CreateRepoPullCommand(repo.Id, branch: "main", repositoryUrl: "https://github.com/octocat/Hello-World.git"),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.IsType<UnauthorizedError>(error);
        var activity = Assert.Single(activities);
        Assert.Equal(ActivityStatus.Failure, activity.Status);
        var info = Assert.IsType<GitRepoWebhookReceived>(activity.Info);
        Assert.Equal("rejected", info.Status);
        Assert.Equal("Webhook authentication failed", info.Reason);
        alertService.Verify(
            x => x.ProcessAsync(
                AlertType.WebhookAuthenticationFailed,
                It.Is<AlertEvaluationContext>(context =>
                    context.Webhooks != null
                    && context.Webhooks.Single().ResourceId == repo.Id
                    && context.Webhooks.Single().Reason == "Webhook authentication failed"),
                It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task StackDeploy_WithMatchingGitPush_DispatchesApplyService()
    {
        var repo = CreateRepository();
        var gitSpec = new GitStack(
            GitRepoId: repo.Id,
            Branch: "main",
            CommitSha: null,
            UpdateBehavior: StackUpdateBehavior.Disabled,
            Webhook: new StackWebhookConfig(Enabled: true),
            ComposePaths: ["compose.yml"]);
        var stack = Stack.Create(
            name: "git-stack",
            createdByActorId: Constants.SystemId,
            StackSource: StackSource.Git,
            platformId: Guid.CreateVersion7(),
            spec: gitSpec);

        var gitRepos = new Mock<IGitReposRepository>();
        gitRepos.Setup(x => x.GetAsync(repo.Id, It.IsAny<CancellationToken>())).ReturnsAsync(repo);
        var stacks = new Mock<IStackRepository>();
        stacks.Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>())).ReturnsAsync(stack);
        List<ActivityEvent> activities = [];

        var applyCalled = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var applyStackService = new Mock<IApplyStackService>();
        applyStackService
            .Setup(x => x.ApplyAsync(
                stack.Id,
                Constants.SystemId,
                It.Is<IReadOnlyList<string>?>(services => services == null),
                true,
                StackApplyOperation.Apply,
                null,
                It.IsAny<CancellationToken>()))
            .Callback(() => applyCalled.TrySetResult())
            .Returns(EmptyStackStream());

        var handler = CreateHandler(
            gitRepos: gitRepos.Object,
            stacks: stacks.Object,
            applyStackService: applyStackService.Object,
            activities: activities);

        var result = await handler.Handle(
            CreateStackDeployCommand(stack.Id, branch: "main", repositoryUrl: "https://github.com/octocat/Hello-World.git"),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var response, out var error), error?.Message);
        Assert.Equal("queued", response.Status);
        await applyCalled.Task.WaitAsync(TimeSpan.FromSeconds(2), TestContext.Current.CancellationToken);
        var activity = Assert.Single(activities);
        Assert.Equal(ActivityEventType.StackWebhookReceived, activity.EventType);
        Assert.Equal(ActivityStatus.Success, activity.Status);
        var info = Assert.IsType<StackWebhookReceived>(activity.Info);
        Assert.Equal("queued", info.Status);
        Assert.Equal("main", info.Branch);
        applyStackService.VerifyAll();
    }

    [Fact]
    public async Task StackDeploy_WhenApplyFails_EmitsWebhookDeployFailureAlert()
    {
        var repo = CreateRepository();
        var gitSpec = new GitStack(
            GitRepoId: repo.Id,
            Branch: "main",
            CommitSha: null,
            UpdateBehavior: StackUpdateBehavior.Disabled,
            Webhook: new StackWebhookConfig(Enabled: true),
            ComposePaths: ["compose.yml"]);
        var stack = Stack.Create(
            name: "git-stack",
            createdByActorId: Constants.SystemId,
            StackSource: StackSource.Git,
            platformId: Guid.CreateVersion7(),
            spec: gitSpec);

        var gitRepos = new Mock<IGitReposRepository>();
        gitRepos.Setup(x => x.GetAsync(repo.Id, It.IsAny<CancellationToken>())).ReturnsAsync(repo);
        var stacks = new Mock<IStackRepository>();
        stacks.Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>())).ReturnsAsync(stack);
        var alertCalled = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var alertService = new Mock<IAlertService>();
        alertService
            .Setup(x => x.ProcessAsync(
                AlertType.WebhookStackGitDeployFailed,
                It.Is<AlertEvaluationContext>(context =>
                    context.StackGitWebhookDeployFailures != null
                    && context.StackGitWebhookDeployFailures.Single().Id == stack.Id
                    && context.StackGitWebhookDeployFailures.Single().Reason == "compose failed"),
                It.IsAny<CancellationToken>()))
            .Callback(() => alertCalled.TrySetResult())
            .Returns(Task.CompletedTask);

        var applyStackService = new Mock<IApplyStackService>();
        applyStackService
            .Setup(x => x.ApplyAsync(
                stack.Id,
                Constants.SystemId,
                It.Is<IReadOnlyList<string>?>(services => services == null),
                true,
                StackApplyOperation.Apply,
                null,
                It.IsAny<CancellationToken>()))
            .Returns(FailedStackStream("compose failed"));

        var handler = CreateHandler(
            gitRepos: gitRepos.Object,
            stacks: stacks.Object,
            applyStackService: applyStackService.Object,
            alertService: alertService.Object);

        var result = await handler.Handle(
            CreateStackDeployCommand(stack.Id, branch: "main", repositoryUrl: "https://github.com/octocat/Hello-World.git"),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var response, out var error), error?.Message);
        Assert.Equal("queued", response.Status);
        await alertCalled.Task.WaitAsync(TimeSpan.FromSeconds(2), TestContext.Current.CancellationToken);
        alertService.VerifyAll();
    }


    private static ReceiveWebhookHandler CreateHandler(
        IGitReposRepository? gitRepos = null,
        IStackRepository? stacks = null,
        ChannelWriter<GitRepoSyncRequest>? gitSyncWriter = null,
        INotificationQueue? notificationQueue = null,
        IApplyStackService? applyStackService = null,
        IAlertService? alertService = null,
        List<ActivityEvent>? activities = null)
    {
        activities ??= [];
        var activityEvents = new Mock<IActivityEventRepository>();
        activityEvents
            .Setup(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()))
            .Callback<ActivityEvent, CancellationToken>((activity, _) => activities.Add(activity))
            .ReturnsAsync(1);

        var actors = new Mock<IActorRepository>();
        actors
            .Setup(x => x.GetById(Constants.SystemId, It.IsAny<CancellationToken>()))
            .ReturnsAsync((global::Domain.Entities.Identity.Actor?)null);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.GitRepositories).Returns(gitRepos ?? Mock.Of<IGitReposRepository>());
        unitOfWork.Setup(x => x.Stacks).Returns(stacks ?? Mock.Of<IStackRepository>());
        unitOfWork.Setup(x => x.ActivityEventRepository).Returns(activityEvents.Object);
        unitOfWork.Setup(x => x.Actors).Returns(actors.Object);
        unitOfWork.Setup(x => x.CommitAsync(It.IsAny<CancellationToken>())).Returns(Task.CompletedTask);

        return new ReceiveWebhookHandler(
            unitOfWork.Object,
            gitSyncWriter ?? Channel.CreateUnbounded<GitRepoSyncRequest>().Writer,
            notificationQueue ?? new TestNotificationQueue(),
            Mock.Of<IGitRepositoryStreamManager>(),
            Mock.Of<IActivityStreamManager>(),
            alertService ?? Mock.Of<IAlertService>(),
            applyStackService ?? Mock.Of<IApplyStackService>(),
            NullLoggerFactory.Instance);
    }

    private static GitRepository CreateRepository(RepoWebhookConfig? webhook = null)
        => new(
            name: "hello-world",
            description: "Webhook test repo",
            url: "https://github.com/octocat/Hello-World.git",
            defaultBranch: "main",
            gitAccountId: null,
            createdByActorId: Constants.SystemId,
            webhook: webhook ?? new RepoWebhookConfig(Enabled: true));

    private static ReceiveWebhook CreateRepoPullCommand(
        Guid repoId,
        string branch,
        string repositoryUrl,
        string repositoryFullName = "octocat/Hello-World")
        => new(
            AuthType: "github",
            ResourceType: "repo",
            ResourceId: repoId,
            Execution: "pull",
            Headers: Headers(("X-GitHub-Event", "push")),
            Body: PushPayload(branch, repositoryUrl, repositoryFullName));

    private static ReceiveWebhook CreateStackDeployCommand(Guid stackId, string branch, string repositoryUrl)
        => new(
            AuthType: "github",
            ResourceType: "stack",
            ResourceId: stackId,
            Execution: "deploy",
            Headers: Headers(("X-GitHub-Event", "push")),
            Body: PushPayload(branch, repositoryUrl, "octocat/Hello-World"));

    private static Dictionary<string, string[]> Headers(params (string Name, string Value)[] headers)
        => headers.ToDictionary(
            header => header.Name,
            header => new[] { header.Value },
            StringComparer.OrdinalIgnoreCase);

    private static byte[] PushPayload(string branch, string repositoryUrl, string repositoryFullName)
        => Encoding.UTF8.GetBytes($$"""
        {
          "ref": "refs/heads/{{branch}}",
          "after": "2f1f6a0c5f6ed3c9b8b1fb8099c2c2f05bb42f5d",
          "repository": {
            "html_url": "{{repositoryUrl.Replace(".git", string.Empty, StringComparison.OrdinalIgnoreCase)}}",
            "clone_url": "{{repositoryUrl}}",
            "full_name": "{{repositoryFullName}}"
          }
        }
        """);

    private static async IAsyncEnumerable<StackStreamItem> EmptyStackStream()
    {
        await Task.CompletedTask;
        yield break;
    }

    private static async IAsyncEnumerable<StackStreamItem> FailedStackStream(string message)
    {
        await Task.CompletedTask;
        yield return StackStreamItem.FromStdErr(message, 1);
    }

    private sealed class TestNotificationQueue : INotificationQueue
    {
        public List<INotificationWorkItem> Items { get; } = [];

        public ChannelReader<INotificationWorkItem> Reader { get; } = Channel.CreateUnbounded<INotificationWorkItem>().Reader;

        public ValueTask EnqueueAsync(INotificationWorkItem item, CancellationToken ct)
        {
            Items.Add(item);
            return ValueTask.CompletedTask;
        }
    }
}
