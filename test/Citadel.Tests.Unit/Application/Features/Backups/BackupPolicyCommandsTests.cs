using Application.Features.Backups.Commands;
using Application.Features.Backups.Models;
using Application.Services.Backups;
using Application.Services.Licensing;
using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Backups;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Backups;
using Hosting.Common.Abstraction;
using LightResults;
using Moq;
using Tests.Common;

namespace Tests.Unit.Application.Features.Backups;

public sealed class BackupPolicyCommandsTests
{
    [Fact]
    public async Task QueueBackupRun_ShouldNotifyCreatedRunAfterCommit()
    {
        var run = CreateQueuedBackupRun();
        var backupRuns = new Mock<IBackupRunRepository>();
        backupRuns
            .Setup(x => x.QueueAsync(
                run.BackupPolicyId,
                It.IsAny<Guid>(),
                BackupRunTrigger.Manual,
                null,
                It.IsAny<Guid>(),
                false,
                It.IsAny<DateTimeOffset>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(new BackupRunQueueResult(BackupRunQueueResultStatus.Queued, run));
        var unitOfWork = CreateUnitOfWork(backupRuns.Object);
        var streamManager = new Mock<IBackupRunStreamManager>();
        var notifications = new List<INotificationWorkItem>();
        var notificationQueue = CreateNotificationQueue(notifications);
        var handler = new QueueBackupRunHandler(
            unitOfWork.Object,
            CreateUserContextAccessor(),
            streamManager.Object,
            notificationQueue.Object,
            new PermissiveLicenseEntitlementService());

        var result = await handler.Handle(
            new QueueBackupRun(run.BackupPolicyId, new QueueBackupRunInputModel()),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var queued, out _));
        Assert.Equal(run.Id, queued.Run.Id);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
        var notification = Assert.Single(notifications);

        await notification.ExecuteAsync(TestContext.Current.CancellationToken);

        streamManager.Verify(x => x.SendBackupRunInfo(run, "create"), Times.Once);
    }

    [Fact]
    public async Task RunBackupPolicy_ShouldNotifyCreatedRunBeforeStreamingExecution()
    {
        var run = CreateQueuedBackupRun();
        var backupRuns = new Mock<IBackupRunRepository>();
        backupRuns
            .Setup(x => x.QueueAsync(
                run.BackupPolicyId,
                It.IsAny<Guid>(),
                BackupRunTrigger.Manual,
                null,
                It.IsAny<Guid>(),
                false,
                It.IsAny<DateTimeOffset>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(new BackupRunQueueResult(BackupRunQueueResultStatus.Queued, run));
        var unitOfWork = CreateUnitOfWork(backupRuns.Object);
        var executionService = new Mock<IBackupRunExecutionService>();
        executionService
            .Setup(x => x.ExecuteQueuedAsync(run.Id, It.IsAny<CancellationToken>()))
            .Returns(ToAsyncEnumerable(new BackupRunStreamItem(run.Id, BackupRunStatus.Succeeded, "Backup finished.")));
        var streamManager = new Mock<IBackupRunStreamManager>();
        var notifications = new List<INotificationWorkItem>();
        var notificationQueue = CreateNotificationQueue(notifications);
        var handler = new RunBackupPolicyHandler(
            unitOfWork.Object,
            executionService.Object,
            CreateUserContextAccessor(),
            streamManager.Object,
            notificationQueue.Object,
            new PermissiveLicenseEntitlementService());

        var items = await ToListAsync(handler.Handle(
            new RunBackupPolicy(run.BackupPolicyId, new QueueBackupRunInputModel()),
            TestContext.Current.CancellationToken));

        Assert.Equal(2, items.Count);
        Assert.Equal(BackupRunStatus.Queued, items[0].Status);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
        var notification = Assert.Single(notifications);

        await notification.ExecuteAsync(TestContext.Current.CancellationToken);

        streamManager.Verify(x => x.SendBackupRunInfo(run, "create"), Times.Once);
        executionService.Verify(x => x.ExecuteQueuedAsync(run.Id, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task CancelBackupRun_ShouldNotifyUpdatedRunAfterCommit()
    {
        var run = CreateQueuedBackupRun();
        run.Cancel(DateTimeOffset.UtcNow);
        var backupRuns = new Mock<IBackupRunRepository>();
        backupRuns
            .Setup(x => x.CancelQueuedOrRunningAsync(
                run.Id,
                It.IsAny<DateTimeOffset>(),
                "Backup run cancelled.",
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(run);
        var unitOfWork = CreateUnitOfWork(backupRuns.Object);
        var coordinator = new Mock<IBackupRunCoordinator>();
        var streamManager = new Mock<IBackupRunStreamManager>();
        var notifications = new List<INotificationWorkItem>();
        var notificationQueue = CreateNotificationQueue(notifications);
        var handler = new CancelBackupRunHandler(
            unitOfWork.Object,
            coordinator.Object,
            streamManager.Object,
            notificationQueue.Object);

        var result = await handler.Handle(new CancelBackupRun(run.Id), TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess());
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
        var notification = Assert.Single(notifications);

        await notification.ExecuteAsync(TestContext.Current.CancellationToken);

        coordinator.Verify(x => x.Cancel(run.Id), Times.Once);
        streamManager.Verify(x => x.SendBackupRunInfo(run, "update"), Times.Once);
    }

    [Fact]
    public async Task CreateBackupPolicy_ShouldRejectCoreFilesystemRepositoryForRemoteDockerVolume()
    {
        var platformId = Guid.CreateVersion7();
        var repository = CreateRepository(new FileSystemBackupRepositorySpec(
            BackupExecutionLocation.Core,
            PlatformId: null,
            Path: "remote-volume-backups"));
        var backupPolicies = CreateBackupPolicyRepository();
        var platforms = new Mock<IPlatformRepository>();
        platforms
            .Setup(x => x.GetInfoAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new PlatformConnectionInfo(platformId, "edge-01", "edge://edge-01", PlatformConnectorType.EdgeAgent));
        var unitOfWork = CreateUnitOfWork(repository, backupPolicies.Object, platforms.Object);
        var handler = CreateCreateHandler(unitOfWork.Object);

        var result = await handler.Handle(
            CreatePolicyCommand(repository.Id, new DockerVolumeBackupSource(platformId, "app-data")),
            TestContext.Current.CancellationToken);

        Assert.False(result.IsSuccess(out _, out var error));
        Assert.Equal(
            "Core filesystem backup repositories cannot back up remote Docker volumes. Use an S3-compatible repository or a filesystem repository on the same platform.",
            error.Message);
        backupPolicies.Verify(
            x => x.AddAsync(
                It.IsAny<BackupPolicy>(),
                It.IsAny<CancellationToken>(),
                It.IsAny<IReadOnlyCollection<Guid>?>(),
                It.IsAny<Guid?>()),
            Times.Never);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task CreateBackupPolicy_ShouldRejectFilesystemRepositoryOnDifferentPlatform()
    {
        var sourcePlatformId = Guid.CreateVersion7();
        var repositoryPlatformId = Guid.CreateVersion7();
        var repository = CreateRepository(new FileSystemBackupRepositorySpec(
            BackupExecutionLocation.Platform,
            repositoryPlatformId,
            Path: "/backups"));
        var backupPolicies = CreateBackupPolicyRepository();
        var platforms = new Mock<IPlatformRepository>();
        platforms
            .Setup(x => x.GetInfoAsync(sourcePlatformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new PlatformConnectionInfo(sourcePlatformId, "local-01", "unix:///var/run/docker.sock", PlatformConnectorType.Local));
        var unitOfWork = CreateUnitOfWork(repository, backupPolicies.Object, platforms.Object);
        var handler = CreateCreateHandler(unitOfWork.Object);

        var result = await handler.Handle(
            CreatePolicyCommand(repository.Id, new DockerVolumeBackupSource(sourcePlatformId, "app-data")),
            TestContext.Current.CancellationToken);

        Assert.False(result.IsSuccess(out _, out var error));
        Assert.Equal("Filesystem backup repository platform must match the backup source platform.", error.Message);
        backupPolicies.Verify(
            x => x.AddAsync(
                It.IsAny<BackupPolicy>(),
                It.IsAny<CancellationToken>(),
                It.IsAny<IReadOnlyCollection<Guid>?>(),
                It.IsAny<Guid?>()),
            Times.Never);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task UpdateBackupPolicy_ShouldRejectRemoteDockerVolumeSourceWithCoreFilesystemRepository()
    {
        var platformId = Guid.CreateVersion7();
        var repository = CreateRepository(new FileSystemBackupRepositorySpec(
            BackupExecutionLocation.Core,
            PlatformId: null,
            Path: "remote-volume-backups"));
        var policy = CreatePolicy(repository.Id, new CitadelSystemBackupSource());
        var backupPolicies = CreateBackupPolicyRepository(policy);
        var platforms = new Mock<IPlatformRepository>();
        platforms
            .Setup(x => x.GetInfoAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new PlatformConnectionInfo(platformId, "agent-01", "http://agent:9000", PlatformConnectorType.Agent));
        var unitOfWork = CreateUnitOfWork(repository, backupPolicies.Object, platforms.Object);
        var handler = CreateUpdateHandler(unitOfWork.Object);

        var result = await handler.Handle(
            new UpdateBackupPolicy(
                policy.Id,
                new UpdateBackupPolicyInputModel(Source: new DockerVolumeBackupSource(platformId, "app-data")),
                UpdateDescription: false,
                UpdateSource: true,
                UpdateBackupRepository: false,
                UpdateCron: false,
                UpdateTimeZone: false,
                UpdateWebhook: false),
            TestContext.Current.CancellationToken);

        Assert.False(result.IsSuccess(out _, out var error));
        Assert.Equal(
            "Core filesystem backup repositories cannot back up remote Docker volumes. Use an S3-compatible repository or a filesystem repository on the same platform.",
            error.Message);
        backupPolicies.Verify(x => x.UpdateAsync(It.IsAny<BackupPolicy>(), It.IsAny<CancellationToken>()), Times.Never);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task UpdateBackupPolicy_ShouldRejectCoreFilesystemRepositoryForExistingRemoteDockerVolumeSource()
    {
        var platformId = Guid.CreateVersion7();
        var originalRepository = CreateRepository(new S3CompatibleBackupRepositorySpec(
            Endpoint: new Uri("https://s3.example.com"),
            Bucket: "citadel-backups",
            Prefix: null,
            Region: null,
            BucketLookup: S3BucketLookup.Auto,
            AccessKeySecretId: Guid.CreateVersion7(),
            SecretKeySecretId: Guid.CreateVersion7(),
            SessionTokenSecretId: null));
        var coreRepository = CreateRepository(new FileSystemBackupRepositorySpec(
            BackupExecutionLocation.Core,
            PlatformId: null,
            Path: "remote-volume-backups"));
        var policy = CreatePolicy(originalRepository.Id, new DockerVolumeBackupSource(platformId, "app-data"));
        var backupPolicies = CreateBackupPolicyRepository(policy);
        var platforms = new Mock<IPlatformRepository>();
        platforms
            .Setup(x => x.GetInfoAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new PlatformConnectionInfo(platformId, "edge-01", "edge://edge-01", PlatformConnectorType.EdgeAgent));
        var unitOfWork = CreateUnitOfWork(coreRepository, backupPolicies.Object, platforms.Object);
        var handler = CreateUpdateHandler(unitOfWork.Object);

        var result = await handler.Handle(
            new UpdateBackupPolicy(
                policy.Id,
                new UpdateBackupPolicyInputModel(BackupRepositoryId: coreRepository.Id),
                UpdateDescription: false,
                UpdateSource: false,
                UpdateBackupRepository: true,
                UpdateCron: false,
                UpdateTimeZone: false,
                UpdateWebhook: false),
            TestContext.Current.CancellationToken);

        Assert.False(result.IsSuccess(out _, out var error));
        Assert.Equal(
            "Core filesystem backup repositories cannot back up remote Docker volumes. Use an S3-compatible repository or a filesystem repository on the same platform.",
            error.Message);
        backupPolicies.Verify(x => x.UpdateAsync(It.IsAny<BackupPolicy>(), It.IsAny<CancellationToken>()), Times.Never);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task RunBackupRestoreVolume_ShouldReturnRejectedWhenBackupRunDoesNotExist()
    {
        var backupRuns = new Mock<IBackupRunRepository>();
        backupRuns
            .Setup(x => x.GetAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((BackupRun?)null);
        var restoreRuns = new Mock<IBackupRestoreRunRepository>();
        var unitOfWork = CreateUnitOfWork(backupRuns.Object, restoreRuns.Object);
        var executionService = new Mock<IBackupRestoreRunExecutionService>();
        var notifications = new List<INotificationWorkItem>();
        var notificationQueue = CreateNotificationQueue(notifications);
        var restoreRunStreamManager = new Mock<IBackupRestoreRunStreamManager>();
        var handler = new RunBackupRestoreVolumeHandler(
            unitOfWork.Object,
            executionService.Object,
            CreateUserContextAccessor(),
            restoreRunStreamManager.Object,
            notificationQueue.Object);

        var items = await ToListAsync(handler.Handle(
            new RunBackupRestoreVolume(
                Guid.CreateVersion7(),
                new RestoreVolumeInputModel(Guid.CreateVersion7(), "restored-data", OverwriteExisting: false)),
            TestContext.Current.CancellationToken));

        var item = Assert.Single(items);
        Assert.Equal(BackupRestoreStatus.Rejected, item.Status);
        Assert.Equal("Backup run not found.", item.Message);
        restoreRuns.Verify(x => x.AddAsync(It.IsAny<BackupRestoreRun>(), It.IsAny<CancellationToken>()), Times.Never);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
        notificationQueue.Verify(x => x.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()), Times.Never);
        executionService.Verify(
            x => x.ExecuteQueuedAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task RunBackupRestoreVolume_ShouldQueueRestoreRunAndStreamExecution()
    {
        var backupRun = CreateSuccessfulBackupRun();
        BackupRestoreRun? restoreRun = null;
        var backupRuns = new Mock<IBackupRunRepository>();
        backupRuns
            .Setup(x => x.GetAsync(backupRun.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(backupRun);
        var restoreRuns = new Mock<IBackupRestoreRunRepository>();
        restoreRuns
            .Setup(x => x.AddAsync(It.IsAny<BackupRestoreRun>(), It.IsAny<CancellationToken>()))
            .Callback<BackupRestoreRun, CancellationToken>((run, _) => restoreRun = run)
            .ReturnsAsync(1);
        var unitOfWork = CreateUnitOfWork(backupRuns.Object, restoreRuns.Object);
        var executionService = new Mock<IBackupRestoreRunExecutionService>();
        executionService
            .Setup(x => x.ExecuteQueuedAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()))
            .Returns((Guid runId, CancellationToken _) => ToAsyncEnumerable(
                new BackupRestoreRunStreamItem(runId, BackupRestoreStatus.Running, "Restoring volume."),
                new BackupRestoreRunStreamItem(runId, BackupRestoreStatus.Succeeded, "Restore finished.")));
        var notifications = new List<INotificationWorkItem>();
        var notificationQueue = CreateNotificationQueue(notifications);
        var restoreRunStreamManager = new Mock<IBackupRestoreRunStreamManager>();
        var handler = new RunBackupRestoreVolumeHandler(
            unitOfWork.Object,
            executionService.Object,
            CreateUserContextAccessor(),
            restoreRunStreamManager.Object,
            notificationQueue.Object);

        var items = await ToListAsync(handler.Handle(
            new RunBackupRestoreVolume(
                backupRun.Id,
                new RestoreVolumeInputModel(Guid.CreateVersion7(), "restored-data", OverwriteExisting: true)),
            TestContext.Current.CancellationToken));

        Assert.NotNull(restoreRun);
        Assert.Collection(
            items,
            item =>
            {
                Assert.Equal(restoreRun.Id, item.RestoreRunId);
                Assert.Equal(BackupRestoreStatus.Queued, item.Status);
            },
            item =>
            {
                Assert.Equal(restoreRun.Id, item.RestoreRunId);
                Assert.Equal(BackupRestoreStatus.Running, item.Status);
            },
            item =>
            {
                Assert.Equal(restoreRun.Id, item.RestoreRunId);
                Assert.Equal(BackupRestoreStatus.Succeeded, item.Status);
            });
        restoreRuns.Verify(x => x.AddAsync(It.IsAny<BackupRestoreRun>(), It.IsAny<CancellationToken>()), Times.Once);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
        Assert.Single(notifications);
        Assert.IsType<BackupRestoreRunNotificationWorkItem>(notifications[0]);
        executionService.Verify(x => x.ExecuteQueuedAsync(restoreRun.Id, It.IsAny<CancellationToken>()), Times.Once);
    }

    private static CreateBackupPolicy CreatePolicyCommand(Guid repositoryId, BackupSourceSpec source)
        => new(new BackupPolicyInputModel(
            Name: "daily-volume-backup",
            Description: null,
            Source: source,
            BackupRepositoryId: repositoryId,
            Enabled: true,
            Cron: null,
            TimeZone: null,
            Webhook: null,
            KeepLastSuccessful: BackupPolicy.DefaultKeepLastSuccessful,
            TimeoutSeconds: BackupPolicy.DefaultTimeoutSeconds,
            AlertOnFailure: true,
            RunAsActorId: null,
            TagIds: []));

    private static CreateBackupPolicyHandler CreateCreateHandler(IUnitOfWork unitOfWork)
        => new(
            unitOfWork,
            CreateUserContextAccessor(),
            Mock.Of<IStackBackupVolumeResolver>(),
            Mock.Of<IDeploymentBackupVolumeResolver>(),
            new PermissiveLicenseEntitlementService());

    private static UpdateBackupPolicyHandler CreateUpdateHandler(IUnitOfWork unitOfWork)
        => new(
            unitOfWork,
            Mock.Of<IStackBackupVolumeResolver>(),
            Mock.Of<IDeploymentBackupVolumeResolver>(),
            new PermissiveLicenseEntitlementService());

    private static Mock<IBackupPolicyRepository> CreateBackupPolicyRepository()
    {
        var backupPolicies = new Mock<IBackupPolicyRepository>();
        backupPolicies
            .Setup(x => x.ExistsByNormalizedNameAsync(It.IsAny<string>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(false);
        return backupPolicies;
    }

    private static Mock<IBackupPolicyRepository> CreateBackupPolicyRepository(BackupPolicy policy)
    {
        var backupPolicies = CreateBackupPolicyRepository();
        backupPolicies
            .Setup(x => x.GetAsync(policy.Id, It.IsAny<CancellationToken>(), It.IsAny<bool>()))
            .ReturnsAsync(policy);
        return backupPolicies;
    }

    private static Mock<IUnitOfWork> CreateUnitOfWork(
        BackupRepository repository,
        IBackupPolicyRepository backupPolicies,
        IPlatformRepository platforms)
    {
        var repositories = new Mock<IBackupRepositoryRepository>();
        repositories
            .Setup(x => x.GetAsync(repository.Id, It.IsAny<CancellationToken>(), It.IsAny<bool>()))
            .ReturnsAsync(repository);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.BackupRepositories).Returns(repositories.Object);
        unitOfWork.Setup(x => x.BackupPolicies).Returns(backupPolicies);
        unitOfWork.Setup(x => x.Platforms).Returns(platforms);
        unitOfWork.Setup(x => x.Stacks).Returns(Mock.Of<IStackRepository>());
        unitOfWork.Setup(x => x.Deployments).Returns(Mock.Of<IDeploymentRepository>());
        unitOfWork.Setup(x => x.CommitAsync(It.IsAny<CancellationToken>())).Returns(Task.CompletedTask);
        return unitOfWork;
    }

    private static Mock<IUnitOfWork> CreateUnitOfWork(
        IBackupRunRepository backupRuns,
        IBackupRestoreRunRepository backupRestoreRuns)
    {
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.BackupRuns).Returns(backupRuns);
        unitOfWork.Setup(x => x.BackupRestoreRuns).Returns(backupRestoreRuns);
        unitOfWork.Setup(x => x.CommitAsync(It.IsAny<CancellationToken>())).Returns(Task.CompletedTask);
        return unitOfWork;
    }

    private static Mock<IUnitOfWork> CreateUnitOfWork(IBackupRunRepository backupRuns)
    {
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.BackupRuns).Returns(backupRuns);
        unitOfWork.Setup(x => x.CommitAsync(It.IsAny<CancellationToken>())).Returns(Task.CompletedTask);
        return unitOfWork;
    }

    private static Mock<INotificationQueue> CreateNotificationQueue(List<INotificationWorkItem> notifications)
    {
        var notificationQueue = new Mock<INotificationQueue>();
        notificationQueue
            .Setup(x => x.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()))
            .Callback<INotificationWorkItem, CancellationToken>((item, _) => notifications.Add(item))
            .Returns(ValueTask.CompletedTask);
        return notificationQueue;
    }

    private static BackupRepository CreateRepository(BackupRepositorySpec spec)
        => new(
            name: "repo",
            description: null,
            spec: spec,
            passwordSecretId: Guid.CreateVersion7(),
            createdByActorId: Guid.CreateVersion7(),
            status: BackupRepositoryStatus.Ready);

    private static BackupPolicy CreatePolicy(Guid repositoryId, BackupSourceSpec source)
        => new(
            name: "policy",
            description: null,
            source: source,
            backupRepositoryId: repositoryId,
            enabled: true,
            cron: null,
            timeZone: null,
            webhook: null,
            keepLastSuccessful: BackupPolicy.DefaultKeepLastSuccessful,
            timeoutSeconds: BackupPolicy.DefaultTimeoutSeconds,
            alertOnFailure: true,
            runAsActorId: Guid.CreateVersion7(),
            createdByActorId: Guid.CreateVersion7());

    private static BackupRun CreateSuccessfulBackupRun()
        => new(
            backupPolicyId: Guid.CreateVersion7(),
            backupRepositoryId: Guid.CreateVersion7(),
            policyNameSnapshot: "policy",
            sourceSnapshot: new DockerVolumeBackupSource(Guid.CreateVersion7(), "app-data"),
            repositoryTypeSnapshot: BackupRepositoryType.FileSystem,
            trigger: BackupRunTrigger.Manual,
            triggerSourceId: null,
            triggeredByActorId: Guid.CreateVersion7(),
            status: BackupRunStatus.Succeeded,
            resticSnapshotId: "snapshot-01",
            snapshotAvailability: BackupSnapshotAvailability.Available,
            completedAt: DateTimeOffset.UtcNow);

    private static BackupRun CreateQueuedBackupRun()
        => new(
            backupPolicyId: Guid.CreateVersion7(),
            backupRepositoryId: Guid.CreateVersion7(),
            policyNameSnapshot: "policy",
            sourceSnapshot: new CitadelSystemBackupSource(),
            repositoryTypeSnapshot: BackupRepositoryType.FileSystem,
            trigger: BackupRunTrigger.Manual,
            triggerSourceId: null,
            triggeredByActorId: Guid.CreateVersion7());

    private static async Task<List<T>> ToListAsync<T>(IAsyncEnumerable<T> stream)
    {
        var items = new List<T>();
        await foreach (var item in stream)
            items.Add(item);
        return items;
    }

    private static async IAsyncEnumerable<T> ToAsyncEnumerable<T>(params T[] items)
    {
        foreach (var item in items)
            yield return item;

        await Task.CompletedTask;
    }

    private static IUserContextAccessor CreateUserContextAccessor()
    {
        var user = new Mock<IUserContext>();
        user.SetupGet(x => x.UserId).Returns(Guid.CreateVersion7());
        user.SetupGet(x => x.ActorId).Returns(Guid.CreateVersion7());
        user.SetupGet(x => x.IsAdmin).Returns(true);
        user.SetupGet(x => x.IsAuthenticated).Returns(true);
        user.SetupGet(x => x.Roles).Returns(["admin"]);

        var accessor = new Mock<IUserContextAccessor>();
        accessor.SetupGet(x => x.Current).Returns(user.Object);
        return accessor.Object;
    }
}
