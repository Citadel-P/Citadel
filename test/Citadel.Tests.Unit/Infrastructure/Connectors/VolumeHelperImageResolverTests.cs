using Application.Configs;
using Application.Services;
using Domain;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Options;

namespace Tests.Unit.Infrastructure.Connectors;

public sealed class VolumeHelperImageResolverTests
{
    [Fact]
    public void Resolve_ShouldUseVersionedCoreImageByDefault()
    {
        var resolver = CreateResolver();

        Assert.Equal($"ghcr.io/citadel-p/citadel:{ApplicationVersion.CoreVersion}", resolver.Resolve(PlatformConnectorType.Local));
    }

    [Fact]
    public void Resolve_ShouldUseVersionedCoreImageInHostDevelopmentUnlessExplicitlyConfigured()
    {
        var configuration = new ConfigurationBuilder()
            .AddInMemoryCollection(new Dictionary<string, string?>
            {
                ["ASPNETCORE_ENVIRONMENT"] = "Development"
            })
            .Build();
        var resolver = CreateResolver(configuration);

        Assert.Equal($"ghcr.io/citadel-p/citadel:{ApplicationVersion.CoreVersion}", resolver.Resolve(PlatformConnectorType.Local));
    }

    [Fact]
    public void Resolve_ShouldUseVersionedCoreImageInContainerDevelopmentUnlessExplicitlyConfigured()
    {
        var configuration = new ConfigurationBuilder()
            .AddInMemoryCollection(new Dictionary<string, string?>
            {
                ["ASPNETCORE_ENVIRONMENT"] = "Development",
                ["DOTNET_RUNNING_IN_CONTAINER"] = "true"
            })
            .Build();
        var resolver = CreateResolver(configuration);

        Assert.Equal($"ghcr.io/citadel-p/citadel:{ApplicationVersion.CoreVersion}", resolver.Resolve(PlatformConnectorType.Local));
    }

    [Theory]
    [InlineData(PlatformConnectorType.Agent)]
    [InlineData(PlatformConnectorType.EdgeAgent)]
    public void Resolve_ShouldUseVersionedAgentImageForAgentConnectors(PlatformConnectorType connectorType)
    {
        var resolver = CreateResolver();

        Assert.Equal($"ghcr.io/citadel-p/citadel.agent:{ApplicationVersion.CoreVersion}", resolver.Resolve(connectorType));
    }

    [Fact]
    public void Resolve_ShouldUseExplicitHelperImageWhenConfigured()
    {
        var configuration = new ConfigurationBuilder()
            .AddInMemoryCollection(new Dictionary<string, string?>
            {
                ["VolumeBrowser:HelperImage"] = "local/citadel-helper:test"
            })
            .Build();
        var resolver = CreateResolver(configuration);

        Assert.Equal("local/citadel-helper:test", resolver.Resolve(PlatformConnectorType.Agent));
        Assert.Equal("local/citadel-helper:test", resolver.Resolve(PlatformConnectorType.Local));
    }

    [Theory]
    [InlineData("citadel-helper:dev", false)]
    [InlineData("local/citadel-helper:dev", false)]
    [InlineData("citadel.dev", false)]
    [InlineData("ghcr.io/citadel-p/citadel:dev", true)]
    [InlineData("registry.example.com/citadel-helper:dev", true)]
    [InlineData("localhost:5000/citadel-helper:dev", true)]
    [InlineData("citadel-helper:1.0", true)]
    public void CanPullHelperImage_ShouldNotPullLocalDevelopmentTags(string image, bool expected)
    {
        Assert.Equal(expected, VolumeContentService.CanPullHelperImage(image));
    }

    private static VolumeHelperImageResolver CreateResolver(IConfiguration? configuration = null)
        => new(configuration ?? new ConfigurationBuilder().Build(), Options.Create(new EdgeAgentOptions()));
}
