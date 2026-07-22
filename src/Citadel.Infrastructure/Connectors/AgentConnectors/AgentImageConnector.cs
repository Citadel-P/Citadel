using System.Runtime.CompilerServices;
using Citadel.Images.V1;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using Grpc.Core;
using Hosting.Common.ErrorTypes;
using Infrastructure.Connectors.Mappers;
using Infrastructure.Repositories;
using LightResults;

namespace Infrastructure.Connectors.AgentConnectors;

internal class AgentImageConnector(IGrpcClientFactory clientFactory) : IImageConnector
{
    public async Task<Result<ImageResult>> GetAsync(string platformAddress, string imageId, CancellationToken cancellationToken)
    {
        try
        {
            var client = clientFactory.GetImageClient(platformAddress);
            var image = await client.GetAsync(new GetImageRequest() { Id = imageId }, cancellationToken: cancellationToken);
            return image.Map();
        }
        catch (RpcException ex)
        {
            return Result.Failure<ImageResult>(new ClientRpcException($"An RPC exception occurred: {ex.Message}", ex.StatusCode));
        }
    }

    public async Task<Result<IReadOnlyList<ImageResult>>> ListImagesAsync(string platformAddress, CancellationToken cancellationToken)
    {
        try
        {
            var client = clientFactory.GetImageClient(platformAddress);
            var images = await client.ListAsync(new ListImagesRequest(), cancellationToken: cancellationToken);
            return images.Images.Map();
        }
        catch (RpcException ex)
        {
            return Result.Failure<IReadOnlyList<ImageResult>>(new ClientRpcException($"An RPC exception occurred: {ex.Message}", ex.StatusCode));
        }
    }

