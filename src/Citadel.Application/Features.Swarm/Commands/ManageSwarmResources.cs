using System.Text;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Swarm;
using Domain.Entities.Platforms;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using static Hosting.Common.Validators;

namespace Application.Features.Swarm.Commands;

[RequirePermission(ResourceType.Platform, PermissionLevel.Write)]
public sealed record CreateSwarmSecret(
    Guid PlatformId,
    string Name,
    string Data,
    IReadOnlyDictionary<string, string> Labels) : ICommand<Result>
{
    internal sealed class Validator : AbstractValidator<CreateSwarmSecret>
    {
        public Validator()
        {
            RuleFor(value => value.PlatformId).NotEmpty();
            RuleFor(value => value.Name).NotEmpty().MaximumLength(255);
            RuleFor(value => value.Data)
                .NotEmpty()
                .Must(static value => value is null || Encoding.UTF8.GetByteCount(value) <= 500 * 1024)
                .WithMessage("Secret data cannot exceed 500 KiB.");
            RuleForEach(value => value.Labels).SetValidator(new KeyPairValidator());
        }
    }
}

[RequirePermission(ResourceType.Platform, PermissionLevel.Write)]
public sealed record UpdateSwarmSecretLabels(
    Guid PlatformId,
    string SecretId,
    long VersionIndex,
    IReadOnlyDictionary<string, string> Labels) : ICommand<Result>
{
    internal sealed class Validator : AbstractValidator<UpdateSwarmSecretLabels>
    {
        public Validator()
        {
            RuleFor(value => value.PlatformId).NotEmpty();
            RuleFor(value => value.SecretId).NotEmpty().MaximumLength(255);
            RuleFor(value => value.VersionIndex).GreaterThanOrEqualTo(0);
            RuleForEach(value => value.Labels).SetValidator(new KeyPairValidator());
        }
    }
}

[RequirePermission(ResourceType.Platform, PermissionLevel.Write)]
public sealed record DeleteSwarmSecrets(Guid PlatformId, IReadOnlyList<string> SecretIds) : ICommand<Result>
{
    internal sealed class Validator : AbstractValidator<DeleteSwarmSecrets>
    {
        public Validator()
        {
            RuleFor(value => value.PlatformId).NotEmpty();
            RuleFor(value => value.SecretIds).NotEmpty().Must(static values => values.Count <= 100)
                .WithMessage("At most 100 secrets can be deleted at once.");
            RuleForEach(value => value.SecretIds).NotEmpty().MaximumLength(255);
        }
    }
}

[RequirePermission(ResourceType.Platform, PermissionLevel.Write)]
public sealed record CreateSwarmConfig(
    Guid PlatformId,
    string Name,
    string Data,
    IReadOnlyDictionary<string, string> Labels) : ICommand<Result>
{
    internal sealed class Validator : AbstractValidator<CreateSwarmConfig>
    {
        public Validator()
        {
            RuleFor(value => value.PlatformId).NotEmpty();
            RuleFor(value => value.Name).NotEmpty().MaximumLength(255);
            RuleFor(value => value.Data)
                .Must(static value => value is null || Encoding.UTF8.GetByteCount(value) <= 1000 * 1024)
                .WithMessage("Config data cannot exceed 1000 KiB.");
            RuleForEach(value => value.Labels).SetValidator(new KeyPairValidator());
        }
    }
}

[RequirePermission(ResourceType.Platform, PermissionLevel.Write)]
public sealed record UpdateSwarmConfigLabels(
    Guid PlatformId,
    string ConfigId,
    long VersionIndex,
    IReadOnlyDictionary<string, string> Labels) : ICommand<Result>
{
    internal sealed class Validator : AbstractValidator<UpdateSwarmConfigLabels>
    {
        public Validator()
        {
            RuleFor(value => value.PlatformId).NotEmpty();
            RuleFor(value => value.ConfigId).NotEmpty().MaximumLength(255);
            RuleFor(value => value.VersionIndex).GreaterThanOrEqualTo(0);
            RuleForEach(value => value.Labels).SetValidator(new KeyPairValidator());
        }
    }
}

[RequirePermission(ResourceType.Platform, PermissionLevel.Write)]
public sealed record DeleteSwarmConfigs(Guid PlatformId, IReadOnlyList<string> ConfigIds) : ICommand<Result>
{
    internal sealed class Validator : AbstractValidator<DeleteSwarmConfigs>
    {
        public Validator()
        {
            RuleFor(value => value.PlatformId).NotEmpty();
            RuleFor(value => value.ConfigIds).NotEmpty().Must(static values => values.Count <= 100)
                .WithMessage("At most 100 configs can be deleted at once.");
            RuleForEach(value => value.ConfigIds).NotEmpty().MaximumLength(255);
        }
    }
}

