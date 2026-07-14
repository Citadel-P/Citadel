using Domain;
using Domain.Entities.Backups;

namespace Tests.Unit.Domain.Entities.Backups;

public sealed class BackupEntitiesTests
{
    [Fact]
    public void BackupRepository_ShouldNormalizeNameAndS3Prefix()
    {
        var repository = new BackupRepository(
            name: "  nightly repo  ",
            description: "  backups  ",
            spec: new S3CompatibleBackupRepositorySpec(
                new Uri("https://minio.example.com"),
                "  citadel  ",
                " /prod/backups/ ",
                "  eu-west-1  ",
                S3BucketLookup.Auto,
                Guid.NewGuid(),
                Guid.NewGuid(),
                null,
                false),
            passwordSecretId: Guid.NewGuid(),
            createdByActorId: Guid.NewGuid());

        repository.Validate();

        var spec = Assert.IsType<S3CompatibleBackupRepositorySpec>(repository.Spec);
        Assert.Equal("nightly repo", repository.Name);
        Assert.Equal("NIGHTLY REPO", repository.NormalizedName);
        Assert.Equal("backups", repository.Description);
        Assert.Equal("citadel", spec.Bucket);
        Assert.Equal("prod/backups", spec.Prefix);
        Assert.Equal("eu-west-1", spec.Region);
    }

    [Fact]
    public void BackupRepository_ShouldRejectLocationChangeAfterReady()
    {
        var repository = new BackupRepository(
            "repo-1",
            null,
            new FileSystemBackupRepositorySpec(BackupExecutionLocation.Core, null, "/backups"),
            Guid.NewGuid(),
            Guid.NewGuid());

        repository.MarkReady(DateTimeOffset.UtcNow);

        var ex = Assert.Throws<InvalidOperationException>(() =>
            repository.Update(
                null,
                new FileSystemBackupRepositorySpec(BackupExecutionLocation.Core, null, "/other")));

        Assert.Equal("Ready backup repository location cannot be changed.", ex.Message);
    }

    [Fact]
    public void BackupPolicy_ShouldArchiveAndDisableScheduling()
    {
        var policy = CreatePolicy();

        policy.Archive(DateTimeOffset.UtcNow);

        Assert.False(policy.Enabled);
        Assert.NotNull(policy.ArchivedAt);
    }

    [Fact]
    public void BackupPolicy_ShouldRejectSourceChangeAfterFirstSuccess()
    {
        var policy = CreatePolicy();
        policy.MarkFirstSuccessfulRun(DateTimeOffset.UtcNow);

        var ex = Assert.Throws<InvalidOperationException>(() =>
            policy.Update(
                description: null,
                source: new DockerVolumeBackupSource(Guid.NewGuid(), "other"),
                backupRepositoryId: null,
                enabled: null,
                cron: null,
                updateCron: false,
                timeZone: null,
                updateTimeZone: false,
                keepLastSuccessful: null,
                timeoutSeconds: null,
                alertOnFailure: null,
                runAsActorId: null));

        Assert.Equal("Backup policy source cannot be changed after a successful run.", ex.Message);
    }

    [Fact]
    public void BackupPolicy_ShouldAcceptStackSource()
    {
        var stackId = Guid.NewGuid();
        var policy = CreatePolicy(new StackBackupSource(stackId));

        policy.Validate();

        var source = Assert.IsType<StackBackupSource>(policy.Source);
        Assert.Equal(stackId, source.StackId);
        Assert.Equal($"stack:{stackId}", source.StableKey);
    }

    [Fact]
    public void BackupPolicy_ShouldRejectStackSourceWithoutStackId()
    {
        var policy = CreatePolicy(new StackBackupSource(Guid.Empty));

        var ex = Assert.Throws<ArgumentException>(policy.Validate);

        Assert.Equal("Stack backup source requires a stack ID. (Parameter 'source')", ex.Message);
    }

    [Fact]
    public void BackupPolicy_ShouldAcceptDeploymentSource()
    {
        var deploymentId = Guid.NewGuid();
        var policy = CreatePolicy(new DeploymentBackupSource(deploymentId));

        policy.Validate();

        var source = Assert.IsType<DeploymentBackupSource>(policy.Source);
        Assert.Equal(deploymentId, source.DeploymentId);
        Assert.Equal($"deployment:{deploymentId}", source.StableKey);
    }

    [Fact]
    public void BackupPolicy_ShouldRejectDeploymentSourceWithoutDeploymentId()
    {
        var policy = CreatePolicy(new DeploymentBackupSource(Guid.Empty));

        var ex = Assert.Throws<ArgumentException>(policy.Validate);

        Assert.Equal("Deployment backup source requires a deployment ID. (Parameter 'source')", ex.Message);
    }

