using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Backups;
using Domain.Entities.Backups;
using Domain.Entities.Platforms;
using Domain.Entities.ResourceBindings;
using Domain.Entities.Stacks;
using Domain.Entities.Tags;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Infrastructure.Persistence;

public sealed class BackupRepositoryTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task BackupRepositories_ShouldPersistValidationPoliciesTagsAndRuns()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var actorId = Constants.SystemId;

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var passwordSecret = new SecretDefinition("RESTIC_PASSWORD_TEST", SecretProviderType.InternalEncrypted);
        await uow.SecretDefinitions.AddAsync(
            passwordSecret,
            new InternalSecretValue(passwordSecret.Id, "encrypted-value"),
            cancellationToken);

        var repository = new BackupRepository(
            "repo-integration",
            "Repository integration test",
            new FileSystemBackupRepositorySpec(BackupExecutionLocation.Core, null, "/backup"),
            passwordSecret.Id,
            actorId);
        await uow.BackupRepositories.AddAsync(repository, cancellationToken);

        var validation = new BackupRepositoryValidation(
            repository.Id,
            BackupExecutionLocation.Core,
            platformId: null,
            BackupRepositoryValidationStatus.Unknown,
            DateTimeOffset.UtcNow,
            "backup.not_implemented",
            "Not implemented in foundation.");
        await uow.BackupRepositoryValidations.UpsertAsync(validation, cancellationToken);

        var tag = Tag.Create("backup-policy-test", "#3366FF", actorId);
        await uow.Tags.AddAsync(tag, cancellationToken);

        var policy = new BackupPolicy(
            "policy-integration",
            "Policy integration test",
            new CitadelSystemBackupSource(),
            repository.Id,
            enabled: true,
            cron: null,
            timeZone: null,
            keepLastSuccessful: BackupPolicy.DefaultKeepLastSuccessful,
            timeoutSeconds: BackupPolicy.DefaultTimeoutSeconds,
            alertOnFailure: true,
            runAsActorId: actorId,
            createdByActorId: actorId);
        var policyRows = await uow.BackupPolicies.AddAsync(policy, cancellationToken, [tag.Id], actorId);

        var run = new BackupRun(
            policy.Id,
            repository.Id,
            policy.Name,
            policy.Source,
            repository.Type,
            BackupRunTrigger.Manual,
            triggerSourceId: null,
            triggeredByActorId: actorId);
        await uow.BackupRuns.AddAsync(run, cancellationToken);
        await uow.CommitAsync(cancellationToken);

        var storedRepository = await uow.BackupRepositories.GetAsync(repository.Id, cancellationToken);
        var storedValidation = await uow.BackupRepositoryValidations.GetAsync(repository.Id, BackupExecutionLocation.Core, null, cancellationToken);
        var taggedPolicies = (await uow.BackupPolicies.GetAllAsync(cancellationToken, [tag.Id])).ToArray();
        var runs = (await uow.BackupRuns.GetByPolicyAsync(policy.Id, 50, cancellationToken)).ToArray();

        var cancelledRows = await uow.BackupRuns.CancelQueuedOrRunningAsync(
            run.Id,
            DateTimeOffset.UtcNow,
            "Cancelled by test.",
            cancellationToken);
        await uow.CommitAsync(cancellationToken);

        var cancelledRun = await uow.BackupRuns.GetAsync(run.Id, cancellationToken);

        Assert.True(policyRows > 0);
        Assert.NotNull(storedRepository);
        Assert.Equal("REPO-INTEGRATION", storedRepository.NormalizedName);
        Assert.NotNull(storedValidation);
        Assert.Equal(BackupRepositoryValidationStatus.Unknown, storedValidation.Status);
        var taggedPolicy = Assert.Single(taggedPolicies);
        Assert.Equal(policy.Id, taggedPolicy.Id);
        Assert.Contains(taggedPolicy.Tags, x => x.Id == tag.Id);
        Assert.Single(runs);
        Assert.Equal(1, cancelledRows);
        Assert.Equal(BackupRunStatus.Cancelled, cancelledRun?.Status);
        Assert.Equal(BackupSnapshotAvailability.NotCreated, cancelledRun?.SnapshotAvailability);
    }

    [Fact]
    public async Task BackupRepositoryOptimizedPaths_ShouldPersistValidationArchiveAndQueueAtomically()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var actorId = Constants.SystemId;

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var passwordSecret = new SecretDefinition("RESTIC_PASSWORD_OPTIMIZED", SecretProviderType.InternalEncrypted);
        await uow.SecretDefinitions.AddAsync(
            passwordSecret,
            new InternalSecretValue(passwordSecret.Id, "encrypted-value"),
            cancellationToken);

        var repository = new BackupRepository(
            "repo-optimized",
            null,
            new FileSystemBackupRepositorySpec(BackupExecutionLocation.Core, null, "/backup"),
            passwordSecret.Id,
            actorId);
        await uow.BackupRepositories.AddAsync(repository, cancellationToken);

        var validation = new BackupRepositoryValidation(
            repository.Id,
            BackupExecutionLocation.Core,
            platformId: null,
            BackupRepositoryValidationStatus.Ready,
            DateTimeOffset.UtcNow,
            null,
            null);
        var validationRows = await uow.BackupRepositories.ApplyValidationResultAsync(
            validation,
            markChecked: true,
            markPruned: false,
            cancellationToken);

        var policy = new BackupPolicy(
            "policy-optimized",
            null,
            new CitadelSystemBackupSource(),
            repository.Id,
            enabled: true,
            cron: null,
            timeZone: null,
            keepLastSuccessful: BackupPolicy.DefaultKeepLastSuccessful,
            timeoutSeconds: BackupPolicy.DefaultTimeoutSeconds,
            alertOnFailure: true,
            runAsActorId: actorId,
            createdByActorId: actorId);
        await uow.BackupPolicies.AddAsync(policy, cancellationToken);

        var firstQueue = await uow.BackupRuns.QueueAsync(
            policy.Id,
            Guid.CreateVersion7(),
            BackupRunTrigger.Manual,
            triggerSourceId: null,
            actorId,
            usePolicyActor: false,
            DateTimeOffset.UtcNow,
            cancellationToken);
        var secondQueue = await uow.BackupRuns.QueueAsync(
            policy.Id,
            Guid.CreateVersion7(),
            BackupRunTrigger.Manual,
            triggerSourceId: null,
            actorId,
            usePolicyActor: false,
            DateTimeOffset.UtcNow,
            cancellationToken);

        var archiveUsedRepository = await uow.BackupRepositories.ArchiveIfUnusedAsync(
            repository.Id,
            DateTimeOffset.UtcNow,
            cancellationToken);

        var unusedRepository = new BackupRepository(
            "repo-optimized-unused",
            null,
            new FileSystemBackupRepositorySpec(BackupExecutionLocation.Core, null, "/backup-unused"),
            passwordSecret.Id,
            actorId);
        await uow.BackupRepositories.AddAsync(unusedRepository, cancellationToken);
        var archiveUnusedRepository = await uow.BackupRepositories.ArchiveIfUnusedAsync(
            unusedRepository.Id,
            DateTimeOffset.UtcNow,
            cancellationToken);

        await uow.CommitAsync(cancellationToken);

        var storedRepository = await uow.BackupRepositories.GetAsync(repository.Id, cancellationToken);
        var storedValidation = await uow.BackupRepositoryValidations.GetAsync(repository.Id, BackupExecutionLocation.Core, null, cancellationToken);
        var storedRuns = (await uow.BackupRuns.GetByPolicyAsync(policy.Id, 50, cancellationToken)).ToArray();
        var archivedRepository = await uow.BackupRepositories.GetAsync(unusedRepository.Id, cancellationToken, includeArchived: true);

        Assert.True(validationRows > 0);
        Assert.Equal(BackupRepositoryStatus.Ready, storedRepository?.Status);
        Assert.NotNull(storedRepository?.LastCheckedAt);
        Assert.NotNull(storedValidation);
        Assert.Equal(BackupRepositoryValidationStatus.Ready, storedValidation.Status);
        Assert.Equal(BackupRunQueueResultStatus.Queued, firstQueue.Status);
        Assert.NotNull(firstQueue.Run);
        Assert.Equal(BackupRunQueueResultStatus.ActiveRunExists, secondQueue.Status);
        Assert.Single(storedRuns);
        Assert.Equal(BackupRepositoryArchiveResult.ActiveOperation, archiveUsedRepository);
        Assert.Equal(BackupRepositoryArchiveResult.Archived, archiveUnusedRepository);
        Assert.NotNull(archivedRepository?.ArchivedAt);
    }

    [Fact]
    public async Task BackupPolicyVolumeCoverage_ShouldReturnCoverageForRequestedVolumesInOneQuery()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var actorId = Constants.SystemId;
        var platformId = Guid.CreateVersion7();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var passwordSecret = new SecretDefinition("RESTIC_PASSWORD_COVERAGE", SecretProviderType.InternalEncrypted);
        await uow.SecretDefinitions.AddAsync(
            passwordSecret,
            new InternalSecretValue(passwordSecret.Id, "encrypted-value"),
            cancellationToken);

        var repository = new BackupRepository(
            "repo-coverage",
            null,
            new FileSystemBackupRepositorySpec(BackupExecutionLocation.Core, null, "/backup-coverage"),
            passwordSecret.Id,
            actorId);
        await uow.BackupRepositories.AddAsync(repository, cancellationToken);
        await uow.BackupRepositoryValidations.UpsertAsync(
            new BackupRepositoryValidation(
                repository.Id,
                BackupExecutionLocation.Core,
                platformId: null,
                BackupRepositoryValidationStatus.Ready,
                DateTimeOffset.UtcNow,
                null,
                null),
            cancellationToken);

        var protectedPolicy = await AddVolumePolicyAsync(uow, repository.Id, platformId, "protected-volume", enabled: true, actorId, cancellationToken);
        var protectedRun = CreateRun(protectedPolicy, repository);
        protectedRun.MarkRunning(DateTimeOffset.UtcNow);
        protectedRun.CompleteSucceeded("snapshot-protected", null, 1, 1, 1, [], DateTimeOffset.UtcNow);
        await uow.BackupRuns.AddAsync(protectedRun, cancellationToken);

        var warningPolicy = await AddVolumePolicyAsync(uow, repository.Id, platformId, "warning-volume", enabled: true, actorId, cancellationToken);
        var warningRun = CreateRun(warningPolicy, repository);
        warningRun.MarkRunning(DateTimeOffset.UtcNow);
        warningRun.CompleteSucceeded(
            "snapshot-warning",
            null,
            1,
            1,
            1,
            [new BackupRunWarning("backup.warning", "Warning")],
            DateTimeOffset.UtcNow);
        await uow.BackupRuns.AddAsync(warningRun, cancellationToken);

        var failedPolicy = await AddVolumePolicyAsync(uow, repository.Id, platformId, "failed-volume", enabled: true, actorId, cancellationToken);
        var failedRun = CreateRun(failedPolicy, repository);
        failedRun.MarkRunning(DateTimeOffset.UtcNow);
        failedRun.Fail(BackupRunStatus.Failed, 1, "backup.failed", "Backup failed.", DateTimeOffset.UtcNow);
        await uow.BackupRuns.AddAsync(failedRun, cancellationToken);

        await AddVolumePolicyAsync(uow, repository.Id, platformId, "disabled-volume", enabled: false, actorId, cancellationToken);
        await uow.CommitAsync(cancellationToken);

        var coverage = await uow.BackupPolicies.GetVolumeCoverageAsync(
            [
                new VolumeBackupCoverageKey(platformId, "protected-volume"),
                new VolumeBackupCoverageKey(platformId, "warning-volume"),
                new VolumeBackupCoverageKey(platformId, "failed-volume"),
                new VolumeBackupCoverageKey(platformId, "disabled-volume"),
                new VolumeBackupCoverageKey(platformId, "unprotected-volume")
            ],
            userId: null,
            ResourceType.BackupPolicy,
            PermissionLevel.Read,
            SpecificPermission.None,
            cancellationToken);

        var byVolume = coverage.ToDictionary(static item => item.Resource.VolumeName, static item => item.Coverage);

        Assert.Equal(BackupCoverageStatus.Protected, byVolume["protected-volume"].Status);
        Assert.Equal(BackupRunStatus.Succeeded, byVolume["protected-volume"].LastRunStatus);
        Assert.Equal(BackupCoverageStatus.Warning, byVolume["warning-volume"].Status);
        Assert.Equal(BackupRunStatus.SucceededWithWarnings, byVolume["warning-volume"].LastRunStatus);
        Assert.Equal(BackupCoverageStatus.Failed, byVolume["failed-volume"].Status);
        Assert.Equal(BackupRunStatus.Failed, byVolume["failed-volume"].LastRunStatus);
        Assert.Equal(BackupCoverageStatus.Warning, byVolume["disabled-volume"].Status);
        Assert.Equal(BackupCoverageStatus.Unprotected, byVolume["unprotected-volume"].Status);
    }

    [Fact]
    public async Task BackupRunItemsAndStackVolumeBindings_ShouldRoundTrip()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var actorId = Constants.SystemId;

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var platform = new Platform(
            "backup-bindings-platform",
            "unix:///var/run/docker.sock",
            networkCount: 0,
            volumeCount: 0,
            imageCount: 0,
            cpuCount: 1,
            memTotal: 1024,
            serverVersion: "test",
            agentVersion: null,
            PlatformStatus.Online,
            PlatformConnectorType.Local,
            new DockerPlatformDescriptor("daemon", 0, 0, 0, 0));
        await uow.Platforms.AddAsync(platform, cancellationToken);

        var passwordSecret = new SecretDefinition("RESTIC_PASSWORD_RUN_ITEMS", SecretProviderType.InternalEncrypted);
        await uow.SecretDefinitions.AddAsync(
            passwordSecret,
            new InternalSecretValue(passwordSecret.Id, "encrypted-value"),
            cancellationToken);

        var repository = new BackupRepository(
            "repo-run-items",
            null,
            new FileSystemBackupRepositorySpec(BackupExecutionLocation.Core, null, "/backup-run-items"),
            passwordSecret.Id,
            actorId);
        await uow.BackupRepositories.AddAsync(repository, cancellationToken);

        var policy = new BackupPolicy(
            "policy-run-items",
            null,
            new StackBackupSource(Guid.CreateVersion7()),
            repository.Id,
            enabled: true,
            cron: null,
            timeZone: null,
            keepLastSuccessful: BackupPolicy.DefaultKeepLastSuccessful,
            timeoutSeconds: BackupPolicy.DefaultTimeoutSeconds,
            alertOnFailure: true,
            runAsActorId: actorId,
            createdByActorId: actorId);
        await uow.BackupPolicies.AddAsync(policy, cancellationToken);

        var run = CreateRun(policy, repository);
        run.MarkPreparing(DateTimeOffset.UtcNow);
        run.MarkRunning(DateTimeOffset.UtcNow);
        run.CompleteSucceeded(null, null, 3, 1024, 256, [], DateTimeOffset.UtcNow);
        await uow.BackupRuns.AddAsync(run, cancellationToken);

        var item = new BackupRunItem(run.Id, platform.Id, "stack-db-data");
        item.MarkRunning(DateTimeOffset.UtcNow);
        item.CompleteSucceeded("snapshot-stack-db", null, 3, 1024, 256, DateTimeOffset.UtcNow);
        var pendingItem = new BackupRunItem(run.Id, platform.Id, "stack-cache");
        await uow.BackupRunItems.AddRangeAsync([item, pendingItem], cancellationToken);
        await uow.BackupRunItems.CancelPendingOrRunningAsync(run.Id, DateTimeOffset.UtcNow, cancellationToken);

        var stack = Stack.Create(
            "stack-bindings",
            actorId,
            StackSource.WebEditor,
            platform.Id,
            new ManualStack(
                """
                services:
                  db:
                    image: postgres
                    volumes:
                      - db-data:/var/lib/postgresql/data
                volumes:
                  db-data:
                """,
                StackUpdateBehavior.Disabled));
        await uow.Stacks.AddAsync(stack, cancellationToken);
        await uow.Stacks.ReplaceReleaseVolumeBindingsAsync(
            stack.CurrentStackReleaseId,
            [
                new StackReleaseVolumeBinding(
                    stack.CurrentStackReleaseId,
                    platform.Id,
                    "stack-bindings_db-data",
                    "db-data",
                    isExternal: false,
                    isAnonymous: false)
            ],
            cancellationToken);

        await uow.CommitAsync(cancellationToken);

        var storedRun = await uow.BackupRuns.GetAsync(run.Id, cancellationToken);
        var storedRuns = (await uow.BackupRuns.GetByPolicyAsync(policy.Id, 50, cancellationToken)).ToArray();
        var storedItems = await uow.BackupRunItems.GetByRunAsync(run.Id, cancellationToken);
        var storedBindings = await uow.Stacks.GetReleaseVolumeBindingsAsync(stack.CurrentStackReleaseId, cancellationToken);

        Assert.NotNull(storedRun);
        Assert.Equal(2, storedRun.Items.Count);
        var storedItem = Assert.Single(storedRun.Items, stored => stored.VolumeName == "stack-db-data");
        Assert.Equal("stack-db-data", storedItem.VolumeName);
        Assert.Equal("snapshot-stack-db", storedItem.ResticSnapshotId);
        var cancelledItem = Assert.Single(storedRun.Items, stored => stored.VolumeName == "stack-cache");
        Assert.Equal(BackupRunItemStatus.Cancelled, cancelledItem.Status);
        Assert.Single(storedRuns);
        Assert.Equal(2, storedRuns[0].Items.Count);
        Assert.Equal(2, storedItems.Count);

        var binding = Assert.Single(storedBindings);
        Assert.Equal("stack-bindings_db-data", binding.VolumeName);
        Assert.Equal("db-data", binding.ComposeVolumeName);
    }

    private static async Task<BackupPolicy> AddVolumePolicyAsync(
        IUnitOfWork uow,
        Guid repositoryId,
        Guid platformId,
        string volumeName,
        bool enabled,
        Guid actorId,
        CancellationToken cancellationToken)
    {
        var policy = new BackupPolicy(
            $"policy-{volumeName}",
            null,
            new DockerVolumeBackupSource(platformId, volumeName),
            repositoryId,
            enabled,
            cron: null,
            timeZone: null,
            keepLastSuccessful: BackupPolicy.DefaultKeepLastSuccessful,
            timeoutSeconds: BackupPolicy.DefaultTimeoutSeconds,
            alertOnFailure: true,
            runAsActorId: actorId,
            createdByActorId: actorId);
        await uow.BackupPolicies.AddAsync(policy, cancellationToken);
        return policy;
    }

    private static BackupRun CreateRun(BackupPolicy policy, BackupRepository repository)
        => new(
            policy.Id,
            repository.Id,
            policy.Name,
            policy.Source,
            repository.Type,
            BackupRunTrigger.Manual,
            triggerSourceId: null,
            triggeredByActorId: Constants.SystemId);
}
