using Application.Services;

namespace Tests.Unit.Application.Services;

public sealed class AgentDockerCommandBuilderTests
{
    [Fact]
    public void BuildRegularAgentCommand_ShouldIncludeHostRootOnce()
    {
        var command = AgentDockerCommandBuilder.BuildRegularAgentCommand(
            "citadel-agent:1.0.0",
            new Dictionary<string, string>());

        Assert.Equal(1, CountOccurrences(command, "-v /:/host:ro"));
    }

    [Fact]
    public void BuildEdgeAgentCommand_ShouldIncludeHostRootOnceForPlatformAgent()
    {
        var command = AgentDockerCommandBuilder.BuildEdgeAgentCommand(
            "citadel-agent:1.0.0",
            new Dictionary<string, string>(),
            includeHostRootMount: true);

        Assert.Equal(1, CountOccurrences(command, "-v /:/host:ro"));
    }

    [Fact]
    public void BuildEdgeAgentCommand_ShouldExcludeHostRootForBuildPoolAgent()
    {
        var command = AgentDockerCommandBuilder.BuildEdgeAgentCommand(
            "citadel-agent:1.0.0",
            new Dictionary<string, string>(),
            includeHostRootMount: false);

        Assert.DoesNotContain("-v /:/host:ro", command);
        Assert.Contains("-v /var/run/docker.sock:/var/run/docker.sock", command);
        Assert.Contains("-v citadel_edge_agent_data:/app/data", command);
    }

    private static int CountOccurrences(string value, string search)
        => value.Split(search, StringSplitOptions.None).Length - 1;
}
