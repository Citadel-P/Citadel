using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Networks;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Application.TaskJobs;
using Domain.Entities.Platforms;

namespace Application.Features.Networks.Commands;

[RequirePermission(ResourceType.Platform, PermissionLevel.Execute)]
public sealed record class DeleteNetworks(Guid PlatformId, string[] Ids) : ICommand<Result>
{
    internal class Validator : AbstractValidator<DeleteNetworks>
    {
        public Validator()
        {
            RuleFor(s => s.PlatformId).NotEmpty().NotNull();
            RuleFor(s => s.Ids).NotNull().NotEmpty();
            RuleFor(s => s.Ids).Must(static ids => ids is null || ids.Length <= 100)
                .WithMessage("At most 100 networks can be deleted at once.");
            RuleForEach(s => s.Ids).ValidHashId();
        }
    }
}

internal class DeleteNetworksHandler(
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<INetworkConnector> connectorFactory,
    IUnitOfWork unitOfWork,
    ISwarmReconciliationCoordinator reconciliationCoordinator) : ICommandHandler<DeleteNetworks, Result>
{
    public async ValueTask<Result> Handle(DeleteNetworks command, CancellationToken cancellationToken)
    {
        if (!platformContainerCache.TryGetCacheEntry(command.PlatformId, out var platform, out var error))
        {
            return Result.Failure(error);
        }

        var networkConnector = connectorFactory.GetConnector(platform.ConnectorType);
        var persistedPlatform = await unitOfWork.Platforms.GetByIdAsync(command.PlatformId, cancellationToken);
        var isSwarmPlatform = persistedPlatform?.PlatformDescriptor is DockerSwarmPlatformDescriptor;
        var ids = command.Ids.Distinct(StringComparer.Ordinal).ToArray();
        if (isSwarmPlatform)
        {
            var localNetworks = await unitOfWork.Swarm.GetNodeNetworksAsync(command.PlatformId, cancellationToken);
            if (ids.Any(id => localNetworks.Any(network =>
                    string.Equals(network.DockerNetworkId, id, StringComparison.OrdinalIgnoreCase))))
            {
                return Result.Failure(new ConflictError(
                    "Node-local Network deletion requires an explicit Node target and is not available."));
            }
        }

        foreach (var id in ids)
        {
            var inspection = await networkConnector.InspectNetworkAsync(
                new InspectNetworkCommand(platform.Address, id), cancellationToken);
            if (inspection.IsFailure(out var inspectionError, out var network))
                return Result.Failure(inspectionError);
            if (network.IsSystem)
                return Result.Failure(new ConflictError($"System network '{network.Name}' cannot be deleted."));
            if (network.Containers.Count != 0)
                return Result.Failure(new ConflictError($"Network '{network.Name}' is in use and cannot be deleted."));
            if (network.Labels.ContainsKey("com.docker.stack.namespace")
                || network.Labels.ContainsKey("com.citadel.stack-id"))
                return Result.Failure(new ConflictError($"Stack-owned network '{network.Name}' cannot be deleted independently."));

            if (!isSwarmPlatform
                || !string.Equals(network.Scope, "swarm", StringComparison.OrdinalIgnoreCase))
                continue;

            var projected = await unitOfWork.Swarm.GetNetworkAsync(command.PlatformId, id, cancellationToken);
            if (projected is null || projected.IsStale)
                return Result.Failure(new ConflictError(
                    $"Swarm network '{network.Name}' has no current inventory observation and cannot be deleted."));
            if (projected.ServiceNames.Count != 0)
                return Result.Failure(new ConflictError($"Network '{network.Name}' is used by one or more Services and cannot be deleted."));
        }

        var deleted = 0;
        try
        {
            foreach (var id in ids)
            {
                var result = await networkConnector.DeleteNetworkAsync(
                    new DeleteDockerNetworkCommand(platform.Address, [id]), cancellationToken);
                if (result.IsSuccess())
                {
                    deleted++;
                    continue;
                }

                result.IsFailure(out var deleteError);
                if (deleteError is NotFoundError)
                {
                    deleted++;
                    continue;
                }

                return deleted == 0
                    ? Result.Failure(deleteError!)
                    : Result.Failure(new ConflictError(
                        $"Deleted {deleted} of {ids.Length} networks before Docker rejected the operation: {deleteError!.Message}"));
            }

            return Result.Success();
        }
        finally
        {
            if (isSwarmPlatform)
                await reconciliationCoordinator.RefreshAsync(command.PlatformId, CancellationToken.None);
        }
    }
}
