using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Networks;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Networks.Queries;

public sealed record ListNetworks (Guid PlatformId, bool? Dangling = null, string? Driver = null, string? Id = null, string? Name = null) : IQuery<Result<IEnumerable<DockerNetwork>>>;

internal class ListNetworksHandler(IConnectorFactory<INetworkConnector> connectorFactory, ApplicationDbContext dbContext) : IQueryHandler<ListNetworks, Result<IEnumerable<DockerNetwork>>>
{
    public async ValueTask<Result<IEnumerable<DockerNetwork>>> Handle(ListNetworks query, CancellationToken cancellationToken)
    {
        var platform = await dbContext.Platforms.Where(s => s.Id == query.PlatformId)
            .Select(s => new { s.Address, s.ConnectorType }).FirstOrDefaultAsync(cancellationToken);
        if (platform == null)
        {
            return Result.Failure<IEnumerable<DockerNetwork>>(new NotFoundError("The provided platform Id doesn't exist"));
        }

        var args = new ListNetworksCommand
        (
            PlatformAddress: platform.Address,
            Id: query.Id,
            Name: query.Name,
            Driver: query.Driver,
            Dangling: query.Dangling
        );

        var networkConnector = connectorFactory.GetConnector(platform.ConnectorType);
        return await networkConnector.ListNetworksAsync(args, cancellationToken);
    }
}
