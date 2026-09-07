using System.Formats.Tar;
using System.Text.Json;
using Hosting.DockerClient.VolumeHelper;

namespace Tests.Unit.Application.VolumeHelper;

public sealed class VolumeHelperCommandTests : IDisposable
{
    public static bool IsLinux => OperatingSystem.IsLinux();

    [Fact(SkipUnless = nameof(IsLinux), Skip = "Requires Linux descriptor-relative filesystem operations.")]
    public async Task StreamDirectory_ShouldStayOnPinnedDirectory_WhenPathIsReplacedBySymlink()
    {
        var volume = Path.Combine(tempDirectory, "volume");
        var original = Directory.CreateDirectory(Path.Combine(volume, "inside")).FullName;
        var outside = Directory.CreateDirectory(Path.Combine(tempDirectory, "outside")).FullName;
        await File.WriteAllTextAsync(Path.Combine(original, "payload"), "safe", TestContext.Current.CancellationToken);
        await File.WriteAllTextAsync(Path.Combine(outside, "payload"), "outside-secret", TestContext.Current.CancellationToken);
        using var stderr = new StringWriter();
        await using var stdout = new FirstWriteStream(() =>
        {
            Directory.Move(original, Path.Combine(volume, "pinned"));
            Directory.CreateSymbolicLink(original, outside);
        });
        var result = await VolumeHelperCommand.RunAsync(["stream-directory", "--root", volume, "--path", "/inside"], stdout, stderr, TestContext.Current.CancellationToken);
        Assert.Equal(0, result);
        stdout.Position = 0;
        await using var reader = new TarReader(stdout);
        var contents = new List<string>();
        while (await reader.GetNextEntryAsync(cancellationToken: TestContext.Current.CancellationToken) is { } entry)
        {
            if (entry.DataStream is null) continue;
            using var text = new StreamReader(entry.DataStream);
            contents.Add(await text.ReadToEndAsync(TestContext.Current.CancellationToken));
        }
        Assert.Equal(["safe"], contents);
    }

    [Theory(SkipUnless = nameof(IsLinux), Skip = "Requires Linux descriptor-relative filesystem operations.")]
    [InlineData("/../outside/payload")]
    [InlineData("/link/payload")]
    public async Task StreamFile_ShouldRejectTraversalAndSymlinkParents(string path)
    {
        var volume = Directory.CreateDirectory(Path.Combine(tempDirectory, "volume")).FullName;
        var outside = Directory.CreateDirectory(Path.Combine(tempDirectory, "outside")).FullName;
        await File.WriteAllTextAsync(Path.Combine(outside, "payload"), "outside-secret", TestContext.Current.CancellationToken);
        Directory.CreateSymbolicLink(Path.Combine(volume, "link"), outside);
        await using var stdout = new MemoryStream();
        using var stderr = new StringWriter();
        var result = await VolumeHelperCommand.RunAsync(["stream-file", "--root", volume, "--path", path], stdout, stderr, TestContext.Current.CancellationToken);
        Assert.NotEqual(0, result);
        Assert.Equal(0, stdout.Length);
    }

    private sealed class FirstWriteStream(Action firstWrite) : MemoryStream
    {
        private Action? callback = firstWrite;
        private void BeforeWrite() => Interlocked.Exchange(ref callback, null)?.Invoke();
        public override void Write(byte[] buffer, int offset, int count) { BeforeWrite(); base.Write(buffer, offset, count); }
        public override void Write(ReadOnlySpan<byte> buffer) { BeforeWrite(); base.Write(buffer); }
        public override ValueTask WriteAsync(ReadOnlyMemory<byte> buffer, CancellationToken cancellationToken = default)
        { BeforeWrite(); return base.WriteAsync(buffer, cancellationToken); }
    }

    [Fact(SkipUnless = nameof(IsLinux), Skip = "Requires Linux FIFO and descriptor semantics.")]
    public async Task StreamFile_ShouldRejectFifoWithoutBlockingOrLeakingDescriptors()
    {
        var fifo = Path.Combine(tempDirectory, "fifo");
        var start = new System.Diagnostics.ProcessStartInfo("mkfifo") { UseShellExecute = false };
        start.ArgumentList.Add(fifo);
        using var process = System.Diagnostics.Process.Start(start)!;
        await process.WaitForExitAsync(TestContext.Current.CancellationToken);
        Assert.Equal(0, process.ExitCode);
        var before = Directory.GetFiles("/proc/self/fd").Length;
        for (var attempt = 0; attempt < 20; attempt++)
        {
            await using var stdout = new MemoryStream();
            using var stderr = new StringWriter();
            var result = await Task.Run(() => VolumeHelperCommand.RunAsync(["stream-file", "--root", tempDirectory, "--path", "/fifo"], stdout, stderr, TestContext.Current.CancellationToken), TestContext.Current.CancellationToken)
                .WaitAsync(TimeSpan.FromSeconds(2), TestContext.Current.CancellationToken);
            Assert.NotEqual(0, result);
            Assert.Equal(0, stdout.Length);
        }
        Assert.InRange(Directory.GetFiles("/proc/self/fd").Length, 0, before + 4);
    }

    [Fact]
    public async Task Idle_ShouldStopWhenCancelledWithoutWritingOutput()
    {
        using var cancellation = new CancellationTokenSource();
        cancellation.Cancel();
        await using var stdout = new MemoryStream();
        using var stderr = new StringWriter();
        var exitCode = await VolumeHelperCommand.RunAsync(["idle"], stdout, stderr, cancellation.Token);
        Assert.Equal(0, exitCode);
        Assert.Equal(0, stdout.Length);
        Assert.Equal(string.Empty, stderr.ToString());
    }

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
