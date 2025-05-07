using Agent.Server.Volumes;
using Grpc.Core;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Volumes.Queries;

public sealed record ListVolumes(Guid PlatformId, bool? Dangling = null, string Driver = null, string Name = null) 
    : IQuery<Result<VolumeListReply>>;

internal class ListVolumesHandler(ApplicationDbContext dbContext, IGrpcClientFactory clientFactory) : IQueryHandler<ListVolumes, Result<VolumeListReply>>
{
    public async ValueTask<Result<VolumeListReply>> Handle(ListVolumes query, CancellationToken cancellationToken)
    {
        var address = await dbContext.Platforms.Where(s => s.Id == query.PlatformId).Select(s => s.Address).FirstOrDefaultAsync(cancellationToken);
        if (address == null)
        {
            return Result.Failure<VolumeListReply>(new NotFoundError("The provided platform Id doesn't exist"));
        }
        try
        {
            var args = new VolumeListMessage
            {
                Driver = query.Driver,
                Name = query.Name,
                Dangling = query.Dangling
            };
            var client = clientFactory.GetVolumeClient(address);
            var result = await client.ListAsync(args, cancellationToken: cancellationToken);
            return new VolumeListReply() { Volumes = { result.Volumes.OrderByDescending(s => s.CreatedAt) } };
        }
        catch (RpcException ex)
        {
            return Result.Failure<VolumeListReply>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }
}