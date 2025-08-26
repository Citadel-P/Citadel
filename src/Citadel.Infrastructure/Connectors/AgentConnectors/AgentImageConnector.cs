using System.Runtime.CompilerServices;
using Citadel.Agent.Images.V1;
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

    public async IAsyncEnumerable<PullImageResult> PullImageProgressStreamAsync(PullImageCommand command, [EnumeratorCancellation] CancellationToken cancellationToken)
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

    Task<Result<IEnumerable<Domain.Contracts.Resources.Images.HistoryImageResult>>> IImageConnector.HistoryImageAsync(HistoryImageCommand command, CancellationToken cancellationToken)
    {
        throw new NotImplementedException();
    }
}
