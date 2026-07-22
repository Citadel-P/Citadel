using Domain;
using Domain.Contracts.Resources.Platforms;
using Grpc.Core;
using Hosting.Common.ErrorTypes;

namespace Infrastructure.Connectors.EdgeAgentConnectors;

internal static class EdgeConnectorHelpers
{
    public static bool TryGetPlatformId(string platformAddress, out Guid platformId, out ClientRpcException? error)
    {
        var platformIdResult = EdgeConnectorAddress.ParsePlatformId(platformAddress);
        if (platformIdResult.IsSuccess(out platformId, out var addressError))
        {
            error = null;
            return true;
        }

        error = new ClientRpcException(addressError?.Message ?? "Edge Agent platform address is invalid.", StatusCode.InvalidArgument);
        return false;
    }

    public static bool TryGetTarget(string platformAddress, out EdgeConnectorTarget target, out ClientRpcException? error)
    {
        var targetResult = EdgeConnectorAddress.ParseTarget(platformAddress);
        if (targetResult.IsSuccess(out target, out var addressError))
        {
            error = null;
            return true;
        }

        error = new ClientRpcException(addressError?.Message ?? "Edge Agent address is invalid.", StatusCode.InvalidArgument);
        return false;
    }

    public static ClientRpcException CommandFailure(EdgeAgentCommandKind kind, EdgeAgentCommandRouterResult response)
        => new(response.ErrorMessage ?? $"Edge Agent command '{kind}' failed.", StatusCode.Unavailable);

    public static PlatformHealthResult OfflineHealth() => new(false);
}
