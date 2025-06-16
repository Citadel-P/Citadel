using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

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

internal sealed class DeleteImagesHandler(IConnectorFactory<IImageConnector> connectorFactory, ApplicationDbContext dbContext) : ICommandHandler<DeleteImages, Result<DeleteImageResult>>
{
    public async ValueTask<Result<DeleteImageResult>> Handle(DeleteImages command, CancellationToken cancellationToken)
    {

        var platform = await dbContext.Platforms.Where(s => s.Id == command.PlatformId).Select(s => new { s.Address, s.ConnectorType }).FirstOrDefaultAsync(cancellationToken);
        if (platform == null)
        {
            return Result.Failure<DeleteImageResult>(new NotFoundError("The provided platform Id does not exist"));
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
