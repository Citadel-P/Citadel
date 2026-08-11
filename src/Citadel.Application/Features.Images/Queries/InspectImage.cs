using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;
using Domain.Entities.Platforms;

namespace Application.Features.Images.Queries;

[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record InspectImage(Guid PlatformId, string ImageId, string? DockerNodeId = null) : IQuery<Result<InspectImageResult>>
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

internal sealed class InspectImageHandler(
    IPlatformContainerCache platformContainerCache,
    IUnitOfWork unitOfWork,
    IConnectorFactory<IImageConnector> connectorFactory,
    ISwarmNodeRuntimeConnector swarmNodeRuntimeConnector) : IQueryHandler<InspectImage, Result<InspectImageResult>>
{
    public async ValueTask<Result<InspectImageResult>> Handle(InspectImage query, CancellationToken cancellationToken)
    {
        var persistedPlatform = await unitOfWork.Platforms.GetByIdAsync(query.PlatformId, cancellationToken);
        if (persistedPlatform?.PlatformDescriptor is DockerSwarmPlatformDescriptor descriptor)
        {
            var dockerNodeId = string.IsNullOrWhiteSpace(query.DockerNodeId)
                ? descriptor.NodeID
                : query.DockerNodeId;
            var swarmResult = await swarmNodeRuntimeConnector.InspectImageAsync(
                persistedPlatform,
                dockerNodeId,
                query.ImageId,
                cancellationToken);
            if (swarmResult.IsSuccess(out var swarmImage))
            {
                swarmImage.PlatformId = query.PlatformId;
                swarmImage.DockerNodeId = dockerNodeId;
            }
            return swarmResult;
        }

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
                inspectResult.PlatformId = query.PlatformId;
            }

            return inspectResult;
        }
        return result;
    }
}
