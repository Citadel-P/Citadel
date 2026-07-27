using System.Buffers;
using System.Text;
using System.Threading.Channels;

namespace Application.Services.SignalR.Context;

internal sealed class LogStreamContext : StreamContext, IDisposable
{
    private static BoundedChannelOptions DefaultChannelOptions() => new(128)
    {
        FullMode = BoundedChannelFullMode.Wait,
        SingleReader = true,
        SingleWriter = false
    };

    public Channel<PooledBuffer> LogChannel { get; private set; } = Channel.CreateBounded<PooledBuffer>(DefaultChannelOptions());
    public CancellationTokenSource Cancellation { get; private set; } = new();
    public CancellationTokenSource WatcherCts { get; private set; } = new();

    private PooledLogBuffer logBuffer = new(1024 * 512);
    private bool disposed;
    public Task? EventWatcherTask { get; set; }

    public override void RemoveSubscriber(string connectionId)
    {
        base.RemoveSubscriber(connectionId);
        if (IsEmpty)
            Dispose();
    }

    /// <summary>Force a reset when container restarts (keeps subscribers, keeps watcher).</summary>
    public void Reset()
    {
        StreamCleanup? cleanup = null;
        using (@lock.EnterScope())
        {
            if (!disposed)
                cleanup = ReplaceStreamUnsafe();
        }

        if (cleanup is { } stream)
            BeginStreamCleanup(stream);
    }

    public bool TryStartStream(Func<LogStreamResources, Task> start)
    {
        using (@lock.EnterScope())
        {
            if (disposed || started)
                return false;

            started = true;
            try
            {
                StreamTask = start(new LogStreamResources(
                    LogChannel,
                    Cancellation.Token,
                    logBuffer));
                return true;
            }
            catch
            {
                started = false;
                StreamTask = null;
                throw;
            }
        }
    }

    public void EnsureWatcher(Func<CancellationToken, Task> start)
    {
        using (@lock.EnterScope())
        {
            if (disposed || EventWatcherTask is not null)
                return;

            EventWatcherTask = start(WatcherCts.Token);
        }
    }

    public void AddToBuffer(ReadOnlySpan<byte> logBytes)
    {
        using (@lock.EnterScope())
        {
            ObjectDisposedException.ThrowIf(disposed, this);
            logBuffer.AddLog(logBytes);
        }
    }

    public string GetBufferedLogsAsString()
    {
        using (@lock.EnterScope())
            return disposed ? string.Empty : logBuffer.GetRecentLogsString();
    }

    public byte[] GetBufferedLogsAsBytes()
    {
        using (@lock.EnterScope())
            return disposed ? [] : logBuffer.GetRecentLogsBytes();
    }

    public void Dispose()
    {
        StreamCleanup streamCleanup;
        WatcherCleanup watcherCleanup;

        using (@lock.EnterScope())
        {
            if (disposed)
                return;

            disposed = true;
            started = false;
            streamCleanup = new StreamCleanup(StreamTask, Cancellation, LogChannel, logBuffer);
            watcherCleanup = new WatcherCleanup(EventWatcherTask, WatcherCts);
            StreamTask = null;
            EventWatcherTask = null;
        }

        BeginStreamCleanup(streamCleanup);
        BeginWatcherCleanup(watcherCleanup);
    }

    internal static void Drain(ChannelReader<PooledBuffer> reader)
    {
        while (reader.TryRead(out var item))
            item.Dispose();
    }

    private StreamCleanup ReplaceStreamUnsafe()
    {
        var cleanup = new StreamCleanup(StreamTask, Cancellation, LogChannel, logBuffer);
        Cancellation = new CancellationTokenSource();
        LogChannel = Channel.CreateBounded<PooledBuffer>(DefaultChannelOptions());
        logBuffer = new PooledLogBuffer(1024 * 512);
        StreamTask = null;
        started = false;
        return cleanup;
    }

    private WatcherCleanup ReplaceWatcherUnsafe()
    {
        var cleanup = new WatcherCleanup(EventWatcherTask, WatcherCts);
        WatcherCts = new CancellationTokenSource();
        EventWatcherTask = null;
        return cleanup;
    }

