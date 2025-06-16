using System.Runtime.CompilerServices;
using Citadel.Agent.Platforms.V1;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using Grpc.Core;
using Hosting.Common.ErrorTypes;
using Infrastructure.Connectors.Mappings;
using Infrastructure.Services.Abstractions;
using LightResults;

namespace Infrastructure.Connectors.AgentConnectors;

internal class AgentPlatformConnector(IGrpcClientFactory clientFactory) : IPlatformConnector
{
    public async Task<PlatformHealth> CheckHealthAsync(string platformAddress, CancellationToken cancellationToken)
    {
        try
        {
            var client = clientFactory.GetPlatformClient(platformAddress);
            var response = await client.CheckHealthAsync(new Google.Protobuf.WellKnownTypes.Empty(), deadline: DateTime.UtcNow.AddSeconds(2), cancellationToken: cancellationToken);
            return new PlatformHealth(Healthy: response.Healthy);
        }
        catch
        {
            return new PlatformHealth(Healthy: false);
        }
    }

    public async Task<Result<Platform>> GetPlatformAsync(GetPlatformCommand command, CancellationToken cancellationToken)
    {
        try
        {
            var platformClient = clientFactory.GetPlatformClient(command.PlatformAddress);
            
            var platform = await platformClient.GetPlatformInfoAsync(new Google.Protobuf.WellKnownTypes.Empty(), cancellationToken: cancellationToken);
            return platform.Map(platformAddress: command.PlatformAddress, platformName: command.PlatformName, type : PlatformConnectorType.Agent);
        }
        catch (RpcException ex)
        {
            return Result.Failure<Platform>(new ClientRpcException($"An RPC exception occurred: {ex.Message}", ex.StatusCode));
        }
    }

    public async IAsyncEnumerable<PlatformStatsBatch> StreamStatsAsync(StreamPlatformStatsCommand command, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var client = clientFactory.GetPlatformClient(command.PlatformAddress);
        using var stream = client.StreamPlatformStats(new PlatformStatsRequest { FetchIntervalMs = command.FetchIntervalMs }, cancellationToken: cancellationToken);
        await foreach (var response in stream.ResponseStream.ReadAllAsync(cancellationToken: cancellationToken))
        {
            yield return response.Map(command.PlatformId);
        }
    }
}
