using Domain;
using Application.Features.Backups.Models;
using Domain.Contracts.Interfaces;
using Domain.Entities.Backups;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Application.Services.Backups;

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
    bool UpdateTimeZone)
    : ICommand<Result<BackupPolicyResult>>;

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Write)]
public sealed record ArchiveBackupPolicy(Guid PolicyId) : ICommand<Result>;

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Execute)]
public sealed record QueueBackupRun(Guid PolicyId, QueueBackupRunInputModel Input) : ICommand<Result<BackupRunResult>>;

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Execute)]
public sealed record CancelBackupRun(Guid RunId) : ICommand<Result>;

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Read, SpecificPermission.Restore)]
public sealed record QueueBackupRestoreRun(Guid RunId, RestoreVolumeInputModel Input) : ICommand<Result<BackupRestoreRunResult>>;

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Read, SpecificPermission.Restore)]
public sealed record CancelBackupRestoreRun(Guid RestoreRunId) : ICommand<Result>;

internal sealed class CreateBackupPolicyHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor)
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

        var policy = new BackupPolicy(
            input.Name,
            input.Description,
            input.Source,
            input.BackupRepositoryId,
            input.Enabled,
            input.Cron,
            input.TimeZone,
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

internal sealed class UpdateBackupPolicyHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<UpdateBackupPolicy, Result<BackupPolicyResult>>
{
    public async ValueTask<Result<BackupPolicyResult>> Handle(UpdateBackupPolicy command, CancellationToken cancellationToken)
    {
        var policy = await unitOfWork.BackupPolicies.GetAsync(command.PolicyId, cancellationToken);
        if (policy is null)
            return Result.Failure<BackupPolicyResult>(new NotFoundError("Backup policy not found."));

        if (command.UpdateBackupRepository && command.Policy.BackupRepositoryId.HasValue)
        {
            var repository = await unitOfWork.BackupRepositories.GetAsync(command.Policy.BackupRepositoryId.Value, cancellationToken);
            if (repository is null)
                return Result.Failure<BackupPolicyResult>(new NotFoundError("Backup repository not found."));
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
