using Domain.Contracts.Resources.Containers;
using System.Text.RegularExpressions;

namespace Application.Services;

internal static partial class ContainerInspectionRedactor
{
    private const string RedactedValue = "********";

    public static ContainerInspectionInfo RedactEnvironment(
        ContainerInspectionInfo inspection)
    {
        if (inspection.Config is null || inspection.Config.Env.Count == 0)
            return inspection;

        var environment = inspection.Config.Env
            .Select(RedactEnvironmentValue)
            .ToArray();

        return inspection with
        {
            Config = inspection.Config with
            {
                Env = environment
            }
        };
    }

    private static string RedactEnvironmentValue(string value)
    {
        var separatorIndex = value.IndexOf('=');
        if (separatorIndex < 0 || separatorIndex == value.Length - 1)
            return value;

        var name = value[..separatorIndex];
        if (!SensitiveEnvironmentName().IsMatch(name))
            return value;

        return $"{name}={RedactedValue}";
    }

    [GeneratedRegex(
        @"(?:^|[_.-])(?:PASSWORD|PASSWD|SECRET|TOKEN|CREDENTIALS?|API[_.-]?KEY|ACCESS[_.-]?KEY|PRIVATE[_.-]?KEY|ENCRYPTION[_.-]?KEY|SIGNING[_.-]?KEY|CONNECTION[_.-]?STRINGS?|DATABASE[_.-]?(?:URL|DSN))(?:$|[_.-])",
        RegexOptions.IgnoreCase | RegexOptions.CultureInvariant)]
    private static partial Regex SensitiveEnvironmentName();
}
