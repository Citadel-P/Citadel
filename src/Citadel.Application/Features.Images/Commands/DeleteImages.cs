using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using FluentValidation;
using Hosting.Common;
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

internal sealed class DeleteImagesHandler(IPlatformContainerCache platformContainerCache, IConnectorFactory<IImageConnector> connectorFactory) : ICommandHandler<DeleteImages, Result<DeleteImageResult>>
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
        return await connectorFactory
            .GetConnector(platform.ConnectorType)
            .DeleteImageAsync(args, cancellationToken: cancellationToken);
    }
}
