using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Backups;
using Domain.Contracts.Resources.Identity;
using Domain.Contracts.Resources.Oidc;
using Domain.Contracts.Resources.Platforms;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities;
using Domain.Entities.Activities;
using Domain.Entities.Alerts;
using Domain.Entities.Automation;
using Domain.Entities.Backups;
using Domain.Entities.Builds;
using Domain.Entities.ResourceBindings;
using Domain.Entities.Tags;
using Domain.Entities.Deployments;
using Domain.Entities.Git;
using Domain.Entities.Identity;
using Domain.Entities.Licensing;
using Domain.Entities.Oidc;
using Domain.Entities.Platforms;
using Domain.Entities.Registries;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.Models;
using LightResults;

namespace Domain.Contracts.Interfaces;

public interface IUnitOfWork : IAsyncDisposable
{
    IUserRepository Users { get; }
    ITeamRepository Teams { get; }
    IRoleRepository Roles { get; }
    IActorRepository Actors { get; }
    IResourceAccessRepository ResourceAccesses { get; }
    IImageRepository Images { get; }
    IPlatformRepository Platforms { get; }
    IEdgeAgentRepository EdgeAgents { get; }
    IRegistryRepository Registries { get; }
    IGitAccountRepository GitAccounts { get; }
    IGitReposRepository GitRepositories { get; }
    IContainerRepository Containers { get; }
    IAlertRuleRepository AlertRules { get; }
    IAlertEventRepository AlertEvents { get; }
    IDeploymentRepository Deployments { get; }
    IStackRepository Stacks { get; }
    IResourceBindingRepository ResourceBindings { get; }
    ISecretDefinitionRepository SecretDefinitions { get; }
    ISecretProviderRepository SecretProviders { get; }
    ITagRepository Tags { get; }
    IResourceTagRepository ResourceTags { get; }
    IOidcProviderRepository OidcProviders { get; }
    IOidcLoginStateRepository OidcLoginStates { get; }
    IOidcExternalLoginRepository OidcExternalLogins { get; }
    IAutomationActionRepository AutomationActions { get; }
    IActionRunRepository ActionRuns { get; }
    IBackupRepositoryRepository BackupRepositories { get; }
    IBackupRepositoryValidationRepository BackupRepositoryValidations { get; }
    IBackupPolicyRepository BackupPolicies { get; }
    IBackupRunRepository BackupRuns { get; }
    IBackupRunItemRepository BackupRunItems { get; }
    IBackupRunLogRepository BackupRunLogs { get; }
    IBackupRestoreRunRepository BackupRestoreRuns { get; }
    IBackupRestoreRunLogRepository BackupRestoreRunLogs { get; }
    IBackupRepositoryLeaseRepository BackupRepositoryLeases { get; }
    IBackupSourceLeaseRepository BackupSourceLeases { get; }
    IBuildProjectRepository BuildProjects { get; }
    IBuildAgentPoolRepository BuildAgentPools { get; }
    IBuildRunRepository BuildRuns { get; }
    IBuildRunLogRepository BuildRunLogs { get; }
    IRefreshTokenRepository RefreshTokens { get; }
    IUserMfaRepository UserMfa { get; }
    IMfaChallengeRepository MfaChallenges { get; }
    IUserPreferencesRepository UserPreferences { get; }
    IInstanceIdentityRepository InstanceIdentity { get; }
    IInstalledLicenseRepository InstalledLicense { get; }
    ILicenseUsageRepository LicenseUsage { get; }
    IPlatformStatRepository PlatformStats { get; }
    IContainerStatRepository ContainerStats { get; }
    IActivityEventRepository ActivityEventRepository { get; }

    Task CommitAsync(CancellationToken cancellationToken);
    Task RollbackAsync();
}

public interface IAutomationActionRepository
{
    Task<int> AddAsync(AutomationAction action, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null, Guid? tagCreatedByActorId = null);
    Task<int> UpdateAsync(AutomationAction action, CancellationToken cancellationToken);
    Task<int> DeleteAsync(Guid id, CancellationToken cancellationToken);
    Task<AutomationAction?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<AutomationAction>> GetAllAsync(CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null);
    Task<IEnumerable<AutomationAction>> GetScheduledAsync(CancellationToken cancellationToken);
    Task<IEnumerable<AutomationAction>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null);
    Task<bool> ExistsByNameAsync(string name, CancellationToken cancellationToken);
    Task<bool> ExistsByNameExceptAsync(string name, Guid id, CancellationToken cancellationToken);
    Task<bool> CanAccessAsync(Guid userId, Guid id, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken);
    Task<bool> TryMarkScheduledAsync(Guid id, DateTime scheduledMinuteUtc, CancellationToken cancellationToken);
    Task<int> MarkProcessingAsync(Guid id, Guid runId, CancellationToken cancellationToken);
    Task<int> MarkIdleAsync(Guid id, Guid runId, CancellationToken cancellationToken);
    Task<IEnumerable<AutomationAction>> GetStuckActionsAsync(CancellationToken cancellationToken = default);
    Task<int> UpdateProcessingAsync(Guid id, ResourceControlState state, long? startedAt, long rowVersion, bool checkRowVersion, Guid? currentRunId, CancellationToken cancellationToken);
}

public interface IActionRunRepository
{
    Task<int> AddAsync(ActionRun run, CancellationToken cancellationToken);
    Task<int> UpdateAsync(ActionRun run, CancellationToken cancellationToken);
    Task<ActionRun?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<ActionRun>> GetByActionAsync(Guid actionId, int limit, CancellationToken cancellationToken);
    Task<IEnumerable<ActionRun>> GetQueuedAsync(int limit, CancellationToken cancellationToken);
    Task<ActionRun?> GetLatestByActionAsync(Guid actionId, CancellationToken cancellationToken);
    Task<bool> HasActiveRunAsync(Guid actionId, CancellationToken cancellationToken);
    Task<bool> TryMarkRunningAsync(Guid id, DateTime startedAt, CancellationToken cancellationToken);
    Task<int> CancelQueuedOrRunningAsync(Guid id, DateTime cancelledAt, string reason, CancellationToken cancellationToken);
    Task<int> RemoveCompletedOlderThanAsync(DateTime completedBefore, CancellationToken cancellationToken);
}

public interface IAutomationProcessRunner
{
    IAsyncEnumerable<AutomationProcessOutput> StreamAsync(
        string fileName,
        IEnumerable<string> arguments,
        IDictionary<string, string>? environmentVariables,
        string workingDirectory,
        CancellationToken cancellationToken);
}

public sealed record AutomationProcessOutput(string? StdOut, string? StdErr, int? ExitCode = null);

public interface IBuildProcessRunner
{
    IAsyncEnumerable<BuildProcessEvent> RunAsync(
        BuildProcessCommand command,
        CancellationToken cancellationToken);
}

public sealed record BuildProcessCommand(
    string PlatformAddress,
    PlatformConnectorType PlatformConnectorType,
    string WorkingDirectory,
    string ContextPath,
    string DockerfilePath,
    string? Target,
    IReadOnlyList<string> ImageReferences,
    IReadOnlyList<BuildProcessBuildArg> BuildArgs,
    IReadOnlyList<BuildProcessSecret> Secrets,
    BuildProcessRegistryCredential? RegistryCredential,
    TimeSpan Timeout,
    int MaxLineBytes = 16_384);

public sealed record BuildProcessBuildArg(string Name, string Value);

public sealed record BuildProcessSecret(string Id, string Value);

public sealed record BuildProcessRegistryCredential(
    string RegistryHost,
    string RegistryAuth);

public enum BuildProcessStream
{
    StdOut,
    StdErr,
    Exit
}

public sealed record BuildProcessEvent(
    BuildProcessStream Stream,
    string? Message = null,
    int? ExitCode = null,
    string? Digest = null);

