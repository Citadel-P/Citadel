using Domain.Entities.Alerts;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record AlertRulesView(IEnumerable<AlertRuleView> AlertRules)
{
    public static AlertRulesView Map((IEnumerable<AlertRule> Rules, IEnumerable<AlertChannel> Channels) data)
        => Map(data.Rules, data.Channels.ToDictionary(c => c.Id));

    public static AlertRulesView Map(IEnumerable<AlertRule> rules, IReadOnlyDictionary<Guid, AlertChannel> channelsById)
    {
        return new AlertRulesView(rules.Select(r => AlertRuleView.Map(r, channelsById)));
    }
}
