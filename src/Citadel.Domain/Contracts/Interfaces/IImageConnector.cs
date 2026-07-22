using Domain.Contracts.Resources.Images;
using LightResults;

namespace Domain.Contracts.Interfaces;

/// <summary>
/// Defines methods for managing container images.
/// </summary>
public interface IImageConnector
{
    Task<Result<ImageResult>> GetAsync(string platformAddress, string imageId, CancellationToken cancellationToken);
    Task<Result<IReadOnlyList<ImageResult>>> ListImagesAsync(string platformAddress, CancellationToken cancellationToken);
    Task<Result<InspectImageResult>> InspectImageAsync(InspectImageCommand inspectImageCommand, CancellationToken cancellationToken);
    Task<Result<DistributionResult>> DistributionInspectAsync(DistributionInspectCommand command, CancellationToken cancellationToken);
    Task<Result<BuildHostCapabilitiesResult>> CheckBuildHostAsync(string platformAddress, CancellationToken cancellationToken);
    Task<Result<ExposedPortsResult>> GetExposedPortsAsync(RunImageInfoCommand runImageInfoCommand, CancellationToken cancellationToken);
    Task<Result<IEnumerable<HistoryImageResult>>> HistoryImageAsync(HistoryImageCommand command, CancellationToken cancellationToken);
    Task<Result<DeleteImageResult>> DeleteImageAsync(DeleteImageCommand deleteImageCommand, CancellationToken cancellationToken);
    IAsyncEnumerable<PullImageStreamItem> PullImageProgressStreamAsync(PullImageCommand pullImageCommand, CancellationToken cancellationToken);
    IAsyncEnumerable<ImageBuildStreamItem> BuildImageProgressStreamAsync(BuildImageCommand buildImageCommand, CancellationToken cancellationToken);
    IAsyncEnumerable<ImageBuildStreamItem> PushImageProgressStreamAsync(PushImageCommand pushImageCommand, CancellationToken cancellationToken);
}
