using System.Runtime.CompilerServices;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using Hosting.DockerClient.Services;
using Hosting.Extensions;
using Infrastructure.Connectors.Mappers;
using LightResults;

namespace Infrastructure.Connectors.LocalConnectors;

internal class LocalImageConnector(IImageService imageService) : IImageConnector
{
    public async Task<Result<IReadOnlyList<ImageResult>>> ListImagesAsync(string platformAddress, CancellationToken cancellationToken)
    {
        var result = await imageService.ListAsync(cancellationToken);
        return ServiceResultHandlers.HandleResult(result, ImageMapper.Map);
    }
    public async Task<Result<InspectImageResult>> InspectImageAsync(InspectImageCommand inspectImageCommand, CancellationToken cancellationToken)
    {
        var result = await imageService.InspectAsync(inspectImageCommand.ImageId, cancellationToken);
        return ServiceResultHandlers.HandleResult(result, ImageMapper.Map);
    }

    public async Task<Result<DeleteImageResult>> DeleteImageAsync(DeleteImageCommand deleteImageCommand, CancellationToken cancellationToken)
    {
        var command = new Hosting.DockerClient.Models.Images.DeleteImageCommand([..deleteImageCommand.Ids], deleteImageCommand.Force, deleteImageCommand.NoPrune);
        var result = await imageService.DeleteAsync(command, cancellationToken);
        return ServiceResultHandlers.HandleResult(result, ImageMapper.Map);
    }

    public async IAsyncEnumerable<PullImageResult> PullImageProgressStreamAsync(PullImageCommand pullImageCommand, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var command = new Hosting.DockerClient.Models.Images.PullImageStreamCommand
        (
            FromImage: pullImageCommand.FromImage,
            FromSrc: pullImageCommand.FromSrc,
            Repo: pullImageCommand.Repo,
            Tag: pullImageCommand.Tag,
            Auth: pullImageCommand.Auth
        );
        await foreach (var message in imageService.StreamPullImage(command, cancellationToken))
        {
            yield return message.Map();
        }
    }
}
