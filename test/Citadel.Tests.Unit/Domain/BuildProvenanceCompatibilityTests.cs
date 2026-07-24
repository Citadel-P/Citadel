using System.Text.Json;
using Domain;
using Domain.Entities.Deployments;
using Domain.Entities.Stacks;

namespace Tests.Unit.Domain;

public sealed class BuildProvenanceCompatibilityTests
{
    [Fact]
    public void BuildImage_ShouldDeserializeLegacyJsonWithoutProvenanceFields()
    {
        var projectId = Guid.CreateVersion7();
        var json = $$"""
                     {
                       "BuildProjectId": "{{projectId}}",
                       "RedeployOnBuild": true
                     }
                     """;

        var image = JsonSerializer.Deserialize(json, DeploymentJsonContext.Default.BuildImage);

        Assert.NotNull(image);
        Assert.Equal(projectId, image.BuildProjectId);
        Assert.True(image.RedeployOnBuild);
        Assert.Null(image.ResolvedImageReference);
        Assert.Null(image.ResolvedBuildRunId);
        Assert.Null(image.AppliedImageReference);
        Assert.Null(image.AppliedBuildRunId);
    }

    [Fact]
    public void StackBuildImageBinding_ShouldDeserializeLegacyJsonWithoutProvenanceFields()
    {
        var projectId = Guid.CreateVersion7();
        var json = $$"""
                     {
                       "ServiceName": "api",
                       "BuildProjectId": "{{projectId}}",
                       "RedeployOnBuild": false
                     }
                     """;

        var binding = JsonSerializer.Deserialize(json, StackJsonContext.Default.StackBuildImageBinding);

        Assert.NotNull(binding);
        Assert.Equal("api", binding.ServiceName);
        Assert.Equal(projectId, binding.BuildProjectId);
        Assert.Null(binding.ResolvedImageReference);
        Assert.Null(binding.ResolvedBuildRunId);
        Assert.Null(binding.AppliedImageReference);
        Assert.Null(binding.AppliedBuildRunId);
    }
}