public interface IBackupRepositoryRepository
{
    Task<int> AddAsync(BackupRepository repository, CancellationToken cancellationToken);
    Task<int> UpdateAsync(BackupRepository repository, CancellationToken cancellationToken);
    Task<int> ArchiveAsync(Guid id, DateTimeOffset archivedAt, CancellationToken cancellationToken);
    Task<BackupRepositoryArchiveResult> ArchiveIfUnusedAsync(Guid id, DateTimeOffset archivedAt, CancellationToken cancellationToken);
    Task<int> ApplyValidationResultAsync(BackupRepositoryValidation validation, bool markChecked, bool markPruned, CancellationToken cancellationToken);
    Task<BackupRepository?> GetAsync(Guid id, CancellationToken cancellationToken, bool includeArchived = false);
    Task<IEnumerable<BackupRepository>> GetAllAsync(CancellationToken cancellationToken, bool includeArchived = false);
    Task<IEnumerable<ResourceInfo>> GetAuthorizedLookupAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken);
    Task<bool> ExistsByNormalizedNameAsync(string normalizedName, CancellationToken cancellationToken);
    Task<bool> ExistsByNormalizedNameExceptAsync(string normalizedName, Guid id, CancellationToken cancellationToken);
    Task<bool> HasActiveOperationAsync(Guid id, CancellationToken cancellationToken);
    Task<bool> HasNonArchivedPolicyAsync(Guid id, CancellationToken cancellationToken);
    Task<int> MarkProcessingAsync(Guid id, Guid runId, CancellationToken cancellationToken);
    Task<int> MarkIdleAsync(Guid id, Guid runId, CancellationToken cancellationToken);
    Task<IEnumerable<BackupRepository>> GetStuckRepositoriesAsync(int staleAfterSeconds = 3600, CancellationToken cancellationToken = default);
    Task<int> UpdateProcessingAsync(Guid id, ResourceControlState state, long? startedAt, long rowVersion, bool checkRowVersion, Guid? currentRunId, CancellationToken cancellationToken);
}

public interface IBackupRepositoryValidationRepository
{
    Task<int> UpsertAsync(BackupRepositoryValidation validation, CancellationToken cancellationToken);
    Task<BackupRepositoryValidation?> GetAsync(Guid repositoryId, BackupExecutionLocation location, Guid? platformId, CancellationToken cancellationToken);
    Task<IEnumerable<BackupRepositoryValidation>> GetByRepositoryAsync(Guid repositoryId, CancellationToken cancellationToken);
}

public interface IBackupPolicyRepository
{
    Task<int> AddAsync(BackupPolicy policy, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null, Guid? tagCreatedByActorId = null);
    Task<int> UpdateAsync(BackupPolicy policy, CancellationToken cancellationToken);
    Task<int> ArchiveAsync(Guid id, DateTimeOffset archivedAt, CancellationToken cancellationToken);
    Task<BackupPolicy?> GetAsync(Guid id, CancellationToken cancellationToken, bool includeArchived = false);
    Task<IEnumerable<BackupPolicy>> GetAllAsync(CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null, bool includeArchived = false);
    Task<IEnumerable<ScheduledBackupPolicy>> GetScheduledAsync(DateTimeOffset scheduledMinuteUtc, CancellationToken cancellationToken);
    Task<IEnumerable<BackupPolicy>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null);
    Task<bool> ExistsByNormalizedNameAsync(string normalizedName, CancellationToken cancellationToken);
    Task<bool> ExistsByNormalizedNameExceptAsync(string normalizedName, Guid id, CancellationToken cancellationToken);
    Task<bool> CanAccessAsync(Guid userId, Guid id, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken);
    Task<IReadOnlyList<VolumeBackupCoverage>> GetVolumeCoverageAsync(
        IReadOnlyCollection<VolumeBackupCoverageKey> volumes,
        Guid? userId,
        ResourceType resourceType,
        PermissionLevel permissionLevel,
        SpecificPermission specificPermission,
        CancellationToken cancellationToken);
    Task<bool> TryMarkScheduledAsync(Guid id, DateTimeOffset scheduledMinuteUtc, CancellationToken cancellationToken);
    Task<int> MarkProcessingAsync(Guid id, Guid runId, CancellationToken cancellationToken);
    Task<int> MarkIdleAsync(Guid id, Guid runId, CancellationToken cancellationToken);
    Task<int> MarkIdleAfterRunAsync(Guid id, Guid runId, bool successful, DateTimeOffset completedAt, CancellationToken cancellationToken);
    Task<IEnumerable<BackupPolicy>> GetStuckPoliciesAsync(CancellationToken cancellationToken = default);
    Task<int> UpdateProcessingAsync(Guid id, ResourceControlState state, long? startedAt, long rowVersion, bool checkRowVersion, Guid? currentRunId, CancellationToken cancellationToken);
}

public interface IBackupRunRepository
{
    Task<int> AddAsync(BackupRun run, CancellationToken cancellationToken);
    Task<BackupRunQueueResult> QueueAsync(
        Guid policyId,
        Guid runId,
        BackupRunTrigger trigger,
        Guid? triggerSourceId,
        Guid triggeredByActorId,
        bool usePolicyActor,
        DateTimeOffset queuedAt,
        CancellationToken cancellationToken);
    Task<BackupRunQueueResult> QueueScheduledAsync(
        Guid policyId,
        Guid runId,
        DateTimeOffset scheduledMinuteUtc,
        CancellationToken cancellationToken);
    Task<int> UpdateAsync(BackupRun run, CancellationToken cancellationToken);
    Task<BackupRun?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<BackupRun>> GetByPolicyAsync(Guid policyId, int limit, CancellationToken cancellationToken);
    Task<IEnumerable<BackupRun>> GetPagedAsync(int limit, CancellationToken cancellationToken);
    Task<IEnumerable<BackupRun>> GetQueuedAsync(int limit, CancellationToken cancellationToken);
    Task<IReadOnlyList<Guid>> GetQueuedIdsAsync(int limit, CancellationToken cancellationToken);
    Task<BackupRunExecutionPlan?> GetExecutionPlanAsync(Guid id, CancellationToken cancellationToken);
    Task<BackupRunExecutionPlan?> TryClaimExecutionPlanAsync(Guid id, DateTimeOffset startedAt, CancellationToken cancellationToken);
    Task<BackupRunFinishOutcome> FinishRunAndMarkPolicyIdleAsync(BackupRun run, Guid policyId, bool successful, DateTimeOffset completedAt, CancellationToken cancellationToken);
    Task<bool> HasActiveRunAsync(Guid policyId, CancellationToken cancellationToken);
    Task<bool> TryMarkPreparingAsync(Guid id, DateTimeOffset startedAt, CancellationToken cancellationToken);
    Task<BackupRun?> CancelQueuedOrRunningAsync(Guid id, DateTimeOffset cancelledAt, string reason, CancellationToken cancellationToken);
}

public interface IBackupRunItemRepository
{
    Task<int> AddRangeAsync(IReadOnlyCollection<BackupRunItem> items, CancellationToken cancellationToken);
    Task<int> UpdateAsync(BackupRunItem item, CancellationToken cancellationToken);
    Task<IReadOnlyList<BackupRunItem>> GetByRunAsync(Guid backupRunId, CancellationToken cancellationToken);
    Task<IReadOnlyList<BackupRunItem>> GetByRunIdsAsync(IReadOnlyCollection<Guid> backupRunIds, CancellationToken cancellationToken);
    Task<int> CancelPendingOrRunningAsync(Guid backupRunId, DateTimeOffset cancelledAt, CancellationToken cancellationToken);
}

public interface IBackupRunLogRepository
{
    Task<int> AddAsync(BackupRunLogEntry entry, CancellationToken cancellationToken);
    Task<int> AddRangeAsync(IReadOnlyCollection<BackupRunLogEntry> entries, CancellationToken cancellationToken);
    Task<IReadOnlyList<BackupRunLogEntry>> GetByRunAsync(Guid backupRunId, CancellationToken cancellationToken);
}

public interface IBackupRestoreRunRepository
{
    Task<int> AddAsync(BackupRestoreRun run, CancellationToken cancellationToken);
    Task<int> UpdateAsync(BackupRestoreRun run, CancellationToken cancellationToken);
    Task<BackupRestoreRun?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<BackupRestoreRun>> GetPagedAsync(int limit, CancellationToken cancellationToken);
    Task<IEnumerable<BackupRestoreRun>> GetByBackupRunAsync(Guid backupRunId, int limit, CancellationToken cancellationToken);
    Task<IEnumerable<BackupRestoreRun>> GetByPolicyAsync(Guid policyId, int limit, CancellationToken cancellationToken);
    Task<IReadOnlyList<Guid>> GetQueuedIdsAsync(int limit, CancellationToken cancellationToken);
    Task<BackupRestoreRunExecutionPlan?> GetExecutionPlanAsync(Guid id, CancellationToken cancellationToken);
    Task<BackupRestoreRunExecutionPlan?> TryClaimExecutionPlanAsync(Guid id, DateTimeOffset startedAt, CancellationToken cancellationToken);
    Task<BackupRestoreRunFinishResult> FinishRunAsync(BackupRestoreRun run, DateTimeOffset completedAt, CancellationToken cancellationToken);
    Task<BackupRestoreRunWithPolicy?> CancelQueuedOrRunningAsync(Guid id, DateTimeOffset cancelledAt, string reason, CancellationToken cancellationToken);
}

public sealed record BackupRestoreRunWithPolicy(BackupRestoreRun Run, Guid BackupPolicyId);

