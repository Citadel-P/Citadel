using Application.Services;
using Domain;
using Domain.Entities.Registries;

namespace Tests.Unit.Application.Services;

public sealed class PullImageServiceTests
{
    [Fact]
    public void ToConnectorCommand_ShouldNotPrefixAnAlreadyQualifiedDigestReference()
    {
        var registry = CreateRegistry(
            "https://registry.example.test",
            new CustomRegistry());
        var input = new PullImageService.PullImageInput(
            Guid.CreateVersion7(),
            registry.Id,
            "registry.example.test/team/api@sha256:abc");

        var command = input.ToConnectorCommand("http://docker.local", registry);

        Assert.Equal("registry.example.test/team/api@sha256:abc", command.FromImage);
    }

    [Fact]
    public void ToConnectorCommand_ShouldPreserveRegistryPortAndAddDefaultTag()
    {
        var registry = CreateRegistry(
            "http://registry.example.test:5000/",
            new CustomRegistry());
        var input = new PullImageService.PullImageInput(
            Guid.CreateVersion7(),
            registry.Id,
            "team/api");

        var command = input.ToConnectorCommand("http://docker.local", registry);

        Assert.Equal("registry.example.test:5000/team/api:latest", command.FromImage);
    }

    [Fact]
    public void ToConnectorCommand_ShouldNotDuplicateGitHubRegistryAndNamespace()
    {
        var registry = CreateRegistry(
            "https://ghcr.io",
            new GitHubRegistry("citadelplane"));
        var input = new PullImageService.PullImageInput(
            Guid.CreateVersion7(),
            registry.Id,
            "ghcr.io/citadelplane/api@sha256:abc");

        var command = input.ToConnectorCommand("http://docker.local", registry);

        Assert.Equal("ghcr.io/citadelplane/api@sha256:abc", command.FromImage);
    }

    private static Registry CreateRegistry(string host, RegistryConfiguration configuration)
        => new(
            name: "registry",
            registryHost: host,
            status: RegistryStatus.Active,
            createdByActorId: Guid.CreateVersion7(),
            configuration: configuration);
}
