using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Networks;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Networks.Queries;

public sealed record ListNetworks (Guid PlatformId, bool? Dangling = null, string? Driver = null, string? Id = null, string? Name = null) : IQuery<Result<IEnumerable<DockerNetwork>>>;

internal class ListNetworksHandler(IUnitOfWork unitOfWork, IConnectorFactory<INetworkConnector> connectorFactory) : IQueryHandler<ListNetworks, Result<IEnumerable<DockerNetwork>>>
{
    public async ValueTask<Result<IEnumerable<DockerNetwork>>> Handle(ListNetworks query, CancellationToken cancellationToken)
    {
        var (address, connectorType) = await unitOfWork.Platforms.GetPlatformInfoAsync(query.PlatformId, cancellationToken);
        if (string.IsNullOrEmpty(address))
        {
            return Result.Failure<IEnumerable<DockerNetwork>>(new NotFoundError("The provided platform Id doesn't exist"));
        }

        var args = new ListNetworksCommand
        (
            PlatformAddress: address,
            Id: query.Id,
            Name: query.Name,
            Driver: query.Driver,
            Dangling: query.Dangling
        );

        var networkConnector = connectorFactory.GetConnector(connectorType);
        return await networkConnector.ListNetworksAsync(args, cancellationToken);
    }
}
