using Domain;
using Domain.Contracts.Resources.Volumes;
using LightResults;

namespace Domain.Contracts.Interfaces;

public interface IVolumePathNormalizer
{
    Result<NormalizedVolumePath> Normalize(string? path);
}

public interface IVolumeContentService
{
    Task<Result<VolumeDirectoryListing>> ListDirectoryAsync(
        ListVolumeDirectoryCommand command,
        CancellationToken cancellationToken);

    Task<Result<VolumeDownloadStream>> OpenDownloadAsync(
        DownloadVolumePathCommand command,
        CancellationToken cancellationToken);
}

public interface IVolumeHelperImageResolver
{
    bool IsExplicitlyConfigured { get; }

    string Resolve(PlatformConnectorType connectorType);
}
