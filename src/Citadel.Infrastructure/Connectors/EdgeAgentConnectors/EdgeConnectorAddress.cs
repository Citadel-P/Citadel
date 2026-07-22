using Hosting.Common.ErrorTypes;
using LightResults;
using Domain;

namespace Infrastructure.Connectors.EdgeAgentConnectors;

internal readonly record struct EdgeConnectorTarget(EdgeAgentResourceType ResourceType, Guid ResourceId);

internal static class EdgeConnectorAddress
{
    public static Result<Guid> ParsePlatformId(string platformAddress)
    {
        const string prefix = "edge://";
        if (!platformAddress.StartsWith(prefix, StringComparison.OrdinalIgnoreCase))
        {
            return Result.Failure<Guid>(new BadRequestError("Edge Agent platform address is invalid."));
        }

        var rawId = platformAddress[prefix.Length..];
        return Guid.TryParse(rawId, out var platformId)
            ? Result.Success(platformId)
            : Result.Failure<Guid>(new BadRequestError("Edge Agent platform id is invalid."));
    }

    public static Result<EdgeConnectorTarget> ParseTarget(string platformAddress)
    {
        const string platformPrefix = "edge://";
        const string buildPoolPrefix = "edge-build-pool://";

        if (platformAddress.StartsWith(buildPoolPrefix, StringComparison.OrdinalIgnoreCase))
        {
            var rawId = platformAddress[buildPoolPrefix.Length..];
            return Guid.TryParse(rawId, out var buildPoolId)
                ? Result.Success(new EdgeConnectorTarget(EdgeAgentResourceType.BuildAgentPool, buildPoolId))
                : Result.Failure<EdgeConnectorTarget>(new BadRequestError("Edge Agent build pool id is invalid."));
        }

        if (platformAddress.StartsWith(platformPrefix, StringComparison.OrdinalIgnoreCase))
        {
            var rawId = platformAddress[platformPrefix.Length..];
            return Guid.TryParse(rawId, out var platformId)
                ? Result.Success(new EdgeConnectorTarget(EdgeAgentResourceType.Platform, platformId))
                : Result.Failure<EdgeConnectorTarget>(new BadRequestError("Edge Agent platform id is invalid."));
        }

        return Result.Failure<EdgeConnectorTarget>(new BadRequestError("Edge Agent address is invalid."));
    }
}