internal sealed class CreateSwarmSecretHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> connectorFactory,
    ISwarmReconciliationCoordinator reconciliationCoordinator)
    : ICommandHandler<CreateSwarmSecret, Result>
{
    public async ValueTask<Result> Handle(CreateSwarmSecret command, CancellationToken cancellationToken)
    {
        var context = await SwarmMutationContext.LoadAsync(unitOfWork, connectorFactory, command.PlatformId, cancellationToken);
        if (!context.IsSuccess(out var value, out var error))
            return Result.Failure(error!);

        return await SwarmMutationExecution.ExecuteAndRefreshAsync(
            () => value.Connector.CreateSecretAsync(
                new CreateSwarmSecretCommand(value.Platform.Address, command.Name, Encoding.UTF8.GetBytes(command.Data), command.Labels),
                cancellationToken),
            reconciliationCoordinator,
            command.PlatformId);
    }
}

internal sealed class UpdateSwarmSecretLabelsHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> connectorFactory,
    ISwarmReconciliationCoordinator reconciliationCoordinator)
    : ICommandHandler<UpdateSwarmSecretLabels, Result>
{
    public async ValueTask<Result> Handle(UpdateSwarmSecretLabels command, CancellationToken cancellationToken)
    {
        var context = await SwarmMutationContext.LoadAsync(unitOfWork, connectorFactory, command.PlatformId, cancellationToken);
        if (!context.IsSuccess(out var value, out var error))
            return Result.Failure(error!);

        var projected = await unitOfWork.Swarm.GetSecretAsync(command.PlatformId, command.SecretId, cancellationToken);
        if (projected is null)
            return Result.Failure(new NotFoundError("Swarm secret does not exist."));
        if (projected.IsStale)
            return Result.Failure(new ConflictError("The secret inventory is stale. Refresh the platform before editing labels."));

        return await SwarmMutationExecution.ExecuteAndRefreshAsync(
            () => value.Connector.UpdateSecretLabelsAsync(
                new UpdateSwarmSecretLabelsCommand(value.Platform.Address, command.SecretId, command.VersionIndex, command.Labels),
                cancellationToken),
            reconciliationCoordinator,
            command.PlatformId);
    }
}

internal sealed class DeleteSwarmSecretsHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> connectorFactory,
    ISwarmReconciliationCoordinator reconciliationCoordinator)
    : ICommandHandler<DeleteSwarmSecrets, Result>
{
    public async ValueTask<Result> Handle(DeleteSwarmSecrets command, CancellationToken cancellationToken)
    {
        var context = await SwarmMutationContext.LoadAsync(unitOfWork, connectorFactory, command.PlatformId, cancellationToken);
        if (!context.IsSuccess(out var value, out var error))
            return Result.Failure(error!);

        var ids = command.SecretIds.Distinct(StringComparer.Ordinal).ToArray();
        foreach (var id in ids)
        {
            var projected = await unitOfWork.Swarm.GetSecretAsync(command.PlatformId, id, cancellationToken);
            if (projected is null)
                return Result.Failure(new NotFoundError($"Swarm secret '{id}' does not exist."));
            if (projected.IsStale)
                return Result.Failure(new ConflictError($"Secret '{projected.Name}' is stale and cannot be deleted."));
            if (projected.ServiceNames.Count != 0)
                return Result.Failure(new ConflictError($"Secret '{projected.Name}' is in use and cannot be deleted."));
        }

        var deleted = 0;
        foreach (var id in ids)
        {
            Result result;
            try
            {
                result = await value.Connector.DeleteSecretAsync(
                    new DeleteSwarmSecretCommand(value.Platform.Address, id), cancellationToken);
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                await reconciliationCoordinator.RefreshAsync(command.PlatformId, CancellationToken.None);
                throw;
            }
            if (result.IsFailure(out error))
            {
                if (error is NotFoundError)
                {
                    deleted++;
                    continue;
                }

                await reconciliationCoordinator.RefreshAsync(command.PlatformId, CancellationToken.None);
                return deleted == 0
                    ? Result.Failure(error!)
                    : Result.Failure(new ConflictError(
                        $"Deleted {deleted} of {ids.Length} secrets before Docker rejected the operation: {error!.Message}"));
            }
            deleted++;
        }

        await reconciliationCoordinator.RefreshAsync(command.PlatformId, CancellationToken.None);
        return Result.Success();
    }
}

internal sealed class CreateSwarmConfigHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> connectorFactory,
    ISwarmReconciliationCoordinator reconciliationCoordinator)
    : ICommandHandler<CreateSwarmConfig, Result>
{
    public async ValueTask<Result> Handle(CreateSwarmConfig command, CancellationToken cancellationToken)
    {
        var context = await SwarmMutationContext.LoadAsync(unitOfWork, connectorFactory, command.PlatformId, cancellationToken);
        if (!context.IsSuccess(out var value, out var error))
            return Result.Failure(error!);

        return await SwarmMutationExecution.ExecuteAndRefreshAsync(
            () => value.Connector.CreateConfigAsync(
                new CreateSwarmConfigCommand(value.Platform.Address, command.Name, Encoding.UTF8.GetBytes(command.Data), command.Labels),
                cancellationToken),
            reconciliationCoordinator,
            command.PlatformId);
    }
}

internal sealed class UpdateSwarmConfigLabelsHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> connectorFactory,
    ISwarmReconciliationCoordinator reconciliationCoordinator)
    : ICommandHandler<UpdateSwarmConfigLabels, Result>
{
    public async ValueTask<Result> Handle(UpdateSwarmConfigLabels command, CancellationToken cancellationToken)
    {
        var context = await SwarmMutationContext.LoadAsync(unitOfWork, connectorFactory, command.PlatformId, cancellationToken);
        if (!context.IsSuccess(out var value, out var error))
            return Result.Failure(error!);

        var projected = await unitOfWork.Swarm.GetConfigAsync(command.PlatformId, command.ConfigId, cancellationToken);
        if (projected is null)
            return Result.Failure(new NotFoundError("Swarm config does not exist."));
        if (projected.IsStale)
            return Result.Failure(new ConflictError("The config inventory is stale. Refresh the platform before editing labels."));

        return await SwarmMutationExecution.ExecuteAndRefreshAsync(
            () => value.Connector.UpdateConfigLabelsAsync(
                new UpdateSwarmConfigLabelsCommand(value.Platform.Address, command.ConfigId, command.VersionIndex, command.Labels),
                cancellationToken),
            reconciliationCoordinator,
            command.PlatformId);
    }
}

internal sealed class DeleteSwarmConfigsHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> connectorFactory,
    ISwarmReconciliationCoordinator reconciliationCoordinator)
    : ICommandHandler<DeleteSwarmConfigs, Result>
{
    public async ValueTask<Result> Handle(DeleteSwarmConfigs command, CancellationToken cancellationToken)
    {
        var context = await SwarmMutationContext.LoadAsync(unitOfWork, connectorFactory, command.PlatformId, cancellationToken);
        if (!context.IsSuccess(out var value, out var error))
            return Result.Failure(error!);

        var ids = command.ConfigIds.Distinct(StringComparer.Ordinal).ToArray();
        foreach (var id in ids)
        {
            var projected = await unitOfWork.Swarm.GetConfigAsync(command.PlatformId, id, cancellationToken);
            if (projected is null)
                return Result.Failure(new NotFoundError($"Swarm config '{id}' does not exist."));
            if (projected.IsStale)
                return Result.Failure(new ConflictError($"Config '{projected.Name}' is stale and cannot be deleted."));
            if (projected.ServiceNames.Count != 0)
                return Result.Failure(new ConflictError($"Config '{projected.Name}' is in use and cannot be deleted."));
        }

        var deleted = 0;
        foreach (var id in ids)
        {
            Result result;
            try
            {
                result = await value.Connector.DeleteConfigAsync(
                    new DeleteSwarmConfigCommand(value.Platform.Address, id), cancellationToken);
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                await reconciliationCoordinator.RefreshAsync(command.PlatformId, CancellationToken.None);
                throw;
            }
            if (result.IsFailure(out error))
            {
                if (error is NotFoundError)
                {
                    deleted++;
                    continue;
                }

                await reconciliationCoordinator.RefreshAsync(command.PlatformId, CancellationToken.None);
                return deleted == 0
                    ? Result.Failure(error!)
                    : Result.Failure(new ConflictError(
                        $"Deleted {deleted} of {ids.Length} configs before Docker rejected the operation: {error!.Message}"));
            }
            deleted++;
        }

        await reconciliationCoordinator.RefreshAsync(command.PlatformId, CancellationToken.None);
        return Result.Success();
    }
}

internal sealed record SwarmMutationContext(Platform Platform, ISwarmConnector Connector)
{
    internal static async Task<Result<SwarmMutationContext>> LoadAsync(
        IUnitOfWork unitOfWork,
        IConnectorFactory<ISwarmConnector> connectorFactory,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetByIdAsync(platformId, cancellationToken);
        if (platform is null)
            return Result.Failure<SwarmMutationContext>(new NotFoundError("Platform does not exist."));
        if (platform.PlatformDescriptor is not DockerSwarmPlatformDescriptor)
            return Result.Failure<SwarmMutationContext>(new BadRequestError("This operation requires a Docker Swarm platform."));
        if (platform.Status != PlatformStatus.Online)
            return Result.Failure<SwarmMutationContext>(new ConflictError("Platform is offline."));

        return Result.Success(new SwarmMutationContext(
            platform,
            connectorFactory.GetConnector(platform.ConnectorType)));
    }
}

internal static class SwarmMutationExecution
{
    internal static async Task<Result> ExecuteAndRefreshAsync(
        Func<Task<Result>> mutation,
        ISwarmReconciliationCoordinator reconciliationCoordinator,
        Guid platformId)
    {
        try
        {
            var result = await mutation();
            await reconciliationCoordinator.RefreshAsync(platformId, CancellationToken.None);
            return result;
        }
        catch (OperationCanceledException)
        {
            await reconciliationCoordinator.RefreshAsync(platformId, CancellationToken.None);
            throw;
        }
    }
}
