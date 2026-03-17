using Domain.Entities.Alerts;
using Hosting.Common.Models;
using WebApi.Routes.Endpoints.Resources.Paging;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record AlertEventsView(PagedResultView<AlertEventView> PagedResult)
{
    public static AlertEventsView Map(PagedResult<AlertEvent> pagedResult)
    {
        return new AlertEventsView(new PagedResultView<AlertEventView>(
            Items: pagedResult.Items.Select(AlertEventView.Map),
            TotalCount: pagedResult.TotalCount,
            Page: pagedResult.Page,
            PageSize: pagedResult.PageSize));
    }
}
