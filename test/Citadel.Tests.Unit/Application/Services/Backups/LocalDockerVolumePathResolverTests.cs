using Application.Services.Backups;

namespace Tests.Unit.Application.Services.Backups;

public sealed class LocalDockerVolumePathResolverTests : IDisposable
{
    private readonly string tempRoot = Path.Combine(
        Path.GetTempPath(),
        $"citadel-volume-path-{Guid.NewGuid():N}");

    [Fact]
    public void TryResolve_ShouldUseDirectlyAccessibleMountpoint()
    {
        var mountpoint = Path.Combine(tempRoot, "direct");
        Directory.CreateDirectory(mountpoint);

        var resolved = LocalDockerVolumePathResolver.TryResolve(
            mountpoint,
            Path.Combine(tempRoot, "host"),
            out var path);

        Assert.True(resolved);
        Assert.Equal(Path.GetFullPath(mountpoint), path);
    }

    [Fact]
    public void TryResolve_ShouldMapDockerMountpointUnderHostRoot()
    {
        const string dockerMountpoint =
            "/var/lib/docker/volumes/example/_data";
        var hostRoot = Path.Combine(tempRoot, "host");
        var mappedMountpoint = Path.Combine(
            hostRoot,
            "var",
            "lib",
            "docker",
            "volumes",
            "example",
            "_data");
        Directory.CreateDirectory(mappedMountpoint);

        var resolved = LocalDockerVolumePathResolver.TryResolve(
            dockerMountpoint,
            hostRoot,
            out var path);

        Assert.True(resolved);
        Assert.Equal(Path.GetFullPath(mappedMountpoint), path);
    }

    [Theory]
    [InlineData("var/lib/docker/volumes/example/_data")]
    [InlineData("/var/lib/../etc")]
    [InlineData("/var\\lib\\docker")]
    [InlineData("/var/lib/docker\0escape")]
    public void TryResolve_ShouldRejectUnsafeDockerMountpoint(
        string dockerMountpoint)
    {
        var hostRoot = Path.Combine(tempRoot, "host");
        Directory.CreateDirectory(hostRoot);

        var resolved = LocalDockerVolumePathResolver.TryResolve(
            dockerMountpoint,
            hostRoot,
            out _);

        Assert.False(resolved);
    }

    public void Dispose()
    {
        if (Directory.Exists(tempRoot))
            Directory.Delete(tempRoot, recursive: true);
    }
}
