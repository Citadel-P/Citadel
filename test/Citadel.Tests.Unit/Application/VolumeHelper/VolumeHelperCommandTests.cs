using System.Formats.Tar;
using System.Text.Json;
using Hosting.DockerClient.VolumeHelper;

namespace Tests.Unit.Application.VolumeHelper;

public sealed class VolumeHelperCommandTests : IDisposable
{
    private readonly string tempDirectory = Path.Combine(Path.GetTempPath(), $"citadel-volume-helper-tests-{Guid.NewGuid():N}");

    public VolumeHelperCommandTests()
    {
        Directory.CreateDirectory(tempDirectory);
    }

    public void Dispose()
    {
        if (Directory.Exists(tempDirectory))
            Directory.Delete(tempDirectory, recursive: true);
    }

    [Fact]
    public async Task List_ShouldReturnDirectoryEntries()
    {
        Directory.CreateDirectory(Path.Combine(tempDirectory, "config"));
        await File.WriteAllTextAsync(Path.Combine(tempDirectory, "config", "appsettings.json"), "{}", TestContext.Current.CancellationToken);

        await using var stdout = new MemoryStream();
        using var stderr = new StringWriter();

        var exitCode = await VolumeHelperCommand.RunAsync(
            [
                "list",
                "--root", tempDirectory,
                "--path", "/",
                "--max-entries", "100",
                "--max-payload-bytes", "1048576"
            ],
            stdout,
            stderr,
            TestContext.Current.CancellationToken);

        stdout.Position = 0;
        var response = await JsonSerializer.DeserializeAsync<VolumeHelperListResponse>(
            stdout,
            cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(0, exitCode);
        Assert.NotNull(response);
        Assert.Null(response.ErrorCode);
        var entry = Assert.Single(response.Entries);
        Assert.Equal("config", entry.Name);
        Assert.Equal("/config", entry.Path);
        Assert.Equal("directory", entry.Type);
    }

    [Fact]
    public async Task Inspect_ShouldReturnStructuredErrorForMissingPath()
    {
        await using var stdout = new MemoryStream();
        using var stderr = new StringWriter();

        var exitCode = await VolumeHelperCommand.RunAsync(
            ["inspect", "--root", tempDirectory, "--path", "/missing"],
            stdout,
            stderr,
            TestContext.Current.CancellationToken);

        stdout.Position = 0;
        var response = await JsonSerializer.DeserializeAsync<VolumeHelperInspectResponse>(
            stdout,
            cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(0, exitCode);
        Assert.NotNull(response);
        Assert.False(response.Exists);
        Assert.Equal("VolumePathNotFound", response.ErrorCode);
    }

    [Fact]
    public async Task StreamFile_ShouldWriteRawBytes()
    {
        var payload = new byte[] { 0, 1, 2, 255 };
        await File.WriteAllBytesAsync(Path.Combine(tempDirectory, "payload.bin"), payload, TestContext.Current.CancellationToken);
        await using var stdout = new MemoryStream();
        using var stderr = new StringWriter();

        var exitCode = await VolumeHelperCommand.RunAsync(
            ["stream-file", "--root", tempDirectory, "--path", "/payload.bin"],
            stdout,
            stderr,
            TestContext.Current.CancellationToken);

        Assert.Equal(0, exitCode);
        Assert.Equal(payload, stdout.ToArray());
    }

    [Fact]
    public async Task StreamDirectory_ShouldWriteTarArchive()
    {
        Directory.CreateDirectory(Path.Combine(tempDirectory, "config"));
        await File.WriteAllTextAsync(
            Path.Combine(tempDirectory, "config", "appsettings.json"),
            "{}",
            TestContext.Current.CancellationToken);

        await using var stdout = new MemoryStream();
        using var stderr = new StringWriter();

        var exitCode = await VolumeHelperCommand.RunAsync(
            ["stream-directory", "--root", tempDirectory, "--path", "/config"],
            stdout,
            stderr,
            TestContext.Current.CancellationToken);

        Assert.Equal(0, exitCode);

        stdout.Position = 0;
        await using var reader = new TarReader(stdout);
        var entries = new List<string>();
        TarEntry? entry;
        while ((entry = await reader.GetNextEntryAsync(copyData: false, cancellationToken: TestContext.Current.CancellationToken)) is not null)
        {
            entries.Add(entry.Name);
        }

        Assert.Contains("config/", entries);
        Assert.Contains("config/appsettings.json", entries);
    }
}
