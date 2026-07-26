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
        using var temp = new TempDirectory();

        repoCache
            .Setup(x => x.SynchronizeAsync(repository, repository.GitAccount, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, "abc123", true));

        gitCli
            .Setup(x => x.MaterializeSnapshotAsync(
                ApplicationStoragePaths.GetRepositoryCachePath(repository),
                "abc123",
                It.IsAny<string>(),
                It.IsAny<CancellationToken>()))
            .Callback<string, string, string, CancellationToken>((_, _, targetPath, _) =>
            {
                Directory.CreateDirectory(targetPath);
                File.WriteAllText(Path.Combine(targetPath, "compose.yml"), "services:\n  app:\n    image: nginx");
                File.WriteAllText(Path.Combine(targetPath, ".env"), "APP_ENV=prod");
            })
            .ReturnsAsync(Result.Success());

        var materializer = new GitStackMaterializer(repoCache.Object, gitCli.Object, new TestStackStoragePathProvider(temp.Path));

        var result = await materializer.MaterializeAsync(
            stack,
            (GitStack)stack.CurrentStackRelease!.Spec,
            repository,
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var payload, out var error), error?.Message);
        Assert.Equal("abc123", payload.ResolvedCommitSha);
        Assert.Equal("main", payload.SourceBranch);
        Assert.Equal(Path.Combine(payload.SnapshotRoot, "compose.yml"), payload.SourceComposeFilePaths.Single());
        Assert.Equal(payload.SnapshotRoot, payload.SourceWorkingDirectory);
        Assert.True(File.Exists(payload.LabelsOverrideFilePath));
        Assert.Contains("com.citadel.managed", await File.ReadAllTextAsync(payload.LabelsOverrideFilePath, TestContext.Current.CancellationToken));
        Assert.Empty(payload.EnvironmentVariables);
        Assert.Equal(["compose.yml"], payload.ComposePaths);
        Assert.Equal([".env"], payload.EnvFilePaths);
        Assert.Equal(Path.Combine(payload.SnapshotRoot, ".env"), payload.SourceEnvFilePaths.Single());
    }

    [Fact]
    public async Task MaterializeAsync_Should_Use_First_Compose_Parent_As_Working_Directory()
    {
        var stack = CreateGitStack(["apps/beszel/compose.yml"], ["apps/beszel/.env"]);
        var repository = CreateRepository();
        var repoCache = new Mock<IRepoCacheManager>();
        var gitCli = new Mock<IGitCliRepository>();
        using var temp = new TempDirectory();

        repoCache
            .Setup(x => x.SynchronizeAsync(repository, repository.GitAccount, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, "abc123", true));

        gitCli
            .Setup(x => x.MaterializeSnapshotAsync(
                ApplicationStoragePaths.GetRepositoryCachePath(repository),
                "abc123",
                It.IsAny<string>(),
                It.IsAny<CancellationToken>()))
            .Callback<string, string, string, CancellationToken>((_, _, targetPath, _) =>
            {
                var appPath = Path.Combine(targetPath, "apps", "beszel");
                Directory.CreateDirectory(appPath);
                File.WriteAllText(Path.Combine(appPath, "compose.yml"), "services:\n  app:\n    image: nginx");
                File.WriteAllText(Path.Combine(appPath, ".env"), "APP_ENV=prod");
            })
            .ReturnsAsync(Result.Success());

        var materializer = new GitStackMaterializer(repoCache.Object, gitCli.Object, new TestStackStoragePathProvider(temp.Path));

        var result = await materializer.MaterializeAsync(
            stack,
            (GitStack)stack.CurrentStackRelease!.Spec,
            repository,
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var payload, out var error), error?.Message);
        Assert.Equal(Path.Combine(payload.SnapshotRoot, "apps", "beszel"), payload.SourceWorkingDirectory);
        Assert.Equal(Path.Combine(payload.SourceWorkingDirectory, "compose.yml"), payload.SourceComposeFilePaths.Single());
        Assert.Equal(["apps/beszel/compose.yml"], payload.ComposePaths);
        Assert.Equal(["apps/beszel/.env"], payload.EnvFilePaths);
        Assert.Equal(Path.Combine(payload.SnapshotRoot, "apps", "beszel", ".env"), payload.SourceEnvFilePaths.Single());
    }

    [Fact]
    public async Task ActivateCurrentAsync_Should_Update_Source_Pointer()
    {
        using var temp = new TempDirectory();
        var stackId = Guid.CreateVersion7();
        var snapshotRoot = Path.Combine(temp.Path, stackId.ToString("D"), "releases", Guid.CreateVersion7().ToString("D"), "source");
        Directory.CreateDirectory(snapshotRoot);
        File.WriteAllText(Path.Combine(snapshotRoot, "compose.yml"), "services: {}");

        var materializer = new GitStackMaterializer(
            Mock.Of<IRepoCacheManager>(),
            Mock.Of<IGitCliRepository>(),
            new TestStackStoragePathProvider(temp.Path));

        await materializer.ActivateCurrentAsync(stackId, snapshotRoot, TestContext.Current.CancellationToken);

        var stackRoot = Path.Combine(temp.Path, stackId.ToString("D"));
        var currentPath = Path.Combine(stackRoot, "current");
        var pointerPath = Path.Combine(stackRoot, "current.source");

        Assert.True(Directory.Exists(currentPath) || File.Exists(pointerPath));
        if (File.Exists(pointerPath))
        {
            Assert.Equal(snapshotRoot, await File.ReadAllTextAsync(pointerPath, TestContext.Current.CancellationToken));
        }
    }

    [Fact]
    public async Task PruneSnapshotsAsync_Should_Delete_Unretained_Release_Directories()
    {
        using var temp = new TempDirectory();
        var stackId = Guid.CreateVersion7();
        var retainedReleaseId = Guid.CreateVersion7();
        var staleReleaseId = Guid.CreateVersion7();
        var releasesRoot = Path.Combine(temp.Path, stackId.ToString("D"), "releases");
        var retainedPath = Path.Combine(releasesRoot, retainedReleaseId.ToString("D"));
        var stalePath = Path.Combine(releasesRoot, staleReleaseId.ToString("D"));
        Directory.CreateDirectory(retainedPath);
        Directory.CreateDirectory(stalePath);
        Directory.CreateDirectory(Path.Combine(releasesRoot, "not-a-guid"));

        var materializer = new GitStackMaterializer(
            Mock.Of<IRepoCacheManager>(),
            Mock.Of<IGitCliRepository>(),
            new TestStackStoragePathProvider(temp.Path));

        await materializer.PruneSnapshotsAsync(
            stackId,
            [retainedReleaseId],
            TestContext.Current.CancellationToken);

        Assert.True(Directory.Exists(retainedPath));
        Assert.False(Directory.Exists(stalePath));
        Assert.True(Directory.Exists(Path.Combine(releasesRoot, "not-a-guid")));
    }

    [Fact]
    public async Task DiscardSnapshotAsync_Should_Delete_Selected_Release_Directory()
    {
        using var temp = new TempDirectory();
        var stackId = Guid.CreateVersion7();
        var releaseId = Guid.CreateVersion7();
        var releasePath = Path.Combine(temp.Path, stackId.ToString("D"), "releases", releaseId.ToString("D"));
        Directory.CreateDirectory(Path.Combine(releasePath, "source"));
        File.WriteAllText(Path.Combine(releasePath, "source", "compose.yml"), "services: {}");

        var materializer = new GitStackMaterializer(
            Mock.Of<IRepoCacheManager>(),
            Mock.Of<IGitCliRepository>(),
            new TestStackStoragePathProvider(temp.Path));

        await materializer.DiscardSnapshotAsync(stackId, releaseId, TestContext.Current.CancellationToken);

        Assert.False(Directory.Exists(releasePath));
    }

    [Fact]
    public async Task MaterializeAsync_Should_Reject_Compose_Path_That_Escapes_Repository_Root()
    {
        var stack = CreateGitStack(["../compose.yml"], []);
        var repository = CreateRepository();
        var repoCache = new Mock<IRepoCacheManager>();
        var gitCli = new Mock<IGitCliRepository>();
        using var temp = new TempDirectory();

        repoCache
            .Setup(x => x.SynchronizeAsync(repository, repository.GitAccount, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, "abc123", true));

        gitCli
            .Setup(x => x.MaterializeSnapshotAsync(
                ApplicationStoragePaths.GetRepositoryCachePath(repository),
                "abc123",
                It.IsAny<string>(),
                It.IsAny<CancellationToken>()))
            .Callback<string, string, string, CancellationToken>((_, _, targetPath, _) => Directory.CreateDirectory(targetPath))
            .ReturnsAsync(Result.Success());

        var materializer = new GitStackMaterializer(repoCache.Object, gitCli.Object, new TestStackStoragePathProvider(temp.Path));

        var result = await materializer.MaterializeAsync(
            stack,
            (GitStack)stack.CurrentStackRelease!.Spec,
            repository,
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Contains("escapes the repository root", error.Message);
        Assert.False(Directory.Exists(Path.Combine(temp.Path, stack.Id.ToString("D"), "releases", stack.CurrentStackReleaseId.ToString("D"))));
    }

    [Fact]
    public async Task MaterializeAsync_Should_Remove_Snapshot_When_Validation_Fails_After_Materialization()
    {
        var stack = CreateGitStack(["missing.yml"], []);
        var repository = CreateRepository();
        var repoCache = new Mock<IRepoCacheManager>();
        var gitCli = new Mock<IGitCliRepository>();
        using var temp = new TempDirectory();

        repoCache
            .Setup(x => x.SynchronizeAsync(repository, repository.GitAccount, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, "abc123", true));

        gitCli
            .Setup(x => x.MaterializeSnapshotAsync(
                ApplicationStoragePaths.GetRepositoryCachePath(repository),
                "abc123",
                It.IsAny<string>(),
                It.IsAny<CancellationToken>()))
            .Callback<string, string, string, CancellationToken>((_, _, targetPath, _) =>
            {
                Directory.CreateDirectory(targetPath);
                File.WriteAllText(Path.Combine(targetPath, "other.yml"), "services: {}");
            })
            .ReturnsAsync(Result.Success());

        var materializer = new GitStackMaterializer(repoCache.Object, gitCli.Object, new TestStackStoragePathProvider(temp.Path));

        var result = await materializer.MaterializeAsync(
            stack,
            (GitStack)stack.CurrentStackRelease!.Spec,
            repository,
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Contains("does not exist", error.Message);
        Assert.False(Directory.Exists(Path.Combine(temp.Path, stack.Id.ToString("D"), "releases", stack.CurrentStackReleaseId.ToString("D"))));
    }

    [Fact]
    public async Task MaterializeAsync_Should_Accept_Root_Watch_Path()
    {
        var stack = CreateGitStack(["compose.yml"], [], watchPaths: ["."]);
        var repository = CreateRepository();
        var repoCache = new Mock<IRepoCacheManager>();
        var gitCli = new Mock<IGitCliRepository>();
        using var temp = new TempDirectory();

        repoCache
            .Setup(x => x.SynchronizeAsync(repository, repository.GitAccount, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, "abc123", true));

        gitCli
            .Setup(x => x.MaterializeSnapshotAsync(
                ApplicationStoragePaths.GetRepositoryCachePath(repository),
                "abc123",
                It.IsAny<string>(),
                It.IsAny<CancellationToken>()))
            .Callback<string, string, string, CancellationToken>((_, _, targetPath, _) =>
            {
                Directory.CreateDirectory(targetPath);
                File.WriteAllText(Path.Combine(targetPath, "compose.yml"), "services: {}");
            })
            .ReturnsAsync(Result.Success());

        var materializer = new GitStackMaterializer(repoCache.Object, gitCli.Object, new TestStackStoragePathProvider(temp.Path));

        var result = await materializer.MaterializeAsync(
            stack,
            (GitStack)stack.CurrentStackRelease!.Spec,
            repository,
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var payload, out var error), error?.Message);
        Assert.Equal(["."], payload.WatchPaths);
    }

    [Fact]
    public async Task MaterializeAsync_Should_Reject_Compose_Path_Through_Symlink_Escaping_Root()
    {
        using var temp = new TempDirectory();
        if (!CanCreateDirectorySymlink(temp.Path))
            return;

        var stack = CreateGitStack(["external/compose.yml"], []);
        var repository = CreateRepository();
        var repoCache = new Mock<IRepoCacheManager>();
        var gitCli = new Mock<IGitCliRepository>();
        var externalDirectory = Path.Combine(temp.Path, "external-source");
        Directory.CreateDirectory(externalDirectory);
        File.WriteAllText(Path.Combine(externalDirectory, "compose.yml"), "services: {}");

        repoCache
            .Setup(x => x.SynchronizeAsync(repository, repository.GitAccount, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, "abc123", true));

        gitCli
            .Setup(x => x.MaterializeSnapshotAsync(
                ApplicationStoragePaths.GetRepositoryCachePath(repository),
                "abc123",
                It.IsAny<string>(),
                It.IsAny<CancellationToken>()))
            .Callback<string, string, string, CancellationToken>((_, _, targetPath, _) =>
            {
                Directory.CreateDirectory(targetPath);
                Directory.CreateSymbolicLink(Path.Combine(targetPath, "external"), externalDirectory);
            })
            .ReturnsAsync(Result.Success());

        var materializer = new GitStackMaterializer(repoCache.Object, gitCli.Object, new TestStackStoragePathProvider(temp.Path));

        var result = await materializer.MaterializeAsync(
            stack,
            (GitStack)stack.CurrentStackRelease!.Spec,
            repository,
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Contains("points outside the repository root", error.Message);
    }

    private static bool CanCreateDirectorySymlink(string root)
    {
        var target = Path.Combine(root, $"symlink-target-{Guid.NewGuid():N}");
        var link = Path.Combine(root, $"symlink-link-{Guid.NewGuid():N}");
        Directory.CreateDirectory(target);

        try
        {
            Directory.CreateSymbolicLink(link, target);
            return true;
        }
        catch
        {
            return false;
        }
        finally
        {
            if (Directory.Exists(link))
                Directory.Delete(link);

            if (Directory.Exists(target))
                Directory.Delete(target, recursive: true);
        }
    }

    private static Stack CreateGitStack(List<string> composePaths, List<string> envFiles, List<string>? watchPaths = null)
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
                AdditionalEnvFileFromRepo: envFiles,
                WatchPaths: watchPaths));
    }

    private static GitRepository CreateRepository()
        => new(
            name: "repo",
            description: null,
            url: "https://github.com/org/repo",
            defaultBranch: "main",
            gitAccountId: null,
            createdByActorId: Guid.CreateVersion7());

    private sealed class TestStackStoragePathProvider(string path) : IStackStoragePathProvider
    {
        public string StacksRoot { get; } = path;
    }

    private sealed class TempDirectory : IDisposable
    {
        public string Path { get; } = System.IO.Path.Combine(System.IO.Path.GetTempPath(), Guid.NewGuid().ToString("N"));

        public TempDirectory()
        {
            Directory.CreateDirectory(Path);
        }

        public void Dispose()
        {
            if (Directory.Exists(Path))
                Directory.Delete(Path, recursive: true);
        }
    }
}
