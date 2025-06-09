using Citadel.Agent.Images.V1;
using FluentValidation;
using Grpc.Core;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Images.Queries;

public sealed record InspectImage(Guid PlatformId, string ImageId) : IQuery<Result<InspectImageReply>>
{
    internal class Validator : AbstractValidator<InspectImage>
    {
        public Validator()
        {
            RuleFor(s => s.PlatformId).NotEmpty().NotNull();
            RuleFor(s => s.ImageId).ValidHashId();
        }
    }
}

internal sealed class InspectImageHandler(ApplicationDbContext dbContext, IGrpcClientFactory clientFactory) : IQueryHandler<InspectImage, Result<InspectImageReply>>
{
    public async ValueTask<Result<InspectImageReply>> Handle(InspectImage query, CancellationToken cancellationToken)
    {
        var address = await dbContext.Platforms.Where(s => s.Id == query.PlatformId).Select(s => s.Address).FirstOrDefaultAsync(cancellationToken);
        if (address == null)
        {
            return Result.Failure<InspectImageReply>(new NotFoundError("The provided platform Id doesn't exist"));
        }
        try
        {
            var args = new InspectImageMessage
            {
                Id = query.ImageId
            };
            var client = clientFactory.GetImageClient(address);
            return await client.InspectAsync(args, cancellationToken: cancellationToken);
        }
        catch (RpcException ex)
        {
            return Result.Failure<InspectImageReply>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }
}
