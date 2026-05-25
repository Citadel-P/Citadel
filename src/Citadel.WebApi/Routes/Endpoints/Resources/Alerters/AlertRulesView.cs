using Application.Permissions;
using Domain.Entities.Alerts;
using Hosting.Common;
using Hosting.Common.Attributes;
using WebApi.Routes.Endpoints.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record AlertRulesView(IEnumerable<AlertRuleView> AlertRules)
{
    internal static async Task<AlertRulesView> Map((IEnumerable<AlertRule> Rules, IEnumerable<AlertChannel> Channels) data, IPermissionEvaluator permissionEvaluator)
    {
        var list = data.Rules as AlertRule[] ?? [.. data.Rules];

        if (list.Length == 0)
            return new AlertRulesView([]);

        var ids = new Guid[list.Length];

        for (var i = 0; i < list.Length; i++)
        {
            ids[i] = list[i].Id;
        }

        var channelsById = data.Channels.ToDictionary(c => c.Id);

        var perms = await permissionEvaluator.EvaluateAsync(ids, ResourceType.Alert);

        var views = new AlertRuleView[list.Length];

        for (var i = 0; i < list.Length; i++)
        {
            var rule = list[i];

            var baseView = AlertRuleView.Map(rule, channelsById);

            perms.TryGetValue(rule.Id, out var meta);

            views[i] = baseView with
            {
                Capabilities = CapabilityMapper.ToResourceCapabilities(
                    meta == default ? PermissionMetadata.Empty : meta)
            };
        }

        return new AlertRulesView(views);
    }
}
