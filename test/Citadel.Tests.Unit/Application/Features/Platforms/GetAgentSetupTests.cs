using Application.Configs;
using Application.Features.Platforms.Commands;
using Application.Features.Platforms.Queries;
using Application.Services;
using Microsoft.Extensions.Options;

namespace Tests.Unit.Application.Features.Platforms;

public sealed class GetAgentSetupTests
{
    [Fact]
    public async Task Handle_ShouldReturnRegularAgentSetupWithoutPrivateKeyMaterial()
    {
        var handler = new GetAgentSetupHandler(
            Options.Create(new EdgeAgentOptions
            {
                AgentImageRepository = "registry.example.com/citadel-agent",
                AgentImageTag = "v1.2.3+build.9"
            }),
            new TestHubPublicKeyProvider("public-key"));

        var result = await handler.Handle(new GetAgentSetup(), TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var setup, out var error), error?.Message);
        Assert.Equal("public-key", setup.HubPublicKey);
        Assert.Equal("public-key", setup.Environment["HUB_PUBLIC_KEY"]);
        Assert.Equal("registry.example.com/citadel-agent:1.2.3", setup.AgentImage);
        Assert.Contains("-p 9000:9000", setup.DockerRunCommand);
        Assert.Contains("-v /var/run/docker.sock:/var/run/docker.sock", setup.DockerRunCommand);
        Assert.Contains("-v /:/host:ro", setup.DockerRunCommand);
        Assert.Contains("-e HUB_PUBLIC_KEY=\"public-key\"", setup.DockerRunCommand);
        Assert.Contains("registry.example.com/citadel-agent:1.2.3", setup.DockerRunCommand);
        Assert.DoesNotContain("PRIVATE", setup.DockerRunCommand, StringComparison.OrdinalIgnoreCase);
        Assert.DoesNotContain("CITADEL_EDGE_ENROLLMENT_TOKEN", setup.DockerRunCommand);
    }

    [Fact]
    public async Task RotateAgentHubKey_ShouldReturnRotatedPublicKeySetup()
    {
        var provider = new TestHubPublicKeyProvider("old-public-key", "new-public-key");
        var handler = new RotateAgentHubKeyHandler(
            Options.Create(new EdgeAgentOptions
            {
                AgentImageRepository = "registry.example.com/citadel-agent",
                AgentImageTag = "v1.2.3"
            }),
            provider);

        var result = await handler.Handle(new RotateAgentHubKey(), TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var setup, out var error), error?.Message);
        Assert.Equal(1, provider.RotateCount);
        Assert.Equal("new-public-key", setup.HubPublicKey);
        Assert.Equal("new-public-key", setup.Environment["HUB_PUBLIC_KEY"]);
        Assert.Contains("-e HUB_PUBLIC_KEY=\"new-public-key\"", setup.DockerRunCommand);
        Assert.DoesNotContain("old-public-key", setup.DockerRunCommand);
    }

    private sealed class TestHubPublicKeyProvider(string publicKey, string? rotatedPublicKey = null) : IAgentHubPublicKeyProvider
    {
        public int RotateCount { get; private set; }

        public string GetPublicKey() => publicKey;

        public string RotateKeyPair()
        {
            RotateCount++;
            return rotatedPublicKey ?? publicKey;
        }
    }
}
