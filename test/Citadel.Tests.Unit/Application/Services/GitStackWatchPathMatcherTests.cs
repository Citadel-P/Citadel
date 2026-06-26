using Application.Services;
using Domain;
using Domain.Entities.Stacks;

namespace Tests.Unit.Application.Services;

public class GitStackWatchPathMatcherTests
{
    [Fact]
    public void HasRelevantChanges_Should_Match_Default_Working_Directory()
    {
        var spec = CreateSpec(
            composePaths: ["stacks/beszel/compose.yml"],
            workingDirectory: "stacks/beszel");
        var source = CreateSource(
            composePaths: ["stacks/beszel/compose.yml"],
            workingDirectory: "stacks/beszel");

        var result = GitStackWatchPathMatcher.HasRelevantChanges(
            spec,
            source,
            ["stacks/beszel/config/config.yml"]);

        Assert.True(result);
    }

    [Fact]
    public void HasRelevantChanges_Should_Ignore_Unrelated_Monorepo_Path()
    {
        var spec = CreateSpec(
            composePaths: ["stacks/beszel/compose.yml"],
            workingDirectory: "stacks/beszel");
        var source = CreateSource(
            composePaths: ["stacks/beszel/compose.yml"],
            workingDirectory: "stacks/beszel");

        var result = GitStackWatchPathMatcher.HasRelevantChanges(
            spec,
            source,
            ["stacks/caddy/compose.yml"]);

        Assert.False(result);
    }

    [Fact]
    public void HasRelevantChanges_Should_Derive_Working_Directory_From_First_Compose_Path()
    {
        var spec = CreateSpec(
            composePaths: ["stacks/beszel/compose.yml"],
            workingDirectory: null);
        var source = CreateSource(
            composePaths: ["stacks/beszel/compose.yml"],
            workingDirectory: null);

        Assert.True(GitStackWatchPathMatcher.HasRelevantChanges(spec, source, ["stacks/beszel/config.yml"]));
        Assert.False(GitStackWatchPathMatcher.HasRelevantChanges(spec, source, ["stacks/caddy/config.yml"]));
    }

    [Fact]
    public void HasRelevantChanges_Should_Use_Explicit_WatchPaths_When_Configured()
    {
        var spec = CreateSpec(
            composePaths: ["stacks/beszel/compose.yml"],
            workingDirectory: "stacks/beszel",
            watchPaths: ["shared/**"]);
        var source = CreateSource(
            composePaths: ["stacks/beszel/compose.yml"],
            workingDirectory: "stacks/beszel");

        Assert.True(GitStackWatchPathMatcher.HasRelevantChanges(spec, source, ["shared/networks.yml"]));
        Assert.False(GitStackWatchPathMatcher.HasRelevantChanges(spec, source, ["stacks/beszel/compose.yml"]));
    }

    private static GitStack CreateSpec(
        List<string> composePaths,
        string? workingDirectory,
        List<string>? watchPaths = null)
        => new(
            GitRepoId: Guid.CreateVersion7(),
            Branch: "main",
            CommitSha: null,
            UpdateBehavior: StackUpdateBehavior.Notify,
            ComposePaths: composePaths,
            WorkingDirectory: workingDirectory,
            WatchPaths: watchPaths);

    private static StackReleaseSource CreateSource(List<string> composePaths, string? workingDirectory)
        => new(
            SourceType: StackSource.Git,
            GitRepositoryId: Guid.CreateVersion7(),
            GitRepositoryName: "repo",
            Branch: "main",
            RequestedCommitSha: null,
            ResolvedCommitSha: "abc123",
            ComposePaths: composePaths,
            EnvFilePaths: [],
            WorkingDirectory: workingDirectory);
}
