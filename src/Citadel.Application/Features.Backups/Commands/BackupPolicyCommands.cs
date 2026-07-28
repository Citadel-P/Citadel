using Domain;
using Application.Features.Backups.Models;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Backups;
using Domain.Entities.Backups;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Application.Services.Backups;
using Application.Services.Licensing;
using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using System.Runtime.CompilerServices;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Activities;
using Application.Permissions;

namespace Application.Features.Backups.Commands;

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Write)]
public sealed record CreateBackupPolicy(BackupPolicyInputModel Policy) : ICommand<Result<BackupPolicyResult>>
{
    internal sealed class Validator : AbstractValidator<CreateBackupPolicy>
    {
        public Validator()
        {
            RuleFor(x => x.Policy.Name).NotEmpty().MaximumLength(128);
            RuleFor(x => x.Policy.Description).MaximumLength(600).When(x => x.Policy.Description is not null);
            RuleFor(x => x.Policy.Source).NotNull();
            RuleFor(x => x.Policy.BackupRepositoryId).NotEmpty();
            RuleFor(x => x.Policy.Cron).MaximumLength(128).When(x => x.Policy.Cron is not null);
            RuleFor(x => x.Policy.TimeZone).MaximumLength(128).When(x => x.Policy.TimeZone is not null);
            RuleFor(x => x.Policy.Webhook!.Secret).MaximumLength(256).When(x => x.Policy.Webhook?.Secret is not null);
            RuleFor(x => x.Policy.Webhook!.BranchFilter).MaximumLength(256).When(x => x.Policy.Webhook?.BranchFilter is not null);
        }
    }
}

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Write, ResourceIdProperty = nameof(UpdateBackupPolicy.PolicyId))]
public sealed record UpdateBackupPolicy(
    Guid PolicyId,
    UpdateBackupPolicyInputModel Policy,
    bool UpdateDescription,
    bool UpdateSource,
    bool UpdateBackupRepository,
    bool UpdateCron,
    bool UpdateTimeZone,
    bool UpdateWebhook)
    : ICommand<Result<BackupPolicyResult>>;

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Write, ResourceIdProperty = nameof(RenameBackupPolicy.PolicyId))]
public sealed record RenameBackupPolicy(Guid PolicyId, string Name) : ICommand<Result<BackupPolicyResult>>
{
    internal sealed class Validator : AbstractValidator<RenameBackupPolicy>
    {
        public Validator()
        {
            RuleFor(x => x.PolicyId).NotEmpty();
            RuleFor(x => x.Name).NotEmpty().MaximumLength(128);
        }
    }
}

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Write, ResourceIdProperty = nameof(PatchBackupPolicyMetadata.PolicyId))]
public sealed record PatchBackupPolicyMetadata(Guid PolicyId, string? Description) : ICommand<Result<BackupPolicyResult>>
{
    internal sealed class Validator : AbstractValidator<PatchBackupPolicyMetadata>
    {
        public Validator()
        {
            RuleFor(x => x.PolicyId).NotEmpty();
            RuleFor(x => x.Description).MaximumLength(600).When(x => x.Description is not null);
        }
    }
}

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Write, ResourceIdProperty = nameof(ArchiveBackupPolicy.PolicyId))]
public sealed record ArchiveBackupPolicy(Guid PolicyId) : ICommand<Result>;

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Execute, ResourceIdProperty = nameof(QueueBackupRun.PolicyId))]
public sealed record QueueBackupRun(Guid PolicyId, QueueBackupRunInputModel Input) : ICommand<Result<BackupRunResult>>;

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Execute, ResourceIdProperty = nameof(RunBackupPolicy.PolicyId))]
public sealed record RunBackupPolicy(Guid PolicyId, QueueBackupRunInputModel Input) : IStreamCommand<BackupRunStreamItem>;

public sealed record CancelBackupRun(Guid RunId) : ICommand<Result>;

public sealed record QueueBackupRestoreRun(Guid RunId, RestoreVolumeInputModel Input) : ICommand<Result<BackupRestoreRunResult>>;

public sealed record RunBackupRestoreVolume(Guid RunId, RestoreVolumeInputModel Input) : IStreamCommand<BackupRestoreRunStreamItem>;

public sealed record CancelBackupRestoreRun(Guid RestoreRunId) : ICommand<Result>;

