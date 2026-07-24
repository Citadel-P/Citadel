using Application.Services.Builds;
using Domain.Entities.Stacks;
using LightResults;
using Moq;

namespace Tests.Unit.Application.Services.Builds;

public sealed class StackBuildImageBindingResolverTests
{
    [Fact]
    public async Task ResolveAsync_ShouldUseResolvedArtifact_WhenBindingContainsCompleteProvenance()
    {
        var projectId = Guid.CreateVersion7();
        var runId = Guid.CreateVersion7();
        var buildImageResolver = new Mock<IBuildImageResolver>();
        var resolver = new StackBuildImageBindingResolver(buildImageResolver.Object);
        var binding = new StackBuildImageBinding(
            ServiceName: "api",
            BuildProjectId: projectId,
            ResolvedImageReference: "registry.example.test/citadel/api:historical",
            ResolvedDigest: "sha256:historical",
            ResolvedBuildRunId: runId);

        var result = await resolver.ResolveAsync([binding], TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var resolved, out var error), error?.Message);
        var resolvedBinding = Assert.Single(resolved.Bindings);
        Assert.Equal(binding.ResolvedImageReference, resolvedBinding.ImageReference);
        Assert.Equal(binding.ResolvedDigest, resolvedBinding.Digest);
        Assert.Equal(runId, resolvedBinding.BuildRunId);
        buildImageResolver.Verify(
            x => x.ResolveLatestAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task ResolveAsync_ShouldResolveLatestArtifact_WhenBindingHasNoResolvedProvenance()
    {
        var projectId = Guid.CreateVersion7();
        var runId = Guid.CreateVersion7();
        var buildImageResolver = new Mock<IBuildImageResolver>();
        buildImageResolver
            .Setup(x => x.ResolveLatestAsync(projectId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new ResolvedBuildImage(
                projectId,
                "api",
                Guid.CreateVersion7(),
                "registry.example.test/citadel/api:latest",
                "sha256:latest",
                runId)));
        var resolver = new StackBuildImageBindingResolver(buildImageResolver.Object);
        var binding = new StackBuildImageBinding("api", projectId);

        var result = await resolver.ResolveAsync([binding], TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var resolved, out var error), error?.Message);
        var resolvedBinding = Assert.Single(resolved.Bindings);
        Assert.Equal("registry.example.test/citadel/api:latest", resolvedBinding.ImageReference);
        Assert.Equal("sha256:latest", resolvedBinding.Digest);
        Assert.Equal(runId, resolvedBinding.BuildRunId);
        buildImageResolver.Verify(
            x => x.ResolveLatestAsync(projectId, It.IsAny<CancellationToken>()),
            Times.Once);
    }
}
