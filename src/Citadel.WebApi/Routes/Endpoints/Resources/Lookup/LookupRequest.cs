using Application.Features.Networks.Queries;
using Domain;
using Microsoft.AspNetCore.Mvc;

namespace WebApi.Routes.Endpoints.Resources.Lookup;

//public sealed class LookupRequest2
//{
//    [FromQuery]
//    public LookupResourceType TargetResourceType { get; init; }

//    [FromQuery]
//    public LookupResourceType? SourceResourceType { get; init; }

//    [FromQuery]
//    public Guid? SourceResourceId { get; init; }

//    [FromQuery]
//    public Guid? PlatformId { get; init; }
//}

public sealed record LookupRequest(
    [FromQuery] LookupResourceType TargetResourceType,
    [FromQuery] LookupResourceType? SourceResourceType = null,
    [FromQuery] Guid? SourceResourceId = null,
    [FromQuery] Guid? PlatformId = null)
{
    public static ValueTask<LookupRequest?> BindAsync(HttpContext context)
    {
        var query = context.Request.Query;

        if (!query.TryGetValue("targetResourceType", out var targetValue) ||
            !Enum.TryParse<LookupResourceType>(targetValue, true, out var targetResourceType))
        {
            return ValueTask.FromResult<LookupRequest?>(null);
        }

        LookupResourceType? sourceResourceType = null;

        if (query.TryGetValue("sourceResourceType", out var sourceValue) &&
            Enum.TryParse<LookupResourceType>(sourceValue, true, out var parsedSource))
        {
            sourceResourceType = parsedSource;
        }

        Guid? sourceResourceId = null;

        if (query.TryGetValue("sourceResourceId", out var sourceIdValue) &&
            Guid.TryParse(sourceIdValue, out var parsedSourceId))
        {
            sourceResourceId = parsedSourceId;
        }

        Guid? platformId = null;

        if (query.TryGetValue("platformId", out var platformValue) &&
            Guid.TryParse(platformValue, out var parsedPlatformId))
        {
            platformId = parsedPlatformId;
        }

        return ValueTask.FromResult<LookupRequest?>(
            new(
                targetResourceType,
                sourceResourceType,
                sourceResourceId,
                platformId));
    }
}