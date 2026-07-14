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
using System.Runtime.CompilerServices;
using Domain.Contracts.Resources.Platforms;

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

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Write)]
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

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Write)]
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

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Write)]
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

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Write)]
public sealed record ArchiveBackupPolicy(Guid PolicyId) : ICommand<Result>;

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Execute)]
public sealed record QueueBackupRun(Guid PolicyId, QueueBackupRunInputModel Input) : ICommand<Result<BackupRunResult>>;

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Execute)]
public sealed record RunBackupPolicy(Guid PolicyId, QueueBackupRunInputModel Input) : IStreamCommand<BackupRunStreamItem>;

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Execute)]
public sealed record CancelBackupRun(Guid RunId) : ICommand<Result>;

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Read, SpecificPermission.Restore)]
public sealed record QueueBackupRestoreRun(Guid RunId, RestoreVolumeInputModel Input) : ICommand<Result<BackupRestoreRunResult>>;

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Read, SpecificPermission.Restore)]
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
                : Result.Failure(new BadRequestError("Citadel system backups can only use a Core filesystem repository or an S3-compatible repository."));
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
    IDeploymentBackupVolumeResolver deploymentBackupVolumeResolver)
    : ICommandHandler<CreateBackupPolicy, Result<BackupPolicyResult>>
{
    public async ValueTask<Result<BackupPolicyResult>> Handle(CreateBackupPolicy command, CancellationToken cancellationToken)
    {
        var input = command.Policy;
        var normalizedName = BackupRepository.ToNormalizedName(input.Name);

        if (await unitOfWork.BackupPolicies.ExistsByNormalizedNameAsync(normalizedName, cancellationToken))
            return Result.Failure<BackupPolicyResult>(new ConflictError("Backup policy name already exists."));

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

        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(new BackupPolicyResult(policy));
    }
}

internal sealed class UpdateBackupPolicyHandler(
    IUnitOfWork unitOfWork,
    IStackBackupVolumeResolver stackBackupVolumeResolver,
    IDeploymentBackupVolumeResolver deploymentBackupVolumeResolver)
    : ICommandHandler<UpdateBackupPolicy, Result<BackupPolicyResult>>
{
    public async ValueTask<Result<BackupPolicyResult>> Handle(UpdateBackupPolicy command, CancellationToken cancellationToken)
    {
        var policy = await unitOfWork.BackupPolicies.GetAsync(command.PolicyId, cancellationToken);
        if (policy is null)
            return Result.Failure<BackupPolicyResult>(new NotFoundError("Backup policy not found."));

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
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(new BackupPolicyResult(policy));
    }
}

internal sealed class RenameBackupPolicyHandler(IUnitOfWork unitOfWork)
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
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(new BackupPolicyResult(policy));
    }
}

internal sealed class PatchBackupPolicyMetadataHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<PatchBackupPolicyMetadata, Result<BackupPolicyResult>>
{
    public async ValueTask<Result<BackupPolicyResult>> Handle(PatchBackupPolicyMetadata command, CancellationToken cancellationToken)
    {
        var policy = await unitOfWork.BackupPolicies.GetAsync(command.PolicyId, cancellationToken);
        if (policy is null)
            return Result.Failure<BackupPolicyResult>(new NotFoundError("Backup policy not found."));

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
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(new BackupPolicyResult(policy));
    }
}

internal sealed class ArchiveBackupPolicyHandler(IUnitOfWork unitOfWork)
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
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success();
    }
}

internal sealed class QueueBackupRunHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor)
    : ICommandHandler<QueueBackupRun, Result<BackupRunResult>>
{
    public async ValueTask<Result<BackupRunResult>> Handle(QueueBackupRun command, CancellationToken cancellationToken)
    {
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

        return Result.Success(new BackupRunResult(queueResult.Run!));
    }
}

internal sealed class RunBackupPolicyHandler(
    IUnitOfWork unitOfWork,
    IBackupRunExecutionService executionService,
    IUserContextAccessor userContextAccessor)
    : IStreamCommandHandler<RunBackupPolicy, BackupRunStreamItem>
{
    public async IAsyncEnumerable<BackupRunStreamItem> Handle(
        RunBackupPolicy command,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
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
    IBackupRunCoordinator runCoordinator)
    : ICommandHandler<CancelBackupRun, Result>
{
    public async ValueTask<Result> Handle(CancelBackupRun command, CancellationToken cancellationToken)
    {
        runCoordinator.Cancel(command.RunId);
        var rows = await unitOfWork.BackupRuns.CancelQueuedOrRunningAsync(
            command.RunId,
            DateTimeOffset.UtcNow,
            "Backup run cancelled.",
            cancellationToken);

        if (rows == 0)
            return Result.Failure(new NotFoundError("Active backup run not found."));

        await unitOfWork.CommitAsync(cancellationToken);
        return Result.Success();
    }
}

internal sealed class QueueBackupRestoreRunHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor)
    : ICommandHandler<QueueBackupRestoreRun, Result<BackupRestoreRunResult>>
{
    public async ValueTask<Result<BackupRestoreRunResult>> Handle(QueueBackupRestoreRun command, CancellationToken cancellationToken)
    {
        var backupRun = await unitOfWork.BackupRuns.GetAsync(command.RunId, cancellationToken);
        if (backupRun is null)
            return Result.Failure<BackupRestoreRunResult>(new NotFoundError("Backup run not found."));

        if (backupRun.SnapshotAvailability != BackupSnapshotAvailability.Available)
            return Result.Failure<BackupRestoreRunResult>(new BadRequestError("Only available backup snapshots can be restored."));

        var run = new BackupRestoreRun(
            backupRun.Id,
            backupRun.BackupRepositoryId,
            command.Input.TargetPlatformId,
            command.Input.TargetVolumeName,
            command.Input.OverwriteExisting,
            userContextAccessor.Current.ActorId);

        await unitOfWork.BackupRestoreRuns.AddAsync(run, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(new BackupRestoreRunResult(run));
    }
}

internal sealed class CancelBackupRestoreRunHandler(
    IUnitOfWork unitOfWork,
    IBackupRestoreRunCoordinator runCoordinator)
    : ICommandHandler<CancelBackupRestoreRun, Result>
{
    public async ValueTask<Result> Handle(CancelBackupRestoreRun command, CancellationToken cancellationToken)
    {
        runCoordinator.Cancel(command.RestoreRunId);
        var rows = await unitOfWork.BackupRestoreRuns.CancelQueuedOrRunningAsync(
            command.RestoreRunId,
            DateTimeOffset.UtcNow,
            "Backup restore run cancelled.",
            cancellationToken);

        if (rows == 0)
            return Result.Failure(new NotFoundError("Active backup restore run not found."));

        await unitOfWork.CommitAsync(cancellationToken);
        return Result.Success();
    }
}
