using System.Formats.Tar;
using System.Text;
using Hosting.DockerClient.Services;

namespace Tests.Unit.DockerClient;

public sealed class BuildImageServiceTests
{
    [Fact]
    public async Task ChunkedReadStream_ShouldDecodeDockerApiChunkedJsonStream()
    {
        await using var source = new MemoryStream(Encoding.ASCII.GetBytes(
            "7\r\n{\"a\":1}\r\n8\r\n{\"b\":2}\n\r\n0\r\n\r\n"));
        await using var stream = new ChunkedReadStream(source);

        using var reader = new StreamReader(stream, Encoding.UTF8);
        var decoded = await reader.ReadToEndAsync(TestContext.Current.CancellationToken);

        Assert.Equal("{\"a\":1}{\"b\":2}\n", decoded);
    }

    [Fact]
    public async Task BuildContextArchive_ShouldIncludeEmptyDirectoriesAndApplyDockerIgnore()
    {
        using var temp = new TempDirectory();
        var source = Path.Combine(temp.Path, "source");
        var emptyDirectory = Path.Combine(source, "empty");
        Directory.CreateDirectory(emptyDirectory);
        Directory.CreateDirectory(Path.Combine(source, ".git"));
        await File.WriteAllTextAsync(Path.Combine(source, "Dockerfile"), "FROM scratch", TestContext.Current.CancellationToken);
        await File.WriteAllTextAsync(Path.Combine(source, "keep.txt"), "keep", TestContext.Current.CancellationToken);
        await File.WriteAllTextAsync(Path.Combine(source, "ignored.txt"), "ignored", TestContext.Current.CancellationToken);
        await File.WriteAllTextAsync(Path.Combine(source, ".dockerignore"), "ignored.txt", TestContext.Current.CancellationToken);
        await File.WriteAllTextAsync(Path.Combine(source, ".git", "config"), "ignored", TestContext.Current.CancellationToken);
        var archivePath = Path.Combine(temp.Path, "context.tar");

        var context = await BuildContextArchive.CreateAsync(
            source,
            Path.Combine(source, "Dockerfile"),
            archivePath,
            TestContext.Current.CancellationToken);

        Assert.Equal("Dockerfile", context.DockerfileEntryName);
        var entries = ReadTarEntries(archivePath);
        Assert.Contains("empty/", entries);
        Assert.Contains("Dockerfile", entries);
        Assert.Contains("keep.txt", entries);
        Assert.Contains(".dockerignore", entries);
        Assert.DoesNotContain("ignored.txt", entries);
        Assert.DoesNotContain(".git/", entries);
        Assert.DoesNotContain(".git/config", entries);
        Assert.DoesNotContain("./PaxHeaders", Encoding.ASCII.GetString(await File.ReadAllBytesAsync(archivePath, TestContext.Current.CancellationToken)));
    }

    private static List<string> ReadTarEntries(string archivePath)
    {
        using var archive = File.OpenRead(archivePath);
        using var reader = new TarReader(archive);
        var entries = new List<string>();
        TarEntry? entry;
        while ((entry = reader.GetNextEntry()) is not null)
            entries.Add(entry.Name);

        return entries;
    }

    private sealed class TempDirectory : IDisposable
    {
        public string Path { get; } = System.IO.Path.Combine(System.IO.Path.GetTempPath(), $"citadel-test-{Guid.NewGuid():N}");

        public TempDirectory()
        {
            Directory.CreateDirectory(Path);
        }

        public void Dispose()
        {
            try
            {
                if (Directory.Exists(Path))
                    Directory.Delete(Path, recursive: true);
            }
            catch
            {
            }
        }
    }
}
