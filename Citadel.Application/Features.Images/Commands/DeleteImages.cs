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

namespace Application.Features.Images.Commands;

public sealed record DeleteImages(Guid PlatformId, string[] Ids, bool Force = false, bool NoPrune = false) : ICommand<Result<DeleteImageResponse>>
{
    internal class Validator : AbstractValidator<DeleteImages>
    {
        public Validator()
        {
            RuleForEach(s => s.Ids).ValidHashId();
            RuleFor(s => s.PlatformId).NotEmpty().NotNull();
        }
    }
}

internal sealed class DeleteImagesHandler(IGrpcClientFactory clientFactory, ApplicationDbContext dbContext) : ICommandHandler<DeleteImages, Result<DeleteImageResponse>>
{
    public async ValueTask<Result<DeleteImageResponse>> Handle(DeleteImages command, CancellationToken cancellationToken)
    {

        var platformAddress = await dbContext.Platforms.Where(s => s.Id == command.PlatformId).Select(s => s.Address).FirstOrDefaultAsync(cancellationToken);
        if (platformAddress == null)
        {
            return Result.Failure<DeleteImageResponse>(new NotFoundError("The provided platform Id does not exist"));
        }

        var client = clientFactory.GetImageClient(platformAddress);
        var request = new DeleteImageRequest
        {
            Ids = { command.Ids },
            Force = command.Force,
            Noprune = command.NoPrune
        };

        try
        {
           return await client.DeleteAsync(request, cancellationToken: cancellationToken);
        }
        catch (RpcException ex)
        {
            return Result.Failure<DeleteImageResponse>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }
}
