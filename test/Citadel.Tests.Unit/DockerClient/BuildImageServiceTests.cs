using System.Formats.Tar;
using System.Buffers.Binary;
using System.IO.Pipelines;
using System.Runtime.CompilerServices;
using System.Text;
using System.Text.Json.Serialization.Metadata;
using Hosting.DockerClient.Models.Images;
using Hosting.DockerClient.HttpClient;
using Hosting.DockerClient.Services;
using Moq;
using IDockerClient = Hosting.DockerClient.IDockerClient;

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

    [Fact]
    public async Task BuildContextArchive_ShouldApplyRootedRecursiveAndNegatedDockerIgnoreRules()
    {
        using var temp = new TempDirectory();
        var source = Path.Combine(temp.Path, "source");
        Directory.CreateDirectory(Path.Combine(source, "nested", "cache"));
        Directory.CreateDirectory(Path.Combine(source, "node_modules"));
        await File.WriteAllTextAsync(Path.Combine(source, "Dockerfile"), "FROM scratch", TestContext.Current.CancellationToken);
        await File.WriteAllTextAsync(Path.Combine(source, "root-only.txt"), "ignored", TestContext.Current.CancellationToken);
        await File.WriteAllTextAsync(Path.Combine(source, "nested", "root-only.txt"), "kept", TestContext.Current.CancellationToken);
        await File.WriteAllTextAsync(Path.Combine(source, "root.log"), "ignored", TestContext.Current.CancellationToken);
        await File.WriteAllTextAsync(Path.Combine(source, "nested", "nested.log"), "ignored", TestContext.Current.CancellationToken);
        await File.WriteAllTextAsync(Path.Combine(source, "nested", "cache", "value.txt"), "ignored", TestContext.Current.CancellationToken);
        await File.WriteAllTextAsync(Path.Combine(source, "node_modules", "drop.txt"), "ignored", TestContext.Current.CancellationToken);
        await File.WriteAllTextAsync(Path.Combine(source, "node_modules", "keep.txt"), "kept", TestContext.Current.CancellationToken);
        await File.WriteAllTextAsync(
            Path.Combine(source, ".dockerignore"),
            "/root-only.txt\n**/*.log\ncache\nnode_modules\n!node_modules/keep.txt\n",
            TestContext.Current.CancellationToken);
        var archivePath = Path.Combine(temp.Path, "context.tar");

        await BuildContextArchive.CreateAsync(
            source,
            Path.Combine(source, "Dockerfile"),
            archivePath,
            TestContext.Current.CancellationToken);

        var entries = ReadTarEntries(archivePath);
        Assert.Contains(".dockerignore", entries);
        Assert.Contains("nested/root-only.txt", entries);
        Assert.Contains("node_modules/keep.txt", entries);
        Assert.DoesNotContain("root-only.txt", entries);
        Assert.DoesNotContain("root.log", entries);
        Assert.DoesNotContain("nested/nested.log", entries);
        Assert.DoesNotContain("nested/cache/value.txt", entries);
        Assert.DoesNotContain("node_modules/drop.txt", entries);
    }

    [Fact]
    public async Task BuildKitSecretSession_ShouldServeRequestedSecret()
    {
        var pair = DuplexStreamPair.Create();
        var connection = new FakeDockerConnection(pair.Server);
        await using var session = new BuildKitSecretSession(
            connection,
            [new BuildImageSecret("npmrc", "registry-token")]);

        await session.StartAsync(TestContext.Current.CancellationToken);

        await pair.Client.WriteAsync("PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n"u8.ToArray(), TestContext.Current.CancellationToken);
        await WriteHttp2FrameAsync(pair.Client, type: 0x4, flags: 0, streamId: 0, [], TestContext.Current.CancellationToken);
        var serverSettings = await ReadHttp2FrameAsync(pair.Client, TestContext.Current.CancellationToken);
        Assert.Equal(0x4, serverSettings.Type);

        await WriteHttp2FrameAsync(pair.Client, type: 0x4, flags: 0x1, streamId: 0, [], TestContext.Current.CancellationToken);
        await WriteHttp2FrameAsync(pair.Client, type: 0x1, flags: 0x4, streamId: 1, [], TestContext.Current.CancellationToken);
        await WriteHttp2FrameAsync(pair.Client, type: 0x0, flags: 0x1, streamId: 1, EncodeGetSecretRequest("npmrc"), TestContext.Current.CancellationToken);

        var frames = new List<Http2Frame>();
        while (frames.Count(frame => frame.StreamId == 1) < 3)
            frames.Add(await ReadHttp2FrameAsync(pair.Client, TestContext.Current.CancellationToken));

        var dataFrame = Assert.Single(frames, frame => frame.StreamId == 1 && frame.Type == 0x0);
        Assert.Equal("registry-token", DecodeGetSecretResponse(dataFrame.Payload));

        Assert.Contains(connection.Request.Headers, header =>
            header.Key == "X-Docker-Expose-Session-Grpc-Method" &&
            header.Value.Contains(BuildKitSecretSession.SecretServiceMethod));
    }

    [Fact]
    public void BuildKitSecretSession_ShouldRejectDuplicateSecretIdsIgnoringCase()
    {
        var ex = Assert.Throws<ArgumentException>(() =>
            new BuildKitSecretSession(
                Mock.Of<IDockerConnection>(),
                [
                    new BuildImageSecret("token", "one"),
                    new BuildImageSecret("TOKEN", "two")
                ]));

        Assert.Contains("mapped more than once", ex.Message);
    }

    [Fact]
    public async Task BuildKitSecretSession_ShouldBoundIncompleteRequestStreams()
    {
        var pair = DuplexStreamPair.Create();
        var session = new BuildKitSecretSession(
            new FakeDockerConnection(pair.Server),
            [new BuildImageSecret("npmrc", "registry-token")]);

        await session.StartAsync(TestContext.Current.CancellationToken);
        await pair.Client.WriteAsync(
            "PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n"u8.ToArray(),
            TestContext.Current.CancellationToken);
        await WriteHttp2FrameAsync(
            pair.Client,
            type: 0x4,
            flags: 0,
            streamId: 0,
            [],
            TestContext.Current.CancellationToken);

        for (var index = 0; index <= BuildKitSecretSession.MaxConcurrentRequestStreams; index++)
        {
            await WriteHttp2FrameAsync(
                pair.Client,
                type: 0x1,
                flags: 0x4,
                streamId: index * 2 + 1,
                [],
                TestContext.Current.CancellationToken);
        }

        var error = await Assert.ThrowsAsync<InvalidDataException>(
            () => session.Completion.WaitAsync(
                TimeSpan.FromSeconds(2),
                TestContext.Current.CancellationToken));
        Assert.Contains("too many concurrent", error.Message);

        await Assert.ThrowsAsync<InvalidDataException>(
            async () => await session.DisposeAsync());
    }

    [Fact]
    public async Task StreamBuildImage_ShouldUseBuildKitSessionForSecrets()
    {
        using var temp = new TempDirectory();
        await File.WriteAllTextAsync(Path.Combine(temp.Path, "Dockerfile"), "FROM scratch", TestContext.Current.CancellationToken);
        var sessionPair = DuplexStreamPair.Create();
        var connection = new QueuedDockerConnection([sessionPair.Server, new MemoryStream(Encoding.UTF8.GetBytes("""{"stream":"ok"}"""))]);
        var service = new ImageService(
            Mock.Of<IDockerClient>(),
            new StreamService(),
            connection);
        var command = new BuildImageStreamCommand(
            ContextDirectory: temp.Path,
            DockerfilePath: Path.Combine(temp.Path, "Dockerfile"),
            Tags: ["citadel/test:latest"],
            BuildArgs: new Dictionary<string, string>(),
            Target: null,
            RegistryAuth: null,
            RegistryHost: null,
            Timeout: TimeSpan.FromMinutes(1),
            Secrets: [new BuildImageSecret("npmrc", "registry-token")]);

        var messages = await service.StreamBuildImage(command, TestContext.Current.CancellationToken)
            .ToListAsync(TestContext.Current.CancellationToken);

        Assert.Single(messages);
        Assert.Equal("ok", messages[0].Stream);
        Assert.Equal(2, connection.Requests.Count);
        Assert.Equal("/session", connection.Requests[0].RequestUri?.PathAndQuery);
        Assert.Contains("session=", connection.Requests[1].RequestUri?.PathAndQuery);
    }

    [Fact]
    public async Task StreamPullImage_DisposingEnumeratorShouldCancelProducer()
    {
        var streamService = new BlockingMessageStreamService();
        var service = new ImageService(
            Mock.Of<IDockerClient>(),
            streamService,
            Mock.Of<IDockerConnection>());
        var command = new PullImageStreamCommand(
            FromImage: "nginx",
            FromSrc: null,
            Repo: null,
            Auth: null,
            Tag: "latest");

        await using (var enumerator = service
                         .StreamPullImage(command, TestContext.Current.CancellationToken)
                         .GetAsyncEnumerator(TestContext.Current.CancellationToken))
        {
            Assert.True(await enumerator.MoveNextAsync());
            Assert.Equal("started", enumerator.Current.Status);
        }

        await streamService.ProducerCancelled.Task.WaitAsync(
            TimeSpan.FromSeconds(2),
            TestContext.Current.CancellationToken);
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

    private sealed class BlockingMessageStreamService : IStreamService
    {
        public TaskCompletionSource ProducerCancelled { get; } = new(
            TaskCreationOptions.RunContinuationsAsynchronously);

        public IAsyncEnumerable<ReadOnlyMemory<byte>> MonitorStreamForBytesAsync(
            Task<Stream> streamTask,
            CancellationToken cancellationToken = default)
            => throw new NotSupportedException();

        public IAsyncEnumerable<string> MonitorStreamForStringsAsync(
            Task<Stream> streamTask,
            CancellationToken cancellationToken = default)
            => throw new NotSupportedException();

        public async IAsyncEnumerable<T> MonitorStreamForMessagesAsync<T>(
            Task<Stream> streamTask,
            JsonTypeInfo<T> jsonTypeInfo,
            [EnumeratorCancellation] CancellationToken cancellationToken = default)
            where T : class
        {
            try
            {
                yield return (T)(object)new JSONMessage { Status = "started" };
                await Task.Delay(Timeout.InfiniteTimeSpan, cancellationToken);
            }
            finally
            {
                if (cancellationToken.IsCancellationRequested)
                    ProducerCancelled.TrySetResult();
            }
        }
    }

    private static byte[] EncodeGetSecretRequest(string id)
    {
        using var message = new MemoryStream();
        message.WriteByte(0x0A);
        WriteVarint(message, (ulong)Encoding.UTF8.GetByteCount(id));
        message.Write(Encoding.UTF8.GetBytes(id));
        return CreateGrpcEnvelope(message.ToArray());
    }

    private static string DecodeGetSecretResponse(byte[] envelope)
    {
        Assert.True(envelope.Length >= 5);
        Assert.Equal(0, envelope[0]);
        var messageLength = BinaryPrimitives.ReadInt32BigEndian(envelope.AsSpan(1, 4));
        var message = envelope.AsSpan(5, messageLength);
        Assert.Equal(0x0A, message[0]);
        var offset = 1;
        var secretLength = (int)ReadVarint(message, ref offset);
        return Encoding.UTF8.GetString(message.Slice(offset, secretLength));
    }

    private static byte[] CreateGrpcEnvelope(byte[] message)
    {
        var envelope = new byte[5 + message.Length];
        BinaryPrimitives.WriteInt32BigEndian(envelope.AsSpan(1, 4), message.Length);
        message.CopyTo(envelope.AsSpan(5));
        return envelope;
    }

    private static async Task WriteHttp2FrameAsync(
        Stream stream,
        byte type,
        byte flags,
        int streamId,
        byte[] payload,
        CancellationToken cancellationToken)
    {
        var header = new byte[9];
        header[0] = (byte)(payload.Length >> 16);
        header[1] = (byte)(payload.Length >> 8);
        header[2] = (byte)payload.Length;
        header[3] = type;
        header[4] = flags;
        BinaryPrimitives.WriteInt32BigEndian(header.AsSpan(5, 4), streamId & 0x7FFFFFFF);
        await stream.WriteAsync(header, cancellationToken);
        if (payload.Length > 0)
            await stream.WriteAsync(payload, cancellationToken);
        await stream.FlushAsync(cancellationToken);
    }

    private static async Task<Http2Frame> ReadHttp2FrameAsync(Stream stream, CancellationToken cancellationToken)
    {
        var header = new byte[9];
        await ReadExactlyAsync(stream, header, cancellationToken);
        var length = (header[0] << 16) | (header[1] << 8) | header[2];
        var payload = new byte[length];
        if (length > 0)
            await ReadExactlyAsync(stream, payload, cancellationToken);

        return new Http2Frame(
            Type: header[3],
            Flags: header[4],
            StreamId: BinaryPrimitives.ReadInt32BigEndian(header.AsSpan(5, 4)) & 0x7FFFFFFF,
            Payload: payload);
    }

    private static async Task ReadExactlyAsync(Stream stream, Memory<byte> buffer, CancellationToken cancellationToken)
    {
        var offset = 0;
        while (offset < buffer.Length)
        {
            var read = await stream.ReadAsync(buffer[offset..], cancellationToken);
            if (read == 0)
                throw new EndOfStreamException();
            offset += read;
        }
    }

    private static void WriteVarint(Stream stream, ulong value)
    {
        while (value >= 0x80)
        {
            stream.WriteByte((byte)(value | 0x80));
            value >>= 7;
        }

        stream.WriteByte((byte)value);
    }

    private static ulong ReadVarint(ReadOnlySpan<byte> value, ref int offset)
    {
        ulong result = 0;
        var shift = 0;
        while (offset < value.Length)
        {
            var b = value[offset++];
            result |= (ulong)(b & 0x7F) << shift;
            if ((b & 0x80) == 0)
                return result;

            shift += 7;
        }

        throw new InvalidOperationException("Invalid varint.");
    }

    private sealed record Http2Frame(byte Type, byte Flags, int StreamId, byte[] Payload);

    private sealed class FakeDockerConnection(Stream stream) : IDockerConnection
    {
        public HttpRequestMessage Request { get; private set; } = null!;

        public Task<Stream> OpenHijackedStreamAsync(HttpRequestMessage request, CancellationToken ct = default)
        {
            Request = request;
            return Task.FromResult(stream);
        }
    }

    private sealed class QueuedDockerConnection(IReadOnlyList<Stream> streams) : IDockerConnection
    {
        private int index;
        public List<HttpRequestMessage> Requests { get; } = [];

        public Task<Stream> OpenHijackedStreamAsync(HttpRequestMessage request, CancellationToken ct = default)
        {
            Requests.Add(request);
            return Task.FromResult(streams[index++]);
        }
    }

    private sealed record DuplexStreamPair(Stream Client, Stream Server)
    {
        public static DuplexStreamPair Create()
        {
            var clientToServer = new Pipe();
            var serverToClient = new Pipe();
            return new DuplexStreamPair(
                new PipeDuplexStream(serverToClient.Reader, clientToServer.Writer),
                new PipeDuplexStream(clientToServer.Reader, serverToClient.Writer));
        }
    }

    private sealed class PipeDuplexStream(PipeReader reader, PipeWriter writer) : Stream
    {
        public override bool CanRead => true;
        public override bool CanSeek => false;
        public override bool CanWrite => true;
        public override long Length => throw new NotSupportedException();
        public override long Position
        {
            get => throw new NotSupportedException();
            set => throw new NotSupportedException();
        }

        public override async ValueTask<int> ReadAsync(Memory<byte> buffer, CancellationToken cancellationToken = default)
        {
            while (true)
            {
                var result = await reader.ReadAsync(cancellationToken);
                var source = result.Buffer;
                if (!source.IsEmpty)
                {
                    var length = (int)Math.Min(buffer.Length, source.Length);
                    foreach (var segment in source.Slice(0, length))
                    {
                        segment.Span.CopyTo(buffer.Span);
                        buffer = buffer[segment.Length..];
                    }
                    reader.AdvanceTo(source.GetPosition(length));
                    return length;
                }

                reader.AdvanceTo(source.Start, source.End);
                if (result.IsCompleted)
                    return 0;
            }
        }

        public override async ValueTask WriteAsync(ReadOnlyMemory<byte> buffer, CancellationToken cancellationToken = default)
        {
            await writer.WriteAsync(buffer, cancellationToken);
        }

        public override Task FlushAsync(CancellationToken cancellationToken)
            => writer.FlushAsync(cancellationToken).AsTask();

        public override void Flush()
        {
        }

        public override int Read(byte[] buffer, int offset, int count)
            => ReadAsync(buffer.AsMemory(offset, count)).AsTask().GetAwaiter().GetResult();

        public override void Write(byte[] buffer, int offset, int count)
            => WriteAsync(buffer.AsMemory(offset, count)).AsTask().GetAwaiter().GetResult();

        public override long Seek(long offset, SeekOrigin origin) => throw new NotSupportedException();
        public override void SetLength(long value) => throw new NotSupportedException();

        protected override void Dispose(bool disposing)
        {
            if (disposing)
            {
                reader.Complete();
                writer.Complete();
            }

            base.Dispose(disposing);
        }
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
