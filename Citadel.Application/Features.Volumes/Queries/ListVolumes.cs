using Citadel.Agent.Volumes.V1;
using Grpc.Core;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Volumes.Queries;

public sealed record ListVolumes(Guid PlatformId, bool? Dangling = null, string? Driver = null, string? Name = null) 
    : IQuery<Result<ListVolumeReply>>;

internal class ListVolumesHandler(ApplicationDbContext dbContext, IGrpcClientFactory clientFactory) : IQueryHandler<ListVolumes, Result<ListVolumeReply>>
{
    public async ValueTask<Result<ListVolumeReply>> Handle(ListVolumes query, CancellationToken cancellationToken)
    {
        var address = await dbContext.Platforms.Where(s => s.Id == query.PlatformId).Select(s => s.Address).FirstOrDefaultAsync(cancellationToken);
        if (address == null)
        {
            return Result.Failure<ListVolumeReply>(new NotFoundError("The provided platform Id doesn't exist"));
        }
        try
        {
            var args = new ListVolumeMessage
            {
                Driver = query.Driver,
                Name = query.Name,
                Dangling = query.Dangling
            };
            var client = clientFactory.GetVolumeClient(address);
            var result = await client.ListAsync(args, cancellationToken: cancellationToken);
            return new ListVolumeReply() { Volumes = { result.Volumes.OrderByDescending(s => s.CreatedAt) } };
        }
        catch (RpcException ex)
        {
            return Result.Failure<ListVolumeReply>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }
}