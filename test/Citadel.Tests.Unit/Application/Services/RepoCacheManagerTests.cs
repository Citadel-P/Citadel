using Application.Services;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Microsoft.Extensions.Logging.Abstractions;
using Moq;

namespace Tests.Unit.Application.Services;

public sealed class RepoCacheManagerTests
{
    [Fact]
    public void ResolveHookWorkingDirectory_RejectsSiblingWithSharedPrefix()
    {
        var root = Path.Combine(Path.GetTempPath(), $"repo-{Guid.NewGuid():N}");
        var sibling = Path.Combine("..", Path.GetFileName(root) + "-other");

        var result = RepoCacheManager.ResolveHookWorkingDirectory(root, sibling);

        Assert.True(result.IsFailure());
    }

    [Fact]
    public void ResolveHookWorkingDirectory_AllowsRepositoryRootAndChildren()
    {
        var root = Path.Combine(Path.GetTempPath(), $"repo-{Guid.NewGuid():N}");

        var rootResult = RepoCacheManager.ResolveHookWorkingDirectory(root, ".");
        var childResult = RepoCacheManager.ResolveHookWorkingDirectory(
            root,
            Path.Combine("scripts", "release"));

        Assert.True(rootResult.IsSuccess(out var resolvedRoot, out _));
        Assert.Equal(Path.GetFullPath(root), resolvedRoot);
        Assert.True(childResult.IsSuccess(out var resolvedChild, out _));
        Assert.Equal(
            Path.GetFullPath(Path.Combine(root, "scripts", "release")),
            resolvedChild);
    }

    [Fact]
    public void ResolveHookWorkingDirectory_RejectsSymlinkEscapingRepository()
    {
        var temp = Path.Combine(Path.GetTempPath(), $"repo-hook-{Guid.NewGuid():N}");
        var root = Path.Combine(temp, "repository");
        var external = Path.Combine(temp, "external");
        var link = Path.Combine(root, "linked");
        Directory.CreateDirectory(root);
        Directory.CreateDirectory(external);

        try
        {
            try
            {
                Directory.CreateSymbolicLink(link, external);
            }
            catch
            {
                return;
            }

            var result = RepoCacheManager.ResolveHookWorkingDirectory(
                root,
                Path.Combine("linked", "scripts"));

            Assert.True(result.IsFailure());
        }
        finally
        {
            if (Directory.Exists(temp))
                Directory.Delete(temp, recursive: true);
        }
    }

    [Fact]
    public async Task DeleteCacheAsync_ShouldReleasePerRepositoryLockEntries()
    {
        var manager = new RepoCacheManager(
            Mock.Of<IGitCliRepository>(),
            NullLogger<RepoCacheManager>.Instance);

        for (var index = 0; index < 250; index++)
        {
            var repository = new GitRepository(
                name: $"lock-release-{index}",
                description: null,
                url: "https://example.invalid/repository.git",
                defaultBranch: "main",
                gitAccountId: null,
                createdByActorId: Guid.CreateVersion7());

            await manager.DeleteCacheAsync(
                repository,
                TestContext.Current.CancellationToken);
        }

        Assert.Equal(0, manager.ActiveLockCount);
    }
}
