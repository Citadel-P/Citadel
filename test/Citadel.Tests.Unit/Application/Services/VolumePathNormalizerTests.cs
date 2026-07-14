using Application.Services;

namespace Tests.Unit.Application.Services;

public sealed class VolumePathNormalizerTests
{
    private readonly VolumePathNormalizer normalizer = new();

    [Theory]
    [InlineData(null, "/")]
    [InlineData("", "/")]
    [InlineData("/", "/")]
    [InlineData("/config", "/config")]
    [InlineData("/config/appsettings.json", "/config/appsettings.json")]
    public void Normalize_ShouldReturnCanonicalPath(string? input, string expected)
    {
        var result = normalizer.Normalize(input);

        Assert.True(result.IsSuccess(out var path, out var error), error?.Message);
        Assert.Equal(expected, path.ApiPath);
    }

    [Theory]
    [InlineData("config")]
    [InlineData("/config/")]
    [InlineData("/config//appsettings.json")]
    [InlineData("/config/./appsettings.json")]
    [InlineData("/config/../secret")]
    [InlineData("/config\\appsettings.json")]
    public void Normalize_ShouldRejectUnsafePath(string input)
    {
        var result = normalizer.Normalize(input);

        Assert.True(result.IsFailure(out var error));
        Assert.NotNull(error);
    }
}
