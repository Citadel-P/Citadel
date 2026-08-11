using System.Text;
using Application.Services;
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
public sealed record UpdateSwarmNode(
    Guid PlatformId,
    string NodeId,
    long VersionIndex,
    string Availability,
    IReadOnlyDictionary<string, string> Labels) : ICommand<Result>
{
    internal sealed class Validator : AbstractValidator<UpdateSwarmNode>
    {
        private static readonly string[] SupportedAvailability = ["Active", "Pause", "Drain"];

        public Validator()
        {
            RuleFor(value => value.PlatformId).NotEmpty();
            RuleFor(value => value.NodeId).NotEmpty().MaximumLength(255);
            RuleFor(value => value.VersionIndex).GreaterThanOrEqualTo(0);
            RuleFor(value => value.Availability)
                .Must(static value => SupportedAvailability.Contains(value, StringComparer.OrdinalIgnoreCase))
                .WithMessage("Availability must be Active, Pause, or Drain.");
            RuleForEach(value => value.Labels).SetValidator(new KeyPairValidator());
        }
    }
}

public sealed record SwarmNodeAvailabilityTarget(string NodeId, long VersionIndex);

[RequirePermission(ResourceType.Platform, PermissionLevel.Write)]
public sealed record UpdateSwarmNodesAvailability(
    Guid PlatformId,
    IReadOnlyList<SwarmNodeAvailabilityTarget> Nodes,
    string Availability) : ICommand<Result>
{
    internal sealed class Validator : AbstractValidator<UpdateSwarmNodesAvailability>
    {
        private static readonly string[] SupportedAvailability = ["Active", "Pause", "Drain"];

        public Validator()
        {
            RuleFor(value => value.PlatformId).NotEmpty();
            RuleFor(value => value.Nodes)
                .NotEmpty()
                .Must(static nodes => nodes is null || nodes.Count <= 100)
                .WithMessage("At most 100 nodes can be updated at once.")
                .Must(static nodes => nodes is null
                    || nodes.Select(node => node.NodeId).Distinct(StringComparer.Ordinal).Count() == nodes.Count)
                .WithMessage("Each node can only be selected once.");
            RuleForEach(value => value.Nodes).ChildRules(node =>
            {
                node.RuleFor(value => value.NodeId).NotEmpty().MaximumLength(255);
                node.RuleFor(value => value.VersionIndex).GreaterThanOrEqualTo(0);
            });
            RuleFor(value => value.Availability)
                .Must(static value => SupportedAvailability.Contains(value, StringComparer.OrdinalIgnoreCase))
                .WithMessage("Availability must be Active, Pause, or Drain.");
        }
    }
}

[RequirePermission(ResourceType.Platform, PermissionLevel.Execute)]
public sealed record RestartSwarmService(Guid PlatformId, string ServiceId) : ICommand<Result>
{
    internal sealed class Validator : AbstractValidator<RestartSwarmService>
    {
        public Validator()
        {
            RuleFor(value => value.PlatformId).NotEmpty();
            RuleFor(value => value.ServiceId).NotEmpty().MaximumLength(255);
        }
    }
}

[RequirePermission(ResourceType.Platform, PermissionLevel.Execute)]
public sealed record DeleteSwarmServices(Guid PlatformId, IReadOnlyList<string> ServiceIds) : ICommand<Result>
{
    internal sealed class Validator : AbstractValidator<DeleteSwarmServices>
    {
        public Validator()
        {
            RuleFor(value => value.PlatformId).NotEmpty();
            RuleFor(value => value.ServiceIds).NotEmpty().Must(static values => values is null || values.Count <= 100)
                .WithMessage("At most 100 services can be deleted at once.");
            RuleForEach(value => value.ServiceIds).NotEmpty().MaximumLength(255);
        }
    }
}

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
            RuleFor(value => value.SecretIds).NotEmpty().Must(static values => values is null || values.Count <= 100)
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
            RuleFor(value => value.ConfigIds).NotEmpty().Must(static values => values is null || values.Count <= 100)
                .WithMessage("At most 100 configs can be deleted at once.");
            RuleForEach(value => value.ConfigIds).NotEmpty().MaximumLength(255);
        }
    }
}

