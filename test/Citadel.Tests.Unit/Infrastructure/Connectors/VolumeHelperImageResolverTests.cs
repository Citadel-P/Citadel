using Application.Services;
using Microsoft.Extensions.Configuration;

namespace Tests.Unit.Infrastructure.Connectors;

public sealed class VolumeHelperImageResolverTests
{
    [Fact]
    public void Resolve_ShouldUseVersionedAgentImageByDefault()
    {
        var resolver = new VolumeHelperImageResolver(new ConfigurationBuilder().Build());

        Assert.Equal("ghcr.io/citadel-p/citadel.agent:1.0", resolver.Resolve());
    }

    [Fact]
    public void Resolve_ShouldUseLocalAgentImageInDevelopmentUnlessExplicitlyConfigured()
    {
        var configuration = new ConfigurationBuilder()
            .AddInMemoryCollection(new Dictionary<string, string?>
            {
                ["ASPNETCORE_ENVIRONMENT"] = "Development"
            })
            .Build();
        var resolver = new VolumeHelperImageResolver(configuration);

        Assert.Equal("citadel-agent:dev", resolver.Resolve());
    }

    [Fact]
    public void Resolve_ShouldUseExplicitHelperImageWhenConfigured()
    {
        var configuration = new ConfigurationBuilder()
            .AddInMemoryCollection(new Dictionary<string, string?>
            {
                ["VolumeBrowser:HelperImage"] = "local/citadel-agent:test"
            })
            .Build();
        var resolver = new VolumeHelperImageResolver(configuration);

        Assert.Equal("local/citadel-agent:test", resolver.Resolve());
    }

    [Fact]
    public void Resolve_ShouldUseConfiguredAgentRepositoryAndTag()
    {
        var configuration = new ConfigurationBuilder()
            .AddInMemoryCollection(new Dictionary<string, string?>
            {
                ["EdgeAgent:AgentImageRepository"] = "local/citadel-agent",
                ["EdgeAgent:AgentImageTag"] = "v1.2.3+build"
            })
            .Build();
        var resolver = new VolumeHelperImageResolver(configuration);

        Assert.Equal("local/citadel-agent:1.2.3", resolver.Resolve());
    }

    [Theory]
    [InlineData("citadel-agent:dev", false)]
    [InlineData("local/citadel-agent:dev", false)]
    [InlineData("ghcr.io/citadel-p/citadel.agent:dev", true)]
    [InlineData("registry.example.com/citadel-agent:dev", true)]
    [InlineData("localhost:5000/citadel-agent:dev", true)]
    [InlineData("citadel-agent:1.0", true)]
    public void CanPullHelperImage_ShouldNotPullLocalDevelopmentTags(string image, bool expected)
    {
        Assert.Equal(expected, VolumeContentService.CanPullHelperImage(image));
    }
}