public interface IBackupRestoreRunLogRepository
{
    Task<int> AddRangeAsync(IReadOnlyCollection<BackupRestoreRunLogEntry> entries, CancellationToken cancellationToken);
    Task<IReadOnlyList<BackupRestoreRunLogEntry>> GetByRunAsync(Guid restoreRunId, CancellationToken cancellationToken);
    Task<BackupRestoreRunLogs> GetByRunWithRunStateAsync(Guid restoreRunId, CancellationToken cancellationToken);
}

public interface IBuildProjectRepository
{
    Task<int> AddAsync(BuildProject project, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null, Guid? tagCreatedByActorId = null);
    Task<int> UpdateAsync(BuildProject project, CancellationToken cancellationToken);
    Task<int> ArchiveAsync(Guid id, DateTimeOffset archivedAt, CancellationToken cancellationToken);
    Task<BuildProject?> GetAsync(Guid id, CancellationToken cancellationToken, bool includeArchived = false);
    Task<IEnumerable<BuildProject>> GetAllAsync(CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null, bool includeArchived = false);
    Task<IEnumerable<BuildProject>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null);
    Task<bool> ExistsByNormalizedNameAsync(string normalizedName, CancellationToken cancellationToken);
    Task<bool> ExistsByNormalizedNameExceptAsync(string normalizedName, Guid id, CancellationToken cancellationToken);
    Task<bool> CanAccessAsync(Guid userId, Guid id, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken);
    Task<int> MarkProcessingAsync(Guid id, Guid runId, CancellationToken cancellationToken);
    Task<int> MarkIdleAsync(Guid id, Guid runId, CancellationToken cancellationToken);
    Task<IEnumerable<BuildProject>> GetStuckProjectsAsync(int graceSeconds = 300, CancellationToken cancellationToken = default);
    Task<int> UpdateProcessingAsync(Guid id, ResourceControlState state, long? startedAt, long rowVersion, bool checkRowVersion, Guid? currentRunId, CancellationToken cancellationToken);
}

public interface IBuildAgentPoolRepository
{
    Task<int> AddAsync(BuildAgentPool pool, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null, Guid? tagCreatedByActorId = null);
    Task<int> UpdateAsync(BuildAgentPool pool, CancellationToken cancellationToken);
    Task<int> ArchiveAsync(Guid id, DateTimeOffset archivedAt, CancellationToken cancellationToken);
    Task<BuildAgentPool?> GetAsync(Guid id, CancellationToken cancellationToken, bool includeArchived = false);
    Task<IEnumerable<BuildAgentPool>> GetAllAsync(CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null, bool includeArchived = false);
    Task<IEnumerable<BuildAgentPool>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null);
    Task<bool> ExistsByNormalizedNameAsync(string normalizedName, CancellationToken cancellationToken);
    Task<bool> ExistsByNormalizedNameExceptAsync(string normalizedName, Guid id, CancellationToken cancellationToken);
    Task<bool> CanAccessAsync(Guid userId, Guid id, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken);
}

public interface IBuildRunRepository
{
    Task<int> AddAsync(BuildRun run, CancellationToken cancellationToken);
    Task<int> UpdateAsync(BuildRun run, CancellationToken cancellationToken);
    Task<BuildRun?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<BuildRun>> GetByProjectAsync(Guid projectId, int limit, CancellationToken cancellationToken);
    Task<IEnumerable<BuildRun>> GetPagedAsync(int limit, CancellationToken cancellationToken);
    Task<BuildRun?> GetLatestByProjectAsync(Guid projectId, CancellationToken cancellationToken);
    Task<BuildRun?> GetLatestSuccessfulByProjectAsync(Guid projectId, CancellationToken cancellationToken);
    Task<IReadOnlyDictionary<Guid, BuildRun>> GetLatestByProjectsAsync(IReadOnlyCollection<Guid> projectIds, CancellationToken cancellationToken);
    Task<IEnumerable<BuildRun>> GetQueuedAsync(int limit, CancellationToken cancellationToken);
    Task<BuildRun?> TryClaimAsync(Guid id, DateTimeOffset startedAt, CancellationToken cancellationToken);
    Task<bool> HasActiveRunAsync(Guid projectId, CancellationToken cancellationToken);
    Task<int> CountActiveByBuildAgentPoolAsync(Guid buildAgentPoolId, CancellationToken cancellationToken);
    Task<BuildRun?> CancelQueuedOrRunningAsync(Guid id, DateTimeOffset cancelledAt, string reason, CancellationToken cancellationToken);
    Task<BuildRun?> InterruptQueuedOrRunningAsync(Guid id, DateTimeOffset interruptedAt, string reason, CancellationToken cancellationToken);
    Task<IReadOnlyList<BuildRun>> DeleteTerminalRunsBeyondRetentionAsync(Guid projectId, int keepRunCount, CancellationToken cancellationToken);
    Task<int> RemoveCompletedOlderThanAsync(DateTime completedBefore, CancellationToken cancellationToken);
}

public interface IBuildRunLogRepository
{
    Task<int> AddAsync(BuildRunLogEntry entry, CancellationToken cancellationToken);
    Task<int> AddRangeAsync(IReadOnlyCollection<BuildRunLogEntry> entries, CancellationToken cancellationToken);
    Task<IReadOnlyList<BuildRunLogEntry>> GetByRunAsync(Guid buildRunId, CancellationToken cancellationToken);
}

public interface IBackupRepositoryLeaseRepository
{
    Task<bool> TryAcquireAsync(
        Guid backupRepositoryId,
        string operationType,
        Guid ownerRunId,
        DateTimeOffset expiresAt,
        DateTimeOffset createdAt,
        CancellationToken cancellationToken);

    Task<int> ReleaseAsync(Guid backupRepositoryId, Guid ownerRunId, CancellationToken cancellationToken);
    Task<int> DeleteExpiredAsync(DateTimeOffset utcNow, CancellationToken cancellationToken);
    Task<BackupRepositoryLeaseAcquireResult> TryAcquireForExistingRepositoryAsync(
        Guid backupRepositoryId,
        string operationType,
        Guid ownerRunId,
        DateTimeOffset expiresAt,
        DateTimeOffset createdAt,
        CancellationToken cancellationToken);
}

public interface IBackupSourceLeaseRepository
{
    Task<bool> TryAcquireAsync(
        string sourceKey,
        string operationType,
        Guid ownerRunId,
        DateTimeOffset expiresAt,
        DateTimeOffset createdAt,
        CancellationToken cancellationToken);

    Task<int> ReleaseAsync(string sourceKey, Guid ownerRunId, CancellationToken cancellationToken);
    Task<int> DeleteExpiredAsync(DateTimeOffset utcNow, CancellationToken cancellationToken);
}

public enum BackupRepositoryArchiveResult
{
    Archived,
    NotFound,
    ActiveOperation,
    HasPolicies
}

public enum BackupRepositoryLeaseAcquireResult
{
    Acquired,
    NotFound,
    Busy
}

public enum BackupRunQueueResultStatus
{
    Queued,
    PolicyNotFound,
    PolicyArchived,
    ActiveRunExists,
    AlreadyScheduled
}

public sealed record BackupRunQueueResult(BackupRunQueueResultStatus Status, BackupRun? Run);

public sealed record ScheduledBackupPolicy(Guid Id, string? Cron, string? TimeZone);

public enum BackupRunFinishResult
{
    Completed,
    AlreadyCancelled
}

public sealed record BackupRunFinishOutcome(BackupRunFinishResult Status, BackupPolicy? Policy);

public sealed record BackupRunExecutionPlan(
    BackupRun Run,
    BackupPolicy Policy,
    BackupRepository Repository);

public enum BackupRestoreRunFinishResult
{
    Completed,
    AlreadyCancelled
}

public sealed record BackupRestoreRunExecutionPlan(
    BackupRestoreRun RestoreRun,
    BackupRun BackupRun,
    BackupRepository Repository);

public sealed record BackupRunLogEntry(
    Guid Id,
    Guid BackupRunId,
    DateTimeOffset CreatedAt,
    string Stream,
    string Message);

public sealed record BackupRestoreRunLogEntry(
    Guid Id,
    Guid BackupRestoreRunId,
    DateTimeOffset CreatedAt,
    string Stream,
    string Message);

public sealed record BackupRestoreRunLogs(
    bool RunExists,
    IReadOnlyList<BackupRestoreRunLogEntry> Logs);

public interface IResticProcessRunner
{
    IAsyncEnumerable<ResticProcessEvent> RunAsync(ResticProcessCommand command, CancellationToken cancellationToken);
}

public sealed record ResticProcessCommand(
    string FileName,
    IReadOnlyList<string> Arguments,
    IReadOnlyDictionary<string, string> Environment,
    string WorkingDirectory,
    TimeSpan Timeout,
    IReadOnlyCollection<string> RedactionValues,
    int MaxLineBytes);

public sealed record ResticProcessEvent(
    ResticProcessStream Stream,
    string? Message = null,
    int? ExitCode = null);

