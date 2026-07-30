using Application.Services;

namespace Tests.Unit.Application.Services;

public sealed class AgentDockerCommandBuilderTests
{
    [Fact]
    public void BuildRegularAgentCommand_ShouldIncludeHostRootOnce()
    {
        var command = AgentDockerCommandBuilder.BuildRegularAgentCommand(
            "citadel-agent:1.0.0",
            new Dictionary<string, string>(),
            requiresTls: true);

        Assert.Equal(1, CountOccurrences(command, "-v /:/host:ro"));
        Assert.Contains("--label com.citadel.system=true", command);
        Assert.Contains("--label com.citadel.system-role=agent", command);
        Assert.Contains(
            "-v /path/to/agent-tls:/etc/citadel/tls:ro",
            command);
    }

    [Fact]
    public void BuildRegularAgentCommand_ShouldNotMountTlsForExplicitInsecureMode()
    {
        var command = AgentDockerCommandBuilder.BuildRegularAgentCommand(
            "citadel-agent:1.0.0",
            new Dictionary<string, string>(),
            requiresTls: false);

        Assert.DoesNotContain("/etc/citadel/tls", command);
    }

    [Fact]
    public void BuildEdgeAgentCommand_ShouldIncludeHostRootOnceForPlatformAgent()
    {
        var command = AgentDockerCommandBuilder.BuildEdgeAgentCommand(
            "citadel-agent:1.0.0",
            new Dictionary<string, string>(),
            includeHostRootMount: true,
            systemRole: "edge-agent");

        Assert.Equal(1, CountOccurrences(command, "-v /:/host:ro"));
        Assert.Contains("--label com.citadel.system=true", command);
        Assert.Contains("--label com.citadel.system-role=edge-agent", command);
    }

    [Fact]
    public void BuildEdgeAgentCommand_ShouldExcludeHostRootForBuildPoolAgent()
    {
        var command = AgentDockerCommandBuilder.BuildEdgeAgentCommand(
            "citadel-agent:1.0.0",
            new Dictionary<string, string>(),
            includeHostRootMount: false,
            systemRole: null);

        Assert.DoesNotContain("-v /:/host:ro", command);
        Assert.Contains("-v /var/run/docker.sock:/var/run/docker.sock", command);
        Assert.Contains("-v citadel_edge_agent_data:/app/data", command);
        Assert.DoesNotContain("com.citadel.system", command);
    }

    private static int CountOccurrences(string value, string search)
        => value.Split(search, StringSplitOptions.None).Length - 1;
}
