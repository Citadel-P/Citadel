using System.Runtime.CompilerServices;
using Citadel.Images.V1;
using Citadel.SharedModels.V1;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using Google.Protobuf;
using LightResults;
using Infrastructure.Connectors.Mappers;
using DomainHistoryImageResult = Domain.Contracts.Resources.Images.HistoryImageResult;

namespace Infrastructure.Connectors.EdgeAgentConnectors;

internal sealed class EdgeImageConnector(IEdgeAgentCommandRouter commandRouter) : IImageConnector
{
    public async Task<Result<ImageResult>> GetAsync(string platformAddress, string imageId, CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetTarget(platformAddress, out var target, out var addressError))
        {
            return Result.Failure<ImageResult>(addressError!);
        }

        var response = await commandRouter.SendUnaryAsync(
            target.ResourceType,
            target.ResourceId,
            EdgeAgentCommandKind.ImageGet,
            new GetImageRequest { Id = imageId }.ToByteArray(),
            TimeSpan.FromSeconds(30),
            correlationId: null,
            cancellationToken);

        return response.IsSuccess && response.Payload is not null
            ? ImageReply.Parser.ParseFrom(response.Payload).Map()
            : Result.Failure<ImageResult>(EdgeConnectorHelpers.CommandFailure(EdgeAgentCommandKind.ImageGet, response));
    }

    public async Task<Result<IReadOnlyList<ImageResult>>> ListImagesAsync(string platformAddress, CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetTarget(platformAddress, out var target, out var addressError))
        {
            return Result.Failure<IReadOnlyList<ImageResult>>(addressError!);
        }

        var response = await commandRouter.SendUnaryAsync(
            target.ResourceType,
            target.ResourceId,
            EdgeAgentCommandKind.ImageList,
            new ListImagesRequest().ToByteArray(),
            TimeSpan.FromSeconds(60),
            correlationId: null,
            cancellationToken);

        return response.IsSuccess && response.Payload is not null
            ? ListImageResponse.Parser.ParseFrom(response.Payload).Images.Map()
            : Result.Failure<IReadOnlyList<ImageResult>>(EdgeConnectorHelpers.CommandFailure(EdgeAgentCommandKind.ImageList, response));
    }

    public async Task<Result<InspectImageResult>> InspectImageAsync(InspectImageCommand inspectImageCommand, CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetTarget(inspectImageCommand.PlatformAddress, out var target, out var addressError))
        {
            return Result.Failure<InspectImageResult>(addressError!);
        }

        var response = await commandRouter.SendUnaryAsync(
            target.ResourceType,
            target.ResourceId,
            EdgeAgentCommandKind.ImageInspect,
            new InspectImageRequest { Id = inspectImageCommand.ImageId }.ToByteArray(),
            TimeSpan.FromSeconds(60),
            correlationId: null,
            cancellationToken);

        return response.IsSuccess && response.Payload is not null
            ? InspectImageResponse.Parser.ParseFrom(response.Payload).Map()
            : Result.Failure<InspectImageResult>(EdgeConnectorHelpers.CommandFailure(EdgeAgentCommandKind.ImageInspect, response));
    }

    public async Task<Result<DistributionResult>> DistributionInspectAsync(DistributionInspectCommand command, CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetTarget(command.PlatformAddress, out var target, out var addressError))
        {
            return Result.Failure<DistributionResult>(addressError!);
        }

        var response = await commandRouter.SendUnaryAsync(
            target.ResourceType,
            target.ResourceId,
            EdgeAgentCommandKind.ImageDistributionInspect,
            new DistributionInspectRequest { ImageName = command.ImageName, Auth = command.Auth }.ToByteArray(),
            TimeSpan.FromSeconds(60),
            correlationId: null,
            cancellationToken);

        return response.IsSuccess && response.Payload is not null
            ? DistributionInspectResponse.Parser.ParseFrom(response.Payload).Map()
            : Result.Failure<DistributionResult>(EdgeConnectorHelpers.CommandFailure(EdgeAgentCommandKind.ImageDistributionInspect, response));
    }

    public async Task<Result<BuildHostCapabilitiesResult>> CheckBuildHostAsync(string platformAddress, CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetTarget(platformAddress, out var target, out var addressError))
        {
            return Result.Failure<BuildHostCapabilitiesResult>(addressError!);
        }

        var response = await commandRouter.SendUnaryAsync(
            target.ResourceType,
            target.ResourceId,
            EdgeAgentCommandKind.ImageCheckBuildHost,
            new Google.Protobuf.WellKnownTypes.Empty().ToByteArray(),
            TimeSpan.FromSeconds(15),
            correlationId: null,
            cancellationToken);

        return response.IsSuccess && response.Payload is not null
            ? CheckBuildHostResponse.Parser.ParseFrom(response.Payload).Map()
            : Result.Failure<BuildHostCapabilitiesResult>(EdgeConnectorHelpers.CommandFailure(EdgeAgentCommandKind.ImageCheckBuildHost, response));
    }

    public async Task<Result<ExposedPortsResult>> GetExposedPortsAsync(RunImageInfoCommand runImageInfoCommand, CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetTarget(runImageInfoCommand.PlatformAddress, out var target, out var addressError))
        {
            return Result.Failure<ExposedPortsResult>(addressError!);
        }

        var response = await commandRouter.SendUnaryAsync(
            target.ResourceType,
            target.ResourceId,
            EdgeAgentCommandKind.ImageExposedPorts,
            new GetExposedPortsRequest { Id = runImageInfoCommand.ImageId }.ToByteArray(),
            TimeSpan.FromSeconds(30),
            correlationId: null,
            cancellationToken);

        return response.IsSuccess && response.Payload is not null
            ? new ExposedPortsResult(GetExposedPortsResponse.Parser.ParseFrom(response.Payload).Ports)
            : Result.Failure<ExposedPortsResult>(EdgeConnectorHelpers.CommandFailure(EdgeAgentCommandKind.ImageExposedPorts, response));
    }

    public async Task<Result<IEnumerable<DomainHistoryImageResult>>> HistoryImageAsync(HistoryImageCommand command, CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetTarget(command.PlatformAddress, out var target, out var addressError))
        {
            return Result.Failure<IEnumerable<DomainHistoryImageResult>>(addressError!);
        }

        var response = await commandRouter.SendUnaryAsync(
            target.ResourceType,
            target.ResourceId,
            EdgeAgentCommandKind.ImageHistory,
            new HistoryImageRequest { Id = command.ImageId }.ToByteArray(),
            TimeSpan.FromSeconds(30),
            correlationId: null,
            cancellationToken);

        return response.IsSuccess && response.Payload is not null
            ? HistoryImageResponse.Parser.ParseFrom(response.Payload).Map()
            : Result.Failure<IEnumerable<DomainHistoryImageResult>>(EdgeConnectorHelpers.CommandFailure(EdgeAgentCommandKind.ImageHistory, response));
    }

    public async Task<Result<DeleteImageResult>> DeleteImageAsync(DeleteImageCommand deleteImageCommand, CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetTarget(deleteImageCommand.PlatformAddress, out var target, out var addressError))
        {
            return Result.Failure<DeleteImageResult>(addressError!);
        }

        var response = await commandRouter.SendUnaryAsync(
            target.ResourceType,
            target.ResourceId,
            EdgeAgentCommandKind.ImageDelete,
            new DeleteImageRequest
            {
                Ids = { deleteImageCommand.Ids },
                Force = deleteImageCommand.Force,
                Noprune = deleteImageCommand.NoPrune
            }.ToByteArray(),
            TimeSpan.FromMinutes(2),
            correlationId: null,
            cancellationToken);

        return response.IsSuccess && response.Payload is not null
            ? DeleteImageResponse.Parser.ParseFrom(response.Payload).Map()
            : Result.Failure<DeleteImageResult>(EdgeConnectorHelpers.CommandFailure(EdgeAgentCommandKind.ImageDelete, response));
    }

    public async IAsyncEnumerable<PullImageStreamItem> PullImageProgressStreamAsync(
        PullImageCommand pullImageCommand,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetTarget(pullImageCommand.PlatformAddress, out var target, out _))
        {
            yield return new PullImageStreamItem(ErrorMessage: "Edge Agent address is invalid.");
            yield break;
        }

        var request = new PullImageRequest
        {
            FromImage = pullImageCommand.FromImage,
            FromSrc = pullImageCommand.FromSrc,
            Repo = pullImageCommand.Repo,
            Tag = pullImageCommand.Tag,
            Auth = pullImageCommand.Auth
        };

        await foreach (var item in commandRouter.SendServerStreamAsync(
                           target.ResourceType,
                           target.ResourceId,
                           EdgeAgentCommandKind.ImagePullStream,
                           request.ToByteArray(),
                           TimeSpan.FromHours(1),
                           correlationId: null,
                           cancellationToken))
        {
            if (item.ErrorMessage is not null)
            {
                yield return new PullImageStreamItem(ErrorMessage: item.ErrorMessage);
                yield break;
            }

            if (item.Completed)
            {
                yield break;
            }

            if (item.Payload is { Length: > 0 })
            {
                yield return PullImageResponse.Parser.ParseFrom(item.Payload).Map();
            }
        }
    }

    public async IAsyncEnumerable<ImageBuildStreamItem> BuildImageProgressStreamAsync(
        BuildImageCommand buildImageCommand,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetTarget(buildImageCommand.PlatformAddress, out var target, out _))
        {
            yield return new ImageBuildStreamItem(
                Id: null,
                Stream: null,
                Status: "error",
                ErrorMessage: "Edge Agent address is invalid.",
                ProgressMessage: null,
                Progress: null,
                Error: new ImageBuildError(400, "Edge Agent address is invalid."));
            yield break;
        }

        var request = new BuildImageRequest
        {
            ContextDirectory = buildImageCommand.ContextDirectory,
            DockerfilePath = buildImageCommand.DockerfilePath,
            Target = buildImageCommand.Target,
            RegistryAuth = buildImageCommand.RegistryAuth,
            RegistryHost = buildImageCommand.RegistryHost,
            TimeoutSeconds = (int)buildImageCommand.Timeout.TotalSeconds,
            MaxLineBytes = buildImageCommand.MaxLineBytes,
            DockerfileArchivePath = buildImageCommand.DockerfileArchivePath
        };
        request.Tags.AddRange(buildImageCommand.Tags);
        request.BuildArgs.Add(buildImageCommand.BuildArgs.ToDictionary());
        request.BuildSecrets.AddRange(buildImageCommand.Secrets?.Select(static secret => new Citadel.Images.V1.BuildImageSecret
        {
            Id = secret.Id,
            Value = secret.Value
        }) ?? []);
        if (buildImageCommand.ContextArchive is { Length: > 0 })
            request.ContextArchive = ByteString.CopyFrom(buildImageCommand.ContextArchive);

        await foreach (var item in commandRouter.SendServerStreamAsync(
                           target.ResourceType,
                           target.ResourceId,
                           EdgeAgentCommandKind.ImageBuildStream,
                           request.ToByteArray(),
                           buildImageCommand.Timeout,
                           correlationId: null,
                           cancellationToken))
        {
            if (item.ErrorMessage is not null)
            {
                yield return new ImageBuildStreamItem(
                    Id: null,
                    Stream: null,
                    Status: "error",
                    ErrorMessage: item.ErrorMessage,
                    ProgressMessage: null,
                    Progress: null,
                    Error: new ImageBuildError(500, item.ErrorMessage));
                yield break;
            }

            if (item.Completed)
            {
                yield break;
            }

            if (item.Payload is { Length: > 0 })
            {
                yield return ImageBuildResponse.Parser.ParseFrom(item.Payload).Map();
            }
        }
    }

    public async IAsyncEnumerable<ImageBuildStreamItem> PushImageProgressStreamAsync(
        PushImageCommand pushImageCommand,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetTarget(pushImageCommand.PlatformAddress, out var target, out _))
        {
            yield return new ImageBuildStreamItem(
                Id: null,
                Stream: null,
                Status: "error",
                ErrorMessage: "Edge Agent address is invalid.",
                ProgressMessage: null,
                Progress: null,
                Error: new ImageBuildError(400, "Edge Agent address is invalid."));
            yield break;
        }

        var request = new PushImageRequest
        {
            ImageReference = pushImageCommand.ImageReference,
            RegistryAuth = pushImageCommand.RegistryAuth
        };

        await foreach (var item in commandRouter.SendServerStreamAsync(
                           target.ResourceType,
                           target.ResourceId,
                           EdgeAgentCommandKind.ImagePushStream,
                           request.ToByteArray(),
                           TimeSpan.FromHours(1),
                           correlationId: null,
                           cancellationToken))
        {
            if (item.ErrorMessage is not null)
            {
                yield return new ImageBuildStreamItem(
                    Id: null,
                    Stream: null,
                    Status: "error",
                    ErrorMessage: item.ErrorMessage,
                    ProgressMessage: null,
                    Progress: null,
                    Error: new ImageBuildError(500, item.ErrorMessage));
                yield break;
            }

            if (item.Completed)
            {
                yield break;
            }

            if (item.Payload is { Length: > 0 })
            {
                yield return ImageBuildResponse.Parser.ParseFrom(item.Payload).Map();
            }
        }
    }
}