public enum ResticProcessStream
{
    StdOut,
    StdErr,
    Exit
}

public interface IInstanceIdentityRepository
{
    Task<CitadelInstanceIdentity> GetOrCreateAsync(
        Guid candidateInstanceId,
        DateTimeOffset createdAt,
        CancellationToken cancellationToken);

    Task<CitadelInstanceIdentity> GetOrCreateLockedAsync(
        Guid candidateInstanceId,
        DateTimeOffset createdAt,
        CancellationToken cancellationToken);

    Task<CitadelInstanceIdentity?> GetAsync(CancellationToken cancellationToken);
}

public interface IInstalledLicenseRepository
{
    Task<InstalledLicense?> GetAsync(CancellationToken cancellationToken);
    Task<int> UpsertAsync(InstalledLicense license, CancellationToken cancellationToken);
    Task<int> DeleteAsync(CancellationToken cancellationToken);
    Task<int> UpdateValidationStatusAsync(
        LicenseStatus status,
        DateTimeOffset validatedAt,
        string? validationErrorCode,
        CancellationToken cancellationToken);
}

public interface ILicenseUsageRepository
{
    Task<LicenseReadModel> GetLicenseReadModelAsync(CancellationToken cancellationToken);
}

public interface IOidcProviderRepository
{
    Task<int> AddAsync(OidcProvider provider, CancellationToken cancellationToken);
    Task<int> UpdateAsync(OidcProvider provider, CancellationToken cancellationToken);
    Task<int> DeleteAsync(Guid id, CancellationToken cancellationToken);
    Task<OidcProvider?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<OidcProvider>> GetAllAsync(CancellationToken cancellationToken);
    Task<IEnumerable<OidcProvider>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken);
    Task<IEnumerable<OidcProvider>> GetEnabledAsync(CancellationToken cancellationToken);
    Task<bool> ExistsByNameAsync(string name, CancellationToken cancellationToken);
    Task<bool> ExistsByNameExceptAsync(string name, Guid id, CancellationToken cancellationToken);
}

public interface IOidcLoginStateRepository
{
    Task<int> AddAsync(OidcLoginState state, CancellationToken cancellationToken);
    Task<OidcLoginState?> GetByStateHashAsync(string stateHash, CancellationToken cancellationToken);
    Task<int> DeleteAsync(Guid id, CancellationToken cancellationToken);
    Task<int> DeleteExpiredAsync(DateTime now, CancellationToken cancellationToken);
}

public interface IOidcExternalLoginRepository
{
    Task<OidcExternalLogin?> GetAsync(Guid providerId, string subject, CancellationToken cancellationToken);
    Task<int> AddAsync(OidcExternalLogin externalLogin, CancellationToken cancellationToken);
    Task<int> UpdateSeenAsync(Guid id, string? email, DateTime updatedAt, CancellationToken cancellationToken);
    Task<int> DeleteByProviderIdAsync(Guid providerId, CancellationToken cancellationToken);
}

public interface IResourceBindingRepository
{
    Task<int> AddAsync(ResourceBinding entry, CancellationToken cancellationToken);
    Task<int> UpdateAsync(ResourceBinding entry, CancellationToken cancellationToken);
    Task<int> DeleteAsync(Guid id, CancellationToken cancellationToken);
    Task<int> ReplaceEntriesAsync(ResourceBindingScope scope, Guid? resourceId, IEnumerable<ResourceBinding> entries, CancellationToken cancellationToken);
    Task<int> ReplaceResourceEntriesAsync(ResourceBindingScope scope, Guid resourceId, IEnumerable<ResourceBinding> entries, CancellationToken cancellationToken);
    Task<IEnumerable<ResourceBinding>> GetEntriesAsync(ResourceBindingScope scope, Guid? resourceId, CancellationToken cancellationToken);
    Task<IEnumerable<ResourceBinding>> GetEffectiveEntriesAsync(ResourceBindingScope scope, Guid resourceId, CancellationToken cancellationToken);
}

public interface ISecretDefinitionRepository
{
    Task<int> AddAsync(SecretDefinition secret, InternalSecretValue? value, CancellationToken cancellationToken);
    Task<int> UpdateAsync(SecretDefinition secret, CancellationToken cancellationToken);
    Task<int> DeleteAsync(Guid id, CancellationToken cancellationToken);
    Task<SecretDefinition?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<SecretDefinition>> GetAllAsync(CancellationToken cancellationToken);
    Task<IEnumerable<SecretDefinition>> GetBoundAsync(ResourceBindingScope scope, Guid? resourceId, CancellationToken cancellationToken);
    Task<InternalSecretValue?> GetInternalValueAsync(Guid secretId, CancellationToken cancellationToken);
    Task<IReadOnlyList<SecretResolutionMaterial>> GetResolutionMaterialsAsync(IReadOnlyCollection<Guid> secretIds, CancellationToken cancellationToken);
    Task<int> DeleteExternalByProviderIdAsync(Guid providerId, CancellationToken cancellationToken);
    Task<bool> ExistsByNameAsync(string name, CancellationToken cancellationToken);
    Task<bool> ExistsByNameExceptAsync(string name, Guid id, CancellationToken cancellationToken);
    Task<bool> IsUsedByResourceBindingAsync(Guid id, CancellationToken cancellationToken);
}

public sealed record SecretResolutionMaterial(
    SecretDefinition Definition,
    InternalSecretValue? InternalValue,
    SecretProvider? Provider);

public interface ISecretProviderRepository
{
    Task<int> AddAsync(SecretProvider provider, CancellationToken cancellationToken);
    Task<int> UpdateAsync(SecretProvider provider, CancellationToken cancellationToken);
    Task<int> DeleteAsync(Guid id, CancellationToken cancellationToken);
    Task<SecretProvider?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<SecretProvider>> GetAllAsync(CancellationToken cancellationToken);
    Task<bool> ExistsByNameAsync(string name, CancellationToken cancellationToken);
    Task<bool> ExistsByNameExceptAsync(string name, Guid id, CancellationToken cancellationToken);
    Task<bool> IsUsedByResourceBindingAsync(Guid id, CancellationToken cancellationToken);
}

public interface ITagRepository
{
    Task<IReadOnlyList<TagWithUsage>> ListAsync(CancellationToken cancellationToken);
    Task<Tag?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<Tag?> GetByNormalizedNameAsync(string normalizedName, CancellationToken cancellationToken);
    Task<bool> ExistsByNormalizedNameAsync(string normalizedName, CancellationToken cancellationToken);
    Task<bool> ExistsByNormalizedNameExceptAsync(string normalizedName, Guid id, CancellationToken cancellationToken);
    Task<int> AddAsync(Tag tag, CancellationToken cancellationToken);
    Task<int> UpdateAsync(Tag tag, CancellationToken cancellationToken);
    Task<int> DeleteAsync(Guid id, CancellationToken cancellationToken);
}

public interface IResourceTagRepository
{
    Task<IReadOnlyList<TagSummary>> GetForResourceAsync(
        TaggableResourceType resourceType,
        Guid resourceId,
        CancellationToken cancellationToken);

    Task<IReadOnlyDictionary<Guid, IReadOnlyList<TagSummary>>> GetForResourcesAsync(
        TaggableResourceType resourceType,
        IReadOnlyCollection<Guid> resourceIds,
        CancellationToken cancellationToken);

    Task<int> ReplaceForResourceAsync(
        TaggableResourceType resourceType,
        Guid resourceId,
        IReadOnlyCollection<Guid> tagIds,
        Guid createdByActorId,
        DateTime now,
        CancellationToken cancellationToken);

    Task<bool> AllTagsExistAsync(
        IReadOnlyCollection<Guid> tagIds,
        CancellationToken cancellationToken);
}

public interface IExternalSecretProviderClient
{
    Task<ExternalSecretProviderConnectionTestResult> TestConnectionAsync(
        SecretProvider provider,
        string token,
        CancellationToken cancellationToken);

    Task<ExternalSecretValueResult> ResolveAsync(
        SecretDefinition secret,
        SecretProvider provider,
        string token,
        CancellationToken cancellationToken);
}

public sealed record ExternalSecretProviderConnectionTestResult(bool Success, string Message)
{
    public static ExternalSecretProviderConnectionTestResult Succeeded(string message) => new(true, message);

    public static ExternalSecretProviderConnectionTestResult Failed(string message) => new(false, message);
}

public sealed record ExternalSecretValueResult(bool IsSuccess, string? Value, string? ErrorMessage)
{
    public static ExternalSecretValueResult Success(string value) => new(true, value, null);

    public static ExternalSecretValueResult Failure(string errorMessage) => new(false, null, errorMessage);
}

