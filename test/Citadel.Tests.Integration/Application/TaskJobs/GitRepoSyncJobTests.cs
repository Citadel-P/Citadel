using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Hosting.Common;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Moq;
using System.Threading.Channels;

namespace Tests.Integration.Application.TaskJobs;

public class GitRepoSyncJobTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<IGitCliRepository> _gitCliRepositoryMock = new();
    private readonly Mock<IRepoCacheManager> _repoCacheManagerMock = new();
    private readonly Mock<IActivityStreamManager> _activityStreamManagerMock = new();
    private readonly Mock<IGitRepositoryStreamManager> _gitRepositoryStreamManagerMock = new();
    private readonly Mock<INotificationQueue> _notificationQueueMock = new();

    private Guid _repoId;
    private Guid _repoWithoutAccountId;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<Microsoft.Extensions.Hosting.IHostedService>();
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
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
            x => x.SynchronizeAsync(It.IsAny<GitRepository>(), It.IsAny<GitAccount?>(), It.IsAny<CancellationToken>()),
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
            .Setup(x => x.SynchronizeAsync(It.IsAny<GitRepository>(), It.IsAny<GitAccount?>(), It.IsAny<CancellationToken>()))
            .Callback<GitRepository, GitAccount?, CancellationToken>((_, account, _) => gitAccount = account)
            .ReturnsAsync(new RepoSyncResult(GitOperation.Clone, "hash", true));

        await RunJobOnceAsync();

        var repo = await WaitForRepoStatusAsync(GitReposStatus.Healthy);

        Assert.NotNull(repo);
        Assert.Equal(GitReposStatus.Healthy, repo.Status);
        Assert.Equal(ResourceControlState.Idle, repo.ControlState);
        Assert.NotNull(gitAccount);
        Assert.Equal("github.com", gitAccount!.Domain);

        _repoCacheManagerMock.Verify(
            x => x.SynchronizeAsync(It.IsAny<GitRepository>(), It.IsAny<GitAccount?>(), It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task ExecuteAsync_WhenSynchronizationFails_MarksRepositoryAsDegraded()
    {
        _gitCliRepositoryMock
            .Setup(x => x.TestConnectionAsync(It.IsAny<string>(), It.IsAny<GitAccount?>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());

        _repoCacheManagerMock
            .Setup(x => x.SynchronizeAsync(It.IsAny<GitRepository>(), It.IsAny<GitAccount?>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, Error: "sync failed"));

        await RunJobOnceAsync();

        var repo = await WaitForRepoStatusAsync(GitReposStatus.Degraded);

        Assert.NotNull(repo);
        Assert.Equal(GitReposStatus.Degraded, repo.Status);
        Assert.Equal(ResourceControlState.Idle, repo.ControlState);
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
            .Setup(x => x.SynchronizeAsync(It.IsAny<GitRepository>(), It.IsAny<GitAccount?>(), It.IsAny<CancellationToken>()))
            .Callback<GitRepository, GitAccount?, CancellationToken>((_, account, _) => syncAccount = account)
            .ReturnsAsync(new RepoSyncResult(GitOperation.Clone, "hash", true));

        await RunJobOnceAsync(_repoWithoutAccountId);

        var repo = await WaitForRepoStatusAsync(_repoWithoutAccountId, GitReposStatus.Healthy);

        Assert.NotNull(repo);
        Assert.Equal(GitReposStatus.Healthy, repo!.Status);
        Assert.Equal(ResourceControlState.Idle, repo.ControlState);
        Assert.Null(testConnectionAccount);
        Assert.Null(syncAccount);
    }

    private async Task RunJobOnceAsync(Guid repoId)
    {
        var channel = Channel.CreateUnbounded<GitRepoSyncRequest>();

        var job = new TestGitRepoSyncJob(
            new InlineDbWorkQueue(Services.GetRequiredService<IServiceScopeFactory>()),
            Services.GetRequiredService<IServiceScopeFactory>(),
            _gitCliRepositoryMock.Object,
            _repoCacheManagerMock.Object,
            _activityStreamManagerMock.Object,
            _notificationQueueMock.Object,
            channel.Reader,
            _gitRepositoryStreamManagerMock.Object,
            Mock.Of<Microsoft.Extensions.Logging.ILogger<GitRepoSyncJob>>());

        var runTask = job.RunAsync(TestContext.Current.CancellationToken);
        await channel.Writer.WriteAsync(new GitRepoSyncRequest(repoId), TestContext.Current.CancellationToken);
        channel.Writer.Complete();
        await runTask;
    }

    private Task RunJobOnceAsync() => RunJobOnceAsync(_repoId);

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

    private sealed class InlineDbWorkQueue(IServiceScopeFactory scopeFactory) : IDbWorkQueue
    {
        private readonly Channel<IDbWorkItem> _channel = Channel.CreateUnbounded<IDbWorkItem>();

        public ChannelReader<IDbWorkItem> Reader => _channel.Reader;

        public async ValueTask EnqueueAsync(IDbWorkItem item, CancellationToken cancellationToken)
        {
            await using var scope = scopeFactory.CreateAsyncScope();
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await item.ExecuteAsync(uow, cancellationToken);
        }
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
        Microsoft.Extensions.Logging.ILogger<GitRepoSyncJob> logger)
        : GitRepoSyncJob(dbWorkQueue, scopeFactory, gitCliRepository, repoCacheManager, activityHub, notificationQueue, gitSyncReader, gitRepoStreamManager, logger)
    {
        public Task RunAsync(CancellationToken cancellationToken) => ExecuteAsync(cancellationToken);
    }
}