file static class BackupPolicySourceValidator
{
    public static async Task<Result> ValidateSourceAsync(
        BackupSourceSpec source,
        BackupRepository repository,
        IUnitOfWork unitOfWork,
        IStackBackupVolumeResolver stackBackupVolumeResolver,
        IDeploymentBackupVolumeResolver deploymentBackupVolumeResolver,
        bool validateVolumeResolution,
        CancellationToken cancellationToken)
    {
        if (validateVolumeResolution)
        {
            var volumeValidation = await ValidateVolumesAsync(
                source,
                stackBackupVolumeResolver,
                deploymentBackupVolumeResolver,
                cancellationToken);
            if (volumeValidation.IsFailure(out var volumeError))
                return Result.Failure(volumeError);
        }

        return await ValidateRepositoryCompatibilityAsync(source, repository, unitOfWork, cancellationToken);
    }

    private static async Task<Result> ValidateVolumesAsync(
        BackupSourceSpec source,
        IStackBackupVolumeResolver stackBackupVolumeResolver,
        IDeploymentBackupVolumeResolver deploymentBackupVolumeResolver,
        CancellationToken cancellationToken)
    {
        if (source is DeploymentBackupSource deployment)
        {
            var deploymentResolution = await deploymentBackupVolumeResolver.ResolveAsync(deployment.DeploymentId, cancellationToken);
            if (!deploymentResolution.IsSuccess(out var resolvedDeployment, out var deploymentError))
                return Result.Failure(deploymentError);

            return resolvedDeployment.Volumes.Count == 0
                ? Result.Failure(new BadRequestError("Deployment has no resolved Docker named volumes to back up."))
                : Result.Success();
        }

        if (source is not StackBackupSource stack)
            return Result.Success();

        var resolution = await stackBackupVolumeResolver.ResolveAsync(stack.StackId, cancellationToken);
        if (!resolution.IsSuccess(out var resolved, out var error))
            return Result.Failure(error);

        return resolved.Volumes.Count == 0
            ? Result.Failure(new BadRequestError("Stack has no resolved Docker named volumes to back up."))
            : Result.Success();
    }

    private static async Task<Result> ValidateRepositoryCompatibilityAsync(
        BackupSourceSpec source,
        BackupRepository repository,
        IUnitOfWork unitOfWork,
        CancellationToken cancellationToken)
    {
        if (repository.Spec is not FileSystemBackupRepositorySpec fs)
            return Result.Success();

        var sourcePlatform = await GetSourcePlatformAsync(source, unitOfWork, cancellationToken);
        if (!sourcePlatform.IsSuccess(out var platform, out var error))
            return Result.Failure(error);

        if (platform is null)
        {
            return fs.Location == BackupExecutionLocation.Core
                ? Result.Success()
                : Result.Failure(new BadRequestError("Citadel backups can only use a Core filesystem repository or an S3-compatible repository."));
        }

        if (fs.Location == BackupExecutionLocation.Core)
        {
            return platform.ConnectorType == PlatformConnectorType.Local
                ? Result.Success()
                : Result.Failure(new BadRequestError("Core filesystem backup repositories cannot back up remote Docker volumes. Use an S3-compatible repository or a filesystem repository on the same platform."));
        }

        return fs.PlatformId == platform.Id
            ? Result.Success()
            : Result.Failure(new BadRequestError("Filesystem backup repository platform must match the backup source platform."));
    }

    private static async Task<Result<PlatformConnectionInfo?>> GetSourcePlatformAsync(
        BackupSourceSpec source,
        IUnitOfWork unitOfWork,
        CancellationToken cancellationToken)
    {
        if (source is CitadelSystemBackupSource)
            return Result.Success<PlatformConnectionInfo?>(null);

        PlatformConnectionInfo? platform = source switch
        {
            DockerVolumeBackupSource volume => await unitOfWork.Platforms.GetInfoAsync(volume.PlatformId, cancellationToken),
            StackBackupSource stack => await unitOfWork.Stacks.GetPlatformByStackIdAsync(stack.StackId, cancellationToken),
            DeploymentBackupSource deployment => await unitOfWork.Deployments.GetPlatformByDeploymentIdAsync(deployment.DeploymentId, cancellationToken),
            _ => null
        };

        if (platform is not null)
            return Result.Success<PlatformConnectionInfo?>(platform);

        return source switch
        {
            DockerVolumeBackupSource => Result.Failure<PlatformConnectionInfo?>(new NotFoundError("Platform not found.")),
            StackBackupSource => Result.Failure<PlatformConnectionInfo?>(new NotFoundError("Stack not found.")),
            DeploymentBackupSource => Result.Failure<PlatformConnectionInfo?>(new NotFoundError("Deployment not found.")),
            _ => Result.Failure<PlatformConnectionInfo?>(new BadRequestError("Unsupported backup source type."))
        };
    }
}

