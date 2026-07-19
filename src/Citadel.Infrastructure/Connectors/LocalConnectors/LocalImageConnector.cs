using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using Hosting.DockerClient.Services;
using Hosting.Extensions;
using Infrastructure.Connectors.Mappers;
using LightResults;
using System.Runtime.CompilerServices;

namespace Infrastructure.Connectors.LocalConnectors;

internal class LocalImageConnector(IImageService imageService) : IImageConnector
{
    public async Task<Result<ImageResult>> GetAsync(string platformAddress, string imageId, CancellationToken cancellationToken)
    {
        var result = await imageService.GetAsync(imageId, cancellationToken);
        return ServiceResultHandlers.HandleResult(result, ImageMappers.Map);
    }

    public async Task<Result<IReadOnlyList<ImageResult>>> ListImagesAsync(string platformAddress, CancellationToken cancellationToken)
    {
        var result = await imageService.ListAsync(cancellationToken);
        return ServiceResultHandlers.HandleResult(result, ImageMappers.Map);
    }

    public async Task<Result<InspectImageResult>> InspectImageAsync(InspectImageCommand inspectImageCommand, CancellationToken cancellationToken)
    {
        var result = await imageService.InspectAsync(inspectImageCommand.ImageId, cancellationToken);
        return ServiceResultHandlers.HandleResult(result, ImageMappers.Map);
    }

    public async Task<Result<DeleteImageResult>> DeleteImageAsync(DeleteImageCommand deleteImageCommand, CancellationToken cancellationToken)
    {
        var command = new Hosting.DockerClient.Models.Images.DeleteImageCommand([..deleteImageCommand.Ids], deleteImageCommand.Force, deleteImageCommand.NoPrune);
        var result = await imageService.DeleteAsync(command, cancellationToken);
        return ServiceResultHandlers.HandleResult(result, ImageMappers.Map);
    }

    public async IAsyncEnumerable<PullImageStreamItem> PullImageProgressStreamAsync(PullImageCommand pullImageCommand, [EnumeratorCancellation] CancellationToken cancellationToken)
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

    public async IAsyncEnumerable<ImageBuildStreamItem> BuildImageProgressStreamAsync(
        BuildImageCommand buildImageCommand,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var command = new Hosting.DockerClient.Models.Images.BuildImageStreamCommand(
            ContextDirectory: buildImageCommand.ContextDirectory,
            DockerfilePath: buildImageCommand.DockerfilePath,
            Tags: buildImageCommand.Tags,
            BuildArgs: buildImageCommand.BuildArgs,
            Target: buildImageCommand.Target,
            RegistryAuth: buildImageCommand.RegistryAuth,
            RegistryHost: buildImageCommand.RegistryHost,
            Timeout: buildImageCommand.Timeout,
            MaxLineBytes: buildImageCommand.MaxLineBytes,
            ContextArchive: buildImageCommand.ContextArchive,
            DockerfileArchivePath: buildImageCommand.DockerfileArchivePath);

        await foreach (var message in imageService.StreamBuildImage(command, cancellationToken))
        {
            yield return MapBuildMessage(message);
        }
    }

    public async IAsyncEnumerable<ImageBuildStreamItem> PushImageProgressStreamAsync(
        PushImageCommand pushImageCommand,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var command = new Hosting.DockerClient.Models.Images.PushImageStreamCommand(
            pushImageCommand.ImageReference,
            pushImageCommand.RegistryAuth);

        await foreach (var message in imageService.StreamPushImage(command, cancellationToken))
        {
            yield return MapBuildMessage(message);
        }
    }

    public async Task<Result<IEnumerable<HistoryImageResult>>> HistoryImageAsync(HistoryImageCommand command, CancellationToken cancellationToken)
    {
        var result = await imageService.HistoryAsync(command.ImageId, cancellationToken);
        return ServiceResultHandlers.HandleResult(result, ImageMappers.Map);
    }

    public async Task<Result<ExposedPortsResult>> GetExposedPortsAsync(RunImageInfoCommand command, CancellationToken cancellationToken)
    {
        var result = await imageService.GetExposedPortsAsync(command.ImageId, cancellationToken);
        return ServiceResultHandlers.HandleResult(result, ImageMappers.Map);
    }

    public async Task<Result<DistributionResult>> DistributionInspectAsync(DistributionInspectCommand command, CancellationToken cancellationToken)
    {
        var result = await imageService.DistributionInspectAsync(command.ImageName, command.Auth, cancellationToken);
        return ServiceResultHandlers.HandleResult(result, ImageMappers.Map);
    }

    private static ImageBuildStreamItem MapBuildMessage(Hosting.DockerClient.HttpClient.JSONMessage message)
        => new(
            message.ID,
            message.Stream,
            message.Status,
            message.ErrorMessage,
            message.ProgressMessage,
            message.Progress is null ? null : new ImageBuildProgress(
                message.Progress.Units,
                message.Progress.Current,
                message.Progress.Total,
                message.Progress.Start),
            message.Error is null ? null : new ImageBuildError(message.Error.Code, message.Error.Message));
}
