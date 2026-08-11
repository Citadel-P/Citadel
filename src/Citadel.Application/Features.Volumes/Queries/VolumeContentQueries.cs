using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Volumes;
using Domain.Entities.Platforms;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Volumes.Queries;

[RequirePermission(ResourceType.Volume, PermissionLevel.Read, SpecificPermission.Browse)]
public sealed record ListVolumeDirectory(
    Guid PlatformId,
    string VolumeName,
    string? Path,
    string? DockerNodeId = null) : IQuery<Result<VolumeDirectoryListing>>
{
    internal sealed class Validator : AbstractValidator<ListVolumeDirectory>
    {
        public Validator()
        {
            RuleFor(x => x.PlatformId).NotEmpty();
            RuleFor(x => x.VolumeName).NotEmpty();
        }
    }
}

[RequirePermission(ResourceType.Volume, PermissionLevel.Read, SpecificPermission.Download)]
public sealed record OpenVolumeDownload(
    Guid PlatformId,
    string VolumeName,
    string? Path,
    string? DockerNodeId = null) : IQuery<Result<VolumeDownloadStream>>
{
    internal sealed class Validator : AbstractValidator<OpenVolumeDownload>
    {
        public Validator()
        {
            RuleFor(x => x.PlatformId).NotEmpty();
            RuleFor(x => x.VolumeName).NotEmpty();
        }
    }
}

internal sealed class ListVolumeDirectoryHandler(
    IUnitOfWork unitOfWork,
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IVolumeConnector> volumeConnectorFactory,
    ISwarmNodeRuntimeConnector swarmNodeRuntimeConnector,
    IVolumePathNormalizer pathNormalizer,
    IVolumeContentService volumeContentService)
    : IQueryHandler<ListVolumeDirectory, Result<VolumeDirectoryListing>>
{
    public async ValueTask<Result<VolumeDirectoryListing>> Handle(ListVolumeDirectory query, CancellationToken cancellationToken)
    {
        var normalizedPath = pathNormalizer.Normalize(query.Path);
        if (normalizedPath.IsFailure(out var pathError, out var path))
            return Result.Failure<VolumeDirectoryListing>(pathError);

        var persistedPlatform = await unitOfWork.Platforms.GetByIdAsync(query.PlatformId, cancellationToken);
        if (persistedPlatform is null)
            return Result.Failure<VolumeDirectoryListing>(new NotFoundError("Platform not found."));

        var isSwarm = persistedPlatform.PlatformDescriptor is DockerSwarmPlatformDescriptor;
        if (isSwarm && string.IsNullOrWhiteSpace(query.DockerNodeId))
        {
            return Result.Failure<VolumeDirectoryListing>(
                new BadRequestError("Select the owning Swarm Node before browsing this Volume."));
        }

        string platformAddress;
        PlatformConnectorType connectorType;
        Result<DockerVolumeResult> volumeExists;
        if (isSwarm)
        {
            platformAddress = persistedPlatform.Address;
            connectorType = persistedPlatform.ConnectorType;
            volumeExists = await swarmNodeRuntimeConnector.InspectVolumeAsync(
                persistedPlatform,
                query.DockerNodeId!,
                query.VolumeName,
                cancellationToken);
        }
        else
        {
            if (!platformContainerCache.TryGetCacheEntry(query.PlatformId, out var platform, out var platformError))
                return Result.Failure<VolumeDirectoryListing>(platformError);

            platformAddress = platform.Address;
            connectorType = platform.ConnectorType;
            volumeExists = await volumeConnectorFactory.GetConnector(connectorType).InspectVolumeAsync(
                new InspectDockerVolumeCommand(platformAddress, query.VolumeName),
                cancellationToken);
        }

        if (volumeExists.IsFailure(out var volumeError))
            return Result.Failure<VolumeDirectoryListing>(volumeError);

        return await volumeContentService.ListDirectoryAsync(
            new ListVolumeDirectoryCommand(
                platformAddress,
                query.PlatformId,
                connectorType,
                query.VolumeName,
                path,
                persistedPlatform,
                query.DockerNodeId),
            cancellationToken);
    }
}

internal sealed class OpenVolumeDownloadHandler(
    IUnitOfWork unitOfWork,
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IVolumeConnector> volumeConnectorFactory,
    ISwarmNodeRuntimeConnector swarmNodeRuntimeConnector,
    IVolumePathNormalizer pathNormalizer,
    IVolumeContentService volumeContentService)
    : IQueryHandler<OpenVolumeDownload, Result<VolumeDownloadStream>>
{
    public async ValueTask<Result<VolumeDownloadStream>> Handle(OpenVolumeDownload query, CancellationToken cancellationToken)
    {
        var normalizedPath = pathNormalizer.Normalize(query.Path);
        if (normalizedPath.IsFailure(out var pathError, out var path))
            return Result.Failure<VolumeDownloadStream>(pathError);

        var persistedPlatform = await unitOfWork.Platforms.GetByIdAsync(query.PlatformId, cancellationToken);
        if (persistedPlatform is null)
            return Result.Failure<VolumeDownloadStream>(new NotFoundError("Platform not found."));

        var isSwarm = persistedPlatform.PlatformDescriptor is DockerSwarmPlatformDescriptor;
        if (isSwarm && string.IsNullOrWhiteSpace(query.DockerNodeId))
        {
            return Result.Failure<VolumeDownloadStream>(
                new BadRequestError("Select the owning Swarm Node before downloading from this Volume."));
        }

        string platformAddress;
        PlatformConnectorType connectorType;
        Result<DockerVolumeResult> volumeExists;
        if (isSwarm)
        {
            platformAddress = persistedPlatform.Address;
            connectorType = persistedPlatform.ConnectorType;
            volumeExists = await swarmNodeRuntimeConnector.InspectVolumeAsync(
                persistedPlatform,
                query.DockerNodeId!,
                query.VolumeName,
                cancellationToken);
        }
        else
        {
            if (!platformContainerCache.TryGetCacheEntry(query.PlatformId, out var platform, out var platformError))
                return Result.Failure<VolumeDownloadStream>(platformError);

            platformAddress = platform.Address;
            connectorType = platform.ConnectorType;
            volumeExists = await volumeConnectorFactory.GetConnector(connectorType).InspectVolumeAsync(
                new InspectDockerVolumeCommand(platformAddress, query.VolumeName),
                cancellationToken);
        }

        if (volumeExists.IsFailure(out var volumeError))
            return Result.Failure<VolumeDownloadStream>(volumeError);

        return await volumeContentService.OpenDownloadAsync(
            new DownloadVolumePathCommand(
                platformAddress,
                query.PlatformId,
                connectorType,
                query.VolumeName,
                path,
                persistedPlatform,
                query.DockerNodeId),
            cancellationToken);
    }
}
