using System.Buffers;
using System.Text;
using System.Threading.Channels;

namespace Application.Services.SignalR.Context;

internal sealed class LogStreamContext : StreamContext, IDisposable
{
    public Channel<ReadOnlyMemory<byte>> Channel { get; } = System.Threading.Channels.Channel.CreateBounded<ReadOnlyMemory<byte>>(ApplicationModule.ChannelDefaultOptions());
    public CancellationTokenSource Cancellation { get; private set; } = new();
    private readonly PooledLogBuffer logBuffer = new(1024 * 512); // default 512 KB

    public override void RemoveSubscriber(string connectionId)
    {
        Task? toObserve = null;
        lock (@lock)
        {
            subscribers.Remove(connectionId);
            if (IsEmpty)
            {
                logBuffer.Clear();

                try { Cancellation.Cancel(); } catch { }
                Channel.Writer.TryComplete();

                started = false;

                toObserve = StreamTask;
                StreamTask = null;

                try { Cancellation.Dispose(); } catch { }
                Cancellation = new CancellationTokenSource();
            }
        }

        if (toObserve != null)
        {
            _ = toObserve.ContinueWith(t =>
            {
                if (t.IsFaulted)
                {
                    // logger?.LogError(t.Exception, "Stream task faulted");
                    GC.KeepAlive(t.Exception);
                }
                t.Dispose();
            }, TaskContinuationOptions.ExecuteSynchronously);
        }
    }

    public void AddToBuffer(ReadOnlySpan<byte> logBytes)
    {
        logBuffer.AddLog(logBytes);
    }

    public string GetBufferedLogsAsString() =>
        logBuffer.GetRecentLogsString();

    public byte[] GetBufferedLogsAsBytes() =>
        logBuffer.GetRecentLogsBytes();

    public void Dispose()
    {
        logBuffer.Dispose();
        Cancellation.Dispose();
    }
}

public sealed class PooledLogBuffer : IDisposable
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
        lock (@lock)
        {
            foreach (byte b in logBytes)
            {
                buffer[writeIndex] = b;
                writeIndex = (writeIndex + 1) % capacity;
                if (lengthUsed < capacity)
                    lengthUsed++;
            }
        }
    }

    public byte[] GetRecentLogsBytes()
    {
        lock (@lock)
        {
            byte[] output = new byte[lengthUsed];
            int start = (writeIndex - lengthUsed + capacity) % capacity;

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
        lock (@lock)
        {
            writeIndex = 0;
            lengthUsed = 0;
            Array.Clear(buffer, 0, buffer.Length);
        }
    }

    public string GetRecentLogsString()
    {
        var bytes = GetRecentLogsBytes();
        return Encoding.UTF8.GetString(bytes);
    }

    public void Dispose()
    {
        ArrayPool<byte>.Shared.Return(buffer);
        buffer = null!;
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
        get { lock (_lock) { return count; } }
    }

    public void Enqueue(T item)
    {
        if (buffer.Length == 0) return;
        lock (_lock)
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
        lock (_lock)
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
        lock (_lock)
        {
            Array.Clear(buffer, 0, buffer.Length);
            head = 0;
            count = 0;
        }
    }
}