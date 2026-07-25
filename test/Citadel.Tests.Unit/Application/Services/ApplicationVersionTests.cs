using Application.Services;

namespace Tests.Unit.Application.Services;

public sealed class ApplicationVersionTests
{
    [Theory]
    [InlineData("1.0.0", "1.0.0")]
    [InlineData("1.0.0.11+dd591dd5a6", "1.0.0")]
    [InlineData("1.0.1-gdd591dd5a6", "1.0.1")]
    [InlineData("2.3.4-beta.1+abcdef123", "2.3.4")]
    [InlineData("1.0", "1.0.0")]
    [InlineData("invalid", "0.0.0")]
    public void GetCoreVersion_Should_Return_Three_Part_Release_Version(
        string informationalVersion,
        string expected)
    {
        Assert.Equal(expected, ApplicationVersion.GetCoreVersion(informationalVersion));
    }
}