    public async Task<Result<InspectImageResult>> InspectImageAsync(InspectImageCommand command, CancellationToken cancellationToken)
    {
        try
        {
            var args = new InspectImageRequest
            {
                Id = command.ImageId
            };
            var client = clientFactory.GetImageClient(command.PlatformAddress);
            var response = await client.InspectAsync(args, cancellationToken: cancellationToken);
            return response.Map();
        }
        catch (RpcException ex)
        {
            return Result.Failure<InspectImageResult>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }

    public async Task<Result<DeleteImageResult>> DeleteImageAsync(DeleteImageCommand command, CancellationToken cancellationToken)
    {
        try
        {
            var client = clientFactory.GetImageClient(command.PlatformAddress);
            var request = new DeleteImageRequest
            {
                Ids = { command.Ids },
                Force = command.Force,
                Noprune = command.NoPrune
            };

            var response = await client.DeleteAsync(request, cancellationToken: cancellationToken);
            return response.Map();
        }
        catch (RpcException ex)
        {
            return Result.Failure<DeleteImageResult>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }

    public async IAsyncEnumerable<PullImageStreamItem> PullImageProgressStreamAsync(PullImageCommand command, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var client = clientFactory.GetImageClient(command.PlatformAddress);
        var request = new PullImageRequest
        {
            FromImage = command.FromImage,
            FromSrc = command.FromSrc,
            Repo = command.Repo,
            Tag = command.Tag,
            Auth = command.Auth
        };

        using var stream = client.Pull(request, cancellationToken: cancellationToken);
        await foreach (var reply in stream.ResponseStream.ReadAllAsync(cancellationToken: cancellationToken))
        {
            yield return reply.Map();
        }
    }

    public async IAsyncEnumerable<ImageBuildStreamItem> BuildImageProgressStreamAsync(
        BuildImageCommand command,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var client = clientFactory.GetImageClient(command.PlatformAddress);
        var request = new BuildImageRequest
        {
            ContextDirectory = command.ContextDirectory,
            DockerfilePath = command.DockerfilePath,
            Target = command.Target,
            RegistryAuth = command.RegistryAuth,
            RegistryHost = command.RegistryHost,
            TimeoutSeconds = (int)command.Timeout.TotalSeconds,
            MaxLineBytes = command.MaxLineBytes,
            DockerfileArchivePath = command.DockerfileArchivePath
        };
        request.Tags.AddRange(command.Tags);
        request.BuildArgs.Add(command.BuildArgs.ToDictionary());
        request.BuildSecrets.AddRange(command.Secrets?.Select(static secret => new Citadel.Images.V1.BuildImageSecret
        {
            Id = secret.Id,
            Value = secret.Value
        }) ?? []);
        if (command.ContextArchive is { Length: > 0 })
            request.ContextArchive = Google.Protobuf.ByteString.CopyFrom(command.ContextArchive);

        using var stream = client.Build(request, cancellationToken: cancellationToken);
        await foreach (var reply in stream.ResponseStream.ReadAllAsync(cancellationToken: cancellationToken))
        {
            yield return reply.Map();
        }
    }

    public async IAsyncEnumerable<ImageBuildStreamItem> PushImageProgressStreamAsync(
        PushImageCommand command,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var client = clientFactory.GetImageClient(command.PlatformAddress);
        var request = new PushImageRequest
        {
            ImageReference = command.ImageReference,
            RegistryAuth = command.RegistryAuth
        };

        using var stream = client.Push(request, cancellationToken: cancellationToken);
        await foreach (var reply in stream.ResponseStream.ReadAllAsync(cancellationToken: cancellationToken))
        {
            yield return reply.Map();
        }
    }

    public async Task<Result<IEnumerable<Domain.Contracts.Resources.Images.HistoryImageResult>>> HistoryImageAsync(HistoryImageCommand command, CancellationToken cancellationToken)
    {
        try
        {
            var client = clientFactory.GetImageClient(command.PlatformAddress);
            var request = new HistoryImageRequest
            {
                Id = command.ImageId
            };

            var response = await client.HistoryAsync(request, cancellationToken: cancellationToken);
            return response.Map();
        }
        catch (RpcException ex)
        {
            return Result.Failure<IEnumerable<Domain.Contracts.Resources.Images.HistoryImageResult>> (new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }

    public async Task<Result<ExposedPortsResult>> GetExposedPortsAsync(RunImageInfoCommand command, CancellationToken cancellationToken)
    {
        try
        {
            var client = clientFactory.GetImageClient(command.PlatformAddress);
            var request = new GetExposedPortsRequest
            {
                Id = command.ImageId
            };

            var response = await client.GetExposedPortsAsync(request, cancellationToken: cancellationToken);
            return new ExposedPortsResult(Ports: response.Ports);
        }
        catch (RpcException ex)
        {
            return Result.Failure<ExposedPortsResult>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }

    public async Task<Result<DistributionResult>> DistributionInspectAsync(DistributionInspectCommand command, CancellationToken cancellationToken)
    {
        try
        {
            var client = clientFactory.GetImageClient(command.PlatformAddress);
            var request = new DistributionInspectRequest
            {
                ImageName = command.ImageName,
                Auth = command.Auth
            };

            var response = await client.DistributionInspectAsync(request, cancellationToken: cancellationToken);
            return response.Map();
        }
        catch (RpcException ex)
        {
            return Result.Failure<DistributionResult>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }

    public async Task<Result<BuildHostCapabilitiesResult>> CheckBuildHostAsync(string platformAddress, CancellationToken cancellationToken)
    {
        try
        {
            var client = clientFactory.GetImageClient(platformAddress);
            var response = await client.CheckBuildHostAsync(new Google.Protobuf.WellKnownTypes.Empty(), deadline: DateTime.UtcNow.AddSeconds(10), cancellationToken: cancellationToken);
            return response.Map();
        }
        catch (RpcException ex)
        {
            return Result.Failure<BuildHostCapabilitiesResult>(new ClientRpcException($"An error occurred while checking build host capabilities, {ex.Message}", ex.StatusCode));
        }
    }
}
