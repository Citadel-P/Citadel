using Domain.Entities.Alerts;
using Hosting.Common.Models;
using WebApi.Routes.Endpoints.Resources.Paging;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record AlertRulesView(PagedResultView<AlertRuleView> PagedResult)
{
    public static AlertRulesView Map(PagedResult<AlertRule> pagedResult)
    {
        return new AlertRulesView(new PagedResultView<AlertRuleView>(
            Items: pagedResult.Items.Select(AlertRuleView.Map),
            TotalCount: pagedResult.TotalCount,
            Page: pagedResult.Page,
            PageSize: pagedResult.PageSize));
    }
}
