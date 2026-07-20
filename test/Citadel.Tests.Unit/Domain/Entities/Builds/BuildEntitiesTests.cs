using Domain.Entities.Builds;

namespace Tests.Unit.Domain.Entities.Builds;

public sealed class BuildEntitiesTests
{
    [Fact]
    public void BuildProject_ShouldValidateCitadelBackedBuildSecrets()
    {
        var project = CreateProject(
            buildSecrets: [new BuildSecretSpec("npmrc", Guid.CreateVersion7())]);

        project.Validate();

        Assert.Equal("npmrc", project.BuildSecrets[0].Id);
    }

    [Fact]
    public void BuildProject_ShouldRejectInvalidBuildSecretId()
    {
        var project = CreateProject(
            buildSecrets: [new BuildSecretSpec("npm token", Guid.CreateVersion7())]);

        var ex = Assert.Throws<ArgumentException>(project.Validate);

        Assert.Contains("Build secret id 'npm token' is invalid", ex.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void BuildProject_ShouldRejectBuildSecretWithoutCitadelSecret()
    {
        var project = CreateProject(
            buildSecrets: [new BuildSecretSpec("npmrc", Guid.Empty)]);

        var ex = Assert.Throws<ArgumentException>(project.Validate);

        Assert.Contains("must reference a Citadel secret", ex.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void BuildProject_ShouldRejectDuplicateBuildSecretIds()
    {
        var project = CreateProject(
            buildSecrets:
            [
                new BuildSecretSpec("npmrc", Guid.CreateVersion7()),
                new BuildSecretSpec("NPMRC", Guid.CreateVersion7())
            ]);

        var ex = Assert.Throws<ArgumentException>(project.Validate);

        Assert.Contains("is mapped more than once", ex.Message, StringComparison.Ordinal);
    }

    private static BuildProject CreateProject(IReadOnlyList<BuildSecretSpec>? buildSecrets = null)
        => new(
            name: "demo",
            description: null,
            enabled: true,
            gitRepositoryId: Guid.CreateVersion7(),
            branch: "main",
            contextPath: ".",
            dockerfilePath: "Dockerfile",
            target: null,
            buildArgs: [],
            buildSecrets: buildSecrets,
            platformId: Guid.CreateVersion7(),
            registryId: Guid.CreateVersion7(),
            imageRepository: "team/demo",
            tagTemplates: ["{branch}-{shortSha}"],
            webhook: null,
            timeoutSeconds: BuildProject.DefaultTimeoutSeconds,
            retentionRunCount: BuildProject.DefaultRetentionRunCount,
            createdByActorId: Guid.CreateVersion7());
}
