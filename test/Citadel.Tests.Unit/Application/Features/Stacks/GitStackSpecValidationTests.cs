using Application.Features.Stacks.Commands;
using Domain;
using Domain.Entities.Stacks;

namespace Tests.Unit.Application.Features.Stacks;

public class GitStackSpecValidationTests
{
    [Fact]
    public void Validate_Should_Accept_Monorepo_GitStack()
    {
        var spec = CreateSpec(
            composePaths: ["stacks/beszel/compose.yml", "stacks/beszel/compose.prod.yml"],
            workingDirectory: "stacks/beszel",
            envFiles: ["stacks/beszel/.env"],
            watchPaths: ["stacks/beszel/**", "shared/networks.yml"]);

        var error = GitStackSpecValidation.Validate(spec);

        Assert.Null(error);
    }

    [Fact]
    public void Validate_Should_Reject_Empty_Compose_Path_List()
    {
        var spec = CreateSpec(composePaths: []);

        var error = GitStackSpecValidation.Validate(spec);

        Assert.Equal("At least one compose path is required for Git stacks.", error);
    }

    [Theory]
    [InlineData("../compose.yml", "Compose path '../compose.yml' escapes the repository root.")]
    [InlineData("/etc/passwd", "Compose path '/etc/passwd' must be relative to the repository root.")]
    [InlineData("C:/repo/compose.yml", "Compose path 'C:/repo/compose.yml' must be relative to the repository root.")]
    public void Validate_Should_Reject_Invalid_Compose_Path(string composePath, string expected)
    {
        var spec = CreateSpec(composePaths: [composePath]);

        var error = GitStackSpecValidation.Validate(spec);

        Assert.Equal(expected, error);
    }

    [Fact]
    public void Validate_Should_Reject_Working_Directory_That_Escapes_Root()
    {
        var spec = CreateSpec(workingDirectory: "../app");

        var error = GitStackSpecValidation.Validate(spec);

        Assert.Equal("Working directory '../app' escapes the repository root.", error);
    }

    [Fact]
    public void Validate_Should_Reject_Watch_Path_That_Escapes_Root()
    {
        var spec = CreateSpec(watchPaths: ["../shared/**"]);

        var error = GitStackSpecValidation.Validate(spec);

        Assert.Equal("Watch path '../shared/**' escapes the repository root.", error);
    }

    private static GitStack CreateSpec(
        List<string>? composePaths = null,
        string? workingDirectory = null,
        List<string>? envFiles = null,
        List<string>? watchPaths = null)
        => new(
            GitRepoId: Guid.CreateVersion7(),
            Branch: "main",
            CommitSha: null,
            UpdateBehavior: StackUpdateBehavior.Notify,
            ComposePaths: composePaths ?? ["compose.yml"],
            WorkingDirectory: workingDirectory,
            ComposeEnvFilesFromRepo: envFiles,
            WatchPaths: watchPaths);
}
