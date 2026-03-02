using System.Text.RegularExpressions;
using Domain;
using Domain.Entities.Alerts;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed partial record AlertRuleView(
    Guid Id,
    string Name,
    AlertType Type,
    AlertSeverity Severity,
    int? CooldownSeconds,
    int? RequiredMatches,
    double? Threshold,
    bool IsEnabled,
    IEnumerable<Guid> Channels,
    IEnumerable<AlertRuleLimitedTo> LimitedTo,
    IEnumerable<AlertRuleQuietHour> QuietHours,
    Guid CreatedByActorId,
    DateTime CreatedAt)
{
    internal static AlertRuleView Map(AlertRule rule)
    {
        return new(
            rule.Id,
            SplitPascalCase(rule.Type.ToString()),
            rule.Type,
            rule.Severity,
            rule.CooldownSeconds,
            rule.RequiredMatches,
            rule.Threshold,
            rule.IsEnabled,
            rule.Channels,
            rule.LimitedTo,
            rule.QuietHours,
            rule.CreatedByActorId,
            rule.CreatedAt);
    }

    public static string SplitPascalCase(string value)
    {
        if (string.IsNullOrEmpty(value))
            return value;

        ReadOnlySpan<char> span = value.AsSpan();
        Span<char> buffer = stackalloc char[span.Length * 2]; 

        int pos = 0;
        bool firstWord = true;

        for (int i = 0; i < span.Length; i++)
        {
            char c = span[i];

            if (i > 0 && char.IsUpper(c))
            {
                buffer[pos++] = ' ';
                firstWord = false;
                buffer[pos++] = char.ToLowerInvariant(c);
            }
            else
            {
                buffer[pos++] = firstWord ? c : char.ToLowerInvariant(c);
            }
        }

        return new string(buffer[..pos]);
    }
}
