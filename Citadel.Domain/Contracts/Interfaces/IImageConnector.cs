using Domain.Contracts.Resources.Images;
using LightResults;

namespace Domain.Contracts.Interfaces;

public interface IImageConnector
{
    Task<Result<IReadOnlyList<ImageResult>>> ListImagesAsync(string platformAddress, CancellationToken cancellationToken);
    Task<Result<InspectImageResult>> InspectImageAsync(InspectImageCommand inspectImageCommand, CancellationToken cancellationToken);
    Task<Result<DeleteImageResult>> DeleteImageAsync(DeleteImageCommand deleteImageCommand, CancellationToken cancellationToken);
    IAsyncEnumerable<PullImageResult> PullImageProgressStreamAsync(PullImageCommand pullImageCommand, CancellationToken cancellationToken);
}