internal sealed class CreateBackupPolicyHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor,
    IStackBackupVolumeResolver stackBackupVolumeResolver,
    IDeploymentBackupVolumeResolver deploymentBackupVolumeResolver,
    ILicenseEntitlementService licenseEntitlementService)
    : ICommandHandler<CreateBackupPolicy, Result<BackupPolicyResult>>
{
    public async ValueTask<Result<BackupPolicyResult>> Handle(CreateBackupPolicy command, CancellationToken cancellationToken)
    {
        var input = command.Policy;
        var normalizedName = BackupRepository.ToNormalizedName(input.Name);

        if (await unitOfWork.BackupPolicies.ExistsByNormalizedNameAsync(normalizedName, cancellationToken))
            return Result.Failure<BackupPolicyResult>(new ConflictError("Backup policy name already exists."));

        if (input.Enabled
            && (!string.IsNullOrWhiteSpace(input.Cron) || input.Webhook?.Enabled == true))
        {
            var entitlement = await licenseEntitlementService.EnsureEnabledAsync(
                LicenseCapability.AutomatedOperations,
                cancellationToken);
            if (entitlement.IsFailure())
                return Result.Failure<BackupPolicyResult>(entitlement.Errors);
        }

        var repository = await unitOfWork.BackupRepositories.GetAsync(input.BackupRepositoryId, cancellationToken);
        if (repository is null)
            return Result.Failure<BackupPolicyResult>(new NotFoundError("Backup repository not found."));

        var sourceValidation = await BackupPolicySourceValidator.ValidateSourceAsync(
            input.Source,
            repository,
            unitOfWork,
            stackBackupVolumeResolver,
            deploymentBackupVolumeResolver,
            validateVolumeResolution: true,
            cancellationToken);
        if (sourceValidation.IsFailure(out var sourceError))
            return Result.Failure<BackupPolicyResult>(sourceError);

        var policy = new BackupPolicy(
            input.Name,
            input.Description,
            input.Source,
            input.BackupRepositoryId,
            input.Enabled,
            input.Cron,
            input.TimeZone,
            input.Webhook,
            input.KeepLastSuccessful ?? BackupPolicy.DefaultKeepLastSuccessful,
            input.TimeoutSeconds ?? BackupPolicy.DefaultTimeoutSeconds,
            input.AlertOnFailure,
            input.RunAsActorId.GetValueOrDefault(userContextAccessor.Current.ActorId),
            userContextAccessor.Current.ActorId);

        try
        {
            policy.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<BackupPolicyResult>(new BadRequestError(ex.Message));
        }

        var affectedRows = await unitOfWork.BackupPolicies.AddAsync(
            policy,
            cancellationToken,
            input.TagIds,
            userContextAccessor.Current.ActorId);

        if (affectedRows == 0)
            return Result.Failure<BackupPolicyResult>(new BadRequestError("One or more tags do not exist."));

        await unitOfWork.ActivityEventRepository.AddAsync(
            BackupPolicyActivity.Create(
                policy,
                userContextAccessor.Current.ActorId,
                ActivityEventType.BackupPolicyCreated,
                new BackupPolicyCreated(BackupPolicyActivity.ToSnapshot(policy))),
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(new BackupPolicyResult(policy));
    }
}

internal sealed class UpdateBackupPolicyHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor,
    IStackBackupVolumeResolver stackBackupVolumeResolver,
    IDeploymentBackupVolumeResolver deploymentBackupVolumeResolver,
    ILicenseEntitlementService licenseEntitlementService)
    : ICommandHandler<UpdateBackupPolicy, Result<BackupPolicyResult>>
{
    public async ValueTask<Result<BackupPolicyResult>> Handle(UpdateBackupPolicy command, CancellationToken cancellationToken)
    {
        var policy = await unitOfWork.BackupPolicies.GetAsync(command.PolicyId, cancellationToken);
        if (policy is null)
            return Result.Failure<BackupPolicyResult>(new NotFoundError("Backup policy not found."));

        var oldPolicy = BackupPolicyActivity.ToSnapshot(policy);

        if (BackupLicenseConfigurationPolicy.ChangesActivePaidTrigger(
                policy,
                command.Policy,
                command.UpdateCron,
                command.UpdateTimeZone,
                command.UpdateWebhook))
        {
            var entitlement = await licenseEntitlementService.EnsureEnabledAsync(
                LicenseCapability.AutomatedOperations,
                cancellationToken);
            if (entitlement.IsFailure())
                return Result.Failure<BackupPolicyResult>(entitlement.Errors);
        }

        BackupRepository? repository = null;
        if (command.UpdateBackupRepository && command.Policy.BackupRepositoryId.HasValue)
        {
            repository = await unitOfWork.BackupRepositories.GetAsync(command.Policy.BackupRepositoryId.Value, cancellationToken);
            if (repository is null)
                return Result.Failure<BackupPolicyResult>(new NotFoundError("Backup repository not found."));
        }

        if ((command.UpdateSource && command.Policy.Source is not null)
            || (command.UpdateBackupRepository && command.Policy.BackupRepositoryId.HasValue))
        {
            var source = command.UpdateSource && command.Policy.Source is not null
                ? command.Policy.Source
                : policy.Source;
            repository ??= await unitOfWork.BackupRepositories.GetAsync(policy.BackupRepositoryId, cancellationToken);
            if (repository is null)
                return Result.Failure<BackupPolicyResult>(new NotFoundError("Backup repository not found."));

            var sourceValidation = await BackupPolicySourceValidator.ValidateSourceAsync(
                source,
                repository,
                unitOfWork,
                stackBackupVolumeResolver,
                deploymentBackupVolumeResolver,
                validateVolumeResolution: command.UpdateSource && command.Policy.Source is not null,
                cancellationToken);
            if (sourceValidation.IsFailure(out var sourceError))
                return Result.Failure<BackupPolicyResult>(sourceError);
        }

        try
        {
            policy.Update(
                command.UpdateDescription ? command.Policy.Description : policy.Description,
                command.UpdateSource ? command.Policy.Source : null,
                command.UpdateBackupRepository ? command.Policy.BackupRepositoryId : null,
                command.Policy.Enabled,
                command.Policy.Cron,
                command.UpdateCron,
                command.Policy.TimeZone,
                command.UpdateTimeZone,
                command.Policy.Webhook,
                command.UpdateWebhook,
                command.Policy.KeepLastSuccessful,
                command.Policy.TimeoutSeconds,
                command.Policy.AlertOnFailure,
                command.Policy.RunAsActorId);
            policy.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<BackupPolicyResult>(new BadRequestError(ex.Message));
        }
        catch (InvalidOperationException ex)
        {
            return Result.Failure<BackupPolicyResult>(new BadRequestError(ex.Message));
        }

        await unitOfWork.BackupPolicies.UpdateAsync(policy, cancellationToken);
        await unitOfWork.ActivityEventRepository.AddAsync(
            BackupPolicyActivity.Create(
                policy,
                userContextAccessor.Current.ActorId,
                ActivityEventType.BackupPolicyUpdated,
                new BackupPolicyUpdated(oldPolicy, BackupPolicyActivity.ToSnapshot(policy))),
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(new BackupPolicyResult(policy));
    }
}

internal sealed class RenameBackupPolicyHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor)
    : ICommandHandler<RenameBackupPolicy, Result<BackupPolicyResult>>
{
    public async ValueTask<Result<BackupPolicyResult>> Handle(RenameBackupPolicy command, CancellationToken cancellationToken)
    {
        var policy = await unitOfWork.BackupPolicies.GetAsync(command.PolicyId, cancellationToken);
        if (policy is null)
            return Result.Failure<BackupPolicyResult>(new NotFoundError("Backup policy not found."));

        var normalizedName = BackupRepository.ToNormalizedName(command.Name);
        if (await unitOfWork.BackupPolicies.ExistsByNormalizedNameExceptAsync(normalizedName, policy.Id, cancellationToken))
            return Result.Failure<BackupPolicyResult>(new ConflictError("Backup policy name already exists."));

        var oldName = policy.Name;
        try
        {
            policy.Rename(command.Name);
            policy.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<BackupPolicyResult>(new BadRequestError(ex.Message));
        }

        await unitOfWork.BackupPolicies.UpdateAsync(policy, cancellationToken);
        await unitOfWork.ActivityEventRepository.AddAsync(
            BackupPolicyActivity.Create(
                policy,
                userContextAccessor.Current.ActorId,
                ActivityEventType.BackupPolicyRenamed,
                new BackupPolicyRenamed(oldName, policy.Name)),
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(new BackupPolicyResult(policy));
    }
}

internal sealed class PatchBackupPolicyMetadataHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor)
    : ICommandHandler<PatchBackupPolicyMetadata, Result<BackupPolicyResult>>
{
    public async ValueTask<Result<BackupPolicyResult>> Handle(PatchBackupPolicyMetadata command, CancellationToken cancellationToken)
    {
        var policy = await unitOfWork.BackupPolicies.GetAsync(command.PolicyId, cancellationToken);
        if (policy is null)
            return Result.Failure<BackupPolicyResult>(new NotFoundError("Backup policy not found."));

        var oldPolicy = BackupPolicyActivity.ToSnapshot(policy);
        try
        {
            policy.UpdateDescription(command.Description);
            policy.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<BackupPolicyResult>(new BadRequestError(ex.Message));
        }

        await unitOfWork.BackupPolicies.UpdateAsync(policy, cancellationToken);
        await unitOfWork.ActivityEventRepository.AddAsync(
            BackupPolicyActivity.Create(
                policy,
                userContextAccessor.Current.ActorId,
                ActivityEventType.BackupPolicyUpdated,
                new BackupPolicyUpdated(oldPolicy, BackupPolicyActivity.ToSnapshot(policy))),
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(new BackupPolicyResult(policy));
    }
}

internal sealed class ArchiveBackupPolicyHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor)
    : ICommandHandler<ArchiveBackupPolicy, Result>
{
    public async ValueTask<Result> Handle(ArchiveBackupPolicy command, CancellationToken cancellationToken)
    {
        var policy = await unitOfWork.BackupPolicies.GetAsync(command.PolicyId, cancellationToken);
        if (policy is null)
            return Result.Failure(new NotFoundError("Backup policy not found."));

        if (await unitOfWork.BackupRuns.HasActiveRunAsync(command.PolicyId, cancellationToken))
            return Result.Failure(new ConflictError("Backup policy has an active run."));

        try
        {
            policy.Archive(DateTimeOffset.UtcNow);
        }
        catch (InvalidOperationException ex)
        {
            return Result.Failure(new ConflictError(ex.Message));
        }

        await unitOfWork.BackupPolicies.UpdateAsync(policy, cancellationToken);
        await unitOfWork.ActivityEventRepository.AddAsync(
            BackupPolicyActivity.Create(
                policy,
                userContextAccessor.Current.ActorId,
                ActivityEventType.BackupPolicyArchived,
                new BackupPolicyArchived(BackupPolicyActivity.ToSnapshot(policy))),
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success();
    }
}

internal static class BackupPolicyActivity
{
    internal static BackupPolicyActivitySnapshot ToSnapshot(BackupPolicy policy)
        => new(
            policy.Id,
            policy.Name,
            policy.Description,
            policy.Source.Type.ToString(),
            policy.Source.StableKey,
            policy.BackupRepositoryId,
            policy.Enabled,
            policy.Cron,
            policy.TimeZone,
            policy.WebhookEnabled,
            policy.KeepLastSuccessful,
            policy.TimeoutSeconds,
            policy.AlertOnFailure,
            policy.RunAsActorId);

    internal static ActivityEvent Create(
        BackupPolicy policy,
        Guid actorId,
        ActivityEventType eventType,
        ActivityEventInfo info)
        => new(
            platformId: null,
            resourceId: policy.Id,
            actorId: actorId,
            resourceName: policy.Name,
            eventType: eventType,
            status: ActivityStatus.Success,
            info: info);
}

internal sealed class QueueBackupRunHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor,
    IBackupRunStreamManager backupRunStreamManager,
    INotificationQueue notificationQueue,
    ILicenseEntitlementService entitlementService)
    : ICommandHandler<QueueBackupRun, Result<BackupRunResult>>
{
    public async ValueTask<Result<BackupRunResult>> Handle(QueueBackupRun command, CancellationToken cancellationToken)
    {
        if (command.Input.Trigger is BackupRunTrigger.Schedule or BackupRunTrigger.Webhook)
        {
            var entitlement = await entitlementService.EnsureEnabledAsync(
                LicenseCapability.AutomatedOperations,
                cancellationToken);
            if (entitlement.IsFailure(out var entitlementError))
                return Result.Failure<BackupRunResult>(entitlementError);
        }

        var queueResult = await unitOfWork.BackupRuns.QueueAsync(
            command.PolicyId,
            Guid.CreateVersion7(),
            command.Input.Trigger,
            command.Input.TriggerSourceId,
            userContextAccessor.Current.ActorId,
            command.Input.Trigger == BackupRunTrigger.Schedule,
            DateTimeOffset.UtcNow,
            cancellationToken);

        if (queueResult.Status == BackupRunQueueResultStatus.PolicyNotFound)
            return Result.Failure<BackupRunResult>(new NotFoundError("Backup policy not found."));
        if (queueResult.Status == BackupRunQueueResultStatus.PolicyArchived)
            return Result.Failure<BackupRunResult>(new BadRequestError("Archived backup policies cannot queue new runs."));
        if (queueResult.Status == BackupRunQueueResultStatus.ActiveRunExists)
            return Result.Failure<BackupRunResult>(new ConflictError("Backup policy already has an active run."));

        await unitOfWork.CommitAsync(cancellationToken);

        await notificationQueue.EnqueueAsync(
            new BackupRunNotificationWorkItem(backupRunStreamManager, queueResult.Run!, "create"),
            cancellationToken);

        return Result.Success(new BackupRunResult(queueResult.Run!));
    }
}

