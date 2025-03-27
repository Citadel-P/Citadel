using Agent.Server.Images;
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

public sealed record DeleteImages(Guid PlatformId, string[] Ids, bool Force = false, bool NoPrune = false) : ICommand<Result<DeleteImagesReply>>
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

internal sealed class DeleteImagesHandler(IGrpcClientFactory clientFactory, ApplicationDbContext dbContext) : ICommandHandler<DeleteImages, Result<DeleteImagesReply>>
{
    public async ValueTask<Result<DeleteImagesReply>> Handle(DeleteImages command, CancellationToken cancellationToken)
    {

        var platformAddress = await dbContext.Platforms.Where(s => s.Id == command.PlatformId).Select(s => s.Address).FirstOrDefaultAsync(cancellationToken);
        if (platformAddress == null)
        {
            return Result.Failure<DeleteImagesReply>(new NotFoundError("The provided platform Id does not exist"));
        }

        var client = clientFactory.GetImageClient(platformAddress);
        var request = new DeleteImagesMessage
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
            return Result.Failure<DeleteImagesReply>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }
}
