using System.Buffers;
using System.Text;
using System.Threading.Channels;

namespace Application.Services.SignalR.Context;

internal sealed class LogStreamContext : StreamContext, IDisposable
{
    public Channel<byte[]> LogChannel { get; set; } = Channel.CreateUnbounded<byte[]>();
    public CancellationTokenSource Cancellation { get; private set; } = new();
    public CancellationTokenSource WatcherCts { get; private set; } = new();

    private readonly PooledLogBuffer logBuffer = new(1024 * 512);
    public Task? EventWatcherTask { get; set; }

    public override void RemoveSubscriber(string connectionId)
    {
        base.RemoveSubscriber(connectionId);
        using (@lock.EnterScope())
        {
            if (IsEmpty)
            {
                ResetInternal();       // stops producer/consumer
                StopWatcherInternal(); // stops watcher
            }
        }
    }

    /// <summary>Force a reset when container restarts (keeps subscribers, keeps watcher).</summary>
    public void Reset()
    {
        using (@lock.EnterScope())
        {
            ResetInternal();
        }
    }

    private void ResetInternal()
    {
        logBuffer.Clear();

        var oldCts = Cancellation;
        Cancellation = new CancellationTokenSource();
        try { oldCts.Cancel(); } catch { }
        try { oldCts.Dispose(); } catch { }

        started = false;

        var oldChannel = LogChannel;
        oldChannel.Writer.TryComplete();
        LogChannel = Channel.CreateUnbounded<byte[]>();

        var t = StreamTask;
        StreamTask = null;
        if (t != null)
        {
            _ = t.ContinueWith(tt =>
            {
                if (tt.IsFaulted) GC.KeepAlive(tt.Exception);
                tt.Dispose();
            }, TaskContinuationOptions.ExecuteSynchronously);
        }
        // NOTE: we DO NOT touch WatcherCts or EventWatcherTask here
    }

    private void StopWatcherInternal()
    {
        var oldWatcherCts = WatcherCts;
        WatcherCts = new CancellationTokenSource();
        try { oldWatcherCts.Cancel(); } catch { }
        try { oldWatcherCts.Dispose(); } catch { }

        var wt = EventWatcherTask;
        EventWatcherTask = null;
        if (wt != null)
        {
            _ = wt.ContinueWith(tt =>
            {
                if (tt.IsFaulted) GC.KeepAlive(tt.Exception);
                tt.Dispose();
            }, TaskContinuationOptions.ExecuteSynchronously);
        }
    }

    public void AddToBuffer(ReadOnlySpan<byte> logBytes) => logBuffer.AddLog(logBytes);
    public string GetBufferedLogsAsString() => logBuffer.GetRecentLogsString();
    public byte[] GetBufferedLogsAsBytes() => logBuffer.GetRecentLogsBytes();

    public void Dispose()
    {
        logBuffer.Dispose();
        Cancellation.Dispose();
        WatcherCts.Dispose();
    }
}

internal sealed class PooledLogBuffer : IDisposable
{
    private byte[] buffer;
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
        using (@lock.EnterScope())
        {
            foreach (var b in logBytes)
            {
                buffer[writeIndex] = b;
                writeIndex++;
                if (writeIndex == capacity)
                    writeIndex = 0;

                if (lengthUsed < capacity)
                    lengthUsed++;
            }
        }
    }

    public byte[] GetRecentLogsBytes()
    {
        using (@lock.EnterScope())
        {
            if (lengthUsed == 0)
                return Array.Empty<byte>();

            byte[] output = new byte[lengthUsed];
            int start = writeIndex - lengthUsed;
            if (start < 0)
                start += capacity;

            if (start + lengthUsed <= capacity)
            {
                // Contiguous block
                Buffer.BlockCopy(buffer, start, output, 0, lengthUsed);
            }
            else
            {
                // Wrapped
                int firstPart = capacity - start;
                Buffer.BlockCopy(buffer, start, output, 0, firstPart);
                Buffer.BlockCopy(buffer, 0, output, firstPart, lengthUsed - firstPart);
            }

            return output;
        }
    }

    public void Clear()
    {
        using (@lock.EnterScope())
        {
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
        if (buffer != null)
        {
            ArrayPool<byte>.Shared.Return(buffer, clearArray: true);
            buffer = null!;
        }
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