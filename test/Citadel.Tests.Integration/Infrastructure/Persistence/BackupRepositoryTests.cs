using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Backups;
using Domain.Entities.Backups;
using Domain.Entities.Deployments;
using Domain.Entities.Platforms;
using Domain.Entities.ResourceBindings;
using Domain.Entities.Stacks;
using Domain.Entities.SwarmServices;
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
            webhook: null,
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

        var cancelled = await uow.BackupRuns.CancelQueuedOrRunningAsync(
            run.Id,
            DateTimeOffset.UtcNow,
            "Cancelled by test.",
            cancellationToken);
        await uow.CommitAsync(cancellationToken);

        var cancelledRun = await uow.BackupRuns.GetAsync(run.Id, cancellationToken);
        var latestRun = await uow.BackupRuns.GetLatestByPolicyAsync(policy.Id, cancellationToken);
        var latestRuns = await uow.BackupRuns.GetLatestByPoliciesAsync(
            [policy.Id, Guid.CreateVersion7()],
            cancellationToken);

        Assert.True(policyRows > 0);
        Assert.NotNull(storedRepository);
        Assert.Equal("REPO-INTEGRATION", storedRepository.NormalizedName);
        Assert.NotNull(storedValidation);
        Assert.Equal(BackupRepositoryValidationStatus.Unknown, storedValidation.Status);
        var taggedPolicy = Assert.Single(taggedPolicies);
        Assert.Equal(policy.Id, taggedPolicy.Id);
        Assert.Contains(taggedPolicy.Tags, x => x.Id == tag.Id);
        Assert.Single(runs);
        Assert.NotNull(cancelled);
        Assert.Equal(run.Id, cancelled.Id);
        Assert.Equal(BackupRunStatus.Cancelled, cancelledRun?.Status);
        Assert.Equal(BackupSnapshotAvailability.NotCreated, cancelledRun?.SnapshotAvailability);
        Assert.Equal(run.Id, latestRun?.Id);
        Assert.Equal(BackupRunStatus.Cancelled, latestRun?.Status);
        Assert.Equal(run.Id, Assert.Single(latestRuns).Value.Id);
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
            webhook: null,
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
        var nodePolicy = await AddPolicyAsync(
            uow,
            repository.Id,
            "policy-node-volume-1",
            new DockerVolumeBackupSource(platformId, "node-volume", DockerNodeId: "node-1"),
            enabled: true,
            actorId,
            cancellationToken);
        var nodeRun = CreateRun(nodePolicy, repository);
        nodeRun.MarkRunning(DateTimeOffset.UtcNow);
        nodeRun.CompleteSucceeded("snapshot-node-1", null, 1, 1, 1, [], DateTimeOffset.UtcNow);
        await uow.BackupRuns.AddAsync(nodeRun, cancellationToken);
        await uow.CommitAsync(cancellationToken);

        var coverage = await uow.BackupPolicies.GetVolumeCoverageAsync(
            [
                new VolumeBackupCoverageKey(platformId, "protected-volume"),
                new VolumeBackupCoverageKey(platformId, "warning-volume"),
                new VolumeBackupCoverageKey(platformId, "failed-volume"),
                new VolumeBackupCoverageKey(platformId, "disabled-volume"),
                new VolumeBackupCoverageKey(platformId, "unprotected-volume"),
                new VolumeBackupCoverageKey(platformId, "node-volume", "node-1"),
                new VolumeBackupCoverageKey(platformId, "node-volume", "node-2")
            ],
            userId: null,
            ResourceType.BackupPolicy,
            PermissionLevel.Read,
            SpecificPermission.None,
            cancellationToken);

        var byVolume = coverage
            .Where(static item => item.Resource.VolumeName != "node-volume")
            .ToDictionary(static item => item.Resource.VolumeName, static item => item.Coverage);

        Assert.Equal(BackupCoverageStatus.Protected, byVolume["protected-volume"].Status);
        Assert.Equal(BackupRunStatus.Succeeded, byVolume["protected-volume"].LastRunStatus);
        Assert.Equal(BackupCoverageStatus.Warning, byVolume["warning-volume"].Status);
        Assert.Equal(BackupRunStatus.SucceededWithWarnings, byVolume["warning-volume"].LastRunStatus);
        Assert.Equal(BackupCoverageStatus.Failed, byVolume["failed-volume"].Status);
        Assert.Equal(BackupRunStatus.Failed, byVolume["failed-volume"].LastRunStatus);
        Assert.Equal(BackupCoverageStatus.Warning, byVolume["disabled-volume"].Status);
        Assert.Equal(BackupCoverageStatus.Unprotected, byVolume["unprotected-volume"].Status);
        var nodeCoverage = coverage.Where(static item => item.Resource.VolumeName == "node-volume").ToArray();
        Assert.Equal(2, nodeCoverage.Length);
        Assert.Equal(
            BackupCoverageStatus.Protected,
            Assert.Single(nodeCoverage, static item => item.Resource.DockerNodeId == "node-1").Coverage.Status);
        Assert.Equal(
            BackupCoverageStatus.Unprotected,
            Assert.Single(nodeCoverage, static item => item.Resource.DockerNodeId == "node-2").Coverage.Status);
    }

    [Fact]
    public async Task PlatformBackupSummaries_ShouldIncludeEveryPlatformScopedSourceAndLatestRunHealth()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var actorId = Constants.SystemId;

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var platform = CreatePlatform("backup-summary-platform");
        var otherPlatform = CreatePlatform(
            "backup-summary-other-platform",
            "unix:///var/run/backup-summary-other.sock");
        var swarmPlatform = CreateSwarmPlatform("backup-summary-swarm-platform");
        await uow.Platforms.AddAsync(platform, cancellationToken);
        await uow.Platforms.AddAsync(otherPlatform, cancellationToken);
        await uow.Platforms.AddAsync(swarmPlatform, cancellationToken);

        var passwordSecret = new SecretDefinition("RESTIC_PASSWORD_PLATFORM_SUMMARY", SecretProviderType.InternalEncrypted);
        await uow.SecretDefinitions.AddAsync(
            passwordSecret,
            new InternalSecretValue(passwordSecret.Id, "encrypted-value"),
            cancellationToken);

        var repository = new BackupRepository(
            "repo-platform-summary",
            null,
            new FileSystemBackupRepositorySpec(BackupExecutionLocation.Core, null, "/backup-platform-summary"),
            passwordSecret.Id,
            actorId);
        await uow.BackupRepositories.AddAsync(repository, cancellationToken);

        await AddVolumePolicyAsync(
            uow,
            repository.Id,
            platform.Id,
            "summary-volume",
            enabled: true,
            actorId,
            cancellationToken);

        var swarmService = new SwarmService(
            "backup-summary-service",
            swarmPlatform.Id,
            actorId,
            new SwarmServiceSpec
            {
                Image = new SwarmExternalImage(Constants.DefaultRegistryId, "nginx:latest"),
                SchedulingMode = SwarmServiceSchedulingMode.Replicated,
                Replicas = 1
            });
        await uow.SwarmServices.AddAsync(swarmService, cancellationToken);
        await AddPolicyAsync(
            uow,
            repository.Id,
            "policy-summary-swarm-service",
            new SwarmServiceBackupSource(swarmService.Id),
            enabled: true,
            actorId,
            cancellationToken);

        var stack = Stack.Create(
            "backup-summary-stack",
            actorId,
            StackSource.WebEditor,
            platform.Id,
            new ManualStack("services: {}", StackUpdateBehavior.Disabled));
        await uow.Stacks.AddAsync(stack, cancellationToken);
        await AddPolicyAsync(
            uow,
            repository.Id,
            "policy-summary-stack",
            new StackBackupSource(stack.Id),
            enabled: false,
            actorId,
            cancellationToken);

        var deployment = new Deployment(
            "backup-summary-deployment",
            actorId,
            platform.Id,
            new DeploymentSpec(
                new ExternalImage(Constants.DefaultRegistryId, "example/app:latest"),
                UpdateBehavior.Disabled));
        await uow.Deployments.AddAsync(deployment, cancellationToken);
        var deploymentPolicy = await AddPolicyAsync(
            uow,
            repository.Id,
            "policy-summary-deployment",
            new DeploymentBackupSource(deployment.Id),
            enabled: true,
            actorId,
            cancellationToken);

        await AddPolicyAsync(
            uow,
            repository.Id,
            "policy-summary-citadel",
            new CitadelSystemBackupSource(),
            enabled: true,
            actorId,
            cancellationToken);
        await AddVolumePolicyAsync(
            uow,
            repository.Id,
            otherPlatform.Id,
            "summary-other-volume",
            enabled: true,
            actorId,
            cancellationToken);

        var failedRun = CreateRun(deploymentPolicy, repository);
        failedRun.MarkRunning(DateTimeOffset.UtcNow);
        failedRun.Fail(
            BackupRunStatus.Failed,
            exitCode: 1,
            errorCode: "backup.failed",
            errorMessage: "Backup failed.",
            DateTimeOffset.UtcNow);
        await uow.BackupRuns.AddAsync(failedRun, cancellationToken);
        await uow.CommitAsync(cancellationToken);

        var summaries = await uow.BackupPolicies.GetPlatformSummariesAsync(
            [platform.Id, otherPlatform.Id, swarmPlatform.Id],
            userId: null,
            ResourceType.BackupPolicy,
            PermissionLevel.Read,
            SpecificPermission.None,
            cancellationToken);

        var summary = Assert.Single(summaries, item => item.PlatformId == platform.Id);
        Assert.Equal(3, summary.PolicyCount);
        Assert.Equal(2, summary.EnabledPolicyCount);
        Assert.Equal(1, summary.DockerVolumePolicyCount);
        Assert.Equal(1, summary.StackPolicyCount);
        Assert.Equal(1, summary.DeploymentPolicyCount);
        Assert.Equal(1, summary.AttentionPolicyCount);
        Assert.Equal(BackupRunStatus.Failed, summary.LastRunStatus);
        Assert.NotNull(summary.LastRunAt);

        var otherSummary = Assert.Single(summaries, item => item.PlatformId == otherPlatform.Id);
        Assert.Equal(1, otherSummary.PolicyCount);
        Assert.Equal(1, otherSummary.DockerVolumePolicyCount);
        Assert.Null(otherSummary.LastRunStatus);

        var swarmSummary = Assert.Single(summaries, item => item.PlatformId == swarmPlatform.Id);
        Assert.Equal(1, swarmSummary.PolicyCount);
        Assert.Equal(1, swarmSummary.SwarmServicePolicyCount);
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
            webhook: null,
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

        var item = new BackupRunItem(
            run.Id,
            platform.Id,
            "stack-db-data",
            dockerNodeId: "node-1",
            nodeHostname: "worker-1");
        item.MarkRunning(DateTimeOffset.UtcNow);
        item.CompleteSucceeded("snapshot-stack-db", null, 3, 1024, 256, DateTimeOffset.UtcNow);
        var pendingItem = new BackupRunItem(
            run.Id,
            platform.Id,
            "stack-cache",
            dockerNodeId: "node-2",
            nodeHostname: "worker-2");
        await uow.BackupRunItems.AddRangeAsync([item, pendingItem], cancellationToken);
        await uow.BackupRunItems.CancelPendingOrRunningAsync(run.Id, DateTimeOffset.UtcNow, cancellationToken);

        var restoreRun = new BackupRestoreRun(
            run.Id,
            repository.Id,
            platform.Id,
            "stack-db-data-restored",
            overwriteExisting: false,
            triggeredByActorId: actorId,
            targetDockerNodeId: "node-3",
            targetNodeHostname: "worker-3",
            sourceBackupRunItemId: item.Id);
        await uow.BackupRestoreRuns.AddAsync(restoreRun, cancellationToken);

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
        var storedRestoreRun = await uow.BackupRestoreRuns.GetAsync(restoreRun.Id, cancellationToken);
        var storedBindings = await uow.Stacks.GetReleaseVolumeBindingsAsync(stack.CurrentStackReleaseId, cancellationToken);

        Assert.NotNull(storedRun);
        Assert.Equal(2, storedRun.Items.Count);
        var storedItem = Assert.Single(storedRun.Items, stored => stored.VolumeName == "stack-db-data");
        Assert.Equal("stack-db-data", storedItem.VolumeName);
        Assert.Equal("snapshot-stack-db", storedItem.ResticSnapshotId);
        Assert.Equal("node-1", storedItem.DockerNodeId);
        Assert.Equal("worker-1", storedItem.NodeHostname);
        var cancelledItem = Assert.Single(storedRun.Items, stored => stored.VolumeName == "stack-cache");
        Assert.Equal(BackupRunItemStatus.Cancelled, cancelledItem.Status);
        Assert.Single(storedRuns);
        Assert.Equal(2, storedRuns[0].Items.Count);
        Assert.Equal(2, storedItems.Count);
        Assert.NotNull(storedRestoreRun);
        Assert.Equal(item.Id, storedRestoreRun.SourceBackupRunItemId);
        Assert.Equal("node-3", storedRestoreRun.TargetDockerNodeId);
        Assert.Equal("worker-3", storedRestoreRun.TargetNodeHostname);

        var binding = Assert.Single(storedBindings);
        Assert.Equal("stack-bindings_db-data", binding.VolumeName);
        Assert.Equal("db-data", binding.ComposeVolumeName);
    }

    [Fact]
    public async Task BackupRestoreRuns_ShouldFilterByPolicyInDatabase()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var actorId = Constants.SystemId;

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var passwordSecret = new SecretDefinition("RESTIC_PASSWORD_RESTORE_POLICY_FILTER", SecretProviderType.InternalEncrypted);
        await uow.SecretDefinitions.AddAsync(
            passwordSecret,
            new InternalSecretValue(passwordSecret.Id, "encrypted-value"),
            cancellationToken);

        var repository = new BackupRepository(
            "repo-restore-policy-filter",
            null,
            new FileSystemBackupRepositorySpec(BackupExecutionLocation.Core, null, "/backup-restore-policy-filter"),
            passwordSecret.Id,
            actorId);
        await uow.BackupRepositories.AddAsync(repository, cancellationToken);

        var platform = CreatePlatform("backup-restore-policy-filter-platform");
        await uow.Platforms.AddAsync(platform, cancellationToken);

        var policy = await AddVolumePolicyAsync(uow, repository.Id, platform.Id, "policy-volume", enabled: true, actorId, cancellationToken);
        var otherPolicy = await AddVolumePolicyAsync(uow, repository.Id, platform.Id, "other-policy-volume", enabled: true, actorId, cancellationToken);

        var firstRun = CreateCompletedRun(policy, repository, "snapshot-filter-001");
        var secondRun = CreateCompletedRun(policy, repository, "snapshot-filter-002");
        var otherRun = CreateCompletedRun(otherPolicy, repository, "snapshot-filter-other");
        await uow.BackupRuns.AddAsync(firstRun, cancellationToken);
        await uow.BackupRuns.AddAsync(secondRun, cancellationToken);
        await uow.BackupRuns.AddAsync(otherRun, cancellationToken);

        var firstRestoreRun = new BackupRestoreRun(firstRun.Id, repository.Id, platform.Id, "target-one", false, actorId);
        var secondRestoreRun = new BackupRestoreRun(secondRun.Id, repository.Id, platform.Id, "target-two", false, actorId);
        var otherRestoreRun = new BackupRestoreRun(otherRun.Id, repository.Id, platform.Id, "target-other", false, actorId);
        await uow.BackupRestoreRuns.AddAsync(firstRestoreRun, cancellationToken);
        await uow.BackupRestoreRuns.AddAsync(secondRestoreRun, cancellationToken);
        await uow.BackupRestoreRuns.AddAsync(otherRestoreRun, cancellationToken);
        await uow.CommitAsync(cancellationToken);

        var restoreRuns = (await uow.BackupRestoreRuns.GetByPolicyAsync(policy.Id, 50, cancellationToken)).ToArray();

        Assert.Equal(2, restoreRuns.Length);
        Assert.Contains(restoreRuns, run => run.Id == firstRestoreRun.Id);
        Assert.Contains(restoreRuns, run => run.Id == secondRestoreRun.Id);
        Assert.DoesNotContain(restoreRuns, run => run.Id == otherRestoreRun.Id);
    }

    private static async Task<BackupPolicy> AddVolumePolicyAsync(
        IUnitOfWork uow,
        Guid repositoryId,
        Guid platformId,
        string volumeName,
        bool enabled,
        Guid actorId,
        CancellationToken cancellationToken)
        => await AddPolicyAsync(
            uow,
            repositoryId,
            $"policy-{volumeName}",
            new DockerVolumeBackupSource(platformId, volumeName),
            enabled,
            actorId,
            cancellationToken);

    private static async Task<BackupPolicy> AddPolicyAsync(
        IUnitOfWork uow,
        Guid repositoryId,
        string name,
        BackupSourceSpec source,
        bool enabled,
        Guid actorId,
        CancellationToken cancellationToken)
    {
        var policy = new BackupPolicy(
            name,
            null,
            source,
            repositoryId,
            enabled,
            cron: null,
            timeZone: null,
            webhook: null,
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

    private static Platform CreatePlatform(string name, string address = "unix:///var/run/docker.sock")
        => new(
            name,
            address,
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

    private static Platform CreateSwarmPlatform(string name)
        => new(
            name,
            "http://localhost.docker",
            networkCount: 0,
            volumeCount: 0,
            imageCount: 0,
            cpuCount: 1,
            memTotal: 1024,
            serverVersion: "test",
            agentVersion: "test",
            PlatformStatus.Online,
            PlatformConnectorType.EdgeAgent,
            new DockerSwarmPlatformDescriptor(
                "manager-1",
                "10.0.0.1",
                "Active",
                true,
                1,
                1,
                "daemon",
                0,
                0,
                0,
                0));

    private static BackupRun CreateCompletedRun(BackupPolicy policy, BackupRepository repository, string snapshotId)
    {
        var run = CreateRun(policy, repository);
        run.MarkRunning(DateTimeOffset.UtcNow);
        run.CompleteSucceeded(snapshotId, null, 1, 1, 1, [], DateTimeOffset.UtcNow);
        return run;
    }
}
