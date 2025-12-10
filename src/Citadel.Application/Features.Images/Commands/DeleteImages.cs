using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Images.Commands;

public sealed record DeleteImages(Guid PlatformId, string[] Ids, bool Force = false, bool NoPrune = false) : ICommand<Result<DeleteImageResult>>
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

internal sealed class DeleteImagesHandler(
    IUnitOfWork unitOfWork,
    IDockerDaemonStreamManager dockerDaemonHub,
    IPlatformContainerCache platformContainerCache, 
    IConnectorFactory<IImageConnector> connectorFactory 
) : ICommandHandler<DeleteImages, Result<DeleteImageResult>>
{
    public async ValueTask<Result<DeleteImageResult>> Handle(DeleteImages command, CancellationToken cancellationToken)
    {
        if (!platformContainerCache.TryGetCacheEntry(command.PlatformId, out var platform, out var error)) 
        {
            return Result.Failure<DeleteImageResult>(error);
        }

        var args = new DeleteImageCommand
        (
            Ids: command.Ids,
            Force: command.Force,
            NoPrune: command.NoPrune,
            PlatformAddress: platform.Address
        );
        var result = await connectorFactory
            .GetConnector(platform.ConnectorType)
            .DeleteImageAsync(args, cancellationToken: cancellationToken);

        // Handle the case where the image is not on the platform but still in db
        if (result.IsFailure(out var errorResult) && errorResult is NotFoundError)
        {
            // Todo: handle bulk delete
            foreach (var id in command.Ids)
            {
                var existing = await unitOfWork.Images.GetByDockerImageIdAsync(id, command.PlatformId, cancellationToken);
                if (existing == null) continue;
                await unitOfWork.Images.DeleteAsync([existing.Id], cancellationToken);
                await dockerDaemonHub.SendImageEvent(existing, "delete");

            }
            await unitOfWork.CommitAsync(cancellationToken);
        }

        return result;
    }
}
