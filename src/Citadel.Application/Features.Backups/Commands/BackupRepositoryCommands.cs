using Domain;
using Application.Features.Backups.Models;
using Application.Services.Backups;
using Domain.Contracts.Interfaces;
using Domain.Entities.Backups;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Backups.Commands;

[RequirePermission(ResourceType.BackupRepository, PermissionLevel.Write)]
public sealed record CreateBackupRepository(BackupRepositoryInputModel Repository) : ICommand<Result<BackupRepositoryResult>>
{
    internal sealed class Validator : AbstractValidator<CreateBackupRepository>
    {
        public Validator()
        {
            RuleFor(x => x.Repository.Name).NotEmpty().MaximumLength(128);
            RuleFor(x => x.Repository.Description).MaximumLength(600).When(x => x.Repository.Description is not null);
            RuleFor(x => x.Repository.PasswordSecretId).NotEmpty();
            RuleFor(x => x.Repository.Spec).NotNull();
        }
    }
}

[RequirePermission(ResourceType.BackupRepository, PermissionLevel.Write)]
public sealed record UpdateBackupRepository(Guid RepositoryId, UpdateBackupRepositoryInputModel Repository, bool UpdateDescription, bool UpdateSpec)
    : ICommand<Result<BackupRepositoryResult>>;

[RequirePermission(ResourceType.BackupRepository, PermissionLevel.Write)]
public sealed record ArchiveBackupRepository(Guid RepositoryId) : ICommand<Result>;

[RequirePermission(ResourceType.BackupRepository, PermissionLevel.Execute)]
public sealed record ValidateBackupRepository(Guid RepositoryId, ValidateBackupRepositoryInputModel Input)
    : ICommand<Result<BackupRepositoryValidation>>;

[RequirePermission(ResourceType.BackupRepository, PermissionLevel.Execute)]
public sealed record InitializeBackupRepository(Guid RepositoryId, ValidateBackupRepositoryInputModel Input)
    : ICommand<Result>;

[RequirePermission(ResourceType.BackupRepository, PermissionLevel.Execute)]
public sealed record CheckBackupRepository(Guid RepositoryId, ValidateBackupRepositoryInputModel Input)
    : ICommand<Result>;

[RequirePermission(ResourceType.BackupRepository, PermissionLevel.Execute)]
public sealed record PruneBackupRepository(Guid RepositoryId, ValidateBackupRepositoryInputModel Input)
    : ICommand<Result>;

internal sealed class CreateBackupRepositoryHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor)
    : ICommandHandler<CreateBackupRepository, Result<BackupRepositoryResult>>
{
    public async ValueTask<Result<BackupRepositoryResult>> Handle(CreateBackupRepository command, CancellationToken cancellationToken)
    {
        var input = command.Repository;
        var normalizedName = BackupRepository.ToNormalizedName(input.Name);

        if (await unitOfWork.BackupRepositories.ExistsByNormalizedNameAsync(normalizedName, cancellationToken))
            return Result.Failure<BackupRepositoryResult>(new ConflictError("Backup repository name already exists."));

        var repository = new BackupRepository(
            input.Name,
            input.Description,
            input.Spec,
            input.PasswordSecretId,
            userContextAccessor.Current.ActorId);

        try
        {
            repository.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<BackupRepositoryResult>(new BadRequestError(ex.Message));
        }

        await unitOfWork.BackupRepositories.AddAsync(repository, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(new BackupRepositoryResult(repository));
    }
}

internal sealed class UpdateBackupRepositoryHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<UpdateBackupRepository, Result<BackupRepositoryResult>>
{
    public async ValueTask<Result<BackupRepositoryResult>> Handle(UpdateBackupRepository command, CancellationToken cancellationToken)
    {
        var repository = await unitOfWork.BackupRepositories.GetAsync(command.RepositoryId, cancellationToken);
        if (repository is null)
            return Result.Failure<BackupRepositoryResult>(new NotFoundError("Backup repository not found."));

        try
        {
            repository.Update(
                command.UpdateDescription ? command.Repository.Description : repository.Description,
                command.UpdateSpec ? command.Repository.Spec : null);
            repository.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<BackupRepositoryResult>(new BadRequestError(ex.Message));
        }
        catch (InvalidOperationException ex)
        {
            return Result.Failure<BackupRepositoryResult>(new BadRequestError(ex.Message));
        }

        await unitOfWork.BackupRepositories.UpdateAsync(repository, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(new BackupRepositoryResult(repository));
    }
}

internal sealed class ArchiveBackupRepositoryHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<ArchiveBackupRepository, Result>
{
    public async ValueTask<Result> Handle(ArchiveBackupRepository command, CancellationToken cancellationToken)
    {
        var result = await unitOfWork.BackupRepositories.ArchiveIfUnusedAsync(command.RepositoryId, DateTimeOffset.UtcNow, cancellationToken);
        if (result == BackupRepositoryArchiveResult.NotFound)
            return Result.Failure(new NotFoundError("Backup repository not found."));
        if (result == BackupRepositoryArchiveResult.ActiveOperation)
            return Result.Failure(new ConflictError("Backup repository has an active backup or restore operation."));
        if (result == BackupRepositoryArchiveResult.HasPolicies)
            return Result.Failure(new ConflictError("Backup repository is used by a non-archived backup policy."));

        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success();
    }
}

internal sealed class ValidateBackupRepositoryHandler(IBackupRepositoryDestinationService destinationService)
    : ICommandHandler<ValidateBackupRepository, Result<BackupRepositoryValidation>>
{
    public async ValueTask<Result<BackupRepositoryValidation>> Handle(ValidateBackupRepository command, CancellationToken cancellationToken)
        => await destinationService.ValidateAsync(
            command.RepositoryId,
            new BackupExecutionContext(command.Input.Location, command.Input.PlatformId),
            cancellationToken);
}

internal sealed class BackupRepositoryOperationHandler(IBackupRepositoryDestinationService destinationService)
    : ICommandHandler<InitializeBackupRepository, Result>,
      ICommandHandler<CheckBackupRepository, Result>,
      ICommandHandler<PruneBackupRepository, Result>
{
    public ValueTask<Result> Handle(InitializeBackupRepository command, CancellationToken cancellationToken)
        => destinationService.InitializeAsync(
            command.RepositoryId,
            new BackupExecutionContext(command.Input.Location, command.Input.PlatformId),
            cancellationToken);

    public ValueTask<Result> Handle(CheckBackupRepository command, CancellationToken cancellationToken)
        => destinationService.CheckAsync(
            command.RepositoryId,
            new BackupExecutionContext(command.Input.Location, command.Input.PlatformId),
            cancellationToken);

    public ValueTask<Result> Handle(PruneBackupRepository command, CancellationToken cancellationToken)
        => destinationService.PruneAsync(
            command.RepositoryId,
            new BackupExecutionContext(command.Input.Location, command.Input.PlatformId),
            cancellationToken);
}
