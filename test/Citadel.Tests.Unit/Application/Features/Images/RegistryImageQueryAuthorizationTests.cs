using Application.Features.Images.Queries;
using Application.Permissions;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Registries;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Moq;

namespace Tests.Unit.Application.Features.Images;

public sealed class RegistryImageQueryAuthorizationTests
{
    [Fact]
    public async Task GetExternalRepositories_ShouldNotUseCredentials_WhenRegistryAccessIsDenied()
    {
        var registry = CreateDockerHubRegistry();
        var (unitOfWork, permissions) = CreateDeniedDependencies(registry);
        var dockerHub = new Mock<IDockerHubRegistryRepository>();
        var github = new Mock<IGitHubCrRepository>();
        var handler = new GetExternalRepositoriesHander(
            unitOfWork.Object,
            dockerHub.Object,
            github.Object,
            permissions.Object);

        var result = await handler.Handle(
            new GetExternalRepositories(registry.Name),
            TestContext.Current.CancellationToken);

        AssertForbidden(result);
        dockerHub.VerifyNoOtherCalls();
        github.VerifyNoOtherCalls();
    }

    [Fact]
    public async Task GetDockerHubRepositories_ShouldNotUseCredentials_WhenRegistryAccessIsDenied()
    {
        var registry = CreateDockerHubRegistry();
        var (unitOfWork, permissions) = CreateDeniedDependencies(registry);
        var dockerHub = new Mock<IDockerHubRegistryRepository>();
        var handler = new GetDockerHubRepositoriesHandler(
            unitOfWork.Object,
            dockerHub.Object,
            permissions.Object);

        var result = await handler.Handle(
            new GetDockerHubRepositories(registry.Name),
            TestContext.Current.CancellationToken);

        AssertForbidden(result);
        dockerHub.VerifyNoOtherCalls();
    }

    [Fact]
    public async Task GetDockerHubRepositoryTags_ShouldNotUseCredentials_WhenRegistryAccessIsDenied()
    {
        var registry = CreateDockerHubRegistry();
        var (unitOfWork, permissions) = CreateDeniedDependencies(registry);
        var dockerHub = new Mock<IDockerHubRegistryRepository>();
        var handler = new GetDockerHubRepositoryTagsHandler(
            unitOfWork.Object,
            dockerHub.Object,
            permissions.Object);

        var result = await handler.Handle(
            new GetDockerHubRepositoryTags(registry.Name, "private/repository"),
            TestContext.Current.CancellationToken);

        AssertForbidden(result);
        dockerHub.VerifyNoOtherCalls();
    }

    [Fact]
    public async Task GetGithubPackageVersions_ShouldNotUseCredentials_WhenRegistryAccessIsDenied()
    {
        var registry = CreateGitHubRegistry();
        var (unitOfWork, permissions) = CreateDeniedDependencies(registry);
        var github = new Mock<IGitHubCrRepository>();
        var handler = new GetGithubPackageVersionsHander(
            unitOfWork.Object,
            github.Object,
            permissions.Object);

        var result = await handler.Handle(
            new GetGithubPackageVersions(registry.Name, "private-package"),
            TestContext.Current.CancellationToken);

        AssertForbidden(result);
        github.VerifyNoOtherCalls();
    }

    private static (Mock<IUnitOfWork> UnitOfWork, Mock<IPermissionEvaluator> Permissions)
        CreateDeniedDependencies(Registry registry)
    {
        var registries = new Mock<IRegistryRepository>();
        registries
            .Setup(repository => repository.GetByNameAsync(
                registry.Name,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(registry);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(work => work.Registries).Returns(registries.Object);

        var permissions = new Mock<IPermissionEvaluator>();
        permissions
            .Setup(evaluator => evaluator.EvaluateAsync(
                registry.Id,
                ResourceType.Registry,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(PermissionMetadata.Empty);

        return (unitOfWork, permissions);
    }

    private static Registry CreateDockerHubRegistry()
        => new(
            "private-dockerhub",
            "docker.io",
            RegistryStatus.Active,
            Guid.CreateVersion7(),
            new DockerHubRegistry("owner", "secret"));

    private static Registry CreateGitHubRegistry()
        => new(
            "private-ghcr",
            "ghcr.io",
            RegistryStatus.Active,
            Guid.CreateVersion7(),
            new GitHubRegistry("owner", true, "secret"));

    private static void AssertForbidden<T>(LightResults.Result<T> result)
    {
        Assert.True(result.IsFailure(out var error));
        Assert.IsType<ForbiddenError>(error);
    }
}
