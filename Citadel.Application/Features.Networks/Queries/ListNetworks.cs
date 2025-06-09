using Citadel.Agent.Networks.V1;
using Grpc.Core;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Networks.Queries;

public sealed record ListNetworks (Guid PlatformId, bool? Dangling = null, string? Driver = null, string? Id = null, string? Name = null) : IQuery<Result<ListNetworksResponse>>;

internal class ListNetworksHandler(ApplicationDbContext dbContext, IGrpcClientFactory clientFactory) : IQueryHandler<ListNetworks, Result<ListNetworksResponse>>
{
    public async ValueTask<Result<ListNetworksResponse>> Handle(ListNetworks query, CancellationToken cancellationToken)
    {
        var address = await dbContext.Platforms.Where(s => s.Id == query.PlatformId).Select(s => s.Address).FirstOrDefaultAsync(cancellationToken);
        if (address == null)
        {
            return Result.Failure<ListNetworksResponse>(new NotFoundError("The provided platform Id doesn't exist"));
        }
        try
        {
            var request = new ListNetworksRequest
            {
                Driver = query.Driver,
                Id = query.Id,
                Name = query.Name,
                Dangling = query.Dangling
            };
            var client = clientFactory.GetNetworkClient(address);
            var response = await client.ListAsync(request, cancellationToken: cancellationToken);
            return new ListNetworksResponse() { Networks = { response.Networks.OrderByDescending(s => s.Created) } };
        }
        catch (RpcException ex)
        {
            return Result.Failure<ListNetworksResponse>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }
}
