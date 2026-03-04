using Domain.Entities.Alerts;
using Hosting.Common.Models;
using WebApi.Routes.Endpoints.Resources.Paging;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record AlertRulesView(PagedResultView<AlertRuleView> PagedResult)
{
    public static AlertRulesView Map((PagedResult<AlertRule> Rules, IEnumerable<AlertChannel> Channels) data)
        => Map(data.Rules, data.Channels.ToDictionary(c => c.Id));

    public static AlertRulesView Map(PagedResult<AlertRule> pagedResult, IReadOnlyDictionary<Guid, AlertChannel> channelsById)
    {
        return new AlertRulesView(new PagedResultView<AlertRuleView>(
            Items: pagedResult.Items.Select(r => AlertRuleView.Map(r, channelsById)),
            TotalCount: pagedResult.TotalCount,
            Page: pagedResult.Page,
            PageSize: pagedResult.PageSize));
    }
}
