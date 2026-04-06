using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Images.Queries;

[RequirePermission(ResourceType.Platform, ResourceAction.View)]
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

internal sealed class InspectImageHandler(IPlatformContainerCache platformContainerCache, IUnitOfWork unitOfWork, IConnectorFactory<IImageConnector> connectorFactory) : IQueryHandler<InspectImage, Result<InspectImageResult>>
{
    public async ValueTask<Result<InspectImageResult>> Handle(InspectImage query, CancellationToken cancellationToken)
    {
        if (!platformContainerCache.TryGetCacheEntry(query.PlatformId, out var platform, out var error))
        {
            return Result.Failure<InspectImageResult>(error);
        }

        var inspectArgs = new InspectImageCommand (PlatformAddress: platform.Address, ImageId: query.ImageId);
        
        var result = await connectorFactory.GetConnector(platform.ConnectorType).InspectImageAsync(inspectArgs, cancellationToken: cancellationToken);
        
        if (result.IsSuccess(out var inspectResult))
        {
            var image = await unitOfWork.Images.GetByDockerImageIdAsync(inspectResult.Id, query.PlatformId, cancellationToken);
            if (image != null)
            {
                inspectResult.Registry = image.Registry;
            }

            return inspectResult;
        }
        return result;
    }
}
