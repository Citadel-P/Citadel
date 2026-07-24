using Application.Services.Alerts;

namespace Tests.Unit.Application.Services.Alerts;

public sealed class PlatformVersionMismatchEvaluatorTests
{
    [Theory]
    [InlineData("1.0", true)]
    [InlineData("1.0.0", true)]
    [InlineData("1.0.0-beta.1+abcdef123", true)]
    [InlineData("1.0+abcdef123", true)]
    [InlineData("1.0-beta", true)]
    [InlineData("1.1.0", false)]
    [InlineData("2.0.0", false)]
    [InlineData("1.00.0", false)]
    [InlineData("1.0preview", false)]
    [InlineData("", false)]
    public void IsCompatible_ShouldMatchMajorMinorCompatibilityVersion(string agentVersion, bool expected)
    {
        Assert.Equal(expected, PlatformVersionMismatchEvaluator.IsCompatible(agentVersion));
    }
}
