using Application.Services;

namespace Tests.Unit.Application.Services;

public class GitRepositoryUrlSanitizerTests
{
    [Fact]
    public void Sanitize_Should_Remove_UserInfo_From_Absolute_Repository_Url()
    {
        var sanitized = GitRepositoryUrlSanitizer.Sanitize("https://user:token@example.com/org/repo");

        Assert.Equal("https://example.com/org/repo", sanitized);
    }

    [Fact]
    public void Sanitize_Should_Keep_Ssh_Scp_Style_Repository_Url()
    {
        var sanitized = GitRepositoryUrlSanitizer.Sanitize("git@example.com:org/repo");

        Assert.Equal("git@example.com:org/repo", sanitized);
    }
}
