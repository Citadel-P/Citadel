using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using Agent.Server.Images;
using Agent.Server.Networks;
using Grpc.Core;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Networks.Queries;

public sealed record ListNetworks (Guid PlatformId, bool? Dangling = false, string Driver = null, string Id = null, string Name = null) : IQuery<Result<ListNetworksReply>>;

internal class ListNetworksHandler(ApplicationDbContext dbContext, IGrpcClientFactory clientFactory) : IQueryHandler<ListNetworks, Result<ListNetworksReply>>
{
    public async ValueTask<Result<ListNetworksReply>> Handle(ListNetworks query, CancellationToken cancellationToken)
    {
        var address = await dbContext.Platforms.Where(s => s.Id == query.PlatformId).Select(s => s.Address).FirstOrDefaultAsync(cancellationToken);
        if (address == null)
        {
            return Result.Failure<ListNetworksReply>(new NotFoundError("The provided platform Id doesn't exist"));
        }
        try
        {
            var args = new ListNetworksMessage
            {
                Driver = query.Driver,
                Id = query.Id,
                Name = query.Name,
                Dangling = query.Dangling ?? false
            };
            var client = clientFactory.GetNetworkClient(address);
            return await client.ListNetworksAsync(args, cancellationToken: cancellationToken);
        }
        catch (RpcException ex)
        {
            return Result.Failure<ListNetworksReply>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }
}
