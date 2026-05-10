using Hosting.Common;
using Microsoft.AspNetCore.Mvc;

namespace WebApi.Routes.Endpoints.Resources.Lookup;

public sealed class LookupRequest
{
    [FromQuery]
    public ResourceType TargetResourceType { get; init; }

    [FromQuery]
    public ResourceType? SourceResourceType { get; init; }

    [FromQuery]
    public Guid? SourceResourceId { get; init; }

    [FromQuery]
    public Guid? PlatformId { get; init; }
}