public interface IActorRepository
{
    Task<Actor?> GetById(Guid id, CancellationToken cancellationToken);
    Task<int> AddAsync(Actor actor, CancellationToken cancellationToken);
    Task<int> UpdateAsync(Actor actor, CancellationToken cancellationToken);
}

public interface IResourceAccessRepository
{
    Task<IEnumerable<ResourceAccessDetails>> GetAllByActorIdAsync(Guid actorId, CancellationToken cancellationToken);
    Task<int> AddAsync(ResourceAccess resourceAccess, CancellationToken cancellationToken);
    Task<int> RemoveAsync(
        Guid actorId,
        ResourceType resourceType,
        Guid resourceId,
        PermissionLevel permissionLevel,
        IEnumerable<SpecificPermission>? specificPermissions,
        CancellationToken cancellationToken);
    Task<int> ReplaceAsync(Guid actorId, IEnumerable<ResourceAccess> resourceAccesses, CancellationToken cancellationToken);
}

public interface IUserRepository 
{
    Task<PermissionMetadata> GetEffectivePermissionsAsync(
        Guid[] actorIds, 
        ResourceType resourceType, 
        Guid? resourceId, 
        CancellationToken ct);
    
    Task<IReadOnlyDictionary<Guid, PermissionMetadata>> GetEffectivePermissionsBatchAsync(
        Guid[] actorIds, 
        ResourceType resourceType, 
        Guid[] resourceIds, 
        CancellationToken ct = default);

    Task<Guid[]> GetActorScopeAsync(Guid userId, CancellationToken ct);
    Task<UserAuthInfo?> GetUserAuthInfoByEmailOrNameAsync(string emailOrName, CancellationToken cancellationToken);
    Task<UserAuthInfo?> GetUserAuthInfoByIdAsync(Guid id, CancellationToken cancellationToken);
    Task<UserAuthInfo?> GetUserAuthInfoByActorIdAsync(Guid actorId, CancellationToken cancellationToken);
    Task<UserAuthInfo?> GetUserAuthInfoByEmailAsync(string email, CancellationToken cancellationToken);
    Task<CurrentProfileDetails?> GetCurrentProfileAsync(Guid userId, CancellationToken cancellationToken);
    Task<User?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<UserDetails?> GetDetailsAsync(Guid userId, CancellationToken cancellationToken);
    Task<IEnumerable<User>> GetAllAsync(CancellationToken cancellationToken);
    Task<PagedResult<UserDetails>> GetPagedAsync(int page, int pageSize, string? name, CancellationToken cancellationToken);
    Task<PagedResult<UserDetails>> GetAuthorizedPagedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, int page, int pageSize, string? name, CancellationToken cancellationToken);
    Task<IEnumerable<UserSearchItem>> SearchAsync(string query, int limit, CancellationToken cancellationToken);
    Task<IEnumerable<UserSearchItem>> SearchAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, string query, int limit, CancellationToken cancellationToken);
    Task<bool> CanAccessAsync(Guid userId, Guid resourceId, CancellationToken cancellationToken);
    Task<IEnumerable<User>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<(bool NameExists, bool EmailExists)> GetConflictsAsync(string name, string email, Guid? excludeId, CancellationToken cancellationToken);
    Task<(User? User, bool IsEnabled, bool NameExists, bool EmailExists)> GetUserUpdateStateAsync(Guid id, string? name, string? email, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(Guid excludeId, CancellationToken cancellationToken);
    Task<bool> ExistsByNameAsync(string name, Guid? excludeId, CancellationToken cancellationToken);
    Task<bool> ExistsByEmailAsync(string email, Guid? excludeId, CancellationToken cancellationToken);
    Task<int> AddAsync(User user, CancellationToken cancellationToken);
    Task<int> UpdateAsync(User user, CancellationToken cancellationToken);
    Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<IEnumerable<ResourceInfo>> GetTeamsLookupAsync(Guid sourceUserId, Guid userId, CancellationToken cancellationToken);
    Task<IEnumerable<Guid>> GetTeamIdsAsync(Guid userId, CancellationToken cancellationToken);
    Task<int> ReplaceTeamsAsync(Guid userId, IEnumerable<Guid> teamIds, CancellationToken cancellationToken);
}

public interface IUserPreferencesRepository
{
    Task<UserPreferences?> GetAsync(Guid userId, CancellationToken cancellationToken);
    Task<int> UpsertAsync(UserPreferences preferences, CancellationToken cancellationToken);
}

public interface IRegistryRepository
{
    Task<Registry?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<Registry?> GetByNameAsync(string name, CancellationToken cancellationToken);
    Task<IEnumerable<Registry>> GetAllAsync(CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null);
    Task<IEnumerable<Registry>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null);
    Task<IEnumerable<Registry>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(string name, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(Guid id, string name, CancellationToken cancellationToken);
    Task<int> AddAsync(Registry registry, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null, Guid? tagCreatedByActorId = null);
    Task<int> UpdateAsync(Registry registry, CancellationToken cancellationToken);

    Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
}

