using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Networks;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;
using Domain.Entities.Platforms;

namespace Application.Features.Networks.Queries;

[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record InspectNetwork(Guid PlatformId, string NetworkId, string? DockerNodeId = null): IQuery<Result<DockerNetworkDetails>>
{
    internal class Validator : AbstractValidator<InspectNetwork>
    {
        public Validator()
        {
            RuleFor(s => s.PlatformId).NotEmpty().NotNull();
            RuleFor(s => s.NetworkId).ValidHashId();
        }
    }
}

internal sealed class InspectNetworkHandler(
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<INetworkConnector> connectorFactory,
    IUnitOfWork unitOfWork,
    ISwarmNodeRuntimeConnector swarmNodeRuntimeConnector)
    : IQueryHandler<InspectNetwork, Result<DockerNetworkDetails>>
{
    public async ValueTask<Result<DockerNetworkDetails>> Handle(InspectNetwork query, CancellationToken cancellationToken)
    {
        var persistedPlatform = await unitOfWork.Platforms.GetByIdAsync(query.PlatformId, cancellationToken);
        if (persistedPlatform?.PlatformDescriptor is DockerSwarmPlatformDescriptor descriptor)
        {
            var dockerNodeId = string.IsNullOrWhiteSpace(query.DockerNodeId)
                ? descriptor.NodeID
                : query.DockerNodeId;
            var swarmResult = await swarmNodeRuntimeConnector.InspectNetworkAsync(
                persistedPlatform,
                dockerNodeId,
                query.NetworkId,
                cancellationToken);
            if (swarmResult.IsSuccess(out var swarmNetwork))
            {
                swarmNetwork.PlatformId = query.PlatformId;
                swarmNetwork.DockerNodeId = dockerNodeId;
            }
            return swarmResult;
        }

        if (!platformContainerCache.TryGetCacheEntry(query.PlatformId, out var platform, out var error))
        {
            return Result.Failure<DockerNetworkDetails>(error);
        }

        var args = new InspectNetworkCommand
        (
            NetworkId: query.NetworkId, 
            PlatformAddress: platform.Address
        );

        var networkConnector = connectorFactory.GetConnector(platform.ConnectorType);
        var result = await networkConnector.InspectNetworkAsync(args, cancellationToken);
        if (result.IsSuccess(out var inspectResult))
        {
            inspectResult.PlatformId = query.PlatformId;
            return inspectResult;
        }
        return result;
    }
}
