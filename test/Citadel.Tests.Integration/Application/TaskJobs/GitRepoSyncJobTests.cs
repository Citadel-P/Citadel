using Application.Services;
using Application.Services.Alerts;
using Application.Services.Licensing;
using Application.Services.SignalR;
using Application.TaskJobs;
using Application.Features.Deployments.Notifications;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Activities;
using Domain.Entities.Git;
using Domain.Entities.Identity;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.Models;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Moq;
using System.Threading.Channels;
using Tests.Common;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.TaskJobs;

public class GitRepoSyncJobTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<IGitCliRepository> _gitCliRepositoryMock = new();
    private readonly Mock<IRepoCacheManager> _repoCacheManagerMock = new();
    private readonly Mock<IActivityStreamManager> _activityStreamManagerMock = new();
    private readonly Mock<IGitRepositoryStreamManager> _gitRepositoryStreamManagerMock = new();
    private readonly Mock<IStackStreamManager> _stackStreamManagerMock = new();
    private readonly Mock<IAlertService> _alertServiceMock = new();
    private readonly Mock<IApplyStackService> _applyStackServiceMock = new();
    private readonly Mock<INotificationQueue> _notificationQueueMock = new();
    private InlineDbWorkQueue? _lastDbWorkQueue;
    private bool? _alertProcessedInsideDbWorkItem;

    private Guid _repoId;
    private Guid _repoWithoutAccountId;
    private Guid _platformId;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<Microsoft.Extensions.Hosting.IHostedService>();
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();
        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        _platformId = platform.Id;

        var gitAccount = new GitAccount(
            name: "GA-TEST",
            domain: "github.com",
            transport: GitTransport.Https,
            authType: GitAuthType.Token,
            createdByActorId: Constants.SystemId,
            configuration: new TokenAuth("dummy-token"));

        await uow.GitAccounts.AddAsync(gitAccount, TestContext.Current.CancellationToken);

        var gitRepository = new GitRepository(
            name: "GR-TEST",
            description: "A git repository for sync testing",
            url: "https://github.com/citadel-p/citadel.git",
            defaultBranch: "main",
            gitAccountId: gitAccount.Id,
            createdByActorId: Constants.SystemId);

        gitRepository.MarkProcessing(Constants.SystemId);

        await uow.GitRepositories.AddAsync(gitRepository, TestContext.Current.CancellationToken);

        var gitRepositoryWithoutAccount = new GitRepository(
            name: "GR-NO-ACCOUNT",
            description: "A git repository without linked account",
            url: "https://github.com/citadel-p/citadel.git",
            defaultBranch: "main",
            gitAccountId: null,
            createdByActorId: Constants.SystemId);

        gitRepositoryWithoutAccount.MarkProcessing(Constants.SystemId);

        await uow.GitRepositories.AddAsync(gitRepositoryWithoutAccount, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        _repoId = gitRepository.Id;
        _repoWithoutAccountId = gitRepositoryWithoutAccount.Id;
    }

    [Fact]
    public async Task ExecuteAsync_WhenAuthenticationFails_MarksRepositoryAsDegraded()
    {
        GitAccount? gitAccount = null;
        _gitCliRepositoryMock
            .Setup(x => x.TestConnectionAsync(It.IsAny<string>(), It.IsAny<GitAccount?>(), It.IsAny<CancellationToken>()))
            .Callback<string, GitAccount?, CancellationToken>((_, account, _) => gitAccount = account)
            .ReturnsAsync(Result.Failure("auth failed"));

        await RunJobOnceAsync();

        var repo = await WaitForRepoStatusAsync(GitReposStatus.Degraded);

        Assert.NotNull(repo);
        Assert.Equal(GitReposStatus.Degraded, repo.Status);
        Assert.Equal(ResourceControlState.Idle, repo.ControlState);
        Assert.NotNull(gitAccount);
        Assert.Equal("github.com", gitAccount!.Domain);

        _repoCacheManagerMock.Verify(
            x => x.SynchronizeAsync(It.IsAny<GitRepository>(), It.IsAny<GitAccount?>(), It.IsAny<string?>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task ExecuteAsync_WhenSyncSucceeds_MarksRepositoryAsHealthy()
    {
        GitAccount? gitAccount = null;
        _gitCliRepositoryMock
            .Setup(x => x.TestConnectionAsync(It.IsAny<string>(), It.IsAny<GitAccount?>(), It.IsAny<CancellationToken>()))
            .Callback<string, GitAccount?, CancellationToken>((_, account, _) => gitAccount = account)
            .ReturnsAsync(Result.Success());

        _repoCacheManagerMock
            .Setup(x => x.SynchronizeAsync(It.IsAny<GitRepository>(), It.IsAny<GitAccount?>(), It.IsAny<string?>(), It.IsAny<CancellationToken>()))
            .Callback<GitRepository, GitAccount?, string?, CancellationToken>((_, account, _, _) => gitAccount = account)
            .ReturnsAsync(new RepoSyncResult(GitOperation.Clone, "hash", true));

        await RunJobOnceAsync();

        var repo = await WaitForRepoStatusAsync(GitReposStatus.Healthy);

        Assert.NotNull(repo);
        Assert.Equal(GitReposStatus.Healthy, repo.Status);
        Assert.Equal(ResourceControlState.Idle, repo.ControlState);
        Assert.NotNull(gitAccount);
        Assert.Equal("github.com", gitAccount!.Domain);
        var gitRef = await GetRepoRefAsync(_repoId, "main");
        Assert.NotNull(gitRef);
        Assert.Equal(GitReposStatus.Healthy, gitRef!.Status);
        Assert.Equal("hash", gitRef.ResolvedCommitSha);

        _repoCacheManagerMock.Verify(
            x => x.SynchronizeAsync(It.IsAny<GitRepository>(), It.IsAny<GitAccount?>(), It.IsAny<string?>(), It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task ExecuteAsync_WhenManualSyncSucceeds_RecordsActivityWithRequestActor()
    {
        var actorId = await CreateActorAsync("manual-sync-user");
        await MarkRepoProcessingAsync(_repoId, actorId);

        _gitCliRepositoryMock
            .Setup(x => x.TestConnectionAsync(It.IsAny<string>(), It.IsAny<GitAccount?>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());

        _repoCacheManagerMock
            .Setup(x => x.SynchronizeAsync(It.IsAny<GitRepository>(), It.IsAny<GitAccount?>(), It.IsAny<string?>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, "manual-hash", true));

        await RunJobOnceAsync(new GitRepoSyncRequest(_repoId, "main", GitRepoSyncTrigger.Manual));

        var activities = await GetGitRepoPullActivitiesAsync(_repoId);
        var activity = Assert.Single(activities.Items);

        Assert.Equal(actorId, activity.CreatedByActorId);
    }

    [Fact]
    public async Task ExecuteAsync_WhenSynchronizationFails_MarksRepositoryAsDegraded()
    {
        _gitCliRepositoryMock
            .Setup(x => x.TestConnectionAsync(It.IsAny<string>(), It.IsAny<GitAccount?>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());

        _repoCacheManagerMock
            .Setup(x => x.SynchronizeAsync(It.IsAny<GitRepository>(), It.IsAny<GitAccount?>(), It.IsAny<string?>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, Error: "sync failed"));

        await RunJobOnceAsync();

        var repo = await WaitForRepoStatusAsync(GitReposStatus.Degraded);

        Assert.NotNull(repo);
        Assert.Equal(GitReposStatus.Degraded, repo.Status);
        Assert.Equal(ResourceControlState.Idle, repo.ControlState);
        var gitRef = await GetRepoRefAsync(_repoId, "main");
        Assert.NotNull(gitRef);
        Assert.Equal(GitReposStatus.Degraded, gitRef!.Status);
        Assert.Equal("sync failed", gitRef.LastError);
    }

    [Fact]
    public async Task ExecuteAsync_WhenWebhookSynchronizationFails_EmitsWebhookSyncFailureAlert()
    {
        _gitCliRepositoryMock
            .Setup(x => x.TestConnectionAsync(It.IsAny<string>(), It.IsAny<GitAccount?>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());

        _repoCacheManagerMock
            .Setup(x => x.SynchronizeAsync(It.IsAny<GitRepository>(), It.IsAny<GitAccount?>(), It.IsAny<string?>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, Error: "sync failed"));

        await RunJobOnceAsync(new GitRepoSyncRequest(_repoId, "main", GitRepoSyncTrigger.Webhook));

        _alertServiceMock.Verify(
            x => x.ProcessAsync(
                AlertType.WebhookGitRepoSyncFailed,
                It.Is<AlertEvaluationContext>(context =>
                    context.GitRepoWebhookSyncFailures != null
                    && context.GitRepoWebhookSyncFailures.Single().Id == _repoId
                    && context.GitRepoWebhookSyncFailures.Single().Reason == "sync failed"),
                It.IsAny<CancellationToken>()),
            Times.Once);
        Assert.False(_alertProcessedInsideDbWorkItem);
    }

    [Fact]
    public async Task ExecuteAsync_WhenPollPullResolvesSameCommit_DoesNotCreateActivity()
    {
        await UpsertRepoRefAsync(_repoId, "main", "hash", GitReposStatus.Healthy);

        _gitCliRepositoryMock
            .Setup(x => x.TestConnectionAsync(It.IsAny<string>(), It.IsAny<GitAccount?>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());

        _repoCacheManagerMock
            .Setup(x => x.SynchronizeAsync(It.IsAny<GitRepository>(), It.IsAny<GitAccount?>(), It.IsAny<string?>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, "hash", true));

        await RunJobOnceAsync(new GitRepoSyncRequest(_repoId, "main", GitRepoSyncTrigger.Poll));

        var repo = await WaitForRepoStatusAsync(GitReposStatus.Healthy);
        var activities = await GetGitRepoPullActivitiesAsync(_repoId);
        var gitRef = await GetRepoRefAsync(_repoId, "main");

        Assert.NotNull(repo);
        Assert.Equal(GitReposStatus.Healthy, repo!.Status);
        Assert.Equal(ResourceControlState.Idle, repo.ControlState);
        Assert.Null(repo.LatestActivityEvent);
        Assert.Empty(activities.Items);
        Assert.NotNull(gitRef);
        Assert.Equal("hash", gitRef!.ResolvedCommitSha);

        _notificationQueueMock.Verify(
            x => x.EnqueueAsync(It.IsAny<ActivityNotificationWorkItem>(), It.IsAny<CancellationToken>()),
            Times.Never);
        _notificationQueueMock.Verify(
            x => x.EnqueueAsync(It.IsAny<GitRepoNotificationWorkItem>(), It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task ExecuteAsync_WhenPollPullRepeatsSameFailure_DoesNotCreateActivity()
    {
        await UpsertRepoRefAsync(_repoId, "main", string.Empty, GitReposStatus.Degraded, "sync failed");

        _gitCliRepositoryMock
            .Setup(x => x.TestConnectionAsync(It.IsAny<string>(), It.IsAny<GitAccount?>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());

        _repoCacheManagerMock
            .Setup(x => x.SynchronizeAsync(It.IsAny<GitRepository>(), It.IsAny<GitAccount?>(), It.IsAny<string?>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, Error: "sync failed"));

        await RunJobOnceAsync(new GitRepoSyncRequest(_repoId, "main", GitRepoSyncTrigger.Poll));

        var repo = await WaitForRepoStatusAsync(GitReposStatus.Degraded);
        var activities = await GetGitRepoPullActivitiesAsync(_repoId);
        var gitRef = await GetRepoRefAsync(_repoId, "main");

        Assert.NotNull(repo);
        Assert.Equal(GitReposStatus.Degraded, repo!.Status);
        Assert.Equal(ResourceControlState.Idle, repo.ControlState);
        Assert.Null(repo.LatestActivityEvent);
        Assert.Empty(activities.Items);
        Assert.NotNull(gitRef);
        Assert.Equal(GitReposStatus.Degraded, gitRef!.Status);
        Assert.Equal("sync failed", gitRef.LastError);

        _notificationQueueMock.Verify(
            x => x.EnqueueAsync(It.IsAny<ActivityNotificationWorkItem>(), It.IsAny<CancellationToken>()),
            Times.Never);
        _notificationQueueMock.Verify(
            x => x.EnqueueAsync(It.IsAny<GitRepoNotificationWorkItem>(), It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task ExecuteAsync_WhenPollSyncFailsForNonDefaultBranch_DoesNotMarkRepositoryAsDegraded()
    {
        var repoId = await CreateHealthyRepositoryAsync("GR-MASTER", "master");

        _gitCliRepositoryMock
            .Setup(x => x.TestConnectionAsync(It.IsAny<string>(), It.IsAny<GitAccount?>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());

        _repoCacheManagerMock
            .Setup(x => x.SynchronizeAsync(It.IsAny<GitRepository>(), It.IsAny<GitAccount?>(), "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, Error: "fatal: couldn't find remote ref main"));

        await RunJobOnceAsync(new GitRepoSyncRequest(repoId, "main", GitRepoSyncTrigger.Poll));

        var repo = await WaitForRepoStatusAsync(repoId, GitReposStatus.Healthy);

        Assert.NotNull(repo);
        Assert.Equal("master", repo!.DefaultBranch);
        Assert.Equal(GitReposStatus.Healthy, repo.Status);
        Assert.Equal(ResourceControlState.Idle, repo.ControlState);

        var gitRef = await GetRepoRefAsync(repoId, "main");
        Assert.NotNull(gitRef);
        Assert.Equal(GitReposStatus.Degraded, gitRef!.Status);
        Assert.Equal("fatal: couldn't find remote ref main", gitRef.LastError);
        Assert.Null(repo.LatestActivityEvent);
    }

    [Fact]
    public async Task ExecuteAsync_WhenRepositoryHasNoLinkedAccount_PassesNullAccount_AndMarksRepositoryAsHealthy()
    {
        GitAccount? testConnectionAccount = null;
        GitAccount? syncAccount = null;

        _gitCliRepositoryMock
            .Setup(x => x.TestConnectionAsync(It.IsAny<string>(), It.IsAny<GitAccount?>(), It.IsAny<CancellationToken>()))
            .Callback<string, GitAccount?, CancellationToken>((_, account, _) => testConnectionAccount = account)
            .ReturnsAsync(Result.Success());

        _repoCacheManagerMock
            .Setup(x => x.SynchronizeAsync(It.IsAny<GitRepository>(), It.IsAny<GitAccount?>(), It.IsAny<string?>(), It.IsAny<CancellationToken>()))
            .Callback<GitRepository, GitAccount?, string?, CancellationToken>((_, account, _, _) => syncAccount = account)
            .ReturnsAsync(new RepoSyncResult(GitOperation.Clone, "hash", true));

        await RunJobOnceAsync(_repoWithoutAccountId);

        var repo = await WaitForRepoStatusAsync(_repoWithoutAccountId, GitReposStatus.Healthy);

        Assert.NotNull(repo);
        Assert.Equal(GitReposStatus.Healthy, repo!.Status);
        Assert.Equal(ResourceControlState.Idle, repo.ControlState);
        Assert.Null(testConnectionAccount);
        Assert.Null(syncAccount);
        var gitRef = await GetRepoRefAsync(_repoWithoutAccountId, "main");
        Assert.NotNull(gitRef);
        Assert.Equal("hash", gitRef!.ResolvedCommitSha);
    }

    [Fact]
    public async Task ExecuteAsync_WhenBranchTrackingGitStackHasOlderSource_MarksStackUpdateAvailable()
    {
        var stackId = await CreateTrackedGitStackAsync("old-commit");

        _gitCliRepositoryMock
            .Setup(x => x.TestConnectionAsync(It.IsAny<string>(), It.IsAny<GitAccount?>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());

        _repoCacheManagerMock
            .Setup(x => x.SynchronizeAsync(It.IsAny<GitRepository>(), It.IsAny<GitAccount?>(), It.IsAny<string?>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, "new-commit", true));

        _gitCliRepositoryMock
            .Setup(x => x.GetChangedPathsAsync(It.IsAny<string>(), "old-commit", "new-commit", It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<string>>(["compose.yml"]));

        await RunJobOnceAsync();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stack = await uow.Stacks.GetAsync(stackId, TestContext.Current.CancellationToken);

        Assert.NotNull(stack);
        var updateState = Assert.IsType<GitStackUpdateState>(stack!.StackUpdateState);
        Assert.Equal("old-commit", updateState.RecreateStackOnNewCommitState.CurrentCommitSha);
        Assert.Equal("new-commit", updateState.RecreateStackOnNewCommitState.RemoteCommitSha);
        Assert.Equal(ActivityEventType.StackGitUpdateAvailable, stack.LatestActivityEvent?.EventType);
        Assert.Equal(ActivityStatus.Information, stack.LatestActivityEvent?.Status);

        _alertServiceMock.Verify(
            x => x.ProcessAsync(AlertType.StackGitUpdateAvailable, It.IsAny<AlertEvaluationContext>(), It.IsAny<CancellationToken>()),
            Times.Once);
        _applyStackServiceMock.Verify(
            x => x.ApplyAsync(
                It.IsAny<Guid>(),
                It.IsAny<Guid>(),
                It.IsAny<IReadOnlyList<string>?>(),
                It.IsAny<bool>(),
                It.IsAny<bool>(),
                It.IsAny<bool>(),
                StackApplyOperation.Apply,
                null,
                It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task ExecuteAsync_WhenRemoteUrlCannotBeResolved_MarksRepositoryAsDegraded()
    {
        const string error = "Repository requires a complete URL when no Git account is selected.";

        await RunJobOnceAsync(
            new GitRepoSyncRequest(_repoWithoutAccountId),
            new InvalidOperationException(error));

        var repo = await WaitForRepoStatusAsync(_repoWithoutAccountId, GitReposStatus.Degraded);
        var gitRef = await GetRepoRefAsync(_repoWithoutAccountId, "main");

        Assert.NotNull(repo);
        Assert.Equal(ResourceControlState.Idle, repo.ControlState);
        Assert.NotNull(gitRef);
        Assert.Equal(error, gitRef.LastError);
        _gitCliRepositoryMock.Verify(
            x => x.TestConnectionAsync(
                It.IsAny<string>(),
                It.IsAny<GitAccount?>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task ExecuteAsync_WhenGitStackAutoDeploys_DoesNotApplyInsideDbWorkItem()
    {
        await CreateTrackedGitStackAsync(
            "old-commit",
            updateBehavior: StackUpdateBehavior.StackAutoDeploy);

        _gitCliRepositoryMock
            .Setup(x => x.TestConnectionAsync(It.IsAny<string>(), It.IsAny<GitAccount?>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
        _repoCacheManagerMock
            .Setup(x => x.SynchronizeAsync(It.IsAny<GitRepository>(), It.IsAny<GitAccount?>(), It.IsAny<string?>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, "new-commit", true));
        _gitCliRepositoryMock
            .Setup(x => x.GetChangedPathsAsync(It.IsAny<string>(), "old-commit", "new-commit", It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<string>>(["compose.yml"]));

        bool? appliedInsideDbWorkItem = null;
        _applyStackServiceMock
            .Setup(x => x.ApplyAsync(
                It.IsAny<Guid>(),
                It.IsAny<Guid>(),
                It.IsAny<IReadOnlyList<string>?>(),
                It.IsAny<bool>(),
                It.IsAny<bool>(),
                It.IsAny<bool>(),
                StackApplyOperation.Apply,
                null,
                It.IsAny<CancellationToken>()))
            .Callback(() => appliedInsideDbWorkItem = _lastDbWorkQueue?.IsExecuting)
            .Returns(EmptyStackApply());

        await RunJobOnceAsync();

        Assert.False(appliedInsideDbWorkItem);
    }

    [Fact]
    public async Task ExecuteAsync_WhenBranchTrackingGitStackHasOnlyUnrelatedPathChanges_DoesNotMarkUpdateAvailable()
    {
        var stackId = await CreateTrackedGitStackAsync(
            "old-commit",
            composePaths: ["stacks/beszel/compose.yml"],
            workingDirectory: "stacks/beszel");

        _gitCliRepositoryMock
            .Setup(x => x.TestConnectionAsync(It.IsAny<string>(), It.IsAny<GitAccount?>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());

        _repoCacheManagerMock
            .Setup(x => x.SynchronizeAsync(It.IsAny<GitRepository>(), It.IsAny<GitAccount?>(), It.IsAny<string?>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, "new-commit", true));

        _gitCliRepositoryMock
            .Setup(x => x.GetChangedPathsAsync(It.IsAny<string>(), "old-commit", "new-commit", It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<string>>(["stacks/caddy/compose.yml", "README.md"]));

        await RunJobOnceAsync();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stack = await uow.Stacks.GetAsync(stackId, TestContext.Current.CancellationToken);

        Assert.NotNull(stack);
        var updateState = Assert.IsType<GitStackUpdateState>(stack!.StackUpdateState);
        Assert.Equal("old-commit", updateState.RecreateStackOnNewCommitState.CurrentCommitSha);
        Assert.Null(updateState.RecreateStackOnNewCommitState.RemoteCommitSha);
        Assert.NotEqual(ActivityEventType.StackGitUpdateAvailable, stack.LatestActivityEvent?.EventType);

        _alertServiceMock.Verify(
            x => x.ProcessAsync(AlertType.StackGitUpdateAvailable, It.IsAny<AlertEvaluationContext>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task ExecuteAsync_WhenMultipleStacksTrackSameMonorepo_OnlyAffectedStackIsMarkedUpdateAvailable()
    {
        var beszelStackId = await CreateTrackedGitStackAsync(
            "old-commit",
            composePaths: ["stacks/beszel/compose.yml"],
            workingDirectory: "stacks/beszel");
        var caddyStackId = await CreateTrackedGitStackAsync(
            "old-commit",
            composePaths: ["stacks/caddy/compose.yml"],
            workingDirectory: "stacks/caddy");

        _gitCliRepositoryMock
            .Setup(x => x.TestConnectionAsync(It.IsAny<string>(), It.IsAny<GitAccount?>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());

        _repoCacheManagerMock
            .Setup(x => x.SynchronizeAsync(It.IsAny<GitRepository>(), It.IsAny<GitAccount?>(), It.IsAny<string?>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, "new-commit", true));

        _gitCliRepositoryMock
            .Setup(x => x.GetChangedPathsAsync(It.IsAny<string>(), "old-commit", "new-commit", It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<string>>(["stacks/beszel/compose.yml"]));

        await RunJobOnceAsync();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var beszel = await uow.Stacks.GetAsync(beszelStackId, TestContext.Current.CancellationToken);
        var caddy = await uow.Stacks.GetAsync(caddyStackId, TestContext.Current.CancellationToken);

        Assert.NotNull(beszel);
        Assert.NotNull(caddy);
        var beszelUpdateState = Assert.IsType<GitStackUpdateState>(beszel!.StackUpdateState);
        Assert.Equal("new-commit", beszelUpdateState.RecreateStackOnNewCommitState.RemoteCommitSha);
        Assert.Equal(ActivityEventType.StackGitUpdateAvailable, beszel.LatestActivityEvent?.EventType);

        var caddyUpdateState = Assert.IsType<GitStackUpdateState>(caddy!.StackUpdateState);
        Assert.Null(caddyUpdateState.RecreateStackOnNewCommitState.RemoteCommitSha);
        Assert.NotEqual(ActivityEventType.StackGitUpdateAvailable, caddy.LatestActivityEvent?.EventType);

        _alertServiceMock.Verify(
            x => x.ProcessAsync(AlertType.StackGitUpdateAvailable, It.IsAny<AlertEvaluationContext>(), It.IsAny<CancellationToken>()),
            Times.Once);
    }

    private Task RunJobOnceAsync(Guid repoId)
        => RunJobOnceAsync(new GitRepoSyncRequest(repoId));

    private async Task RunJobOnceAsync(
        GitRepoSyncRequest request,
        InvalidOperationException? remoteUrlError = null)
    {
        var channel = Channel.CreateUnbounded<GitRepoSyncRequest>();
        var dbWorkQueue = new InlineDbWorkQueue(Services.GetRequiredService<IServiceScopeFactory>());
        _lastDbWorkQueue = dbWorkQueue;
        var remoteUrlSetup = _repoCacheManagerMock
            .Setup(x => x.GetRemoteUrl(It.IsAny<GitRepository>(), It.IsAny<GitAccount?>()));
        if (remoteUrlError is null)
            remoteUrlSetup.Returns<GitRepository, GitAccount?>((repo, _) => repo.Url);
        else
            remoteUrlSetup.Throws(remoteUrlError);
        _alertServiceMock
            .Setup(x => x.ProcessAsync(It.IsAny<AlertType>(), It.IsAny<AlertEvaluationContext>(), It.IsAny<CancellationToken>()))
            .Callback(() => _alertProcessedInsideDbWorkItem = dbWorkQueue.IsExecuting)
            .Returns(Task.CompletedTask);
        _alertProcessedInsideDbWorkItem = null;

        var job = new TestGitRepoSyncJob(
            dbWorkQueue,
            Services.GetRequiredService<IServiceScopeFactory>(),
            _gitCliRepositoryMock.Object,
            _repoCacheManagerMock.Object,
            _activityStreamManagerMock.Object,
            _notificationQueueMock.Object,
            channel.Reader,
            _gitRepositoryStreamManagerMock.Object,
            _stackStreamManagerMock.Object,
            _alertServiceMock.Object,
            _applyStackServiceMock.Object,
            new PermissiveLicenseEntitlementService(),
            new GitRepoSyncInFlightTracker(),
            Mock.Of<Microsoft.Extensions.Logging.ILogger<GitRepoSyncJob>>());

        var runTask = job.RunAsync(TestContext.Current.CancellationToken);
        await channel.Writer.WriteAsync(request, TestContext.Current.CancellationToken);
        channel.Writer.Complete();
        await runTask;
    }

    private Task RunJobOnceAsync() => RunJobOnceAsync(_repoId);

    private async Task<Guid> CreateHealthyRepositoryAsync(string name, string defaultBranch)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var repository = new GitRepository(
            name: name,
            description: null,
            url: $"https://github.com/citadel-p/{name}.git",
            defaultBranch: defaultBranch,
            gitAccountId: null,
            createdByActorId: Constants.SystemId);

        repository.ReleaseProcessing(GitReposStatus.Healthy);

        await uow.GitRepositories.AddAsync(repository, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        return repository.Id;
    }

    private async Task<Guid> CreateTrackedGitStackAsync(
        string deployedCommit,
        List<string>? composePaths = null,
        string? workingDirectory = null,
        StackUpdateBehavior updateBehavior = StackUpdateBehavior.Notify)
    {
        composePaths ??= ["compose.yml"];
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stack = Stack.Create(
            name: $"git-stack-{Guid.NewGuid():N}",
            createdByActorId: Constants.SystemId,
            StackSource: StackSource.Git,
            platformId: _platformId,
            spec: new GitStack(
                GitRepoId: _repoId,
                Branch: "main",
                CommitSha: null,
                UpdateBehavior: updateBehavior,
                ComposePaths: composePaths,
                WorkingDirectory: workingDirectory));

        stack.ReleaseProcessing(StackReleaseStatus.Healthy);
        stack.CurrentStackRelease!.UpdateSource(new StackReleaseSource(
            SourceType: StackSource.Git,
            GitRepositoryId: _repoId,
            GitRepositoryName: "GR-TEST",
            Branch: "main",
            RequestedCommitSha: null,
            ResolvedCommitSha: deployedCommit,
            ComposePaths: composePaths,
            EnvFilePaths: [],
            WorkingDirectory: workingDirectory));

        await uow.Stacks.AddAsync(stack, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        return stack.Id;
    }

    private async Task<GitRepository?> WaitForRepoStatusAsync(Guid repoId, GitReposStatus expectedStatus)
    {
        for (var i = 0; i < 30; i++)
        {
            await using var scope = Services.CreateAsyncScope();
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var repo = await uow.GitRepositories.GetAsync(repoId, TestContext.Current.CancellationToken);

            if (repo?.Status == expectedStatus)
                return repo;

            await Task.Delay(100, TestContext.Current.CancellationToken);
        }

        await using var finalScope = Services.CreateAsyncScope();
        var finalUow = finalScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return await finalUow.GitRepositories.GetAsync(repoId, TestContext.Current.CancellationToken);
    }

    private Task<GitRepository?> WaitForRepoStatusAsync(GitReposStatus expectedStatus)
        => WaitForRepoStatusAsync(_repoId, expectedStatus);

    private async Task<GitRepositoryRef?> GetRepoRefAsync(Guid repoId, string branch)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return await uow.GitRepositories.GetRefAsync(repoId, branch, TestContext.Current.CancellationToken);
    }

    private async Task<Guid> CreateActorAsync(string name)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var actor = Actor.Create(ActorType.User, new ActorMetadata(name));
        await uow.Actors.AddAsync(actor, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        return actor.Id;
    }

    private async Task MarkRepoProcessingAsync(Guid repoId, Guid actorId)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var repo = await uow.GitRepositories.GetWithAccountAsync(repoId, TestContext.Current.CancellationToken);
        Assert.NotNull(repo);
        repo!.MarkProcessing(actorId);
        await uow.GitRepositories.UpdateAsync(repo, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    private async Task UpsertRepoRefAsync(
        Guid repoId,
        string branch,
        string resolvedCommitSha,
        GitReposStatus status,
        string? lastError = null)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.GitRepositories.UpsertRefAsync(
            new GitRepositoryRef(repoId, branch, resolvedCommitSha, status, lastError),
            TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    private async Task<PagedResult<ActivityEvent>> GetGitRepoPullActivitiesAsync(Guid repoId)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return await uow.ActivityEventRepository.GetPagedAsync(
            repoId,
            ActivityResourceType.GitRepository,
            ActivityEventType.GitRepoPulled,
            1,
            50,
            TestContext.Current.CancellationToken);
    }

    private sealed class InlineDbWorkQueue(IServiceScopeFactory scopeFactory) : IDbWorkQueue
    {
        private readonly Channel<IDbWorkItem> _channel = Channel.CreateUnbounded<IDbWorkItem>();

        public ChannelReader<IDbWorkItem> Reader => _channel.Reader;
        public bool IsExecuting { get; private set; }

        public async ValueTask EnqueueAsync(IDbWorkItem item, CancellationToken cancellationToken)
        {
            await using var scope = scopeFactory.CreateAsyncScope();
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            IsExecuting = true;
            try
            {
                await item.ExecuteAsync(uow, cancellationToken);
            }
            finally
            {
                IsExecuting = false;
            }
        }

        public ValueTask EnqueueAndWaitAsync(IDbWorkItem item, CancellationToken cancellationToken)
            => EnqueueAsync(item, cancellationToken);
    }

    private static async IAsyncEnumerable<StackStreamItem> EmptyStackApply()
    {
        await Task.CompletedTask;
        yield break;
    }

    private sealed class TestGitRepoSyncJob(
        IDbWorkQueue dbWorkQueue,
        IServiceScopeFactory scopeFactory,
        IGitCliRepository gitCliRepository,
        IRepoCacheManager repoCacheManager,
        IActivityStreamManager activityHub,
        INotificationQueue notificationQueue,
        ChannelReader<GitRepoSyncRequest> gitSyncReader,
        IGitRepositoryStreamManager gitRepoStreamManager,
        IStackStreamManager stackStreamManager,
        IAlertService alertService,
        IApplyStackService applyStackService,
        ILicenseEntitlementService entitlementService,
        GitRepoSyncInFlightTracker inFlightTracker,
        Microsoft.Extensions.Logging.ILogger<GitRepoSyncJob> logger)
        : GitRepoSyncJob(
            dbWorkQueue,
            scopeFactory,
            gitCliRepository,
            repoCacheManager,
            activityHub,
            notificationQueue,
            gitSyncReader,
            gitRepoStreamManager,
            stackStreamManager,
            alertService,
            applyStackService,
            entitlementService,
            inFlightTracker,
            logger)
    {
        public Task RunAsync(CancellationToken cancellationToken) => ExecuteAsync(cancellationToken);
    }
}
