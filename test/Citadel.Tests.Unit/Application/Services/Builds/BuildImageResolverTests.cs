using Application.Services.Builds;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Entities.Builds;
using Microsoft.Extensions.DependencyInjection;
using Moq;

namespace Tests.Unit.Application.Services.Builds;

public sealed class BuildImageResolverTests
{
    [Theory]
    [InlineData("registry.example.test/team/api:latest", "sha256:abc", "registry.example.test/team/api@sha256:abc")]
    [InlineData("registry.example.test:5000/team/api:latest", "sha256:abc", "registry.example.test:5000/team/api@sha256:abc")]
    [InlineData("registry.example.test/team/api@sha256:old", "sha256:new", "registry.example.test/team/api@sha256:new")]
    [InlineData("registry.example.test/team/api:latest", null, "registry.example.test/team/api:latest")]
    public void PinToDigest_ShouldReturnExpectedReference(string imageReference, string? digest, string expected)
    {
        Assert.Equal(expected, BuildImageReference.PinToDigest(imageReference, digest));
    }

    [Fact]
    public async Task ResolveAsync_ShouldUseStoredLegacyArtifactWithoutLoadingLatestRun()
    {
        var project = CreateProject();
        var buildProjects = new Mock<IBuildProjectRepository>();
        buildProjects
            .Setup(x => x.GetAsync(project.Id, It.IsAny<CancellationToken>(), true))
            .ReturnsAsync(project);
        var buildRuns = new Mock<IBuildRunRepository>();
        var resolver = CreateResolver(buildProjects.Object, buildRuns.Object);

        var result = await resolver.ResolveAsync(
            project.Id,
            "registry.example.test/team/api:historical",
            "sha256:historical",
            resolvedBuildRunId: null,
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var image, out var error), error?.Message);
        Assert.Equal("registry.example.test/team/api@sha256:historical", image.ImageReference);
        Assert.Equal("sha256:historical", image.Digest);
        Assert.Null(image.RunId);
        buildRuns.Verify(
            x => x.GetLatestSuccessfulByProjectAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task ResolveAsync_ShouldPinLatestSuccessfulRunToItsDigest()
    {
        var project = CreateProject();
        var run = CreateSuccessfulRun(project);
        var buildProjects = new Mock<IBuildProjectRepository>();
        buildProjects
            .Setup(x => x.GetAsync(project.Id, It.IsAny<CancellationToken>(), true))
            .ReturnsAsync(project);
        var buildRuns = new Mock<IBuildRunRepository>();
        buildRuns
            .Setup(x => x.GetLatestSuccessfulByProjectAsync(project.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(run);
        var resolver = CreateResolver(buildProjects.Object, buildRuns.Object);

        var result = await resolver.ResolveAsync(
            project.Id,
            resolvedImageReference: null,
            resolvedDigest: null,
            resolvedBuildRunId: null,
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var image, out var error), error?.Message);
        Assert.Equal("registry.example.test/team/api@sha256:latest", image.ImageReference);
        Assert.Equal(run.RegistrySnapshot.Id, image.RegistryId);
        Assert.Equal(run.Id, image.RunId);
    }

    [Fact]
    public async Task ResolveAsync_ShouldUseRetainedRunRegistryForStoredArtifact()
    {
        var project = CreateProject();
        var run = CreateSuccessfulRun(project);
        var buildProjects = new Mock<IBuildProjectRepository>();
        buildProjects
            .Setup(x => x.GetAsync(project.Id, It.IsAny<CancellationToken>(), true))
            .ReturnsAsync(project);
        var buildRuns = new Mock<IBuildRunRepository>();
        buildRuns
            .Setup(x => x.GetAsync(run.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(run);
        var resolver = CreateResolver(buildProjects.Object, buildRuns.Object);

        var result = await resolver.ResolveAsync(
            project.Id,
            "registry.example.test/team/api:historical",
            "sha256:historical",
            run.Id,
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var image, out var error), error?.Message);
        Assert.Equal(run.RegistrySnapshot.Id, image.RegistryId);
        Assert.Equal(run.Id, image.RunId);
    }

    private static BuildImageResolver CreateResolver(
        IBuildProjectRepository buildProjects,
        IBuildRunRepository buildRuns)
    {
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.BuildProjects).Returns(buildProjects);
        unitOfWork.Setup(x => x.BuildRuns).Returns(buildRuns);
        var services = new ServiceCollection()
            .AddSingleton(unitOfWork.Object)
            .BuildServiceProvider();
        return new BuildImageResolver(services.GetRequiredService<IServiceScopeFactory>());
    }

    private static BuildProject CreateProject()
        => new(
            name: "api",
            description: null,
            enabled: true,
            gitRepositoryId: Guid.CreateVersion7(),
            branch: "main",
            contextPath: ".",
            dockerfilePath: "Dockerfile",
            target: null,
            buildArgs: [],
            buildSecrets: [],
            platformId: Guid.CreateVersion7(),
            registryId: Guid.CreateVersion7(),
            imageRepository: "team/api",
            tagTemplates: ["latest"],
            webhook: null,
            timeoutSeconds: BuildProject.DefaultTimeoutSeconds,
            retentionRunCount: BuildProject.DefaultRetentionRunCount,
            createdByActorId: Guid.CreateVersion7());

    private static BuildRun CreateSuccessfulRun(BuildProject project)
        => new(
            buildProjectId: project.Id,
            projectNameSnapshot: project.Name,
            gitRepositoryId: project.GitRepositoryId,
            gitRepositoryNameSnapshot: "repository",
            branch: project.Branch,
            resolvedCommitSha: "abcdef1234567890",
            contextPath: project.ContextPath,
            dockerfilePath: project.DockerfilePath,
            target: project.Target,
            buildArgsSnapshot: [],
            buildSecretIdsSnapshot: [],
            platformSnapshot: new BuildPlatformSnapshot(
                project.PlatformId,
                "docker",
                "http://docker.local",
                PlatformConnectorType.Local),
            registrySnapshot: new BuildRegistrySnapshot(
                Guid.CreateVersion7(),
                "registry",
                "registry.example.test"),
            imageRepository: project.ImageRepository,
            tagTemplatesSnapshot: project.TagTemplates,
            imageReferences: ["registry.example.test/team/api:latest"],
            trigger: BuildRunTrigger.Manual,
            triggerSourceId: null,
            triggeredByActorId: project.CreatedByActorId,
            timeoutSeconds: project.TimeoutSeconds,
            status: BuildRunStatus.Succeeded,
            imageDigest: "sha256:latest");
}