public interface IStackRepository
{
    Task<Stack?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<Stack?> GetInfoAsync(Guid id, CancellationToken cancellationToken);
    Task<StackDriftStack?> GetDriftStackAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<StackDriftStack>> GetDriftMonitorStacksAsync(CancellationToken cancellationToken);
    Task<IEnumerable<string>> GetContainerIdsAsync(Guid stackId, CancellationToken cancellationToken);
    Task<IEnumerable<Stack>> GetAllAsync(CancellationToken cancellationToken);
    Task<IEnumerable<Stack>> GetBuildImageConsumerStacksAsync(Guid buildProjectId, CancellationToken cancellationToken);
    Task<IEnumerable<Stack>> GetInfoAsync(CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null, Guid? platformId = null);
    Task<IEnumerable<Container>> GetContainersAsync(Guid stackId, CancellationToken cancellationToken);
    Task<IEnumerable<Stack>> GetAuthorizedInfoAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null, Guid? platformId = null);
    Task<bool> CanAccessAsync(Guid userId, Guid stackId, CancellationToken cancellationToken);
    Task<PlatformConnectionInfo?> GetPlatformByStackIdAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<Stack>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<IEnumerable<ResourceInfo>> GetPlatformLookupAsync(Guid stackId, Guid userId, CancellationToken cancellationToken);
    Task<IEnumerable<ResourceInfo>> GetRegistryLookupAsync(Guid stackId, Guid userId, CancellationToken cancellationToken);
    Task<IEnumerable<ResourceInfo>> GetGitRepositoryLookupAsync(Guid stackId, Guid userId, CancellationToken cancellationToken);
    Task<IEnumerable<StackRelease>> GetReleasesByStackIdAsync(Guid stackId, CancellationToken cancellationToken);
    Task<IReadOnlyList<StackReleaseVolumeBinding>> GetReleaseVolumeBindingsAsync(Guid releaseId, CancellationToken cancellationToken);
    Task<int> ReplaceReleaseVolumeBindingsAsync(Guid releaseId, IReadOnlyCollection<StackReleaseVolumeBinding> bindings, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(Guid id, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(string name, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(Guid id, string name, CancellationToken cancellationToken);
    Task<int> AddAsync(Stack stack, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null, Guid? tagCreatedByActorId = null);
    Task<int> AddReleaseAsync(StackRelease release, CancellationToken cancellationToken);
    Task<int> UpdateAsync(Stack stack, CancellationToken cancellationToken);
    Task<int> UpdateReleaseStatusAsync(Guid releaseId, StackReleaseStatus status, CancellationToken cancellationToken);
    Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<IEnumerable<Stack>> GetStuckStacksAsync(int staleAfterSeconds = 3600, CancellationToken cancellationToken = default);
    Task<IEnumerable<GitStackBranchSubscription>> GetGitStackBranchSubscriptionsAsync(CancellationToken cancellationToken);
    Task<IEnumerable<Stack>> GetBranchTrackingGitStacksAsync(Guid gitRepositoryId, string branch, CancellationToken cancellationToken);
    Task<bool> UpdateProcessingAsync(Guid id, StackReleaseStatus status, ResourceControlState state, long? startedAt, long rowVersion, bool? checkRowVersion, Guid? controlTriggeredBy, CancellationToken cancellationToken);
}

public interface IGitReposRepository
{
    Task<GitRepository?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<GitRepository?> GetWithAccountAsync(Guid id, CancellationToken cancellationToken);
    Task<GitRepository?> GetByNameAsync(string name, CancellationToken cancellationToken);
    Task<IEnumerable<GitRepository>> GetAllAsync(CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null);
    Task<IEnumerable<GitRepository>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null);
    Task<IEnumerable<GitRepository>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(string name, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(Guid id, string name, CancellationToken cancellationToken);
    Task<int> AddAsync(GitRepository gitRepository, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null, Guid? tagCreatedByActorId = null);
    Task<int> UpdateAsync(GitRepository gitRepository, CancellationToken cancellationToken);
    Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<IEnumerable<GitRepository>> GetStuckRepositoriesAsync(int staleAfterSeconds = 3600, CancellationToken cancellationToken = default);
    Task<int> UpdateProcessingAsync(Guid id, GitReposStatus status, ResourceControlState state, long? startedAt, long rowVersion, bool checkRowVersion, Guid? controlTriggeredBy, CancellationToken cancellationToken);
    Task<GitRepositoryRef?> GetRefAsync(Guid gitRepositoryId, string branch, CancellationToken cancellationToken);
    Task<IEnumerable<GitRepositoryRef>> GetRefsByRepositoryIdAsync(Guid gitRepositoryId, CancellationToken cancellationToken);
    Task<int> UpsertRefAsync(GitRepositoryRef gitRepositoryRef, CancellationToken cancellationToken);
}

public interface IGitAccountRepository
{
    Task<GitAccount?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<GitAccount?> GetByNameAsync(string name, CancellationToken cancellationToken);
    Task<IEnumerable<GitAccount>> GetAllAsync(CancellationToken cancellationToken);
    Task<IEnumerable<GitAccount>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken);
    Task<IEnumerable<GitAccount>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(string name, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(Guid id, CancellationToken cancellationToken);
    Task<bool> IsNameTakenAsync(Guid id, string name, CancellationToken cancellationToken);
    Task<int> AddAsync(GitAccount gitAccount, CancellationToken cancellationToken);
    Task<int> UpdateAsync(GitAccount gitAccount, CancellationToken cancellationToken);
    Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
}

public interface IContainerRepository
{
    Task<Container?> GetByIdAsync(string dockerContainerId, CancellationToken cancellationToken);
    Task<Container?> GetContainerInfoAsync(string dockerContainerId, CancellationToken cancellationToken);
    Task<IEnumerable<Container>> GetByIdsAsync(string[] dockerContainerIds, CancellationToken cancellationToken);
    Task<IEnumerable<Container>> GetStaleByDockerIdsAsync(string[] dockerContainerIds, Guid[] resolvedContainerIds, CancellationToken cancellationToken);
    Task<IEnumerable<Container>> GetByIdAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<IEnumerable<Container>> GetStuckContainersAsync(int timeout_s = 60, CancellationToken cancellationToken = default);
    Task<Container?> GetByDeploymentIdAsync(Guid deploymentId, CancellationToken cancellationToken);
    Task<IEnumerable<Container>> GetByDeploymentIdsAsync(IEnumerable<Guid> deploymentIds, CancellationToken cancellationToken);
    Task<IEnumerable<Container>> GetByPlatformIdAsync(Guid platformId, CancellationToken cancellationToken);
    Task<IEnumerable<Container>?> GetContainersInfoAsync(Guid platformId, CancellationToken cancellationToken);

    Task<int> AddAsync(Container container, CancellationToken cancellationToken);
    Task<int> BulkUpsertAsync(IEnumerable<Container> containers, CancellationToken cancellationToken);

    Task<int> UpdateAsync(Container container, CancellationToken cancellationToken);
    Task<int> UpdateContainersStateAsync(IEnumerable<Guid> ids, ContainerStateStatus state, CancellationToken cancellationToken);
    Task<int> UpdateProcessingAsync(Guid id, ResourceControlState state, long? startedAt, long rowVersion, bool? checkRowVersion, Guid? controlTriggeredBy, CancellationToken cancellationToken);

    Task<int> DeleteAsync(IEnumerable<Guid> containersId, CancellationToken cancellationToken);
}

public interface IContainerStatRepository
{
    Task<IEnumerable<ContainerStat>> GetStatsAggregatedAsync(string containerId, int hours, CancellationToken cancellationToken);
    Task<IEnumerable<ContainerStat>> GetStatsAggregatedLast24HoursAsync(string containerId, CancellationToken cancellationToken);
    Task<int> BulkInsertAsync(IEnumerable<ContainerStat> stats, CancellationToken cancellationToken);
    Task<int> RemoveOlderThanAsync(long createdBeforeEpochSeconds, CancellationToken cancellationToken);
}

public interface IActivityEventRepository
{
    Task<int> AddAsync(ActivityEvent activityEvent, CancellationToken cancellationToken);
    Task<ActivityEvent?> GetByIdAsync(Guid id, CancellationToken cancellationToken);
    Task<PagedResult<ActivityEvent>> GetPagedAsync(Guid? resourceId, ActivityResourceType? resourceType, ActivityEventType? eventType, int page,
        int pageSize, CancellationToken cancellationToken);

    Task<int> RemoveOlderThanAsync(long createdBeforeEpochSeconds, CancellationToken cancellationToken);
}

public interface IRefreshTokenRepository
{
    Task<int> CountAsync(Guid userId, CancellationToken cancellationToken);
    Task<int> AddAsync(RefreshToken refreshToken, CancellationToken cancellationToken);
    Task<UserAuthInfo?> GetUserAuthInfoByRefreshTokenIdAsync(Guid id, CancellationToken cancellationToken);
    Task<Guid?> GetActiveTokenIdAsync(Guid id, Guid userId, DateTime now, CancellationToken cancellationToken);
    Task<IReadOnlyList<UserSessionRecord>> GetActiveSessionsAsync(Guid userId, DateTime now, CancellationToken cancellationToken);
    Task<int> TouchAsync(Guid id, DateTime lastSeenAt, string? userAgent, string? ipAddress, CancellationToken cancellationToken);
    Task<int> DeleteOwnedSessionAsync(Guid sessionId, Guid userId, Guid? currentSessionId, CancellationToken cancellationToken);
    Task<int> DeleteOtherTokensAsync(Guid userId, Guid keepTokenId, CancellationToken cancellationToken);
    Task<int> DeleteAllTokensAsync(Guid userId, CancellationToken cancellationToken);

    Task<int> DeleteAsync(Guid id, CancellationToken cancellationToken);
    Task<int> DeleteOldestTokensAsync(Guid userId, int tokensToRemoveCount, CancellationToken cancellationToken);
    Task<int> DeleteExpiredAsync(DateTime now, CancellationToken cancellationToken);
}

public interface IUserMfaRepository
{
    Task<UserMfaSettings?> GetSettingsAsync(Guid userId, CancellationToken cancellationToken);
    Task<int> CountUnusedRecoveryCodesAsync(Guid userId, CancellationToken cancellationToken);
    Task<int> UpsertSettingsAsync(UserMfaSettings settings, CancellationToken cancellationToken);
    Task<int> TryAcceptTimeStepAsync(Guid userId, long matchedTimeStep, CancellationToken cancellationToken);
    Task<int> DeleteSettingsAsync(Guid userId, CancellationToken cancellationToken);
    Task<int> ReplaceRecoveryCodesAsync(Guid userId, IReadOnlyCollection<UserMfaRecoveryCode> codes, CancellationToken cancellationToken);
    Task<int> DeleteRecoveryCodesAsync(Guid userId, CancellationToken cancellationToken);
    Task<int> TryUseRecoveryCodeAsync(Guid userId, string codeHash, DateTime usedAt, CancellationToken cancellationToken);
    Task<int> AddSetupSessionAsync(MfaSetupSession session, CancellationToken cancellationToken);
    Task<MfaSetupSession?> GetSetupSessionAsync(Guid id, CancellationToken cancellationToken);
    Task<MfaSetupSession?> GetActiveSetupSessionByUserAsync(Guid userId, DateTime now, CancellationToken cancellationToken);
    Task<int> TryConsumeSetupSessionAsync(Guid id, DateTime consumedAt, CancellationToken cancellationToken);
    Task<int> DeleteSetupSessionsAsync(Guid userId, CancellationToken cancellationToken);
    Task<int> DeleteExpiredSetupSessionsAsync(DateTime now, CancellationToken cancellationToken);
}

public interface IMfaChallengeRepository
{
    Task<int> AddAsync(MfaChallenge challenge, CancellationToken cancellationToken);
    Task<MfaChallenge?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<int> TryIncrementFailedAttemptsAsync(Guid id, DateTime now, int maxFailedAttempts, CancellationToken cancellationToken);
    Task<int> TryConsumeAsync(Guid id, DateTime consumedAt, int maxFailedAttempts, CancellationToken cancellationToken);
    Task<int> DeleteForUserAsync(Guid userId, CancellationToken cancellationToken);
    Task<int> DeleteExpiredAsync(DateTime now, CancellationToken cancellationToken);
}

public interface IPlatformStatRepository
{
    Task<IEnumerable<PlatformStat>> GetStatsAggregatedAsync(Guid platformId, int hours, CancellationToken cancellationToken);
    Task<IEnumerable<PlatformStat>> GetStatsAggregatedLast24HoursAsync(Guid platformId, CancellationToken cancellationToken);
    Task<int> BulkInsertAsync(IEnumerable<PlatformStat> stats, CancellationToken cancellationToken);
    Task<int> RemoveOlderThanAsync(long createdBeforeEpochSeconds, CancellationToken cancellationToken);
}

public interface ITeamRepository
{
    Task<Team?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<TeamDetails?> GetDetailsAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<Team>> GetAllAsync(CancellationToken cancellationToken);
    Task<PagedResult<TeamDetails>> GetPagedAsync(int page, int pageSize, string? name, CancellationToken cancellationToken);
    Task<PagedResult<TeamDetails>> GetAuthorizedPagedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, int page, int pageSize, string? name, CancellationToken cancellationToken);
    Task<IEnumerable<TeamSearchItem>> SearchAsync(string query, int limit, CancellationToken cancellationToken);
    Task<IEnumerable<TeamSearchItem>> SearchAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, string query, int limit, CancellationToken cancellationToken);
    Task<IEnumerable<Team>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<bool> GetConflictsAsync(string name, Guid? excludeId, CancellationToken cancellationToken);
    Task<(Team? Team, bool IsEnabled, bool NameExists)> GetTeamUpdateStateAsync(Guid id, string? name, CancellationToken cancellationToken);
    Task<bool> ExistsByNameAsync(string name, Guid? excludeId, CancellationToken cancellationToken);
    Task<int> AddAsync(Team team, CancellationToken cancellationToken);
    Task<int> UpdateAsync(Team team, CancellationToken cancellationToken);
    Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<IEnumerable<Guid>> GetUserIdsAsync(Guid teamId, CancellationToken cancellationToken);
    Task<(TeamDetails? Team, bool UserExists, bool HasMember)> GetMemberAssignmentStateAsync(Guid teamId, Guid userId, CancellationToken cancellationToken);
    Task<int> AddMemberAsync(Guid teamId, Guid userId, CancellationToken cancellationToken);
    Task<int> RemoveMemberAsync(Guid teamId, Guid userId, CancellationToken cancellationToken);
    Task<int> ReplaceMembersAsync(Guid teamId, IEnumerable<Guid> userIds, CancellationToken cancellationToken);
    Task<IEnumerable<Guid>> GetUserIdsByActorIdAsync(Guid actorId, CancellationToken cancellationToken);
}

