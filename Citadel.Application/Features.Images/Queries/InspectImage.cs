using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Images.Queries;

public sealed record InspectImage(Guid PlatformId, string ImageId) : IQuery<Result<InspectImageResult>>
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

internal sealed class InspectImageHandler(IConnectorFactory<IImageConnector> connectorFactory, ApplicationDbContext dbContext) : IQueryHandler<InspectImage, Result<InspectImageResult>>
{
    public async ValueTask<Result<InspectImageResult>> Handle(InspectImage query, CancellationToken cancellationToken)
    {
        var platform = await dbContext.Platforms.Where(s => s.Id == query.PlatformId)
            .Select(s => new { s.Address, s.ConnectorType }).FirstOrDefaultAsync(cancellationToken);
        if (platform == null)
        {
            return Result.Failure<InspectImageResult>(new NotFoundError("The provided platform Id doesn't exist"));
        }

        var args = new InspectImageCommand
        (
            PlatformAddress: platform.Address,
            ImageId: query.ImageId
        );
        return await connectorFactory
            .GetConnector(platform.ConnectorType)
            .InspectImageAsync(args, cancellationToken: cancellationToken);
    }
}