internal sealed class RunBackupPolicyHandler(
    IUnitOfWork unitOfWork,
    IBackupRunExecutionService executionService,
    IUserContextAccessor userContextAccessor,
    IBackupRunStreamManager backupRunStreamManager,
    INotificationQueue notificationQueue,
    ILicenseEntitlementService entitlementService)
    : IStreamCommandHandler<RunBackupPolicy, BackupRunStreamItem>
{
    public async IAsyncEnumerable<BackupRunStreamItem> Handle(
        RunBackupPolicy command,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        if (command.Input.Trigger is BackupRunTrigger.Schedule or BackupRunTrigger.Webhook)
        {
            var entitlement = await entitlementService.EnsureEnabledAsync(
                LicenseCapability.AutomatedOperations,
                cancellationToken);
            if (entitlement.IsFailure(out var entitlementError))
            {
                yield return new BackupRunStreamItem(
                    Guid.Empty,
                    BackupRunStatus.Rejected,
                    entitlementError.Message);
                yield break;
            }
        }

        var queueResult = await unitOfWork.BackupRuns.QueueAsync(
            command.PolicyId,
            Guid.CreateVersion7(),
            command.Input.Trigger,
            command.Input.TriggerSourceId,
            userContextAccessor.Current.ActorId,
            command.Input.Trigger == BackupRunTrigger.Schedule,
            DateTimeOffset.UtcNow,
            cancellationToken);

        if (queueResult.Status != BackupRunQueueResultStatus.Queued)
        {
            yield return QueueError(queueResult.Status);
            yield break;
        }

        await unitOfWork.CommitAsync(cancellationToken);

        var run = queueResult.Run!;
        await notificationQueue.EnqueueAsync(
            new BackupRunNotificationWorkItem(backupRunStreamManager, run, "create"),
            cancellationToken);

        yield return new BackupRunStreamItem(
            RunId: run.Id,
            Status: BackupRunStatus.Queued,
            Message: $"Backup run queued for \"{run.PolicyNameSnapshot}\".");

        await foreach (var item in executionService.ExecuteQueuedAsync(run.Id, cancellationToken))
            yield return item;
    }

    private static BackupRunStreamItem QueueError(BackupRunQueueResultStatus status)
        => new(
            RunId: Guid.Empty,
            Status: BackupRunStatus.Rejected,
            Message: status switch
            {
                BackupRunQueueResultStatus.PolicyNotFound => "Backup policy not found.",
                BackupRunQueueResultStatus.PolicyArchived => "Archived backup policies cannot queue new runs.",
                BackupRunQueueResultStatus.ActiveRunExists => "Backup policy already has an active run.",
                _ => "Backup run could not be queued."
            },
            Stream: "stderr");
}

