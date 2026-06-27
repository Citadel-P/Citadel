using Application.Features.GitRepositories.Queries;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Hosting.Common;
using LightResults;
using Moq;

namespace Tests.Unit.Application.Features.GitRepositories;

public sealed class DiscoverGitRepositoryComposeProjectsTests
{
    [Fact]
    public async Task Handle_Should_Discover_Monorepo_Compose_Projects_With_Ordered_Overrides()
    {
        var repo = new GitRepository(
            name: "homelab",
            description: null,
            url: "https://github.com/example/homelab.git",
            defaultBranch: "main",
            gitAccountId: null,
            createdByActorId: Constants.SystemId);

        var gitRepos = new Mock<IGitReposRepository>();
        gitRepos
            .Setup(x => x.GetWithAccountAsync(repo.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(repo);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.GitRepositories).Returns(gitRepos.Object);

        var repoCacheManager = new Mock<IRepoCacheManager>();
        repoCacheManager
            .Setup(x => x.SynchronizeAsync(repo, repo.GitAccount, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, "abc123", true));

        var gitCli = new Mock<IGitCliRepository>();
        gitCli
            .Setup(x => x.MaterializeSnapshotAsync(repo.GetCachePath(), "abc123", It.IsAny<string>(), It.IsAny<CancellationToken>()))
            .Callback<string, string, string, CancellationToken>((_, _, targetPath, _) =>
            {
                Directory.CreateDirectory(Path.Combine(targetPath, "stacks", "beszel"));
                Directory.CreateDirectory(Path.Combine(targetPath, "stacks", "caddy"));
                Directory.CreateDirectory(Path.Combine(targetPath, "test"));
                File.WriteAllText(Path.Combine(targetPath, "stacks", "beszel", "compose.yml"), "services: {}\n");
                File.WriteAllText(Path.Combine(targetPath, "stacks", "beszel", "compose.override.yml"), "services: {}\n");
                File.WriteAllText(Path.Combine(targetPath, "stacks", "beszel", ".env"), "APP_ENV=prod\n");
                File.WriteAllText(Path.Combine(targetPath, "stacks", "caddy", "docker-compose.yaml"), "services: {}\n");
                File.WriteAllText(Path.Combine(targetPath, "test", "docker-compose.yaml"), "services: {}\n");
                File.WriteAllText(Path.Combine(targetPath, "docker-compose.yaml"), "services: {}\n");
                File.WriteAllText(Path.Combine(targetPath, "docker-compose-postgresql.yaml"), "services: {}\n");
                File.WriteAllText(Path.Combine(targetPath, "README.md"), "ignored\n");
            })
            .ReturnsAsync(Result.Success());

        var handler = new DiscoverGitRepositoryComposeProjectsHandler(
            unitOfWork.Object,
            repoCacheManager.Object,
            gitCli.Object);

        var result = await handler.Handle(
            new DiscoverGitRepositoryComposeProjects(repo.Id, "main"),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var discovery, out var error), error?.Message);
        Assert.Equal(repo.Id, discovery.RepositoryId);
        Assert.Equal("main", discovery.Branch);
        Assert.Equal("abc123", discovery.ResolvedCommitSha);

        var beszel = Assert.Single(discovery.Projects, project => project.WorkingDirectory == "stacks/beszel");
        Assert.Equal(["stacks/beszel/compose.yml", "stacks/beszel/compose.override.yml"], beszel.ComposePaths);
        Assert.Equal(["stacks/beszel/.env"], beszel.EnvFilePaths);
        Assert.Equal(["stacks/beszel/**"], beszel.SuggestedWatchPaths);

        var caddy = Assert.Single(discovery.Projects, project => project.WorkingDirectory == "stacks/caddy");
        Assert.Equal(["stacks/caddy/docker-compose.yaml"], caddy.ComposePaths);
        Assert.Empty(caddy.EnvFilePaths);

        var root = Assert.Single(discovery.Projects, project => project.WorkingDirectory == ".");
        Assert.Equal(["docker-compose.yaml", "docker-compose-postgresql.yaml"], root.ComposePaths);

        var test = Assert.Single(discovery.Projects, project => project.WorkingDirectory == "test");
        Assert.Equal(["test/docker-compose.yaml"], test.ComposePaths);
    }
}
