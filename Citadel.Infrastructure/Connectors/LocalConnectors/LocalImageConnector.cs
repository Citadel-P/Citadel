using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using LightResults;

namespace Infrastructure.Connectors.LocalConnectors;

internal class LocalImageConnector : IImageConnector
{
    public Task<Result<InspectImageResult>> InspectImageAsync(InspectImageCommand inspectImageCommand, CancellationToken cancellationToken)
    {
        throw new NotImplementedException();
    }

    public Task<Result<IReadOnlyList<ImageResult>>> ListImagesAsync(string platformAddress, CancellationToken cancellationToken)
    {
        throw new NotImplementedException();
    }

    public Task<Result<DeleteImageResult>> DeleteImageAsync(DeleteImageCommand deleteImageCommand, CancellationToken cancellationToken)
    {
        throw new NotImplementedException();
    }

    public IAsyncEnumerable<PullImageResult> PullImageProgressStreamAsync(PullImageCommand pullImageCommand, CancellationToken cancellationToken)
    {
        throw new NotImplementedException();
    }
}
