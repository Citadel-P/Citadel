using Hosting.Common;
using Domain.Contracts.Resources.Networks;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;
using Application.Services;

namespace Application.Features.Networks.Queries;

[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record ListNetworks (Guid PlatformId, bool? Dangling = null, string? Driver = null, string? Id = null, string? Name = null) : IQuery<Result<IEnumerable<DockerNetworkResult>>>;

internal class ListNetworksHandler(INetworkService networkService) : IQueryHandler<ListNetworks, Result<IEnumerable<DockerNetworkResult>>>
{
    public async ValueTask<Result<IEnumerable<DockerNetworkResult>>> Handle(ListNetworks query, CancellationToken cancellationToken)
        => await networkService.List(query, cancellationToken);
    
}
