using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Stacks;
using System.Text;
using System.Text.RegularExpressions;

namespace Application.Services;

internal static partial class StackProjectNameResolver
{
    private static readonly Regex ComposeProjectNameRegex = DcProjectNameRegex();

    public static string Resolve(Stack stack)
        => Resolve(stack.Name, stack.Id, stack.CurrentStackRelease?.Spec);

    public static string Resolve(StackDriftStack stack)
        => Resolve(stack.Name, stack.Id, stack.Spec);

    public static string Resolve(string stackName, Guid stackId, StackSpec? spec)
    {
        if (!string.IsNullOrWhiteSpace(spec?.ProjectName))
        {
            return ValidateExplicitProjectName(spec.ProjectName);
        }

        return NormalizeComposeProjectName(stackName, stackId);
    }

    private static string ValidateExplicitProjectName(string projectName)
    {
        var value = projectName.Trim();
        if (!ComposeProjectNameRegex.IsMatch(value))
        {
            throw new InvalidOperationException(
                "Docker Compose project name must start with a lowercase letter or digit and contain only lowercase letters, digits, dashes, or underscores.");
        }

        return value;
    }

    private static string NormalizeComposeProjectName(string value, Guid stackId)
    {
        var builder = new StringBuilder(value.Length);
        var lastWasSeparator = false;

        foreach (var c in value.Trim().ToLowerInvariant())
        {
            if (char.IsAsciiLetterOrDigit(c))
            {
                builder.Append(c);
                lastWasSeparator = false;
            }
            else if ((c is '-' or '_') && builder.Length > 0 && !lastWasSeparator)
            {
                builder.Append(c);
                lastWasSeparator = true;
            }
            else if (builder.Length > 0 && !lastWasSeparator)
            {
                builder.Append('-');
                lastWasSeparator = true;
            }
        }

        var normalized = builder.ToString().Trim('-', '_');
        if (string.IsNullOrWhiteSpace(normalized))
        {
            return $"stack-{stackId:N}";
        }

        return char.IsAsciiLetterOrDigit(normalized[0])
            ? normalized
            : $"stack-{normalized}";
    }

    [GeneratedRegex("^[a-z0-9][a-z0-9_-]*$", RegexOptions.Compiled)]
    private static partial Regex DcProjectNameRegex();
}