internal sealed class CancelBackupRunHandler(
    IUnitOfWork unitOfWork,
    IPermissionEvaluator permissionEvaluator,
    IBackupRunCoordinator runCoordinator,
    IBackupRunStreamManager backupRunStreamManager,
    INotificationQueue notificationQueue)
    : ICommandHandler<CancelBackupRun, Result>
{
    public async ValueTask<Result> Handle(CancelBackupRun command, CancellationToken cancellationToken)
    {
        var existingRun = await unitOfWork.BackupRuns.GetAsync(command.RunId, cancellationToken);
        if (existingRun is null)
            return Result.Failure(new NotFoundError("Active backup run not found."));

        var permission = await permissionEvaluator.EvaluateAsync(
            existingRun.BackupPolicyId,
            ResourceType.BackupPolicy,
            cancellationToken);
        if (!permission.Has(PermissionLevel.Execute, SpecificPermission.None))
            return Result.Failure(new ForbiddenError("Missing permission [Execute] on [BackupPolicy]"));

        runCoordinator.Cancel(command.RunId);
        var run = await unitOfWork.BackupRuns.CancelQueuedOrRunningAsync(
            command.RunId,
            DateTimeOffset.UtcNow,
            "Backup run cancelled.",
            cancellationToken);

        if (run is null)
            return Result.Failure(new NotFoundError("Active backup run not found."));

        await unitOfWork.CommitAsync(cancellationToken);

        await notificationQueue.EnqueueAsync(
            new BackupRunNotificationWorkItem(backupRunStreamManager, run),
            cancellationToken);

        return Result.Success();
    }
}

