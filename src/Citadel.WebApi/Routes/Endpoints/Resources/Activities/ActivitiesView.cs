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