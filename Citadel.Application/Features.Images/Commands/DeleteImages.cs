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

internal sealed class DeleteImagesHandler(IUnitOfWork unitOfWork, IConnectorFactory<IImageConnector> connectorFactory) : ICommandHandler<DeleteImages, Result<DeleteImageResult>>
{
    public async ValueTask<Result<DeleteImageResult>> Handle(DeleteImages command, CancellationToken cancellationToken)
    {
        var (address, connectorType) = await unitOfWork.Platforms.GetPlatformInfoAsync(command.PlatformId, cancellationToken);
        if (string.IsNullOrEmpty(address))
        {
            return Result.Failure<DeleteImageResult>(new NotFoundError("The provided platform Id does not exist"));
        }

        var args = new DeleteImageCommand
        (
            Ids: command.Ids,
            Force: command.Force,
            NoPrune: command.NoPrune,
            PlatformAddress: address
        );
        return await connectorFactory
            .GetConnector(connectorType)
            .DeleteImageAsync(args, cancellationToken: cancellationToken);
    }
}
