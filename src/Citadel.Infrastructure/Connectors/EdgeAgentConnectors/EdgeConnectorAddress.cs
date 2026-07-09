using Hosting.Common.ErrorTypes;
using LightResults;

namespace Infrastructure.Connectors.EdgeAgentConnectors;

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
}