    private static void BeginStreamCleanup(StreamCleanup cleanup)
    {
        try { cleanup.Cancellation.Cancel(); } catch { }
        cleanup.Channel.Writer.TryComplete();
        _ = CleanupStreamAsync(cleanup);
    }

    private static async Task CleanupStreamAsync(StreamCleanup cleanup)
    {
        try
        {
            if (cleanup.StreamTask is not null)
                await cleanup.StreamTask.ConfigureAwait(false);
        }
        catch
        {
        }
        finally
        {
            cleanup.Channel.Writer.TryComplete();
            Drain(cleanup.Channel.Reader);
            cleanup.Buffer.Dispose();
            cleanup.Cancellation.Dispose();
        }
    }

    private static void BeginWatcherCleanup(WatcherCleanup cleanup)
    {
        try { cleanup.Cancellation.Cancel(); } catch { }
        _ = CleanupWatcherAsync(cleanup);
    }

    private static async Task CleanupWatcherAsync(WatcherCleanup cleanup)
    {
        try
        {
            if (cleanup.WatcherTask is not null)
                await cleanup.WatcherTask.ConfigureAwait(false);
        }
        catch
        {
        }
        finally
        {
            cleanup.Cancellation.Dispose();
        }
    }

    private readonly record struct StreamCleanup(
        Task? StreamTask,
        CancellationTokenSource Cancellation,
        Channel<PooledBuffer> Channel,
        PooledLogBuffer Buffer);

    private readonly record struct WatcherCleanup(
        Task? WatcherTask,
        CancellationTokenSource Cancellation);
}

internal sealed class LogStreamResources(
    Channel<PooledBuffer> channel,
    CancellationToken cancellationToken,
    PooledLogBuffer buffer)
{
    public Channel<PooledBuffer> Channel { get; } = channel;
    public CancellationToken CancellationToken { get; } = cancellationToken;
    public void AddToBuffer(ReadOnlySpan<byte> data) => buffer.AddLog(data);
}

internal static class LogStreamPipeline
{
    public static async Task RunAsync(
        Channel<PooledBuffer> channel,
        Func<CancellationToken, Task> produce,
        Func<CancellationToken, Task> consume,
        CancellationToken cancellationToken)
    {
        using var pipelineCancellation = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        var producerTask = produce(pipelineCancellation.Token);
        var consumerTask = consume(pipelineCancellation.Token);

        try
        {
            var firstCompleted = await Task.WhenAny(producerTask, consumerTask).ConfigureAwait(false);
            if (firstCompleted == producerTask)
            {
                var error = producerTask.IsFaulted
                    ? producerTask.Exception?.GetBaseException()
                    : null;
                channel.Writer.TryComplete(error);
            }
            else
            {
                pipelineCancellation.Cancel();
                channel.Writer.TryComplete();
            }

            await Task.WhenAll(producerTask, consumerTask).ConfigureAwait(false);
        }
        finally
        {
            try { pipelineCancellation.Cancel(); } catch { }
            channel.Writer.TryComplete();
            LogStreamContext.Drain(channel.Reader);
        }
    }
}

internal sealed class PooledLogBuffer : IDisposable
{
    private byte[]? buffer;
    private int writeIndex;
    private int lengthUsed;
    private readonly int capacity;
    private readonly Lock @lock = new();

    public PooledLogBuffer(int capacityBytes)
    {
        capacity = capacityBytes;
        buffer = ArrayPool<byte>.Shared.Rent(capacity);
    }

    public void AddLog(ReadOnlySpan<byte> logBytes)
    {
        if (logBytes.IsEmpty) return;

        using (@lock.EnterScope())
        {
            var target = buffer ?? throw new ObjectDisposedException(nameof(PooledLogBuffer));
            int bytesToWrite = Math.Min(logBytes.Length, capacity);
            ReadOnlySpan<byte> source = logBytes[^bytesToWrite..];

            int spaceAtEnd = capacity - writeIndex;
            if (source.Length <= spaceAtEnd)
            {
                source.CopyTo(target.AsSpan(writeIndex));
            }
            else
            {
                source[..spaceAtEnd].CopyTo(target.AsSpan(writeIndex));
                source[spaceAtEnd..].CopyTo(target.AsSpan(0));
            }

            writeIndex = (writeIndex + source.Length) % capacity;
            lengthUsed = Math.Min(lengthUsed + source.Length, capacity);
        }
    }

