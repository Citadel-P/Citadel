using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using FluentValidation;
using Hosting.Common;
using LightResults;
using Mediator;

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

internal sealed class InspectImageHandler(IPlatformContainerCache platformContainerCache, IConnectorFactory<IImageConnector> connectorFactory) : IQueryHandler<InspectImage, Result<InspectImageResult>>
{
    public async ValueTask<Result<InspectImageResult>> Handle(InspectImage query, CancellationToken cancellationToken)
    {
        if (!platformContainerCache.TryGetCacheEntry(query.PlatformId, out var platform, out var error))
        {
            return Result.Failure<InspectImageResult>(error);
        }

        var inspectArgs = new InspectImageCommand (PlatformAddress: platform.Address, ImageId: query.ImageId);
        
        return await connectorFactory.GetConnector(platform.ConnectorType).InspectImageAsync(inspectArgs, cancellationToken: cancellationToken);
    }
}
