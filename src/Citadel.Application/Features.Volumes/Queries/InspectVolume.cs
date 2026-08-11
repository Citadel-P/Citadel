using Hosting.Common;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Volumes;
using FluentValidation;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;
using Domain.Entities.Platforms;

namespace Application.Features.Volumes.Queries;

[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record InspectVolume (Guid PlatformId, string Name, string? DockerNodeId = null) : IQuery<Result<DockerVolumeResult>>
{
    internal class Validator : AbstractValidator<InspectVolume>
    {
        public Validator()
        {
            RuleFor(s => s.PlatformId).NotEmpty().NotNull();
            RuleFor(s => s.Name).NotNull();
        }
    }
}

internal sealed class InspectVolumeHandler(
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IVolumeConnector> connectorFactory,
    IUnitOfWork unitOfWork,
    ISwarmNodeRuntimeConnector swarmNodeRuntimeConnector) : IQueryHandler<InspectVolume, Result<DockerVolumeResult>>
{
    public async ValueTask<Result<DockerVolumeResult>> Handle(InspectVolume query, CancellationToken cancellationToken)
    {
        var persistedPlatform = await unitOfWork.Platforms.GetByIdAsync(query.PlatformId, cancellationToken);
        if (persistedPlatform?.PlatformDescriptor is DockerSwarmPlatformDescriptor descriptor)
        {
            var dockerNodeId = string.IsNullOrWhiteSpace(query.DockerNodeId)
                ? descriptor.NodeID
                : query.DockerNodeId;
            var swarmResult = await swarmNodeRuntimeConnector.InspectVolumeAsync(
                persistedPlatform,
                dockerNodeId,
                query.Name,
                cancellationToken);
            if (swarmResult.IsSuccess(out var swarmVolume))
            {
                swarmVolume.PlatformId = query.PlatformId;
                swarmVolume.DockerNodeId = dockerNodeId;
            }
            return swarmResult;
        }

        if (!platformContainerCache.TryGetCacheEntry(query.PlatformId, out var platform, out var error))
        {
            return Result.Failure<DockerVolumeResult>(error);
        }

        var command = new InspectDockerVolumeCommand
        (
            Name: query.Name,
            PlatformAddress: platform.Address
        );

        var volumeConnector = connectorFactory.GetConnector(platform.ConnectorType);
        var result = await volumeConnector.InspectVolumeAsync(command, cancellationToken);
        if (result.IsSuccess(out var volume))
        {
            volume.PlatformId = query.PlatformId;
        }
        return result;
    }
}
