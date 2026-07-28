using Application.Services;

namespace Tests.Unit.Application.Services;

public sealed class GitRepositoryPathNormalizerTests
{
    private readonly GitRepositoryPathNormalizer normalizer = new();

    [Theory]
    [InlineData("", "")]
    [InlineData("./deploy//compose.yaml/", "deploy/compose.yaml")]
    [InlineData(" directory / file ", " directory / file ")]
    public void NormalizeDirectory_Should_Normalize_Without_Trimming_Filenames(string input, string expected)
    {
        var result = normalizer.NormalizeDirectory(input);

        Assert.True(result.IsSuccess(out var path, out var error), error?.Message);
        Assert.Equal(expected, path);
    }

    [Theory]
    [InlineData("../compose.yaml")]
    [InlineData("deploy/../compose.yaml")]
    [InlineData("/compose.yaml")]
    [InlineData("C:/compose.yaml")]
    [InlineData(".git/config")]
    [InlineData("deploy\\compose.yaml")]
    public void NormalizeFile_Should_Reject_Unsafe_Paths(string input)
    {
        var result = normalizer.NormalizeFile(input);

        Assert.True(result.IsFailure());
    }

    [Fact]
    public void NormalizeFile_Should_Preserve_Leading_And_Trailing_Whitespace()
    {
        var result = normalizer.NormalizeFile(" config/ compose.yaml ");

        Assert.True(result.IsSuccess(out var path, out var error), error?.Message);
        Assert.Equal(" config/ compose.yaml ", path);
    }

}