public interface IRoleRepository
{
    Task<Role?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<Role>> GetAllAsync(CancellationToken cancellationToken);
    Task<IEnumerable<Role>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken);
    Task<IEnumerable<Role>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<bool> ExistsByNameAsync(string name, Guid? excludeId, CancellationToken cancellationToken);
    Task<int> AddAsync(Role role, CancellationToken cancellationToken);
    Task<int> RenameAsync(Role role, CancellationToken cancellationToken);
    Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<IEnumerable<ResourceInfo>> GetUserRoleLookupAsync(Guid sourceUserId, Guid userId, CancellationToken cancellationToken);
    Task<IEnumerable<Guid>> GetActorRoleIdsAsync(Guid actorId, CancellationToken cancellationToken);
    Task<int> AddActorRoleAsync(Guid actorId, Guid roleId, CancellationToken cancellationToken);
    Task<int> RemoveActorRoleAsync(Guid actorId, Guid roleId, CancellationToken cancellationToken);
    Task<int> ReplaceActorRolesAsync(Guid actorId, IEnumerable<Guid> roleIds, CancellationToken cancellationToken);
    Task<IEnumerable<Permission>> GetPermissionsAsync(Guid roleId, CancellationToken cancellationToken);
    Task<int> ReplacePermissionsAsync(Guid roleId, IEnumerable<Permission> permissions, CancellationToken cancellationToken);
    Task<IDictionary<Guid, Guid[]>> GetActorRoleIdsAsync(IEnumerable<Guid> actorIds, CancellationToken cancellationToken);
    Task<IEnumerable<Guid>> GetActorIdsByRoleIdAsync(Guid roleId, CancellationToken cancellationToken);
}

public interface IPlatformRepository
{
    Task<Platform?> GetByIdAsync(Guid platformId, CancellationToken cancellationToken);
    Task<Platform?> GetByNameAsync(string name, CancellationToken cancellationToken);
    Task<IEnumerable<Platform>?> GetPlatformsWithLatestStatAsync(CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null);
    Task<IEnumerable<Platform>> GetAuthorizedWithLatestStatAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null);
    Task<IEnumerable<Platform>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken);
    Task<Platform?> GetPlatformWithLatestStatAsync(Guid platformId, CancellationToken cancellationToken);
    Task<IEnumerable<Platform>> GetPlatformsWithLatestStatByIdsAsync(IReadOnlyCollection<Guid> platformIds, CancellationToken cancellationToken);
    Task<PlatformConnectionInfo?> GetPlatformByContainerIdAsync(string dockerContainerId, CancellationToken cancellationToken);
    Task<PlatformConnectionInfo?> GetInfoAsync(Guid platformId, CancellationToken cancellationToken);
    Task<IEnumerable<PlatformConnectionInfo>> GetPlatformsInfoAsync(CancellationToken cancellationToken);
    Task<bool> CanAccessAsync(Guid userId, Guid platformId, CancellationToken cancellationToken);
    Task<IEnumerable<ResourceInfo>> GetDeploymentLookupAsync(Guid platformId, Guid userId, CancellationToken cancellationToken);
    Task<IEnumerable<ResourceInfo>> GetStackLookupAsync(Guid platformId, Guid userId, CancellationToken cancellationToken);
    Task<IEnumerable<ResourceInfo>> GetRegistryLookupAsync(Guid platformId, Guid userId, CancellationToken cancellationToken);

    Task<int?> PlatformNameExistsAsync(string name, Guid excludePlatformId, CancellationToken cancellationToken);
    Task<bool> NameOrAddressExistsAsync(string name, string address, CancellationToken cancellationToken);

    Task<int> AddAsync(Platform platform, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null, Guid? tagCreatedByActorId = null);
    Task<int> UpdateAsync(Platform platform, CancellationToken cancellationToken);
    public Task<bool> ExistsAsync(Guid id, CancellationToken cancellationToken);

    Task<int> DeleteAsync(Guid platformId, CancellationToken cancellationToken);
}

public interface IEdgeAgentRepository
{
    Task<int> AddEnrollmentAsync(EdgeAgentEnrollment enrollment, CancellationToken cancellationToken);
    Task<EdgeAgentEnrollment?> GetActiveEnrollmentAsync(Guid platformId, DateTime utcNow, CancellationToken cancellationToken);
    Task<EdgeAgentEnrollment?> GetActiveEnrollmentAsync(EdgeAgentResourceType resourceType, Guid resourceId, DateTime utcNow, CancellationToken cancellationToken);
    Task<EdgeAgentEnrollment?> GetEnrollmentByTokenHashAsync(string tokenHash, CancellationToken cancellationToken);
    Task<int> MarkEnrollmentUsedAsync(Guid enrollmentId, DateTime usedAtUtc, CancellationToken cancellationToken);
    Task<EdgeAgentBinding?> GetBindingByPlatformIdAsync(Guid platformId, CancellationToken cancellationToken);
    Task<EdgeAgentBinding?> GetBindingByResourceAsync(EdgeAgentResourceType resourceType, Guid resourceId, CancellationToken cancellationToken);
    Task<EdgeAgentPlatformState?> GetPlatformStateByPlatformIdAsync(Guid platformId, CancellationToken cancellationToken);
    Task<EdgeAgentBinding?> GetBindingByAgentAsync(Guid platformId, Guid agentId, CancellationToken cancellationToken);
    Task<EdgeAgentBinding?> GetBindingByAgentAsync(EdgeAgentResourceType resourceType, Guid resourceId, Guid agentId, CancellationToken cancellationToken);
    Task<int> AddBindingAsync(EdgeAgentBinding binding, CancellationToken cancellationToken);
    Task<int> UpdateBindingConnectedAsync(Guid platformId, DateTime connectedAtUtc, string hostname, string agentVersion, string capabilitiesJson, CancellationToken cancellationToken);
    Task<int> UpdateBindingConnectedAsync(EdgeAgentResourceType resourceType, Guid resourceId, DateTime connectedAtUtc, string hostname, string agentVersion, string capabilitiesJson, CancellationToken cancellationToken);
    Task<int> UpdateBindingHeartbeatAsync(Guid platformId, DateTime heartbeatAtUtc, string? hostname, string? agentVersion, string? capabilitiesJson, CancellationToken cancellationToken);
    Task<int> UpdateBindingHeartbeatAsync(EdgeAgentResourceType resourceType, Guid resourceId, DateTime heartbeatAtUtc, string? hostname, string? agentVersion, string? capabilitiesJson, CancellationToken cancellationToken);
    Task<int> UpdateBindingDisconnectedAsync(Guid platformId, DateTime disconnectedAtUtc, CancellationToken cancellationToken);
    Task<int> UpdateBindingDisconnectedAsync(EdgeAgentResourceType resourceType, Guid resourceId, DateTime disconnectedAtUtc, CancellationToken cancellationToken);
    Task<int> RevokeBindingAsync(Guid platformId, DateTime revokedAtUtc, CancellationToken cancellationToken);
    Task<int> RevokeBindingAsync(EdgeAgentResourceType resourceType, Guid resourceId, DateTime revokedAtUtc, CancellationToken cancellationToken);
}

