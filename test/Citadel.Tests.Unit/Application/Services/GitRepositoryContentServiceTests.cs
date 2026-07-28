using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Git;
using Domain.Entities.Git;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.Logging.Abstractions;
using Moq;

namespace Tests.Unit.Application.Services;

public sealed class GitRepositoryContentServiceTests
{
    private const string CommitSha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    [Fact]
    public async Task ListDirectoryAsync_Should_Not_Fall_Back_When_Stored_Commit_Is_Missing()
    {
        using var context = new BrowserTestContext();
        context.SetStoredRef(CommitSha);
        context.Git
            .Setup(x => x.ResolveCommitAsync(context.CachePath, CommitSha, It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Failure<string>("fatal: bad object"));

        var result = await context.Service.ListDirectoryAsync(
            context.Repository.Id,
            commitSha: null,
            path: null,
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error, out _));
        Assert.IsType<NotFoundError>(error);
        context.Git.Verify(
            x => x.ResolveSnapshotCommitAsync(It.IsAny<string>(), It.IsAny<string>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task ListDirectoryAsync_Should_Return_Conflict_For_Invalid_Cache()
    {
        using var context = new BrowserTestContext();
        context.SetStoredRef(CommitSha);
        context.Git
            .Setup(x => x.ResolveCommitAsync(context.CachePath, CommitSha, It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Failure<string>("fatal: not a git repository"));

        var result = await context.Service.ListDirectoryAsync(
            context.Repository.Id,
            commitSha: null,
            path: null,
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error, out _));
        Assert.IsType<ConflictError>(error);
    }

    [Fact]
    public async Task ReadFileAsync_Should_Return_Metadata_Without_Decoding_Binary_Content()
    {
        using var context = new BrowserTestContext();
        context.Git
            .Setup(x => x.ResolveCommitAsync(context.CachePath, CommitSha, It.IsAny<CancellationToken>()))
            .ReturnsAsync(CommitSha);
        context.Git
            .Setup(x => x.GetTreeEntryAsync(
                context.CachePath,
                CommitSha,
                "asset.bin",
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(new GitTreeEntry(
                "asset.bin",
                "asset.bin",
                GitRepositoryEntryType.File,
                3,
                "100644",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"));
        context.Git
            .Setup(x => x.ReadBlobAsync(
                context.CachePath,
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                1024 * 1024,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(new GitBlob(new byte[] { (byte)'A', 0, (byte)'B' }, IsTruncated: false));

        var result = await context.Service.ReadFileAsync(
            context.Repository.Id,
            CommitSha,
            "asset.bin",
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var file, out var error), error?.Message);
        Assert.True(file.IsBinary);
        Assert.Null(file.Content);
        Assert.Equal("Binary files cannot be previewed.", file.PreviewUnavailableReason);
    }

    [Fact]
    public async Task ListDirectoryAsync_Should_Return_Concrete_Commit_For_Legacy_Cache()
    {
        using var context = new BrowserTestContext();
        context.Repositories
            .Setup(x => x.GetRefAsync(context.Repository.Id, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync((GitRepositoryRef?)null);
        context.Git
            .Setup(x => x.ResolveSnapshotCommitAsync(context.CachePath, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(CommitSha);
        context.Git
            .Setup(x => x.ResolveCommitAsync(context.CachePath, CommitSha, It.IsAny<CancellationToken>()))
            .ReturnsAsync(CommitSha);
        context.Git
            .Setup(x => x.ListTreeAsync(
                context.CachePath,
                CommitSha,
                string.Empty,
                1000,
                8 * 1024 * 1024,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(new GitTreeListing([], IsTruncated: false));

        var result = await context.Service.ListDirectoryAsync(
            context.Repository.Id,
            commitSha: null,
            path: null,
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var listing, out var error), error?.Message);
        Assert.Equal(CommitSha, listing.CommitSha);
    }

    private sealed class BrowserTestContext : IDisposable
    {
        private readonly Mock<IUnitOfWork> unitOfWork = new(MockBehavior.Strict);

        public BrowserTestContext()
        {
            Repository = new GitRepository(
                name: $"browser-{Guid.NewGuid():N}",
                description: null,
                url: "https://github.com/citadel/test.git",
                defaultBranch: "main",
                gitAccountId: null,
                createdByActorId: Guid.CreateVersion7());
            CachePath = ApplicationStoragePaths.GetRepositoryCachePath(Repository);
            Directory.CreateDirectory(Path.Combine(CachePath, ".git"));

            unitOfWork
                .SetupGet(x => x.GitRepositories)
                .Returns(Repositories.Object);
            Repositories
                .Setup(x => x.GetAsync(Repository.Id, It.IsAny<CancellationToken>()))
                .ReturnsAsync(Repository);

            Service = new GitRepositoryContentService(
                unitOfWork.Object,
                Git.Object,
                new GitRepositoryPathNormalizer(),
                NullLogger<GitRepositoryContentService>.Instance);
        }

        public GitRepository Repository { get; }
        public string CachePath { get; }
        public Mock<IGitReposRepository> Repositories { get; } = new(MockBehavior.Strict);
        public Mock<IGitCliRepository> Git { get; } = new(MockBehavior.Strict);
        public GitRepositoryContentService Service { get; }

        public void SetStoredRef(string commitSha)
        {
            Repositories
                .Setup(x => x.GetRefAsync(Repository.Id, "main", It.IsAny<CancellationToken>()))
                .ReturnsAsync(new GitRepositoryRef(
                    Repository.Id,
                    "main",
                    commitSha,
                    GitReposStatus.Healthy));
        }

        public void Dispose()
        {
            if (Directory.Exists(CachePath))
                Directory.Delete(CachePath, recursive: true);
        }
    }
}
