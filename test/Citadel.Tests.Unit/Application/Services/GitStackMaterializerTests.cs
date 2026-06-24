using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Domain.Entities.Stacks;
using LightResults;
using Moq;

namespace Tests.Unit.Application.Services;

public class GitStackMaterializerTests
{
    [Fact]
    public async Task MaterializeAsync_Should_Return_Compose_And_Env_From_Resolved_Commit()
    {
        var stack = CreateGitStack(["compose.yml"], [".env"]);
        var repository = CreateRepository();
        var repoCache = new Mock<IRepoCacheManager>();
        var gitCli = new Mock<IGitCliRepository>();

        repoCache
            .Setup(x => x.SynchronizeAsync(repository, repository.GitAccount, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, "abc123", true));

        gitCli
            .Setup(x => x.MaterializeSnapshotAsync(repository.GetCachePath(), "abc123", It.IsAny<string>(), It.IsAny<CancellationToken>()))
            .Callback<string, string, string, CancellationToken>((_, _, targetPath, _) =>
            {
                Directory.CreateDirectory(targetPath);
                File.WriteAllText(Path.Combine(targetPath, "compose.yml"), "services:\n  app:\n    image: nginx");
                File.WriteAllText(Path.Combine(targetPath, ".env"), "APP_ENV=prod");
            })
            .ReturnsAsync(Result.Success());

        var materializer = new GitStackMaterializer(repoCache.Object, gitCli.Object);

        var result = await materializer.MaterializeAsync(
            stack,
            (GitStack)stack.CurrentStackRelease!.Spec,
            repository,
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var payload, out var error), error.Message);
        Assert.Equal("abc123", payload.ResolvedCommitSha);
        Assert.Equal("main", payload.SourceBranch);
        Assert.Contains("image: nginx", payload.ComposeContent);
        Assert.Contains("APP_ENV=prod", payload.EnvironmentVariables);
        Assert.Equal(["compose.yml"], payload.ComposePaths);
        Assert.Equal([".env"], payload.EnvFilePaths);
    }

    [Fact]
    public async Task MaterializeAsync_Should_Reject_Compose_Path_That_Escapes_Repository_Root()
    {
        var stack = CreateGitStack(["../compose.yml"], []);
        var repository = CreateRepository();
        var repoCache = new Mock<IRepoCacheManager>();
        var gitCli = new Mock<IGitCliRepository>();

        repoCache
            .Setup(x => x.SynchronizeAsync(repository, repository.GitAccount, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, "abc123", true));

        gitCli
            .Setup(x => x.MaterializeSnapshotAsync(repository.GetCachePath(), "abc123", It.IsAny<string>(), It.IsAny<CancellationToken>()))
            .Callback<string, string, string, CancellationToken>((_, _, targetPath, _) => Directory.CreateDirectory(targetPath))
            .ReturnsAsync(Result.Success());

        var materializer = new GitStackMaterializer(repoCache.Object, gitCli.Object);

        var result = await materializer.MaterializeAsync(
            stack,
            (GitStack)stack.CurrentStackRelease!.Spec,
            repository,
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Contains("escapes the repository root", error.Message);
    }

    private static Stack CreateGitStack(List<string> composePaths, List<string> envFiles)
    {
        return Stack.Create(
            name: "git-stack",
            createdByActorId: Guid.CreateVersion7(),
            StackSource: StackSource.Git,
            platformId: Guid.CreateVersion7(),
            spec: new GitStack(
                GitRepoId: Guid.CreateVersion7(),
                Branch: "main",
                CommitSha: null,
                UpdateBehavior: StackUpdateBehavior.Notify,
                ComposePaths: composePaths,
                AdditionalEnvFileFromRepo: envFiles));
    }

    private static GitRepository CreateRepository()
        => new(
            name: "repo",
            description: null,
            url: "https://github.com/org/repo",
            defaultBranch: "main",
            gitAccountId: null,
            createdByActorId: Guid.CreateVersion7());
}
