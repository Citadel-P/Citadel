using Domain.Entities.Git;

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
}
