using Domain;
using Domain.Contracts.Resources.Git;
using Domain.Entities.Activities;
using Domain.Entities.Git;
using Hosting.Common.MergePatch;
using System.Text.Json;

namespace Tests.Unit.Domain.Entities.Git;

public class GitRepositoryTests
{
    [Theory]
    [InlineData("https://github.com/citadel-p/citadel.git", "/app/data/repos/citadel")]
    [InlineData("https://github.com/citadel-p/citadel", "/app/data/repos/citadel")]
    [InlineData("git@github.com:citadel-p/citadel.git", "/app/data/repos/citadel")]
    public void GetCachePath_UsesRemoteRepositoryName(string url, string expectedPath)
    {
        var repository = new GitRepository(
            name: "GR-TEST",
            description: null,
            url: url,
            defaultBranch: "main",
            gitAccountId: null,
            createdByActorId: Guid.NewGuid());

        var cachePath = repository.GetCachePath();

        Assert.Equal(expectedPath, cachePath);
    }

    [Fact]
    public void GetCachePath_FallsBackToSanitizedName_WhenUrlDoesNotContainRepositorySegment()
    {
        var repository = new GitRepository(
            name: "My Repo",
            description: null,
            url: string.Empty,
            defaultBranch: "main",
            gitAccountId: null,
            createdByActorId: Guid.NewGuid());

        var cachePath = repository.GetCachePath();

        Assert.Equal("/app/data/repos/My-Repo", cachePath);
    }

    [Fact]
    public void ToSnapshot_IncludesResolvedCommitSha_WhenProvided()
    {
        var repository = CreateRepository();

        var snapshot = repository.ToSnapshot(resolvedCommitSha: "abc123");

        Assert.Equal("abc123", snapshot.ResolvedCommitSha);
    }

    [Fact]
    public void GitRepoClonedActivity_SerializesResolvedCommitShaInRepositorySnapshot()
    {
        var repository = CreateRepository();
        var info = new GitRepoCloned(
            repository.ToSnapshot(resolvedCommitSha: "abc123"),
            new RepoSyncResultSnapshot(CommitSha: "abc123"));

        var json = JsonSerializer.Serialize<ActivityEventInfo>(info, EventInfoJsonContext.Default.ActivityEventInfo);

        Assert.Contains("\"ResolvedCommitSha\":\"abc123\"", json);
        Assert.Contains("\"CommitSha\":\"abc123\"", json);
    }

    [Theory]
    [InlineData(GitRepositorySyncMode.PullInterval, 10, 10)]
    [InlineData(GitRepositorySyncMode.PullInterval, 0, 1)]
    [InlineData(GitRepositorySyncMode.PullInterval, null, 5)]
    [InlineData(GitRepositorySyncMode.Manual, 10, null)]
    public void UpdateSyncPolicy_NormalizesSyncMode(
        GitRepositorySyncMode syncMode,
        int? syncIntervalMinutes,
        int? expectedSyncIntervalMinutes)
    {
        var repository = new GitRepository(
            name: "GR-TEST",
            description: null,
            url: "https://github.com/citadel-p/citadel",
            defaultBranch: "main",
            gitAccountId: null,
            createdByActorId: Guid.NewGuid(),
            webhook: new RepoWebhookConfig(Enabled: true, Secret: "secret"));

        repository.UpdateSyncPolicy(syncMode, syncIntervalMinutes);

        Assert.Equal(syncMode, repository.SyncMode);
        Assert.Equal(expectedSyncIntervalMinutes, repository.SyncIntervalMinutes);
        Assert.True(repository.Webhook?.Enabled);
        Assert.Equal("secret", repository.Webhook?.Secret);
    }

    [Fact]
    public void ApplyMergePatch_PreservesPatchedSyncPolicy()
    {
        var repository = CreateRepository();
        var patch = JsonMergePatchDocument<GitRepository>.FromJson("""
        {
          "syncMode": "PullInterval",
          "syncIntervalMinutes": 10
        }
        """);

        var patched = patch.ApplyTo(repository, GitJsonContext.Default.GitRepository);

        Assert.Equal(GitRepositorySyncMode.PullInterval, patched.SyncMode);
        Assert.Equal(10, patched.SyncIntervalMinutes);
    }

    private static GitRepository CreateRepository()
        => new(
            name: "GR-TEST",
            description: null,
            url: "https://github.com/citadel-p/citadel",
            defaultBranch: "main",
            gitAccountId: null,
            createdByActorId: Guid.NewGuid());
}