    public byte[] GetRecentLogsBytes()
    {
        using (@lock.EnterScope())
        {
            var source = buffer ?? throw new ObjectDisposedException(nameof(PooledLogBuffer));
            if (lengthUsed == 0)
                return Array.Empty<byte>();

            byte[] output = new byte[lengthUsed];
            int start = writeIndex - lengthUsed;
            if (start < 0)
                start += capacity;

            if (start + lengthUsed <= capacity)
            {
                // Contiguous block
                Buffer.BlockCopy(source, start, output, 0, lengthUsed);
            }
            else
            {
                // Wrapped
                int firstPart = capacity - start;
                Buffer.BlockCopy(source, start, output, 0, firstPart);
                Buffer.BlockCopy(source, 0, output, firstPart, lengthUsed - firstPart);
            }

            return output;
        }
    }

    public void Clear()
    {
        using (@lock.EnterScope())
        {
            ObjectDisposedException.ThrowIf(buffer is null, this);
            writeIndex = 0;
            lengthUsed = 0;
        }
    }

    public string GetRecentLogsString()
    {
        var bytes = GetRecentLogsBytes();
        return Encoding.UTF8.GetString(bytes);
    }

    public void Dispose()
    {
        byte[]? rented;
        using (@lock.EnterScope())
        {
            rented = buffer;
            buffer = null;
            writeIndex = 0;
            lengthUsed = 0;
        }

        if (rented is not null)
            ArrayPool<byte>.Shared.Return(rented, clearArray: true);
    }
}

internal sealed class RingBuffer<T>
{
    readonly T[] buffer;
    int head;   // index of oldest
    int count;
    readonly Lock _lock = new();

    public RingBuffer(int capacity)
    {
        if (capacity < 0) capacity = 0;
        buffer = new T[capacity];
    }

    public int Count
    {
        get { using (_lock.EnterScope()) { return count; } }
    }

    public void Enqueue(T item)
    {
        if (buffer.Length == 0) return;
        using (_lock.EnterScope())
        {
            if (count < buffer.Length)
            {
                int idx = (head + count) % buffer.Length;
                buffer[idx] = item;
                count++;
            }
            else
            {
                // overwrite oldest
                buffer[head] = item;
                head = (head + 1) % buffer.Length;
            }
        }
    }

    public T[] ToArray()
    {
        using (_lock.EnterScope())
        {
            if (count == 0) return Array.Empty<T>();
            var arr = new T[count];
            if (head + count <= buffer.Length)
            {
                Array.Copy(buffer, head, arr, 0, count);
            }
            else
            {
                int first = buffer.Length - head;
                Array.Copy(buffer, head, arr, 0, first);
                Array.Copy(buffer, 0, arr, first, count - first);
            }
            return arr;
        }
    }

    public void Clear()
    {
        if (buffer.Length == 0) return;
        using (_lock.EnterScope())
        {
            Array.Clear(buffer, 0, buffer.Length);
            head = 0;
            count = 0;
        }
    }
}

internal sealed class PooledBuffer(byte[] buffer, int length) : IDisposable
{
    private byte[]? buffer = buffer;

    public byte[] Buffer => buffer ?? throw new ObjectDisposedException(nameof(PooledBuffer));
    public int Length { get; } = length;

    public ReadOnlySpan<byte> Span => Buffer.AsSpan(0, Length);

    public ReadOnlyMemory<byte> Memory => Buffer.AsMemory(0, Length);

    public void Dispose()
    {
        var rented = Interlocked.Exchange(ref buffer, null);
        if (rented is not null)
            ArrayPool<byte>.Shared.Return(rented);
    }
}