internal sealed class CreateSwarmSecretHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> connectorFactory,
    ISwarmManagerIdentityValidator managerIdentityValidator,
    ISwarmReconciliationCoordinator reconciliationCoordinator)
    : ICommandHandler<CreateSwarmSecret, Result>
{
    public async ValueTask<Result> Handle(CreateSwarmSecret command, CancellationToken cancellationToken)
    {
        var context = await SwarmMutationContext.LoadAsync(unitOfWork, connectorFactory, managerIdentityValidator, command.PlatformId, cancellationToken);
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

internal sealed class UpdateSwarmNodeHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> connectorFactory,
    ISwarmManagerIdentityValidator managerIdentityValidator,
    ISwarmReconciliationCoordinator reconciliationCoordinator)
    : ICommandHandler<UpdateSwarmNode, Result>
{
    public async ValueTask<Result> Handle(UpdateSwarmNode command, CancellationToken cancellationToken)
    {
        var context = await SwarmMutationContext.LoadAsync(unitOfWork, connectorFactory, managerIdentityValidator, command.PlatformId, cancellationToken);
        if (!context.IsSuccess(out var value, out var error))
            return Result.Failure(error!);

        var projected = await unitOfWork.Swarm.GetNodeAsync(command.PlatformId, command.NodeId, cancellationToken);
        if (projected is null)
            return Result.Failure(new NotFoundError($"Swarm node '{command.NodeId}' does not exist."));
        if (projected.IsStale)
            return Result.Failure(new ConflictError($"Node '{projected.Hostname}' is stale and cannot be changed."));
        if (projected.VersionIndex != command.VersionIndex)
            return Result.Failure(new ConflictError($"Node '{projected.Hostname}' changed. Reload it before saving."));

        return await SwarmMutationExecution.ExecuteAndRefreshAsync(
            () => value.Connector.UpdateNodeAsync(
                new UpdateSwarmNodeCommand(
                    value.Platform.Address,
                    command.NodeId,
                    command.VersionIndex,
                    command.Availability,
                    command.Labels),
                cancellationToken),
            reconciliationCoordinator,
            command.PlatformId);
    }
}

internal sealed class UpdateSwarmNodesAvailabilityHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> connectorFactory,
    ISwarmManagerIdentityValidator managerIdentityValidator,
    ISwarmReconciliationCoordinator reconciliationCoordinator)
    : ICommandHandler<UpdateSwarmNodesAvailability, Result>
{
    public async ValueTask<Result> Handle(UpdateSwarmNodesAvailability command, CancellationToken cancellationToken)
    {
        var context = await SwarmMutationContext.LoadAsync(unitOfWork, connectorFactory, managerIdentityValidator, command.PlatformId, cancellationToken);
        if (!context.IsSuccess(out var value, out var error))
            return Result.Failure(error!);

        var updates = new List<UpdateSwarmNodeCommand>(command.Nodes.Count);
        foreach (var target in command.Nodes)
        {
            var projected = await unitOfWork.Swarm.GetNodeAsync(command.PlatformId, target.NodeId, cancellationToken);
            if (projected is null)
                return Result.Failure(new NotFoundError($"Swarm node '{target.NodeId}' does not exist."));
            if (projected.IsStale)
                return Result.Failure(new ConflictError($"Node '{projected.Hostname}' is stale and cannot be changed."));
            if (projected.VersionIndex != target.VersionIndex)
                return Result.Failure(new ConflictError($"Node '{projected.Hostname}' changed. Reload it before saving."));
            if (string.Equals(projected.Availability, command.Availability, StringComparison.OrdinalIgnoreCase))
                continue;

            updates.Add(new UpdateSwarmNodeCommand(
                value.Platform.Address,
                target.NodeId,
                target.VersionIndex,
                command.Availability,
                projected.Labels));
        }

        if (updates.Count == 0)
            return Result.Success();

        var updated = 0;
        try
        {
            foreach (var update in updates)
            {
                var result = await value.Connector.UpdateNodeAsync(update, cancellationToken);
                if (result.IsSuccess())
                {
                    updated++;
                    continue;
                }

                result.IsFailure(out error);
                return updated == 0
                    ? Result.Failure(error!)
                    : Result.Failure(new ConflictError(
                        $"Updated {updated} of {updates.Count} nodes before Docker rejected the operation: {error!.Message}"));
            }

            return Result.Success();
        }
        finally
        {
            await reconciliationCoordinator.RefreshAsync(command.PlatformId, CancellationToken.None);
        }
    }
}

