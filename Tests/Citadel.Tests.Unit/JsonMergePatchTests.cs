using Hosting.Common.MergePatch;
using Domain;
using Domain.Entities;
using Domain.Entities.Registries;
using Infrastructure.EntityFramework.Configurations;

namespace Tests.Unit;

public class JsonMergePatchTests
{
    [Fact]
    public async Task ApplyMergePatch_ShouldMergeSimpleObjects()
    {
        // Arrange
        var original = new DockerHubRegistry(UserName: "username-1", PAT: "fake-pat-1");
        var patch = JsonMergePatchDocument<DockerHubRegistry>.FromJson("""
            {
                "PAT": "patched-pat"
            }
            """);

        // Act
        var result = patch.ApplyTo(original, RegistryJsonContext.Default.DockerHubRegistry);

        // Assert
        await Verify(result);
    }

    [Fact]
    public async Task ApplyMergePatch_ShouldRemovePropertyWithNullPatchValue()
    {
        // Arrange
        var original = new DockerHubRegistry(UserName: "username-1", PAT: "fake-pat-1");
        var patch = JsonMergePatchDocument<DockerHubRegistry>.FromJson("""
            {
                "UserName": null
            }
            """);

        // Act
        var result = patch.ApplyTo(original, RegistryJsonContext.Default.DockerHubRegistry);

        // Assert
        await Verify(result);
    }

    [Fact]
    public async Task ApplyMergePatch_ShouldMergeNestedObjectsRecursively()
    {
        // Arrange
        var original = new Registry(
            name: "MyRegistry",
            url: "http://localhost:1234/registry",
            type: RegistryType.DockerHub,
            configuration: new DockerHubRegistry(
                UserName: "username-1",
                PAT: "fake-pat-1")
            );

        var patch = JsonMergePatchDocument<Registry>.FromJson("""
            {
                "Configuration": {
                  "$type": "DockerHub",
                  "UserName": "username-2"
                }
            }
            """);

        // Act
        var result = patch.ApplyTo(original, RegistryJsonContext.Default.Registry);

        // Assert
        await Verify(result);
    }

}

