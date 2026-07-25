using Application.Features.Webhooks.Commands;
using Application.Services;
using Application.Services.Alerts;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Activities;
using Domain.Entities.Backups;
using Domain.Entities.Builds;
using Domain.Entities.Git;
using Domain.Entities.Registries;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.Logging.Abstractions;
using Moq;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Threading.Channels;
using Tests.Common;

namespace Tests.Unit.Application.Features.Webhooks;

public sealed class ReceiveWebhookTests
{
    [Theory]
    [InlineData("src/Dockerfile", true)]
    [InlineData("src/app/Program.cs", true)]
    [InlineData("docs/readme.md", false)]
    [InlineData("src-old/app.cs", false)]
    public void BuildWebhookChangeMatcher_ShouldMatchContextOrDockerfileOnly(string changedPath, bool expected)
    {
        var project = new BuildProject(
            name: "api-image",
            description: null,
            enabled: true,
            gitRepositoryId: Guid.CreateVersion7(),
            branch: "main",
            contextPath: "src",
            dockerfilePath: "src/Dockerfile",
            target: null,
            buildArgs: [],
            buildSecrets: [],
            platformId: Guid.CreateVersion7(),
            registryId: Guid.CreateVersion7(),
            imageRepository: "team/api",
            tagTemplates: ["{branch}-{shortSha}"],
            webhook: new BuildWebhookConfig(Enabled: true),
            timeoutSeconds: BuildProject.DefaultTimeoutSeconds,
            retentionRunCount: BuildProject.DefaultRetentionRunCount,
            createdByActorId: Constants.SystemId);

        Assert.Equal(expected, BuildWebhookChangeMatcher.HasRelevantChanges(project, [changedPath]));
    }

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
        Assert.Equal("main", info.DispatchedBranch);
        Assert.Equal("2f1f6a0c5f6ed3c9b8b1fb8099c2c2f05bb42f5d", info.DispatchedCommitSha);
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
            UpdateBehavior: StackUpdateBehavior.StackAutoDeploy,
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
                false,
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
        Assert.Equal("main", info.DispatchedBranch);
        Assert.Equal("2f1f6a0c5f6ed3c9b8b1fb8099c2c2f05bb42f5d", info.DispatchedCommitSha);
        applyStackService.VerifyAll();
    }

    [Fact]
    public async Task StackDeploy_WithNotifyOnlyPolicy_QueuesRepoSyncAndDoesNotDeploy()
    {
        var repo = CreateRepository();
        var gitSpec = new GitStack(
            GitRepoId: repo.Id,
            Branch: "main",
            CommitSha: null,
            UpdateBehavior: StackUpdateBehavior.Notify,
            Webhook: new StackWebhookConfig(Enabled: true),
            ComposePaths: ["compose.yml"]);
        var stack = Stack.Create(
            name: "git-stack",
            createdByActorId: Constants.SystemId,
            StackSource: StackSource.Git,
            platformId: Guid.CreateVersion7(),
            spec: gitSpec);

        var channel = Channel.CreateUnbounded<GitRepoSyncRequest>();
        var gitRepos = new Mock<IGitReposRepository>();
        gitRepos.Setup(x => x.GetAsync(repo.Id, It.IsAny<CancellationToken>())).ReturnsAsync(repo);
        var stacks = new Mock<IStackRepository>();
        stacks.Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>())).ReturnsAsync(stack);
        var applyStackService = new Mock<IApplyStackService>();
        List<ActivityEvent> activities = [];

        var handler = CreateHandler(
            gitRepos: gitRepos.Object,
            stacks: stacks.Object,
            gitSyncWriter: channel.Writer,
            applyStackService: applyStackService.Object,
            activities: activities);

        var result = await handler.Handle(
            CreateStackDeployCommand(
                stack.Id,
                branch: "main",
                repositoryUrl: "https://github.com/octocat/Hello-World.git",
                changedPaths: ["compose.yml"]),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var response, out var error), error?.Message);
        Assert.Equal("queued", response.Status);
        Assert.Equal("Stack update notification queued", response.Reason);
        Assert.True(channel.Reader.TryRead(out var syncRequest));
        Assert.Equal(repo.Id, syncRequest.RepoId);
        Assert.Equal("main", syncRequest.Branch);
        Assert.Equal(GitRepoSyncTrigger.Webhook, syncRequest.Trigger);

        var activity = Assert.Single(activities);
        var info = Assert.IsType<StackWebhookReceived>(activity.Info);
        Assert.Equal("queued", info.Status);
        Assert.Equal("Stack update notification queued", info.Reason);
        Assert.Equal("main", info.DispatchedBranch);
        Assert.Equal("2f1f6a0c5f6ed3c9b8b1fb8099c2c2f05bb42f5d", info.DispatchedCommitSha);
        applyStackService.Verify(
            x => x.ApplyAsync(
                It.IsAny<Guid>(),
                It.IsAny<Guid>(),
                It.IsAny<IReadOnlyList<string>?>(),
                It.IsAny<bool>(),
                It.IsAny<bool>(),
                It.IsAny<bool>(),
                It.IsAny<StackApplyOperation>(),
                It.IsAny<StackSnapshot?>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task StackDeploy_WithDisabledPolicy_ReturnsNoOpAndDoesNotDeploy()
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

        var channel = Channel.CreateUnbounded<GitRepoSyncRequest>();
        var gitRepos = new Mock<IGitReposRepository>();
        gitRepos.Setup(x => x.GetAsync(repo.Id, It.IsAny<CancellationToken>())).ReturnsAsync(repo);
        var stacks = new Mock<IStackRepository>();
        stacks.Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>())).ReturnsAsync(stack);
        var applyStackService = new Mock<IApplyStackService>();

        var handler = CreateHandler(
            gitRepos: gitRepos.Object,
            stacks: stacks.Object,
            gitSyncWriter: channel.Writer,
            applyStackService: applyStackService.Object);

        var result = await handler.Handle(
            CreateStackDeployCommand(
                stack.Id,
                branch: "main",
                repositoryUrl: "https://github.com/octocat/Hello-World.git",
                changedPaths: ["compose.yml"]),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var response, out var error), error?.Message);
        Assert.Equal("noop", response.Status);
        Assert.Equal("Stack Git updates are disabled", response.Reason);
        Assert.False(channel.Reader.TryRead(out _));
        applyStackService.Verify(
            x => x.ApplyAsync(
                It.IsAny<Guid>(),
                It.IsAny<Guid>(),
                It.IsAny<IReadOnlyList<string>?>(),
                It.IsAny<bool>(),
                It.IsAny<bool>(),
                It.IsAny<bool>(),
                It.IsAny<StackApplyOperation>(),
                It.IsAny<StackSnapshot?>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task StackDeploy_WhenStackIsPinnedToCommit_ReturnsNoOpAndDoesNotDeploy()
    {
        var repo = CreateRepository();
        var gitSpec = new GitStack(
            GitRepoId: repo.Id,
            Branch: "main",
            CommitSha: "2f1f6a0c5f6ed3c9b8b1fb8099c2c2f05bb42f5d",
            UpdateBehavior: StackUpdateBehavior.Disabled,
            Webhook: new StackWebhookConfig(Enabled: true),
            ComposePaths: ["compose.yml"]);
        var stack = Stack.Create(
            name: "git-stack",
            createdByActorId: Constants.SystemId,
            StackSource: StackSource.Git,
            platformId: Guid.CreateVersion7(),
            spec: gitSpec);

        var stacks = new Mock<IStackRepository>();
        stacks.Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>())).ReturnsAsync(stack);
        var applyStackService = new Mock<IApplyStackService>();
        List<ActivityEvent> activities = [];

        var handler = CreateHandler(
            stacks: stacks.Object,
            applyStackService: applyStackService.Object,
            activities: activities);

        var result = await handler.Handle(
            CreateStackDeployCommand(stack.Id, branch: "main", repositoryUrl: "https://github.com/octocat/Hello-World.git"),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var response, out var error), error?.Message);
        Assert.Equal("noop", response.Status);
        Assert.Equal("Stack is pinned to a commit", response.Reason);
        var activity = Assert.Single(activities);
        var info = Assert.IsType<StackWebhookReceived>(activity.Info);
        Assert.Equal("noop", info.Status);
        Assert.Equal("Stack is pinned to a commit", info.Reason);
        applyStackService.Verify(
            x => x.ApplyAsync(
                It.IsAny<Guid>(),
                It.IsAny<Guid>(),
                It.IsAny<IReadOnlyList<string>?>(),
                It.IsAny<bool>(),
                It.IsAny<bool>(),
                It.IsAny<bool>(),
                It.IsAny<StackApplyOperation>(),
                It.IsAny<StackSnapshot?>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Theory]
    [InlineData("X-Gitea-Signature")]
    [InlineData("X-Forgejo-Signature")]
    public async Task RepoPull_WithGitHubCompatibleSignatureHeader_QueuesWebhookSync(string signatureHeader)
    {
        const string secret = "forgejo-secret";
        var repo = CreateRepository(new RepoWebhookConfig(Enabled: true, Secret: secret));
        var body = PushPayload("main", "https://github.com/octocat/Hello-World.git", "octocat/Hello-World");
        var channel = Channel.CreateUnbounded<GitRepoSyncRequest>();
        var gitRepos = new Mock<IGitReposRepository>();
        gitRepos.Setup(x => x.GetAsync(repo.Id, It.IsAny<CancellationToken>())).ReturnsAsync(repo);
        gitRepos.Setup(x => x.UpdateAsync(repo, It.IsAny<CancellationToken>())).ReturnsAsync(1);

        var handler = CreateHandler(gitRepos: gitRepos.Object, gitSyncWriter: channel.Writer);

        var result = await handler.Handle(
            new ReceiveWebhook(
                AuthType: "github",
                ResourceType: "repo",
                ResourceId: repo.Id,
                Execution: "pull",
                Headers: Headers(
                    ("X-GitHub-Event", "push"),
                    (signatureHeader, SignHex(secret, body))),
                Body: body),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var response, out var error), error?.Message);
        Assert.Equal("queued", response.Status);
        Assert.True(channel.Reader.TryRead(out var syncRequest));
        Assert.Equal(repo.Id, syncRequest.RepoId);
        Assert.Equal("main", syncRequest.Branch);
    }

    [Fact]
    public async Task StackDeploy_WithOnlyUnrelatedPathChanges_ReturnsNoOpAndDoesNotDeploy()
    {
        var repo = CreateRepository();
        var gitSpec = new GitStack(
            GitRepoId: repo.Id,
            Branch: "main",
            CommitSha: null,
            UpdateBehavior: StackUpdateBehavior.StackAutoDeploy,
            Webhook: new StackWebhookConfig(Enabled: true),
            ComposePaths: ["stacks/app/compose.yml"],
            WorkingDirectory: "stacks/app");
        var stack = Stack.Create(
            name: "git-stack",
            createdByActorId: Constants.SystemId,
            StackSource: StackSource.Git,
            platformId: Guid.CreateVersion7(),
            spec: gitSpec);
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

        var gitRepos = new Mock<IGitReposRepository>();
        gitRepos.Setup(x => x.GetAsync(repo.Id, It.IsAny<CancellationToken>())).ReturnsAsync(repo);
        var stacks = new Mock<IStackRepository>();
        stacks.Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>())).ReturnsAsync(stack);
        var applyStackService = new Mock<IApplyStackService>();
        var alertService = new Mock<IAlertService>();
        List<ActivityEvent> activities = [];

        var handler = CreateHandler(
            gitRepos: gitRepos.Object,
            stacks: stacks.Object,
            applyStackService: applyStackService.Object,
            alertService: alertService.Object,
            activities: activities);

        var result = await handler.Handle(
            CreateStackDeployCommand(
                stack.Id,
                branch: "main",
                repositoryUrl: "https://github.com/octocat/Hello-World.git",
                changedPaths: ["stacks/other/compose.yml", "README.md"]),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var response, out var error), error?.Message);
        Assert.Equal("noop", response.Status);
        Assert.Equal("No relevant path changes", response.Reason);
        var activity = Assert.Single(activities);
        var info = Assert.IsType<StackWebhookReceived>(activity.Info);
        Assert.Equal("noop", info.Status);
        Assert.Equal("No relevant path changes", info.Reason);
        applyStackService.Verify(
            x => x.ApplyAsync(
                It.IsAny<Guid>(),
                It.IsAny<Guid>(),
                It.IsAny<IReadOnlyList<string>?>(),
                It.IsAny<bool>(),
                It.IsAny<bool>(),
                It.IsAny<bool>(),
                It.IsAny<StackApplyOperation>(),
                It.IsAny<StackSnapshot?>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
        alertService.Verify(
            x => x.ProcessAsync(AlertType.WebhookDispatchFailed, It.IsAny<AlertEvaluationContext>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task StackDeploy_WithoutPayloadPaths_UsesRepositoryDiffAndSkipsUnrelatedChanges()
    {
        var repo = CreateRepository();
        var gitSpec = new GitStack(
            GitRepoId: repo.Id,
            Branch: "main",
            CommitSha: null,
            UpdateBehavior: StackUpdateBehavior.StackAutoDeploy,
            Webhook: new StackWebhookConfig(Enabled: true),
            ComposePaths: ["stacks/app/compose.yml"],
            WorkingDirectory: "stacks/app");
        var stack = Stack.Create(
            name: "git-stack",
            createdByActorId: Constants.SystemId,
            StackSource: StackSource.Git,
            platformId: Guid.CreateVersion7(),
            spec: gitSpec);
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

        var gitRepos = new Mock<IGitReposRepository>();
        gitRepos.Setup(x => x.GetAsync(repo.Id, It.IsAny<CancellationToken>())).ReturnsAsync(repo);
        var stacks = new Mock<IStackRepository>();
        stacks.Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>())).ReturnsAsync(stack);
        var repoCacheManager = new Mock<IRepoCacheManager>();
        repoCacheManager
            .Setup(x => x.SynchronizeAsync(repo, repo.GitAccount, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, Hash: "new-commit", Success: true));
        var gitCliRepository = new Mock<IGitCliRepository>();
        gitCliRepository
            .Setup(x => x.GetChangedPathsAsync(repo.GetCachePath(), "old-commit", "new-commit", It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<string>>(["stacks/other/compose.yml", "README.md"]));
        var applyStackService = new Mock<IApplyStackService>();
        var alertService = new Mock<IAlertService>();
        List<ActivityEvent> activities = [];

        var handler = CreateHandler(
            gitRepos: gitRepos.Object,
            stacks: stacks.Object,
            applyStackService: applyStackService.Object,
            alertService: alertService.Object,
            repoCacheManager: repoCacheManager.Object,
            gitCliRepository: gitCliRepository.Object,
            activities: activities);

        var result = await handler.Handle(
            CreateStackDeployCommand(stack.Id, branch: "main", repositoryUrl: "https://github.com/octocat/Hello-World.git"),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var response, out var error), error?.Message);
        Assert.Equal("noop", response.Status);
        Assert.Equal("No relevant path changes", response.Reason);
        var activity = Assert.Single(activities);
        var info = Assert.IsType<StackWebhookReceived>(activity.Info);
        Assert.Equal("noop", info.Status);
        Assert.Equal("No relevant path changes", info.Reason);
        applyStackService.Verify(
            x => x.ApplyAsync(
                It.IsAny<Guid>(),
                It.IsAny<Guid>(),
                It.IsAny<IReadOnlyList<string>?>(),
                It.IsAny<bool>(),
                It.IsAny<bool>(),
                It.IsAny<bool>(),
                It.IsAny<StackApplyOperation>(),
                It.IsAny<StackSnapshot?>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
        alertService.Verify(
            x => x.ProcessAsync(AlertType.WebhookDispatchFailed, It.IsAny<AlertEvaluationContext>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task StackDeploy_WithoutPayloadPaths_UsesRepositoryDiffAndDeploysRelevantChanges()
    {
        var repo = CreateRepository();
        var gitSpec = new GitStack(
            GitRepoId: repo.Id,
            Branch: "main",
            CommitSha: null,
            UpdateBehavior: StackUpdateBehavior.StackAutoDeploy,
            Webhook: new StackWebhookConfig(Enabled: true),
            ComposePaths: ["stacks/app/compose.yml"],
            WorkingDirectory: "stacks/app");
        var stack = Stack.Create(
            name: "git-stack",
            createdByActorId: Constants.SystemId,
            StackSource: StackSource.Git,
            platformId: Guid.CreateVersion7(),
            spec: gitSpec);
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

        var gitRepos = new Mock<IGitReposRepository>();
        gitRepos.Setup(x => x.GetAsync(repo.Id, It.IsAny<CancellationToken>())).ReturnsAsync(repo);
        var stacks = new Mock<IStackRepository>();
        stacks.Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>())).ReturnsAsync(stack);
        var repoCacheManager = new Mock<IRepoCacheManager>();
        repoCacheManager
            .Setup(x => x.SynchronizeAsync(repo, repo.GitAccount, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, Hash: "new-commit", Success: true));
        var gitCliRepository = new Mock<IGitCliRepository>();
        gitCliRepository
            .Setup(x => x.GetChangedPathsAsync(repo.GetCachePath(), "old-commit", "new-commit", It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<string>>(["stacks/app/compose.yml"]));

        var applyCalled = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var applyStackService = new Mock<IApplyStackService>();
        applyStackService
            .Setup(x => x.ApplyAsync(
                stack.Id,
                Constants.SystemId,
                It.Is<IReadOnlyList<string>?>(services => services == null),
                true,
                false,
                true,
                StackApplyOperation.Apply,
                null,
                It.IsAny<CancellationToken>()))
            .Callback(() => applyCalled.TrySetResult())
            .Returns(EmptyStackStream());

        List<ActivityEvent> activities = [];
        var handler = CreateHandler(
            gitRepos: gitRepos.Object,
            stacks: stacks.Object,
            applyStackService: applyStackService.Object,
            repoCacheManager: repoCacheManager.Object,
            gitCliRepository: gitCliRepository.Object,
            activities: activities);

        var result = await handler.Handle(
            CreateStackDeployCommand(stack.Id, branch: "main", repositoryUrl: "https://github.com/octocat/Hello-World.git"),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var response, out var error), error?.Message);
        Assert.Equal("queued", response.Status);
        await applyCalled.Task.WaitAsync(TimeSpan.FromSeconds(2), TestContext.Current.CancellationToken);
        var activity = Assert.Single(activities);
        var info = Assert.IsType<StackWebhookReceived>(activity.Info);
        Assert.Equal("main", info.DispatchedBranch);
        Assert.Equal("new-commit", info.DispatchedCommitSha);
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
            UpdateBehavior: StackUpdateBehavior.StackAutoDeploy,
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
                false,
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

    [Fact]
    public async Task BuildRun_WithOnlyUnrelatedPayloadPaths_ReturnsNoOpAndDoesNotQueue()
    {
        var repo = CreateRepository();
        var project = CreateBuildProject(repo.Id, contextPath: "services/api", dockerfilePath: "services/api/Dockerfile");
        var buildProjects = new Mock<IBuildProjectRepository>();
        buildProjects.Setup(x => x.GetAsync(project.Id, It.IsAny<CancellationToken>(), false)).ReturnsAsync(project);
        var gitRepos = new Mock<IGitReposRepository>();
        gitRepos.Setup(x => x.GetAsync(repo.Id, It.IsAny<CancellationToken>())).ReturnsAsync(repo);
        var buildRuns = new Mock<IBuildRunRepository>();
        buildRuns.Setup(x => x.HasActiveRunAsync(project.Id, It.IsAny<CancellationToken>())).ReturnsAsync(false);
        List<ActivityEvent> activities = [];

        var handler = CreateHandler(
            gitRepos: gitRepos.Object,
            buildProjects: buildProjects.Object,
            buildRuns: buildRuns.Object,
            activities: activities);

        var result = await handler.Handle(
            CreateBuildRunCommand(
                project.Id,
                branch: "main",
                repositoryUrl: repo.Url,
                changedPaths: ["docs/readme.md", "services/web/package.json"]),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var response, out var error), error?.Message);
        Assert.Equal("noop", response.Status);
        Assert.Equal("No relevant path changes", response.Reason);
        buildRuns.Verify(x => x.AddAsync(It.IsAny<BuildRun>(), It.IsAny<CancellationToken>()), Times.Never);
        var activity = Assert.Single(activities);
        Assert.Equal(ActivityEventType.BuildWebhookReceived, activity.EventType);
        Assert.Equal(ActivityStatus.Information, activity.Status);
        var info = Assert.IsType<BuildWebhookReceived>(activity.Info);
        Assert.Equal("noop", info.Status);
        Assert.Equal("No relevant path changes", info.Reason);
    }

    [Fact]
    public async Task BuildRun_WithoutPayloadPaths_UsesRepositoryDiffAndSkipsUnrelatedChanges()
    {
        var repo = CreateRepository();
        var project = CreateBuildProject(repo.Id, contextPath: "services/api", dockerfilePath: "services/api/Dockerfile");
        var latestRun = CreateSucceededBuildRun(project, repo, "old-commit");
        var buildProjects = new Mock<IBuildProjectRepository>();
        buildProjects.Setup(x => x.GetAsync(project.Id, It.IsAny<CancellationToken>(), false)).ReturnsAsync(project);
        var gitRepos = new Mock<IGitReposRepository>();
        gitRepos.Setup(x => x.GetAsync(repo.Id, It.IsAny<CancellationToken>())).ReturnsAsync(repo);
        var buildRuns = new Mock<IBuildRunRepository>();
        buildRuns.Setup(x => x.HasActiveRunAsync(project.Id, It.IsAny<CancellationToken>())).ReturnsAsync(false);
        buildRuns.Setup(x => x.GetLatestByProjectAsync(project.Id, It.IsAny<CancellationToken>())).ReturnsAsync(latestRun);
        var repoCacheManager = new Mock<IRepoCacheManager>();
        repoCacheManager
            .Setup(x => x.SynchronizeAsync(repo, repo.GitAccount, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, Hash: "new-commit", Success: true));
        var gitCliRepository = new Mock<IGitCliRepository>();
        gitCliRepository
            .Setup(x => x.GetChangedPathsAsync(repo.GetCachePath(), "old-commit", "new-commit", It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<string>>(["services/web/package.json", "README.md"]));
        List<ActivityEvent> activities = [];

        var handler = CreateHandler(
            gitRepos: gitRepos.Object,
            buildProjects: buildProjects.Object,
            buildRuns: buildRuns.Object,
            repoCacheManager: repoCacheManager.Object,
            gitCliRepository: gitCliRepository.Object,
            activities: activities);

        var result = await handler.Handle(
            CreateBuildRunCommand(project.Id, branch: "main", repositoryUrl: repo.Url),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var response, out var error), error?.Message);
        Assert.Equal("noop", response.Status);
        Assert.Equal("No relevant path changes", response.Reason);
        buildRuns.Verify(x => x.AddAsync(It.IsAny<BuildRun>(), It.IsAny<CancellationToken>()), Times.Never);
        var info = Assert.IsType<BuildWebhookReceived>(Assert.Single(activities).Info);
        Assert.Equal("No relevant path changes", info.Reason);
    }

    [Fact]
    public async Task BuildRun_WithoutPayloadPaths_UsesRepositoryDiffAndQueuesRelevantChanges()
    {
        var repo = CreateRepository();
        var project = CreateBuildProject(repo.Id, contextPath: "services/api", dockerfilePath: "services/api/Dockerfile");
        var latestRun = CreateSucceededBuildRun(project, repo, "old-commit");
        var platform = new PlatformConnectionInfo(project.PlatformId, "local", "unix:///var/run/docker.sock", PlatformConnectorType.Local);
        var registry = CreateRegistry(project.RegistryId);
        var queuedRuns = new List<BuildRun>();
        var buildProjects = new Mock<IBuildProjectRepository>();
        buildProjects.Setup(x => x.GetAsync(project.Id, It.IsAny<CancellationToken>(), false)).ReturnsAsync(project);
        buildProjects.Setup(x => x.GetAsync(project.Id, It.IsAny<CancellationToken>(), true)).ReturnsAsync(project);
        buildProjects.Setup(x => x.MarkProcessingAsync(project.Id, It.IsAny<Guid>(), It.IsAny<CancellationToken>())).ReturnsAsync(1);
        var gitRepos = new Mock<IGitReposRepository>();
        gitRepos.Setup(x => x.GetAsync(repo.Id, It.IsAny<CancellationToken>())).ReturnsAsync(repo);
        var buildRuns = new Mock<IBuildRunRepository>();
        buildRuns.Setup(x => x.HasActiveRunAsync(project.Id, It.IsAny<CancellationToken>())).ReturnsAsync(false);
        buildRuns.Setup(x => x.GetLatestByProjectAsync(project.Id, It.IsAny<CancellationToken>())).ReturnsAsync(latestRun);
        buildRuns
            .Setup(x => x.AddAsync(It.IsAny<BuildRun>(), It.IsAny<CancellationToken>()))
            .Callback<BuildRun, CancellationToken>((run, _) => queuedRuns.Add(run))
            .ReturnsAsync(1);
        var platforms = new Mock<IPlatformRepository>();
        platforms.Setup(x => x.GetInfoAsync(project.PlatformId, It.IsAny<CancellationToken>())).ReturnsAsync(platform);
        var registries = new Mock<IRegistryRepository>();
        registries.Setup(x => x.GetAsync(project.RegistryId, It.IsAny<CancellationToken>())).ReturnsAsync(registry);
        var repoCacheManager = new Mock<IRepoCacheManager>();
        repoCacheManager
            .Setup(x => x.SynchronizeAsync(repo, repo.GitAccount, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, Hash: "new-commit", Success: true));
        var gitCliRepository = new Mock<IGitCliRepository>();
        gitCliRepository
            .Setup(x => x.GetChangedPathsAsync(repo.GetCachePath(), "old-commit", "new-commit", It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<string>>(["services/api/Program.cs"]));
        List<ActivityEvent> activities = [];

        var handler = CreateHandler(
            gitRepos: gitRepos.Object,
            buildProjects: buildProjects.Object,
            buildRuns: buildRuns.Object,
            platforms: platforms.Object,
            registries: registries.Object,
            repoCacheManager: repoCacheManager.Object,
            gitCliRepository: gitCliRepository.Object,
            activities: activities);

        var result = await handler.Handle(
            CreateBuildRunCommand(project.Id, branch: "main", repositoryUrl: repo.Url),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var response, out var error), error?.Message);
        Assert.Equal("queued", response.Status);
        Assert.Equal("main", Assert.IsType<BuildWebhookReceived>(activities.Last().Info).DispatchedBranch);
        Assert.Equal("new-commit", Assert.IsType<BuildWebhookReceived>(activities.Last().Info).DispatchedCommitSha);
        var run = Assert.Single(queuedRuns);
        Assert.Equal(BuildRunTrigger.Webhook, run.Trigger);
        Assert.Equal("new-commit", run.ResolvedCommitSha);
        Assert.Equal("registry.example.test/team/api:main-new-commit", Assert.Single(run.ImageReferences));
    }

    [Fact]
    public async Task BuildRun_WhenProjectAlreadyHasActiveRun_ReturnsNoOpAndDoesNotQueue()
    {
        var repo = CreateRepository();
        var project = CreateBuildProject(repo.Id, contextPath: "services/api", dockerfilePath: "services/api/Dockerfile");
        var buildProjects = new Mock<IBuildProjectRepository>();
        buildProjects.Setup(x => x.GetAsync(project.Id, It.IsAny<CancellationToken>(), false)).ReturnsAsync(project);
        var gitRepos = new Mock<IGitReposRepository>();
        gitRepos.Setup(x => x.GetAsync(repo.Id, It.IsAny<CancellationToken>())).ReturnsAsync(repo);
        var buildRuns = new Mock<IBuildRunRepository>();
        buildRuns.Setup(x => x.HasActiveRunAsync(project.Id, It.IsAny<CancellationToken>())).ReturnsAsync(true);
        List<ActivityEvent> activities = [];

        var handler = CreateHandler(
            gitRepos: gitRepos.Object,
            buildProjects: buildProjects.Object,
            buildRuns: buildRuns.Object,
            activities: activities);

        var result = await handler.Handle(
            CreateBuildRunCommand(
                project.Id,
                branch: "main",
                repositoryUrl: repo.Url,
                changedPaths: ["services/api/Program.cs"]),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var response, out var error), error?.Message);
        Assert.Equal("noop", response.Status);
        Assert.Equal("Build project already has an active run", response.Reason);
        buildRuns.Verify(x => x.AddAsync(It.IsAny<BuildRun>(), It.IsAny<CancellationToken>()), Times.Never);
        buildProjects.Verify(x => x.MarkProcessingAsync(It.IsAny<Guid>(), It.IsAny<Guid>(), It.IsAny<CancellationToken>()), Times.Never);
        var info = Assert.IsType<BuildWebhookReceived>(Assert.Single(activities).Info);
        Assert.Equal("noop", info.Status);
        Assert.Equal("Build project already has an active run", info.Reason);
    }

    [Fact]
    public async Task BuildRun_WhenMarkProcessingLosesRace_ReturnsNoOpAndDoesNotPersistRun()
    {
        var repo = CreateRepository();
        var project = CreateBuildProject(repo.Id, contextPath: "services/api", dockerfilePath: "services/api/Dockerfile");
        var platform = new PlatformConnectionInfo(project.PlatformId, "local", "unix:///var/run/docker.sock", PlatformConnectorType.Local);
        var registry = CreateRegistry(project.RegistryId);
        var buildProjects = new Mock<IBuildProjectRepository>();
        buildProjects.Setup(x => x.GetAsync(project.Id, It.IsAny<CancellationToken>(), false)).ReturnsAsync(project);
        buildProjects.Setup(x => x.MarkProcessingAsync(project.Id, It.IsAny<Guid>(), It.IsAny<CancellationToken>())).ReturnsAsync(0);
        var gitRepos = new Mock<IGitReposRepository>();
        gitRepos.Setup(x => x.GetAsync(repo.Id, It.IsAny<CancellationToken>())).ReturnsAsync(repo);
        var buildRuns = new Mock<IBuildRunRepository>();
        buildRuns.Setup(x => x.HasActiveRunAsync(project.Id, It.IsAny<CancellationToken>())).ReturnsAsync(false);
        var platforms = new Mock<IPlatformRepository>();
        platforms.Setup(x => x.GetInfoAsync(project.PlatformId, It.IsAny<CancellationToken>())).ReturnsAsync(platform);
        var registries = new Mock<IRegistryRepository>();
        registries.Setup(x => x.GetAsync(project.RegistryId, It.IsAny<CancellationToken>())).ReturnsAsync(registry);
        List<ActivityEvent> activities = [];

        var handler = CreateHandler(
            gitRepos: gitRepos.Object,
            buildProjects: buildProjects.Object,
            buildRuns: buildRuns.Object,
            platforms: platforms.Object,
            registries: registries.Object,
            activities: activities);

        var result = await handler.Handle(
            CreateBuildRunCommand(
                project.Id,
                branch: "main",
                repositoryUrl: repo.Url,
                changedPaths: ["services/api/Program.cs"]),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var response, out var error), error?.Message);
        Assert.Equal("noop", response.Status);
        Assert.Equal("Build project already has an active run", response.Reason);
        buildRuns.Verify(x => x.AddAsync(It.IsAny<BuildRun>(), It.IsAny<CancellationToken>()), Times.Never);
        buildProjects.Verify(x => x.MarkProcessingAsync(project.Id, It.IsAny<Guid>(), It.IsAny<CancellationToken>()), Times.Once);
        var info = Assert.IsType<BuildWebhookReceived>(Assert.Single(activities).Info);
        Assert.Equal("noop", info.Status);
        Assert.Equal("Build project already has an active run", info.Reason);
    }

    [Fact]
    public async Task BackupPolicyRun_WithMatchingPush_QueuesWebhookBackupRunAsPolicyActor()
    {
        var policy = CreateBackupPolicy();
        var queuedRun = new BackupRun(
            policy.Id,
            policy.BackupRepositoryId,
            policy.Name,
            policy.Source,
            BackupRepositoryType.FileSystem,
            BackupRunTrigger.Webhook,
            triggerSourceId: policy.Id,
            triggeredByActorId: policy.RunAsActorId);
        var backupPolicies = new Mock<IBackupPolicyRepository>();
        backupPolicies.Setup(x => x.GetAsync(policy.Id, It.IsAny<CancellationToken>(), false)).ReturnsAsync(policy);
        var backupRuns = new Mock<IBackupRunRepository>();
        backupRuns
            .Setup(x => x.QueueAsync(
                policy.Id,
                It.IsAny<Guid>(),
                BackupRunTrigger.Webhook,
                policy.Id,
                Constants.SystemId,
                true,
                It.IsAny<DateTimeOffset>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(new BackupRunQueueResult(BackupRunQueueResultStatus.Queued, queuedRun));

        var handler = CreateHandler(backupPolicies: backupPolicies.Object, backupRuns: backupRuns.Object);

        var result = await handler.Handle(
            CreateBackupPolicyRunCommand(policy.Id, branch: "main"),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var response, out var error), error?.Message);
        Assert.Equal("queued", response.Status);
        backupRuns.VerifyAll();
    }

    [Fact]
    public async Task BackupPolicyRun_WhenPolicyIsDisabled_ReturnsNoOpAndDoesNotQueue()
    {
        var policy = CreateBackupPolicy(enabled: false);
        var backupPolicies = new Mock<IBackupPolicyRepository>();
        backupPolicies.Setup(x => x.GetAsync(policy.Id, It.IsAny<CancellationToken>(), false)).ReturnsAsync(policy);
        var backupRuns = new Mock<IBackupRunRepository>();

        var handler = CreateHandler(backupPolicies: backupPolicies.Object, backupRuns: backupRuns.Object);

        var result = await handler.Handle(
            CreateBackupPolicyRunCommand(policy.Id, branch: "main"),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var response, out var error), error?.Message);
        Assert.Equal("noop", response.Status);
        Assert.Equal("Backup policy is disabled", response.Reason);
        backupRuns.Verify(
            x => x.QueueAsync(
                It.IsAny<Guid>(),
                It.IsAny<Guid>(),
                It.IsAny<BackupRunTrigger>(),
                It.IsAny<Guid?>(),
                It.IsAny<Guid>(),
                It.IsAny<bool>(),
                It.IsAny<DateTimeOffset>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
    }


    private static ReceiveWebhookHandler CreateHandler(
        IGitReposRepository? gitRepos = null,
        IStackRepository? stacks = null,
        IBackupPolicyRepository? backupPolicies = null,
        IBackupRunRepository? backupRuns = null,
        IBuildProjectRepository? buildProjects = null,
        IBuildRunRepository? buildRuns = null,
        IPlatformRepository? platforms = null,
        IRegistryRepository? registries = null,
        ChannelWriter<GitRepoSyncRequest>? gitSyncWriter = null,
        INotificationQueue? notificationQueue = null,
        IApplyStackService? applyStackService = null,
        IAlertService? alertService = null,
        IRepoCacheManager? repoCacheManager = null,
        IGitCliRepository? gitCliRepository = null,
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
        var buildProjectStream = new Mock<IBuildProjectStreamManager>();
        buildProjectStream
            .Setup(x => x.SendBuildProjectInfo(It.IsAny<BuildProject>(), It.IsAny<string>(), It.IsAny<BuildRun?>()))
            .Returns(Task.CompletedTask);
        var buildRunStream = new Mock<IBuildRunStreamManager>();
        buildRunStream
            .Setup(x => x.SendBuildRunInfo(It.IsAny<BuildRun>(), It.IsAny<string>()))
            .Returns(Task.CompletedTask);
        var activityStream = new Mock<IActivityStreamManager>();
        activityStream
            .Setup(x => x.SendActivityInfo(It.IsAny<ActivityEvent>()))
            .Returns(Task.CompletedTask);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.GitRepositories).Returns(gitRepos ?? Mock.Of<IGitReposRepository>());
        unitOfWork.Setup(x => x.Stacks).Returns(stacks ?? Mock.Of<IStackRepository>());
        unitOfWork.Setup(x => x.BackupPolicies).Returns(backupPolicies ?? Mock.Of<IBackupPolicyRepository>());
        unitOfWork.Setup(x => x.BackupRuns).Returns(backupRuns ?? Mock.Of<IBackupRunRepository>());
        unitOfWork.Setup(x => x.BuildProjects).Returns(buildProjects ?? Mock.Of<IBuildProjectRepository>());
        unitOfWork.Setup(x => x.BuildRuns).Returns(buildRuns ?? Mock.Of<IBuildRunRepository>());
        unitOfWork.Setup(x => x.Platforms).Returns(platforms ?? Mock.Of<IPlatformRepository>());
        unitOfWork.Setup(x => x.Registries).Returns(registries ?? Mock.Of<IRegistryRepository>());
        unitOfWork.Setup(x => x.ActivityEventRepository).Returns(activityEvents.Object);
        unitOfWork.Setup(x => x.Actors).Returns(actors.Object);
        unitOfWork.Setup(x => x.CommitAsync(It.IsAny<CancellationToken>())).Returns(Task.CompletedTask);

        return new ReceiveWebhookHandler(
            unitOfWork.Object,
            gitSyncWriter ?? Channel.CreateUnbounded<GitRepoSyncRequest>().Writer,
            notificationQueue ?? new TestNotificationQueue(),
            Mock.Of<IGitRepositoryStreamManager>(),
            activityStream.Object,
            alertService ?? Mock.Of<IAlertService>(),
            applyStackService ?? Mock.Of<IApplyStackService>(),
            buildProjectStream.Object,
            buildRunStream.Object,
            repoCacheManager ?? Mock.Of<IRepoCacheManager>(),
            gitCliRepository ?? Mock.Of<IGitCliRepository>(),
            Mock.Of<IAutomationRunQueueService>(),
            new PermissiveLicenseEntitlementService(),
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

    private static BuildProject CreateBuildProject(Guid gitRepositoryId, string contextPath = ".", string dockerfilePath = "Dockerfile")
        => new(
            name: "api-image",
            description: "Webhook build",
            enabled: true,
            gitRepositoryId: gitRepositoryId,
            branch: "main",
            contextPath: contextPath,
            dockerfilePath: dockerfilePath,
            target: null,
            buildArgs: [],
            buildSecrets: [],
            platformId: Guid.CreateVersion7(),
            registryId: Guid.CreateVersion7(),
            imageRepository: "team/api",
            tagTemplates: ["{branch}-{shortSha}"],
            webhook: new BuildWebhookConfig(Enabled: true),
            timeoutSeconds: BuildProject.DefaultTimeoutSeconds,
            retentionRunCount: BuildProject.DefaultRetentionRunCount,
            createdByActorId: Constants.SystemId);

    private static BuildRun CreateSucceededBuildRun(BuildProject project, GitRepository repository, string commitSha)
    {
        var run = new BuildRun(
            project.Id,
            project.Name,
            repository.Id,
            repository.Name,
            project.Branch,
            commitSha,
            project.ContextPath,
            project.DockerfilePath,
            project.Target,
            project.BuildArgs,
            [.. project.BuildSecrets.Select(static secret => secret.Id)],
            new BuildPlatformSnapshot(project.PlatformId, "local", "unix:///var/run/docker.sock", PlatformConnectorType.Local),
            new BuildRegistrySnapshot(project.RegistryId, "registry", "registry.example.test"),
            project.ImageRepository,
            project.TagTemplates,
            [$"registry.example.test/{project.ImageRepository}:main-{commitSha}"],
            BuildRunTrigger.Webhook,
            project.Id,
            Constants.SystemId,
            project.TimeoutSeconds);

        run.MarkPreparing(DateTimeOffset.UtcNow);
        run.MarkRunning(DateTimeOffset.UtcNow);
        run.CompleteSucceeded("sha256:old", run.ImageReferences, 0, DateTimeOffset.UtcNow);
        return run;
    }

    private static Registry CreateRegistry(Guid id)
        => Registry.FromPersistence(
            id,
            "registry",
            null,
            RegistryStatus.Active,
            "registry.example.test",
            DateTime.UtcNow,
            Constants.SystemId,
            new CustomRegistry());

    private static BackupPolicy CreateBackupPolicy(bool enabled = true)
        => new(
            name: "backup-policy",
            description: "Webhook backup policy",
            source: new CitadelSystemBackupSource(),
            backupRepositoryId: Guid.CreateVersion7(),
            enabled: enabled,
            cron: null,
            timeZone: null,
            webhook: new BackupWebhookConfig(Enabled: true, BranchFilter: "main"),
            keepLastSuccessful: BackupPolicy.DefaultKeepLastSuccessful,
            timeoutSeconds: BackupPolicy.DefaultTimeoutSeconds,
            alertOnFailure: true,
            runAsActorId: Guid.CreateVersion7(),
            createdByActorId: Constants.SystemId);

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

    private static ReceiveWebhook CreateStackDeployCommand(
        Guid stackId,
        string branch,
        string repositoryUrl,
        IReadOnlyList<string>? changedPaths = null)
        => new(
            AuthType: "github",
            ResourceType: "stack",
            ResourceId: stackId,
            Execution: "deploy",
            Headers: Headers(("X-GitHub-Event", "push")),
            Body: PushPayload(branch, repositoryUrl, "octocat/Hello-World", changedPaths));

    private static ReceiveWebhook CreateBackupPolicyRunCommand(Guid policyId, string branch)
        => new(
            AuthType: "github",
            ResourceType: "backup-policy",
            ResourceId: policyId,
            Execution: "run",
            Headers: Headers(("X-GitHub-Event", "push")),
            Body: PushPayload(branch, "https://github.com/octocat/Hello-World.git", "octocat/Hello-World"));

    private static ReceiveWebhook CreateBuildRunCommand(
        Guid projectId,
        string branch,
        string repositoryUrl,
        IReadOnlyList<string>? changedPaths = null)
        => new(
            AuthType: "github",
            ResourceType: "build",
            ResourceId: projectId,
            Execution: "run",
            Headers: Headers(("X-GitHub-Event", "push")),
            Body: PushPayload(branch, repositoryUrl, "octocat/Hello-World", changedPaths));

    private static Dictionary<string, string[]> Headers(params (string Name, string Value)[] headers)
        => headers.ToDictionary(
            header => header.Name,
            header => new[] { header.Value },
            StringComparer.OrdinalIgnoreCase);

    private static string SignHex(string secret, byte[] body)
        => Convert.ToHexString(HMACSHA256.HashData(Encoding.UTF8.GetBytes(secret), body)).ToLowerInvariant();

    private static byte[] PushPayload(
        string branch,
        string repositoryUrl,
        string repositoryFullName,
        IReadOnlyList<string>? changedPaths = null)
    {
        var commitsJson = changedPaths is null
            ? string.Empty
            : $$"""
              ,
              "commits": [
                { "added": [], "modified": {{JsonSerializer.Serialize(changedPaths)}}, "removed": [] }
              ]
              """;

        return Encoding.UTF8.GetBytes($$"""
        {
          "ref": "refs/heads/{{branch}}",
          "after": "2f1f6a0c5f6ed3c9b8b1fb8099c2c2f05bb42f5d",
          "repository": {
            "html_url": "{{repositoryUrl.Replace(".git", string.Empty, StringComparison.OrdinalIgnoreCase)}}",
            "clone_url": "{{repositoryUrl}}",
            "full_name": "{{repositoryFullName}}"
          }
          {{commitsJson}}
        }
        """);
    }

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
