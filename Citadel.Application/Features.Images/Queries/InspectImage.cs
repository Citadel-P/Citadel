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

public sealed record InspectImage(Guid PlatformId, string ImageId) : IQuery<Result<InspectImageResponse>>
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

internal sealed class InspectImageHandler(ApplicationDbContext dbContext, IGrpcClientFactory clientFactory) : IQueryHandler<InspectImage, Result<InspectImageResponse>>
{
    public async ValueTask<Result<InspectImageResponse>> Handle(InspectImage query, CancellationToken cancellationToken)
    {
        var address = await dbContext.Platforms.Where(s => s.Id == query.PlatformId).Select(s => s.Address).FirstOrDefaultAsync(cancellationToken);
        if (address == null)
        {
            return Result.Failure<InspectImageResponse>(new NotFoundError("The provided platform Id doesn't exist"));
        }
        try
        {
            var args = new InspectImageRequest
            {
                Id = query.ImageId
            };
            var client = clientFactory.GetImageClient(address);
            return await client.InspectAsync(args, cancellationToken: cancellationToken);
        }
        catch (RpcException ex)
        {
            return Result.Failure<InspectImageResponse>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }
}