public interface IEdgeAgentCommandRouter
{
    Task<EdgeAgentCommandRouterResult> SendUnaryAsync(
        EdgeAgentResourceType resourceType,
        Guid resourceId,
        EdgeAgentCommandKind kind,
        byte[] payload,
        TimeSpan timeout,
        string? correlationId,
        CancellationToken cancellationToken);

    Task<EdgeAgentCommandRouterResult> SendUnaryAsync(
        Guid platformId,
        EdgeAgentCommandKind kind,
        byte[] payload,
        TimeSpan timeout,
        string? correlationId,
        CancellationToken cancellationToken);

    IAsyncEnumerable<EdgeAgentStreamItem> SendServerStreamAsync(
        EdgeAgentResourceType resourceType,
        Guid resourceId,
        EdgeAgentCommandKind kind,
        byte[] payload,
        TimeSpan timeout,
        string? correlationId,
        CancellationToken cancellationToken);

    IAsyncEnumerable<EdgeAgentStreamItem> SendServerStreamAsync(
        Guid platformId,
        EdgeAgentCommandKind kind,
        byte[] payload,
        TimeSpan timeout,
        string? correlationId,
        CancellationToken cancellationToken);

    Task<Result<EdgeAgentInteractiveCommand>> StartInteractiveAsync(
        EdgeAgentResourceType resourceType,
        Guid resourceId,
        EdgeAgentCommandKind kind,
        byte[] payload,
        TimeSpan timeout,
        string? correlationId,
        CancellationToken cancellationToken);

    Task<Result<EdgeAgentInteractiveCommand>> StartInteractiveAsync(
        Guid platformId,
        EdgeAgentCommandKind kind,
        byte[] payload,
        TimeSpan timeout,
        string? correlationId,
        CancellationToken cancellationToken);

    Task<Result> SendStreamInputAsync(
        Guid platformId,
        string commandId,
        byte[] payload,
        CancellationToken cancellationToken);

    Task<Result> CancelAsync(
        Guid platformId,
        string commandId,
        string reason,
        CancellationToken cancellationToken);
}

public interface IImageRepository
{
    Task<Image?> GetByIdAsync(Guid id, Guid platformId, CancellationToken cancellationToken);
    Task<IEnumerable<Image>> GetByIdAsync(string[] ids, Guid platformId, CancellationToken cancellationToken);
    Task<int> UpdateProcessingAsync(Guid id, ResourceControlState state, long? startedAt, long rowVersion, bool? checkRowVersion, CancellationToken cancellationToken);
    Task<IEnumerable<Image>> GetStuckImagesAsync(int timeout_s = 60, CancellationToken cancellationToken = default);
    Task<Image?> GetByDockerImageIdAsync(string dockerImageId, Guid platformId, CancellationToken cancellationToken);
    Task<IEnumerable<Image>> GetByPlatformIdAsync(Guid platformId, CancellationToken cancellationToken);

    Task<int> AddOrUpdateAsync(Image image, CancellationToken cancellationToken);
    Task<int> BulkUpsertAsync(IEnumerable<Image> images, CancellationToken cancellationToken);

    Task<int> DeleteAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
}

public interface IDeploymentRepository
{
    Task<Deployment?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<Deployment?> GetInfoAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<Deployment>> GetAllAsync(CancellationToken cancellationToken);
    Task<IEnumerable<Deployment>> GetBuildImageConsumersAsync(Guid buildProjectId, CancellationToken cancellationToken);
    Task<IEnumerable<Deployment>> GetInfoAsync(CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null, Guid? platformId = null);
    Task<string?> GetContainerIdAsync(Guid deploymentId, CancellationToken cancellationToken);
    Task<PlatformConnectionInfo?> GetPlatformByDeploymentIdAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<Deployment>> GetAuthorizedInfoAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null, Guid? platformId = null);
    Task<bool> CanAccessAsync(Guid userId, Guid deploymentId, CancellationToken cancellationToken);
    Task<IEnumerable<ResourceInfo>> GetPlatformLookupAsync(Guid deploymentId, Guid userId, CancellationToken cancellationToken);
    Task<IEnumerable<ResourceInfo>> GetRegistryLookupAsync(Guid deploymentId, Guid userId, CancellationToken cancellationToken);
    Task<IEnumerable<ResourceInfo>> GetImageLookupAsync(Guid deploymentId, CancellationToken cancellationToken);
    Task<IEnumerable<Deployment>> GetByPlatformIdAsync(Guid platformId, CancellationToken cancellationToken);
    Task<IEnumerable<Deployment>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<IEnumerable<Deployment>> GetStuckDeploymentsAsync(int staleAfterSeconds = 3600, CancellationToken cancellationToken = default);
    Task<bool> ExistsAsync(Guid platformId, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(string name, Guid platformId, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(Guid id, string name, Guid platformId, CancellationToken cancellationToken);
    Task<int> AddAsync(Deployment deployment, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null, Guid? tagCreatedByActorId = null);
    Task<int> UpdateAsync(Deployment deployment, CancellationToken cancellationToken);
    Task<int> UpdateProcessingAsync(Guid id, DeploymentStatus status, ResourceControlState state, long? startedAt, long rowVersion, bool? checkRowVersion, Guid? controlTriggeredBy, CancellationToken cancellationToken);
    Task<int> UpdateStatusAsync(IEnumerable<Guid> ids, DeploymentStatus status, CancellationToken cancellationToken);

    Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
}

public interface IAlertRuleRepository
{
    Task<AlertRule?> GetByIdAsync(Guid alertRuleId, CancellationToken cancellationToken);
    Task<IEnumerable<AlertRule>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<IEnumerable<AlertRule>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken);
    Task<AlertRuleState?> GetStateAsync(Guid alertRuleId, Guid resourceId, CancellationToken cancellationToken);
    Task<IEnumerable<AlertRule>> GetAllAsync(CancellationToken cancellationToken = default);
    Task<PagedResult<AlertRule>> GetPagedAsync(int page, int pageSize, CancellationToken cancellationToken);
    Task<AlertChannel?> GetChannelByIdAsync(Guid channelId, CancellationToken cancellationToken);
    Task<IEnumerable<AlertChannel>> GetAllChannelsAsync(CancellationToken cancellationToken);
    Task<IEnumerable<AlertChannel>> GetAuthorizedChannelsAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken);
    Task<IEnumerable<AlertChannel>> GetAuthorizedAlertChannelsAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken);
    Task<int> AddAlertRuleAsync(AlertRule alertRule, CancellationToken cancellationToken);
    Task<int> AddChannelAsync(AlertChannel alertChannel, CancellationToken cancellationToken);
    Task<int> UpdateAsync(AlertRule alertRule, CancellationToken cancellationToken);
    Task<int> UpsertAlertRuleStateAsync(AlertRuleState alertRuleState, CancellationToken cancellationToken);
    Task<int> UpdateChannelAsync(AlertChannel channel, CancellationToken cancellationToken);
    Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<int> RemoveChannelsRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<bool> CanAccessAsync(Guid userId, Guid alertRuleId, CancellationToken cancellationToken);
}

public interface IAlertEventRepository
{
    Task<Guid> AddAsync(AlertEvent alertEvent, CancellationToken cancellationToken);
    Task<AlertEvent?> GetByIdAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<AlertEvent>> GetByIdAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<PagedResult<AlertEvent>> GetAuthorizedPagedAsync(Guid userId, ResourceType permissionResourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, Guid? resourceId, AlertType? alertType, AlertResourceType? resourceType,
        int page, int pageSize, CancellationToken cancellationToken, bool? unresolvedOnly = null);
    Task<PagedResult<AlertEvent>> GetPagedAsync(Guid? resourceId, AlertType? alertType, AlertResourceType? resourceType,
        int page, int pageSize, CancellationToken cancellationToken, bool? unresolvedOnly = null);
    Task<int> UpdateAsync(AlertEvent alertEvent, CancellationToken cancellationToken);
    Task<int> BulkUpdateAsync(IEnumerable<AlertEvent> alertEvents, CancellationToken cancellationToken);
    Task<int> CountUnresolvedAsync(CancellationToken cancellationToken);
}
