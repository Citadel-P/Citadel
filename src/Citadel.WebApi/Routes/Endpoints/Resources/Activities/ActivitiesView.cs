using Domain;
using Domain.Entities.Activities;
using Hosting.Common.Models;
using WebApi.Routes.Endpoints.Resources.Paging;

namespace WebApi.Routes.Endpoints.Resources.Activities;

public sealed record ActivitiesView(PagedResultView<ActivityView> PagedResult)
{
    public static ActivitiesView Map(PagedResult<ActivityEvent> pagedResult)
    {
        return new ActivitiesView(new PagedResultView<ActivityView>(
            Items: pagedResult.Items.Select(ActivityView.Map),
            TotalCount: pagedResult.TotalCount,
            Page: pagedResult.Page,
            PageSize: pagedResult.PageSize
            ));
    }
}

public sealed record LatestActivityView(
    Guid Id,
    ActivityResourceType ResourceType,
    ActivityEventType EventType,
    ActivityStatus Status,
    ActivityEventInfo Info,
    DateTime CreatedAt)
{
    internal static LatestActivityView? Map(ActivityEvent? activityEvent)
    {
        if (activityEvent is null)
            return null;
        return new LatestActivityView(
            activityEvent.Id,
            activityEvent.ResourceType,
            activityEvent.EventType,
            activityEvent.Status,
            activityEvent.Info,
            activityEvent.CreatedAt);
    }

    
};

internal static class LatestActivityViewMapper
{
    internal static LatestActivityView? Map(this ActivityEvent? activityEvent)
    {
        if (activityEvent is null)
            return null;
        return new LatestActivityView(
            activityEvent.Id,
            activityEvent.ResourceType,
            activityEvent.EventType,
            activityEvent.Status,
            activityEvent.Info,
            activityEvent.CreatedAt);
    }
}