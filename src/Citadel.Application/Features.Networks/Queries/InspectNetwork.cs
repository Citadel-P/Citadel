using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Networks;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Networks.Queries;

public sealed record InspectNetwork(Guid PlatformId, string NetworkId): IQuery<Result<DockerNetworkDetails>>
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

internal sealed class InspectNetworkHandler(IPlatformContainerCache platformContainerCache, IConnectorFactory<INetworkConnector> connectorFactory) 
    : IQueryHandler<InspectNetwork, Result<DockerNetworkDetails>>
{
    public async ValueTask<Result<DockerNetworkDetails>> Handle(InspectNetwork query, CancellationToken cancellationToken)
    {
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
        return await networkConnector.InspectNetworkAsync(args, cancellationToken);
    }
}