internal sealed class QueueBackupRestoreRunHandler(
    IUnitOfWork unitOfWork,
    IPermissionEvaluator permissionEvaluator,
    IUserContextAccessor userContextAccessor,
    IBackupRestoreRunStreamManager backupRestoreRunStreamManager,
    INotificationQueue notificationQueue)
    : ICommandHandler<QueueBackupRestoreRun, Result<BackupRestoreRunResult>>
{
    public async ValueTask<Result<BackupRestoreRunResult>> Handle(QueueBackupRestoreRun command, CancellationToken cancellationToken)
    {
        var result = await BackupRestoreRunQueuer.QueueAsync(
            unitOfWork,
            permissionEvaluator,
            userContextAccessor,
            command.RunId,
            command.Input,
            cancellationToken);

        if (result.IsSuccess(out var queued))
        {
            await notificationQueue.EnqueueAsync(
                new BackupRestoreRunNotificationWorkItem(backupRestoreRunStreamManager, queued.Run, queued.BackupPolicyId, "create"),
                cancellationToken);
        }

        return result;
    }
}

internal sealed class RunBackupRestoreVolumeHandler(
    IUnitOfWork unitOfWork,
    IPermissionEvaluator permissionEvaluator,
    IBackupRestoreRunExecutionService executionService,
    IUserContextAccessor userContextAccessor,
    IBackupRestoreRunStreamManager backupRestoreRunStreamManager,
    INotificationQueue notificationQueue)
    : IStreamCommandHandler<RunBackupRestoreVolume, BackupRestoreRunStreamItem>
{
    public async IAsyncEnumerable<BackupRestoreRunStreamItem> Handle(
        RunBackupRestoreVolume command,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var result = await BackupRestoreRunQueuer.QueueAsync(
            unitOfWork,
            permissionEvaluator,
            userContextAccessor,
            command.RunId,
            command.Input,
            cancellationToken);

        if (!result.IsSuccess(out var queued, out var error))
        {
            yield return new BackupRestoreRunStreamItem(
                RestoreRunId: Guid.Empty,
                Status: BackupRestoreStatus.Rejected,
                Message: error.Message,
                Stream: "stderr");
            yield break;
        }

        var run = queued.Run;
        await notificationQueue.EnqueueAsync(
            new BackupRestoreRunNotificationWorkItem(backupRestoreRunStreamManager, run, queued.BackupPolicyId, "create"),
            cancellationToken);

        yield return new BackupRestoreRunStreamItem(
            RestoreRunId: run.Id,
            Status: BackupRestoreStatus.Queued,
            Message: $"Restore run queued for volume \"{run.TargetVolumeName}\".");

        await foreach (var item in executionService.ExecuteQueuedAsync(run.Id, cancellationToken))
            yield return item;
    }
}

