using Infrastructure.EdgeAgents;

namespace Tests.Unit.Infrastructure.EdgeAgents;

public sealed class EdgeAgentCapabilitiesTests
{
    [Fact]
    public void TryValidate_ShouldAcceptRequiredCommands()
    {
        var result = EdgeAgentCapabilities.TryValidate(
            """{"commands":["platform.checkHealth","containers.list","containers.logs"]}""",
            out var normalized,
            out var error);

        Assert.True(result, error);
        Assert.Contains("platform.checkHealth", normalized);
    }

    [Fact]
    public void TryValidate_ShouldRejectMissingRequiredCommand()
    {
        var result = EdgeAgentCapabilities.TryValidate(
            """{"commands":["containers.list","containers.logs"]}""",
            out _,
            out var error);

        Assert.False(result);
        Assert.Contains("platform.checkHealth", error);
    }
}
