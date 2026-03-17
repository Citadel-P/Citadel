using Application.Features.Alerters.Queries;
using Domain;
using Microsoft.AspNetCore.Mvc;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record AlertEventFilter(
    [FromQuery] Guid? ResourceId = null,
    [FromQuery] AlertType? AlertType = null,
    [FromQuery] AlertResourceType? ResourceType = null,
    [FromQuery] bool? UnresolvedOnly = null,
    [FromQuery] int Page = 1,
    [FromQuery] int PageSize = 50)
{
    internal GetAlertEvents ToQuery()
        => new(ResourceId, AlertType, ResourceType, UnresolvedOnly, Page, PageSize);

    public static ValueTask<AlertEventFilter> BindAsync(HttpContext context)
    {
        var query = context.Request.Query;

        Guid? resourceId = null;
        if (query.TryGetValue("resourceId", out var resourceIdStr) &&
            Guid.TryParse(resourceIdStr, out var parsedResourceId))
        {
            resourceId = parsedResourceId;
        }

        AlertType? alertType = null;
        if (query.TryGetValue("alertType", out var alertTypeStr) &&
            Enum.TryParse<AlertType>(alertTypeStr, true, out var parsedAlertType))
        {
            alertType = parsedAlertType;
        }

        AlertResourceType? resourceType = null;
        if (query.TryGetValue("resourceType", out var resourceTypeStr) &&
            Enum.TryParse<AlertResourceType>(resourceTypeStr, true, out var parsedResourceType))
        {
            resourceType = parsedResourceType;
        }

        bool? unresolvedOnly = null;
        if (query.TryGetValue("unresolvedOnly", out var unresolvedOnlyStr) &&
            bool.TryParse(unresolvedOnlyStr, out var parsedUnresolvedOnly))
        {
            unresolvedOnly = parsedUnresolvedOnly;
        }

        var page = query.TryGetValue("page", out var pageStr) && int.TryParse(pageStr, out var parsedPage)
            ? parsedPage
            : 1;

        var pageSize = query.TryGetValue("pageSize", out var pageSizeStr) && int.TryParse(pageSizeStr, out var parsedPageSize)
            ? parsedPageSize
            : 50;

        return ValueTask.FromResult(new AlertEventFilter(resourceId, alertType, resourceType, unresolvedOnly, page, pageSize));
    }
}
