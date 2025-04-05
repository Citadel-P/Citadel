using Agent.Server.Images;
using Grpc.Core;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Images.Queries;

public sealed record GetAllLocalImages(Guid PlatformId): IQuery<Result<IEnumerable<ImageReply>>>;

internal class GetAllLocalImagesHandler(ApplicationDbContext dbContext, IGrpcClientFactory clientFactory) : IQueryHandler<GetAllLocalImages, Result<IEnumerable<ImageReply>>>
{
    public async ValueTask<Result<IEnumerable<ImageReply>>> Handle(GetAllLocalImages query, CancellationToken cancellationToken)
    {
        var address = await dbContext.Platforms.Where(s => s.Id == query.PlatformId).Select(s => s.Address).FirstOrDefaultAsync(cancellationToken);
        if (address == null) 
        {
            return Result.Failure<IEnumerable<ImageReply>>(new NotFoundError("The provided platform Id doesn't exist"));
        }

        try
        {
            var client = clientFactory.GetImageClient(address);
            var images = await client.GetAllAsync(new ListImagesMessage(), cancellationToken: cancellationToken);
            return images.Images;
        }
        catch (RpcException ex)
        {
            return Result.Failure<IEnumerable<ImageReply>>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }
}