internal sealed class RestartSwarmServiceHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> connectorFactory,
    ISwarmManagerIdentityValidator managerIdentityValidator,
    ISwarmReconciliationCoordinator reconciliationCoordinator)
    : ICommandHandler<RestartSwarmService, Result>
{
    public async ValueTask<Result> Handle(RestartSwarmService command, CancellationToken cancellationToken)
    {
        var context = await SwarmMutationContext.LoadAsync(unitOfWork, connectorFactory, managerIdentityValidator, command.PlatformId, cancellationToken);
        if (!context.IsSuccess(out var value, out var error))
            return Result.Failure(error!);

        var validation = await NativeSwarmServiceValidation.ValidateAsync(
            unitOfWork, command.PlatformId, command.ServiceId, cancellationToken);
        if (validation.IsFailure(out error))
            return Result.Failure(error!);

        return await SwarmMutationExecution.ExecuteAndRefreshAsync(
            () => value.Connector.RestartServiceAsync(
                new RestartSwarmServiceCommand(value.Platform.Address, command.ServiceId),
                cancellationToken),
            reconciliationCoordinator,
            command.PlatformId);
    }
}

internal sealed class DeleteSwarmServicesHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> connectorFactory,
    ISwarmManagerIdentityValidator managerIdentityValidator,
    ISwarmReconciliationCoordinator reconciliationCoordinator)
    : ICommandHandler<DeleteSwarmServices, Result>
{
    public async ValueTask<Result> Handle(DeleteSwarmServices command, CancellationToken cancellationToken)
    {
        var context = await SwarmMutationContext.LoadAsync(unitOfWork, connectorFactory, managerIdentityValidator, command.PlatformId, cancellationToken);
        if (!context.IsSuccess(out var value, out var error))
            return Result.Failure(error!);

        var ids = command.ServiceIds.Distinct(StringComparer.Ordinal).ToArray();
        foreach (var id in ids)
        {
            var validation = await NativeSwarmServiceValidation.ValidateAsync(
                unitOfWork, command.PlatformId, id, cancellationToken);
            if (validation.IsFailure(out error))
                return Result.Failure(error!);
        }

        var deleted = 0;
        foreach (var id in ids)
        {
            Result result;
            try
            {
                result = await value.Connector.DeleteInventoryServiceAsync(
                    new DeleteSwarmInventoryServiceCommand(value.Platform.Address, id),
                    cancellationToken);
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
                        $"Deleted {deleted} of {ids.Length} services before Docker rejected the operation: {error!.Message}"));
            }

            deleted++;
        }

        await reconciliationCoordinator.RefreshAsync(command.PlatformId, CancellationToken.None);
        return Result.Success();
    }
}

internal static class NativeSwarmServiceValidation
{
    internal static async Task<Result> ValidateAsync(
        IUnitOfWork unitOfWork,
        Guid platformId,
        string serviceId,
        CancellationToken cancellationToken)
    {
        var projected = await unitOfWork.Swarm.GetServiceAsync(platformId, serviceId, cancellationToken);
        if (projected is null)
            return Result.Failure(new NotFoundError($"Swarm service '{serviceId}' does not exist."));
        if (projected.IsStale)
            return Result.Failure(new ConflictError($"Service '{projected.Name}' is stale and cannot be changed."));

        var managed = await unitOfWork.SwarmServices.GetByDockerServiceIdAsync(
            platformId, serviceId, cancellationToken);
        return managed is null
            ? Result.Success()
            : Result.Failure(new ConflictError(
                $"Service '{projected.Name}' is managed by Citadel and must be changed through its managed Service."));
    }
}

internal sealed class UpdateSwarmSecretLabelsHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> connectorFactory,
    ISwarmManagerIdentityValidator managerIdentityValidator,
    ISwarmReconciliationCoordinator reconciliationCoordinator)
    : ICommandHandler<UpdateSwarmSecretLabels, Result>
{
    public async ValueTask<Result> Handle(UpdateSwarmSecretLabels command, CancellationToken cancellationToken)
    {
        var context = await SwarmMutationContext.LoadAsync(unitOfWork, connectorFactory, managerIdentityValidator, command.PlatformId, cancellationToken);
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
    ISwarmManagerIdentityValidator managerIdentityValidator,
    ISwarmReconciliationCoordinator reconciliationCoordinator)
    : ICommandHandler<DeleteSwarmSecrets, Result>
{
    public async ValueTask<Result> Handle(DeleteSwarmSecrets command, CancellationToken cancellationToken)
    {
        var context = await SwarmMutationContext.LoadAsync(unitOfWork, connectorFactory, managerIdentityValidator, command.PlatformId, cancellationToken);
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
    ISwarmManagerIdentityValidator managerIdentityValidator,
    ISwarmReconciliationCoordinator reconciliationCoordinator)
    : ICommandHandler<CreateSwarmConfig, Result>
{
    public async ValueTask<Result> Handle(CreateSwarmConfig command, CancellationToken cancellationToken)
    {
        var context = await SwarmMutationContext.LoadAsync(unitOfWork, connectorFactory, managerIdentityValidator, command.PlatformId, cancellationToken);
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
    ISwarmManagerIdentityValidator managerIdentityValidator,
    ISwarmReconciliationCoordinator reconciliationCoordinator)
    : ICommandHandler<UpdateSwarmConfigLabels, Result>
{
    public async ValueTask<Result> Handle(UpdateSwarmConfigLabels command, CancellationToken cancellationToken)
    {
        var context = await SwarmMutationContext.LoadAsync(unitOfWork, connectorFactory, managerIdentityValidator, command.PlatformId, cancellationToken);
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
    ISwarmManagerIdentityValidator managerIdentityValidator,
    ISwarmReconciliationCoordinator reconciliationCoordinator)
    : ICommandHandler<DeleteSwarmConfigs, Result>
{
    public async ValueTask<Result> Handle(DeleteSwarmConfigs command, CancellationToken cancellationToken)
    {
        var context = await SwarmMutationContext.LoadAsync(unitOfWork, connectorFactory, managerIdentityValidator, command.PlatformId, cancellationToken);
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
        ISwarmManagerIdentityValidator managerIdentityValidator,
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

        var managerIdentity = await managerIdentityValidator.ValidateAsync(platform, cancellationToken);
        if (managerIdentity.IsFailure(out var managerIdentityError))
            return Result.Failure<SwarmMutationContext>(managerIdentityError!);

        return Result.Success(new SwarmMutationContext(
            platform,
            connectorFactory.GetConnector(platform.ConnectorType)));
    }
}

internal static class SwarmMutationExecution
{
    internal static async Task<Result> ExecuteAndRefreshAsync<T>(
        Func<Task<Result<T>>> mutation,
        ISwarmReconciliationCoordinator reconciliationCoordinator,
        Guid platformId)
    {
        try
        {
            var result = await mutation();
            await reconciliationCoordinator.RefreshAsync(platformId, CancellationToken.None);
            return result.IsSuccess(out _, out var error)
                ? Result.Success()
                : Result.Failure(error!);
        }
        catch (OperationCanceledException)
        {
            await reconciliationCoordinator.RefreshAsync(platformId, CancellationToken.None);
            throw;
        }
    }

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
