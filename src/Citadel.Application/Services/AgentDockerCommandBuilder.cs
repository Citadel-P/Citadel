namespace Application.Services;

internal static class AgentDockerCommandBuilder
{
    public static string BuildRegularAgentCommand(string agentImage, IReadOnlyDictionary<string, string> environment)
        => Build(
            agentImage,
            environment,
            [
                "-p 9000:9000",
                "-v /var/run/docker.sock:/var/run/docker.sock"
            ]);

    public static string BuildEdgeAgentCommand(string agentImage, IReadOnlyDictionary<string, string> environment)
        => Build(
            agentImage,
            environment,
            [
                "-v /var/run/docker.sock:/var/run/docker.sock",
                "-v citadel_edge_agent_data:/app/data"
            ]);

    private static string Build(string agentImage, IReadOnlyDictionary<string, string> environment, IReadOnlyList<string> options)
    {
        var lines = new List<string>
        {
            "docker run -d \\",
            "  --name citadel-agent \\",
            "  --restart=always \\"
        };

        lines.AddRange(options.Select(option => $"  {option} \\"));
        lines.AddRange(environment.Select(kv => $"  -e {kv.Key}=\"{EscapeDockerValue(kv.Value)}\" \\"));
        lines.Add($"  {agentImage}");

        return string.Join("\n", lines);
    }

    private static string EscapeDockerValue(string value) =>
        value
            .Replace("\\", "\\\\", StringComparison.Ordinal)
            .Replace("\"", "\\\"", StringComparison.Ordinal);
}
