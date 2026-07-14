using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Volumes;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Volumes.Queries;

[RequirePermission(ResourceType.Volume, PermissionLevel.Read, SpecificPermission.Browse)]
public sealed record ListVolumeDirectory(
    Guid PlatformId,
    string VolumeName,
    string? Path) : IQuery<Result<VolumeDirectoryListing>>
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
    string? Path) : IQuery<Result<VolumeDownloadStream>>
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
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IVolumeConnector> volumeConnectorFactory,
    IVolumePathNormalizer pathNormalizer,
    IVolumeContentService volumeContentService)
    : IQueryHandler<ListVolumeDirectory, Result<VolumeDirectoryListing>>
{
    public async ValueTask<Result<VolumeDirectoryListing>> Handle(ListVolumeDirectory query, CancellationToken cancellationToken)
    {
        var normalizedPath = pathNormalizer.Normalize(query.Path);
        if (normalizedPath.IsFailure(out var pathError, out var path))
            return Result.Failure<VolumeDirectoryListing>(pathError);

        if (!platformContainerCache.TryGetCacheEntry(query.PlatformId, out var platform, out var platformError))
            return Result.Failure<VolumeDirectoryListing>(platformError);

        var volumeConnector = volumeConnectorFactory.GetConnector(platform.ConnectorType);
        var volumeExists = await volumeConnector.InspectVolumeAsync(
            new InspectDockerVolumeCommand(platform.Address, query.VolumeName),
            cancellationToken);

        if (volumeExists.IsFailure(out var volumeError))
            return Result.Failure<VolumeDirectoryListing>(volumeError);

        return await volumeContentService.ListDirectoryAsync(
            new ListVolumeDirectoryCommand(
                platform.Address,
                query.PlatformId,
                platform.ConnectorType,
                query.VolumeName,
                path),
            cancellationToken);
    }
}

internal sealed class OpenVolumeDownloadHandler(
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IVolumeConnector> volumeConnectorFactory,
    IVolumePathNormalizer pathNormalizer,
    IVolumeContentService volumeContentService)
    : IQueryHandler<OpenVolumeDownload, Result<VolumeDownloadStream>>
{
    public async ValueTask<Result<VolumeDownloadStream>> Handle(OpenVolumeDownload query, CancellationToken cancellationToken)
    {
        var normalizedPath = pathNormalizer.Normalize(query.Path);
        if (normalizedPath.IsFailure(out var pathError, out var path))
            return Result.Failure<VolumeDownloadStream>(pathError);

        if (!platformContainerCache.TryGetCacheEntry(query.PlatformId, out var platform, out var platformError))
            return Result.Failure<VolumeDownloadStream>(platformError);

        var volumeConnector = volumeConnectorFactory.GetConnector(platform.ConnectorType);
        var volumeExists = await volumeConnector.InspectVolumeAsync(
            new InspectDockerVolumeCommand(platform.Address, query.VolumeName),
            cancellationToken);

        if (volumeExists.IsFailure(out var volumeError))
            return Result.Failure<VolumeDownloadStream>(volumeError);

        return await volumeContentService.OpenDownloadAsync(
            new DownloadVolumePathCommand(
                platform.Address,
                query.PlatformId,
                platform.ConnectorType,
                query.VolumeName,
                path),
            cancellationToken);
    }
}
