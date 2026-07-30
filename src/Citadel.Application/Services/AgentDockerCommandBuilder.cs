namespace Application.Services;

internal static class AgentDockerCommandBuilder
{
    public static IReadOnlyDictionary<string, string>
        BuildRegularAgentEnvironment(
            string hubPublicKey,
            bool requiresTls)
    {
        var environment = new Dictionary<string, string>
        {
            ["HUB_PUBLIC_KEY"] = hubPublicKey
        };
        if (requiresTls)
        {
            environment["CITADEL_AGENT_TLS_MODE"] = "Direct";
            environment["CITADEL_AGENT_TLS_CERTIFICATE_PATH"] =
                "/etc/citadel/tls/agent-fullchain.pem";
            environment["CITADEL_AGENT_TLS_PRIVATE_KEY_PATH"] =
                "/etc/citadel/tls/agent-key.pem";
        }

        return environment;
    }

    public static string BuildRegularAgentCommand(
        string agentImage,
        IReadOnlyDictionary<string, string> environment,
        bool requiresTls)
    {
        var options = new List<string>
        {
            "--name citadel-agent",
            "--restart=always",
            "--label com.citadel.system=true",
            "--label com.citadel.system-role=agent",
            "-p 9000:9000",
            "-v /var/run/docker.sock:/var/run/docker.sock",
            "-v /:/host:ro"
        };
        if (requiresTls)
        {
            options.Add(
                "-v /path/to/agent-tls:/etc/citadel/tls:ro");
        }

        return Build(agentImage, environment, options);
    }

    public static string BuildEdgeAgentCommand(
        string agentImage,
        IReadOnlyDictionary<string, string> environment,
        bool includeHostRootMount,
        string? systemRole,
        string containerName = "citadel-agent",
        string dataVolumeName = "citadel_edge_agent_data")
    {
        var options = new List<string>
        {
            $"--name {containerName}",
            "--restart=always",
            "-v /var/run/docker.sock:/var/run/docker.sock"
        };
        if (!string.IsNullOrWhiteSpace(systemRole))
        {
            options.Add("--label com.citadel.system=true");
            options.Add($"--label com.citadel.system-role={systemRole}");
        }
        if (includeHostRootMount)
            options.Add("-v /:/host:ro");
        options.Add($"-v {dataVolumeName}:/app/data");

        return Build(agentImage, environment, options);
    }

    private static string Build(string agentImage, IReadOnlyDictionary<string, string> environment, IReadOnlyList<string> options)
    {
        var lines = new List<string>
        {
            "docker run -d \\"
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