file static class BackupRestoreRunQueuer
{
    public static async ValueTask<Result<BackupRestoreRunResult>> QueueAsync(
        IUnitOfWork unitOfWork,
        IPermissionEvaluator permissionEvaluator,
        IUserContextAccessor userContextAccessor,
        Guid runId,
        RestoreVolumeInputModel input,
        CancellationToken cancellationToken)
    {
        var backupRun = await unitOfWork.BackupRuns.GetAsync(runId, cancellationToken);
        if (backupRun is null)
            return Result.Failure<BackupRestoreRunResult>(new NotFoundError("Backup run not found."));

        var permission = await permissionEvaluator.EvaluateAsync(
            backupRun.BackupPolicyId,
            ResourceType.BackupPolicy,
            cancellationToken);
        if (!permission.Has(PermissionLevel.Read, SpecificPermission.Restore))
        {
            return Result.Failure<BackupRestoreRunResult>(
                new ForbiddenError("Missing permission [Read] with specific [Restore] on [BackupPolicy]"));
        }

        if (backupRun.SnapshotAvailability != BackupSnapshotAvailability.Available)
            return Result.Failure<BackupRestoreRunResult>(new BadRequestError("Only available backup snapshots can be restored."));

        if (backupRun.SourceSnapshot is CitadelSystemBackupSource)
            return Result.Failure<BackupRestoreRunResult>(
                new BadRequestError("Citadel backups must be restored offline. See the control-plane recovery documentation."));

        if (backupRun.SourceSnapshot is not DockerVolumeBackupSource)
            return Result.Failure<BackupRestoreRunResult>(
                new BadRequestError("Only Docker volume backup snapshots can be restored through this operation."));

        var run = new BackupRestoreRun(
            backupRun.Id,
            backupRun.BackupRepositoryId,
            input.TargetPlatformId,
            input.TargetVolumeName,
            input.OverwriteExisting,
            userContextAccessor.Current.ActorId);

        await unitOfWork.BackupRestoreRuns.AddAsync(run, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(new BackupRestoreRunResult(run, backupRun.BackupPolicyId));
    }
}

internal sealed class CancelBackupRestoreRunHandler(
    IUnitOfWork unitOfWork,
    IPermissionEvaluator permissionEvaluator,
    IBackupRestoreRunCoordinator runCoordinator,
    IBackupRestoreRunStreamManager backupRestoreRunStreamManager,
    INotificationQueue notificationQueue)
    : ICommandHandler<CancelBackupRestoreRun, Result>
{
    public async ValueTask<Result> Handle(CancelBackupRestoreRun command, CancellationToken cancellationToken)
    {
        var existingRestoreRun = await unitOfWork.BackupRestoreRuns.GetAsync(command.RestoreRunId, cancellationToken);
        if (existingRestoreRun is null)
            return Result.Failure(new NotFoundError("Active backup restore run not found."));

        var backupRun = await unitOfWork.BackupRuns.GetAsync(existingRestoreRun.BackupRunId, cancellationToken);
        if (backupRun is null)
            return Result.Failure(new NotFoundError("Backup run not found."));

        var permission = await permissionEvaluator.EvaluateAsync(
            backupRun.BackupPolicyId,
            ResourceType.BackupPolicy,
            cancellationToken);
        if (!permission.Has(PermissionLevel.Read, SpecificPermission.Restore))
        {
            return Result.Failure(
                new ForbiddenError("Missing permission [Read] with specific [Restore] on [BackupPolicy]"));
        }

        runCoordinator.Cancel(command.RestoreRunId);
        var cancelled = await unitOfWork.BackupRestoreRuns.CancelQueuedOrRunningAsync(
            command.RestoreRunId,
            DateTimeOffset.UtcNow,
            "Backup restore run cancelled.",
            cancellationToken);

        if (cancelled is null)
            return Result.Failure(new NotFoundError("Active backup restore run not found."));

        await unitOfWork.CommitAsync(cancellationToken);

        await notificationQueue.EnqueueAsync(
            new BackupRestoreRunNotificationWorkItem(backupRestoreRunStreamManager, cancelled.Run, cancelled.BackupPolicyId),
            cancellationToken);

        return Result.Success();
    }
}
