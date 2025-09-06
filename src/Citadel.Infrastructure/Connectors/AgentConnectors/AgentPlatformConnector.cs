using System.Runtime.CompilerServices;
using Citadel.Agent.Platforms.V1;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Platforms;
using Grpc.Core;
using Hosting.Common.ErrorTypes;
using Infrastructure.Connectors.Mappers;
using Infrastructure.Repositories;
using LightResults;

namespace Infrastructure.Connectors.AgentConnectors;

internal class AgentPlatformConnector(IGrpcClientFactory clientFactory) : IPlatformConnector
{
    public async Task<PlatformHealthResult> CheckHealthAsync(string platformAddress, CancellationToken cancellationToken)
    {
        try
        {
            var client = clientFactory.GetPlatformClient(platformAddress);
            var response = await client.CheckHealthAsync(new Google.Protobuf.WellKnownTypes.Empty(), deadline: DateTime.UtcNow.AddSeconds(2), cancellationToken: cancellationToken);
            return new PlatformHealthResult(Healthy: response.Healthy);
        }
        catch
        {
            return new PlatformHealthResult(Healthy: false);
        }
    }

    public async Task<Result<PlatformResult>> GetPlatformAsync(GetPlatformCommand command, CancellationToken cancellationToken)
    {
        try
        {
            var platformClient = clientFactory.GetPlatformClient(command.PlatformAddress);

            var platform = await platformClient.GetPlatformInfoAsync(new Google.Protobuf.WellKnownTypes.Empty(), cancellationToken: cancellationToken);
            return platform.Map(platformAddress: command.PlatformAddress, platformName: command.PlatformName);
        }
        catch (RpcException ex)
        {
            return Result.Failure<PlatformResult>(new ClientRpcException($"An RPC exception occurred: {ex.Message}", ex.StatusCode));
        }
    }

    public async IAsyncEnumerable<PlatformStatsResult> StreamStatsAsync(StreamPlatformStatsCommand command, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var client = clientFactory.GetPlatformClient(command.PlatformAddress);
        using var stream = client.StreamPlatformStats(new PlatformStatsRequest { FetchIntervalMs = command.FetchIntervalMs }, cancellationToken: cancellationToken);
        await foreach (var response in stream.ResponseStream.ReadAllAsync(cancellationToken: cancellationToken))
        {
            yield return response.Map();
        }
    }

    public async IAsyncEnumerable<DaemonEventInfo> StreamDaemonEventAsync(StreamDaemonEventCommand command, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var containerClient = clientFactory.GetPlatformClient(command.PlatformAddress);
        using var streamCall = containerClient.StreamDaemonEvent(new Google.Protobuf.WellKnownTypes.Empty(), cancellationToken: cancellationToken);
        await foreach (var response in streamCall.ResponseStream.ReadAllAsync(cancellationToken: cancellationToken))
        {
            yield return response.Map();
        }
    }
}
