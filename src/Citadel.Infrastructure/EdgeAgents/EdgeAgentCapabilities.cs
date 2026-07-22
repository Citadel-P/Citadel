using System.Text;
using System.Text.Json;

namespace Infrastructure.EdgeAgents;

internal static class EdgeAgentCapabilities
{
    public static bool TryValidate(string? capabilitiesJson, out string normalizedJson, out string? error)
    {
        normalizedJson = "{}";
        error = null;

        if (string.IsNullOrWhiteSpace(capabilitiesJson))
        {
            error = "Edge Agent capabilities are required.";
            return false;
        }

        if (Encoding.UTF8.GetByteCount(capabilitiesJson) > EdgeAgentDefaults.MaxCapabilitiesJsonBytes)
        {
            error = "Edge Agent capabilities payload is too large.";
            return false;
        }

        try
        {
            using var document = JsonDocument.Parse(capabilitiesJson);
            if (document.RootElement.ValueKind != JsonValueKind.Object ||
                !document.RootElement.TryGetProperty("commands", out var commandsElement) ||
                commandsElement.ValueKind != JsonValueKind.Array)
            {
                error = "Edge Agent capabilities must include a commands array.";
                return false;
            }

            var commands = new HashSet<string>(StringComparer.Ordinal);
            foreach (var command in commandsElement.EnumerateArray())
            {
                if (command.ValueKind == JsonValueKind.String &&
                    !string.IsNullOrWhiteSpace(command.GetString()))
                {
                    commands.Add(command.GetString()!);
                }
            }

            foreach (var requiredCommand in EdgeAgentDefaults.RequiredCommands)
            {
                if (!commands.Contains(requiredCommand))
                {
                    error = $"Edge Agent does not advertise required capability '{requiredCommand}'.";
                    return false;
                }
            }

            normalizedJson = capabilitiesJson.Trim();
            return true;
        }
        catch (JsonException)
        {
            error = "Edge Agent capabilities must be valid JSON.";
            return false;
        }
    }

    public static string? NormalizeHeartbeatCapabilities(string? capabilitiesJson)
        => TryValidate(capabilitiesJson, out var normalizedJson, out _)
            ? normalizedJson
            : null;

    public static bool HasRequiredCommands(string capabilitiesJson, IReadOnlyCollection<string> requiredCommands, out string missingCommand)
    {
        missingCommand = string.Empty;

        try
        {
            using var document = JsonDocument.Parse(capabilitiesJson);
            if (document.RootElement.ValueKind != JsonValueKind.Object ||
                !document.RootElement.TryGetProperty("commands", out var commandsElement) ||
                commandsElement.ValueKind != JsonValueKind.Array)
            {
                missingCommand = requiredCommands.FirstOrDefault() ?? string.Empty;
                return false;
            }

            var commands = new HashSet<string>(StringComparer.Ordinal);
            foreach (var command in commandsElement.EnumerateArray())
            {
                if (command.ValueKind == JsonValueKind.String &&
                    !string.IsNullOrWhiteSpace(command.GetString()))
                {
                    commands.Add(command.GetString()!);
                }
            }

            missingCommand = requiredCommands.FirstOrDefault(command => !commands.Contains(command)) ?? string.Empty;
            return missingCommand.Length == 0;
        }
        catch (JsonException)
        {
            missingCommand = requiredCommands.FirstOrDefault() ?? string.Empty;
            return false;
        }
    }
}