    [Fact]
    public void BackupRun_ShouldSetSnapshotAvailabilityOnSuccessAndFailure()
    {
        var run = CreateRun();
        var now = DateTimeOffset.UtcNow;

        run.MarkPreparing(now);
        run.MarkRunning(now.AddSeconds(1));
        run.CompleteSucceeded("snap-1", null, 12, 1024, 256, [], now.AddSeconds(2));

        Assert.Equal(BackupRunStatus.Succeeded, run.Status);
        Assert.Equal(BackupSnapshotAvailability.Available, run.SnapshotAvailability);
        Assert.Equal("snap-1", run.ResticSnapshotId);

        run.MarkSnapshotExpired();

        Assert.Equal(BackupSnapshotAvailability.Expired, run.SnapshotAvailability);

        var failed = CreateRun();
        failed.Fail(BackupRunStatus.Failed, 1, "backup.failed", "Restic failed.", now);

        Assert.Equal(BackupRunStatus.Failed, failed.Status);
        Assert.Equal(BackupSnapshotAvailability.NotCreated, failed.SnapshotAvailability);
    }

    [Fact]
    public void BackupRun_ShouldAllowMultiItemSuccessWithoutParentSnapshot()
    {
        var run = CreateRun(new StackBackupSource(Guid.NewGuid()));
        var now = DateTimeOffset.UtcNow;

        run.MarkPreparing(now);
        run.MarkRunning(now.AddSeconds(1));
        run.CompleteSucceeded(null, null, 4, 2048, 512, [], now.AddSeconds(2));

        Assert.Equal(BackupRunStatus.Succeeded, run.Status);
        Assert.Equal(BackupSnapshotAvailability.Available, run.SnapshotAvailability);
        Assert.Null(run.ResticSnapshotId);
        Assert.Equal(4, run.FilesProcessed);
    }

    [Fact]
    public void BackupRunItem_ShouldTrackVolumeSnapshotLifecycle()
    {
        var item = new BackupRunItem(Guid.NewGuid(), Guid.NewGuid(), "  postgres-data  ");
        var now = DateTimeOffset.UtcNow;

        item.MarkRunning(now);
        item.CompleteSucceeded("snapshot-volume-1", "parent-1", 3, 1024, 256, now.AddSeconds(1));

        Assert.Equal("postgres-data", item.VolumeName);
        Assert.Equal(BackupRunItemStatus.Succeeded, item.Status);
        Assert.Equal("snapshot-volume-1", item.ResticSnapshotId);
        Assert.Equal("parent-1", item.ParentSnapshotId);
        Assert.Equal(3, item.FilesProcessed);
    }

    [Fact]
    public void BackupRestoreRun_ShouldRecordWarningsAsSucceededWithWarnings()
    {
        var restore = new BackupRestoreRun(
            Guid.NewGuid(),
            Guid.NewGuid(),
            Guid.NewGuid(),
            "volume-restored",
            overwriteExisting: false,
            triggeredByActorId: Guid.NewGuid());
        var now = DateTimeOffset.UtcNow;

        restore.MarkPreparing(now);
        restore.MarkRunning(now.AddSeconds(1));
        restore.CompleteSucceeded(
            targetVolumeCreatedByCitadel: true,
            affectedContainers: [],
            warnings: [new BackupRunWarning("backup.restore.cleanup_failed", "Cleanup failed.")],
            now.AddSeconds(2));

        Assert.Equal(BackupRestoreStatus.SucceededWithWarnings, restore.Status);
        Assert.True(restore.TargetVolumeCreatedByCitadel);
        Assert.Single(restore.Warnings);
    }

    private static BackupPolicy CreatePolicy(BackupSourceSpec? source = null)
        => new(
            "  policy-1  ",
            " policy description ",
            source ?? new DockerVolumeBackupSource(Guid.NewGuid(), "  postgres-data  "),
            Guid.NewGuid(),
            enabled: true,
            cron: "0 2 * * *",
            timeZone: "UTC",
            keepLastSuccessful: BackupPolicy.DefaultKeepLastSuccessful,
            timeoutSeconds: BackupPolicy.DefaultTimeoutSeconds,
            alertOnFailure: true,
            runAsActorId: Guid.NewGuid(),
            createdByActorId: Guid.NewGuid());

    private static BackupRun CreateRun(BackupSourceSpec? source = null)
        => new(
            Guid.NewGuid(),
            Guid.NewGuid(),
            "policy-1",
            source ?? new DockerVolumeBackupSource(Guid.NewGuid(), "postgres-data"),
            BackupRepositoryType.S3Compatible,
            BackupRunTrigger.Manual,
            null,
            Guid.NewGuid());
}
