using Application.Features.Activities.Queries;
using Domain;
using Microsoft.AspNetCore.Mvc;

namespace WebApi.Routes.Endpoints.Resources.Activities;

public sealed record ActivityFilter(
    [FromQuery] Guid? ResourceId = null,
    [FromQuery] ActivityResourceType? ResourceType = null,
    [FromQuery] ActivityEventType? EventType = null,
    [FromQuery] int Page = 1,
    [FromQuery] int PageSize = 50
    )
{
    internal GetActivities ToQuery()
        => new(
            ResourceId,
            ResourceType,
            EventType,
            Page,
            PageSize);

    public static ValueTask<ActivityFilter> BindAsync(HttpContext context)
    {
        var query = context.Request.Query;

        Guid? resourceId = null;
        if (query.TryGetValue("resourceId", out var resourceIdStr) &&
            Guid.TryParse(resourceIdStr, out var parsedResourceId))
        {
            resourceId = parsedResourceId;
        }

        ActivityResourceType? resourceType = null;
        if (query.TryGetValue("resourceType", out var resourceTypeStr) &&
            Enum.TryParse<ActivityResourceType>(resourceTypeStr, true, out var parsedResourceType))
        {
            resourceType = parsedResourceType;
        }

        ActivityEventType? eventType = null;
        if (query.TryGetValue("eventType", out var eventTypeStr) &&
            Enum.TryParse<ActivityEventType>(eventTypeStr, true, out var parsedEventType))
        {
            eventType = parsedEventType;
        }

        var page = query.TryGetValue("page", out var pageStr) && int.TryParse(pageStr, out var parsedPage)
            ? parsedPage
            : 1;

        var pageSize = query.TryGetValue("pageSize", out var pageSizeStr) && int.TryParse(pageSizeStr, out var parsedPageSize)
            ? parsedPageSize
            : 50;

        var result = new ActivityFilter(resourceId, resourceType, eventType, page, pageSize);
        return ValueTask.FromResult(result);
    }
}