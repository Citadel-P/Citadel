using Citadel.Platforms.V1;
using Citadel.SharedModels.V1;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Platforms;
using Google.Protobuf;
using Google.Protobuf.WellKnownTypes;
using Grpc.Core;
using Hosting.Common.ErrorTypes;
using Infrastructure.Connectors.Mappers;
using LightResults;
using System.Runtime.CompilerServices;

namespace Infrastructure.Connectors.EdgeAgentConnectors;

internal sealed class EdgePlatformConnector(IEdgeAgentCommandRouter commandRouter) : IPlatformConnector
{
    public async Task<PlatformHealthResult> CheckHealthAsync(string platformAddress, CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(platformAddress, out var platformId, out _))
        {
            return EdgeConnectorHelpers.OfflineHealth();
        }

        var response = await commandRouter.SendUnaryAsync(
            platformId,
            EdgeAgentCommandKind.PlatformCheckHealth,
            new Empty().ToByteArray(),
            TimeSpan.FromSeconds(5),
            correlationId: null,
            cancellationToken);

        if (!response.IsSuccess || response.Payload is null)
        {
            return new PlatformHealthResult(false);
        }

        var health = CheckHealthResponse.Parser.ParseFrom(response.Payload);
        return new PlatformHealthResult(health.Healthy);
    }

    public async Task<Result<PlatformResult>> GetPlatformAsync(GetPlatformCommand command, CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(command.PlatformAddress, out var platformId, out var addressError))
        {
            return Result.Failure<PlatformResult>(addressError!);
        }

        var response = await commandRouter.SendUnaryAsync(
            platformId,
            EdgeAgentCommandKind.PlatformGetInfo,
            new Empty().ToByteArray(),
            TimeSpan.FromSeconds(15),
            correlationId: null,
            cancellationToken);

        if (!response.IsSuccess || response.Payload is null)
        {
            return Result.Failure<PlatformResult>(EdgeConnectorHelpers.CommandFailure(EdgeAgentCommandKind.PlatformGetInfo, response));
        }

        return PlatformInfoResponse.Parser.ParseFrom(response.Payload).Map(command.PlatformName, command.PlatformAddress);
    }

    public async Task<Result<PrunePlatformResult>> PruneAsync(PrunePlatformCommand command, CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(command.PlatformAddress, out var platformId, out var addressError))
        {
            return Result.Failure<PrunePlatformResult>(addressError!);
        }

        var request = new PruneRequest { Resource = command.Resource.MapToProto() };
        var response = await commandRouter.SendUnaryAsync(
            platformId,
            EdgeAgentCommandKind.PlatformPrune,
            request.ToByteArray(),
            TimeSpan.FromMinutes(5),
            correlationId: null,
            cancellationToken);

        if (!response.IsSuccess || response.Payload is null)
        {
            return Result.Failure<PrunePlatformResult>(EdgeConnectorHelpers.CommandFailure(EdgeAgentCommandKind.PlatformPrune, response));
        }

        return PruneResponse.Parser.ParseFrom(response.Payload).Map();
    }

    public async IAsyncEnumerable<PlatformStatsResult> StreamStatsAsync(
        StreamPlatformStatsCommand command,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(command.PlatformAddress, out var platformId, out _))
        {
            yield break;
        }

        var request = new PlatformStatsRequest { FetchIntervalMs = command.FetchIntervalMs };
        await foreach (var item in commandRouter.SendServerStreamAsync(
                           platformId,
                           EdgeAgentCommandKind.PlatformStatsStream,
                           request.ToByteArray(),
                           Timeout.InfiniteTimeSpan,
                           correlationId: null,
                           cancellationToken))
        {
            if (item.Completed || item.ErrorMessage is not null)
            {
                yield break;
            }

            if (item.Payload is { Length: > 0 })
            {
                yield return PlatformStatsResponse.Parser.ParseFrom(item.Payload).Map();
            }
        }
    }

    public async IAsyncEnumerable<DaemonEventInfo> StreamDaemonEventAsync(
        StreamDaemonEventCommand command,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(command.PlatformAddress, out var platformId, out _))
        {
            yield break;
        }

        await foreach (var item in commandRouter.SendServerStreamAsync(
                           platformId,
                           EdgeAgentCommandKind.PlatformDaemonEventsStream,
                           new Empty().ToByteArray(),
                           Timeout.InfiniteTimeSpan,
                           correlationId: null,
                           cancellationToken))
        {
            if (item.Completed || item.ErrorMessage is not null)
            {
                yield break;
            }

            if (item.Payload is { Length: > 0 })
            {
                yield return DaemonEventResponse.Parser.ParseFrom(item.Payload).Map();
            }
        }
    }
}
