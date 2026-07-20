using Application.Features.Builds.Commands;
using Application.Features.Builds.Models;
using Domain;
using Domain.Entities.Activities;
using Domain.Entities.Builds;
using System.Text.Json;

namespace Tests.Unit.Application.Features.Builds;

public sealed class BuildProjectValidationTests
{
    [Fact]
    public void CreateBuildProject_ShouldRejectInvalidBuildSecretId()
    {
        var command = new CreateBuildProject(CreateInput(
            buildSecrets: [new BuildSecretSpec("npm token", Guid.CreateVersion7())]));

        var result = new CreateBuildProject.Validator().Validate(command);

        Assert.False(result.IsValid);
        Assert.Contains(result.Errors, error => error.ErrorMessage.Contains("BuildKit secret id", StringComparison.Ordinal));
    }

    [Fact]
    public void CreateBuildProject_ShouldRejectEmptyBuildSecretReference()
    {
        var command = new CreateBuildProject(CreateInput(
            buildSecrets: [new BuildSecretSpec("npmrc", Guid.Empty)]));

        var result = new CreateBuildProject.Validator().Validate(command);

        Assert.False(result.IsValid);
        Assert.Contains(result.Errors, error => error.PropertyName.Contains("SecretId", StringComparison.Ordinal));
    }

    [Fact]
    public void CreateBuildProject_ShouldRejectDuplicateBuildSecretIds()
    {
        var command = new CreateBuildProject(CreateInput(
            buildSecrets:
            [
                new BuildSecretSpec("npmrc", Guid.CreateVersion7()),
                new BuildSecretSpec("NPMRC", Guid.CreateVersion7())
            ]));

        var result = new CreateBuildProject.Validator().Validate(command);

        Assert.False(result.IsValid);
        Assert.Contains(result.Errors, error => error.ErrorMessage.Contains("unique", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void UpdateBuildProject_ShouldIgnoreBuildSecrets_WhenFlagIsFalse()
    {
        var command = new UpdateBuildProject(
            ProjectId: Guid.CreateVersion7(),
            Project: new UpdateBuildProjectInputModel(BuildSecrets: [new BuildSecretSpec("bad id", Guid.Empty)]),
            UpdateDescription: false,
            UpdateBuildArgs: false,
            UpdateBuildSecrets: false,
            UpdateWebhook: false);

        var result = new UpdateBuildProject.Validator().Validate(command);

        Assert.True(result.IsValid);
    }

    [Fact]
    public void UpdateBuildProject_ShouldValidateBuildSecrets_WhenFlagIsTrue()
    {
        var command = new UpdateBuildProject(
            ProjectId: Guid.CreateVersion7(),
            Project: new UpdateBuildProjectInputModel(BuildSecrets: [new BuildSecretSpec("bad id", Guid.Empty)]),
            UpdateDescription: false,
            UpdateBuildArgs: false,
            UpdateBuildSecrets: true,
            UpdateWebhook: false);

        var result = new UpdateBuildProject.Validator().Validate(command);

        Assert.False(result.IsValid);
        Assert.Contains(result.Errors, error => error.ErrorMessage.Contains("BuildKit secret id", StringComparison.Ordinal));
    }

    [Fact]
    public void BuildActivitySnapshot_ShouldIncludeBuildSecretReferences()
    {
        var secretId = Guid.CreateVersion7();
        var project = CreateProject(
            buildSecrets: [new BuildSecretSpec("npmrc", secretId)]);

        var snapshot = BuildActivity.Snapshot(project);

        Assert.NotNull(snapshot.BuildSecrets);
        var buildSecret = Assert.Single(snapshot.BuildSecrets);
        Assert.Equal("npmrc", buildSecret.Id);
        Assert.Equal(secretId, buildSecret.SecretId);
    }

    [Fact]
    public void BuildUpdatedActivityInfo_ShouldSerializeBuildSecretReferences()
    {
        var project = CreateProject(
            buildSecrets: [new BuildSecretSpec("npmrc", Guid.CreateVersion7())]);
        var snapshot = BuildActivity.Snapshot(project);

        var json = JsonSerializer.Serialize(
            new BuildUpdated(snapshot, snapshot),
            EventInfoJsonContext.Default.BuildUpdated);

        Assert.Contains("\"BuildSecrets\"", json, StringComparison.Ordinal);
        Assert.Contains("\"SecretId\"", json, StringComparison.Ordinal);
    }

    private static BuildProjectInputModel CreateInput(IReadOnlyList<BuildSecretSpec>? buildSecrets = null)
        => new(
            Name: "demo",
            Description: null,
            Enabled: true,
            GitRepositoryId: Guid.CreateVersion7(),
            Branch: "main",
            ContextPath: ".",
            DockerfilePath: "Dockerfile",
            Target: null,
            BuildArgs: [],
            BuildSecrets: buildSecrets,
            PlatformId: Guid.CreateVersion7(),
            RegistryId: Guid.CreateVersion7(),
            ImageRepository: "team/demo",
            TagTemplates: ["{branch}-{shortSha}"],
            Webhook: null,
            TimeoutSeconds: 1800,
            RetentionRunCount: 20);

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
            timeoutSeconds: 1800,
            retentionRunCount: 20,
            createdByActorId: Guid.CreateVersion7());
}
