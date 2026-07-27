using System.Text;
using Application.Services.SignalR.Context;
using System.Buffers;

namespace Tests.Unit.Application.Services.SignalR;

public class LogStreamContextTest
{
    [Fact]
    public void AddToBuffer_GetBufferedLogsAsStringAndBytes_ReturnsAddedContent()
    {
        using var ctx = new LogStreamContext();

        var text = "hello-world";
        var bytes = Encoding.UTF8.GetBytes(text);

        ctx.AddToBuffer(bytes);

        Assert.Equal(text, ctx.GetBufferedLogsAsString());
        Assert.Equal(bytes, ctx.GetBufferedLogsAsBytes());
    }

    [Fact]
    public void Reset_ClearsBufferedData()
    {
        using var ctx = new LogStreamContext();

        ctx.AddToBuffer(Encoding.UTF8.GetBytes("some-data"));
        Assert.NotEmpty(ctx.GetBufferedLogsAsString());

        ctx.Reset();

        Assert.Equal(string.Empty, ctx.GetBufferedLogsAsString());
        Assert.Empty(ctx.GetBufferedLogsAsBytes());
    }

    [Fact]
    public void RemoveSubscriber_WhenLastSubscriber_RemovesTasksAndClearsBuffer()
    {
        var ctx = new LogStreamContext();

        // simulate active watcher/stream tasks and buffered data
        ctx.EventWatcherTask = Task.CompletedTask;
        ctx.StreamTask = Task.CompletedTask;

        ctx.AddSubscriber("conn-1");
        ctx.AddToBuffer(Encoding.UTF8.GetBytes("log"));

        // remove last subscriber -> should call ResetInternal + StopWatcherInternal
        ctx.RemoveSubscriber("conn-1");

        // After removing last subscriber, internal tasks should be cleared and buffer empty
        Assert.Null(ctx.EventWatcherTask);
        Assert.Null(ctx.StreamTask);
        Assert.Equal(string.Empty, ctx.GetBufferedLogsAsString());

        ctx.Dispose();
    }

    [Fact]
    public void Dispose_DoesNotThrow()
    {
        var ctx = new LogStreamContext();
        ctx.AddToBuffer(Encoding.UTF8.GetBytes("data-before-dispose"));

        var ex = Record.Exception(() => ctx.Dispose());
        Assert.Null(ex);
    }

    [Fact]
    public void PooledLogBuffer_WrapsAndKeepsMostRecentBytes()
    {
        var small = new PooledLogBuffer(5);

        // add 6 bytes -> buffer capacity is 5 so the oldest byte is overwritten
        var data = Encoding.UTF8.GetBytes("abcdef"); // 6 bytes
        small.AddLog(data);

        var recent = small.GetRecentLogsString();
        Assert.Equal("bcdef", recent);

        small.Dispose();
    }

    [Fact]
    public void RingBuffer_Enqueue_OverwritesOldest_And_Clear_Works()
    {
        var rb = new RingBuffer<int>(3);

        rb.Enqueue(1);
        rb.Enqueue(2);
        rb.Enqueue(3);

        Assert.Equal(3, rb.Count);

        rb.Enqueue(4); // should overwrite oldest (1), leaving [2,3,4]
        Assert.Equal(3, rb.Count);

        var arr = rb.ToArray();
        Assert.Equal(new[] { 2, 3, 4 }, arr);

        rb.Clear();
        Assert.Equal(0, rb.Count);
        Assert.Empty(rb.ToArray());
    }

    [Fact]
    public async Task Reset_DisposesQueuedPooledBuffers()
    {
        using var context = new LogStreamContext();
        var pooled = new PooledBuffer(ArrayPool<byte>.Shared.Rent(8), 1);
        await context.LogChannel.Writer.WriteAsync(pooled, TestContext.Current.CancellationToken);

        context.Reset();

        Assert.Throws<ObjectDisposedException>(() => pooled.Buffer);
    }

    [Fact]
    public async Task Reset_DoesNotReleaseGenerationResourcesUntilItsStreamStops()
    {
        using var context = new LogStreamContext();
        var streamStarted = new TaskCompletionSource<LogStreamResources>(
            TaskCreationOptions.RunContinuationsAsynchronously);
        var finishStream = new TaskCompletionSource(
            TaskCreationOptions.RunContinuationsAsynchronously);

        Assert.True(context.TryStartStream(async resources =>
        {
            streamStarted.TrySetResult(resources);
            await finishStream.Task;
        }));

        var resources = await streamStarted.Task;
        var pooled = new PooledBuffer(ArrayPool<byte>.Shared.Rent(8), 1);
        await resources.Channel.Writer.WriteAsync(pooled, TestContext.Current.CancellationToken);

        context.Reset();

        resources.AddToBuffer("still-owned"u8);
        Assert.NotNull(pooled.Buffer);

        finishStream.TrySetResult();
        await AssertEventuallyAsync(() =>
            Assert.Throws<ObjectDisposedException>(() => pooled.Buffer));
    }

    [Fact]
    public async Task Pipeline_WhenConsumerFails_CancelsProducerAndDisposesQueuedBuffers()
    {
        using var context = new LogStreamContext();
        var first = new PooledBuffer(ArrayPool<byte>.Shared.Rent(8), 1);
        var second = new PooledBuffer(ArrayPool<byte>.Shared.Rent(8), 1);

        var pipeline = LogStreamPipeline.RunAsync(
            context.LogChannel,
            async token =>
            {
                await context.LogChannel.Writer.WriteAsync(first, token);
                await context.LogChannel.Writer.WriteAsync(second, token);
                await Task.Delay(Timeout.InfiniteTimeSpan, token);
            },
            _ => Task.FromException(new InvalidOperationException("dispatch failed")),
            TestContext.Current.CancellationToken);

        await Assert.ThrowsAnyAsync<Exception>(() => pipeline);
        Assert.Throws<ObjectDisposedException>(() => first.Buffer);
        Assert.Throws<ObjectDisposedException>(() => second.Buffer);
    }

    private static async Task AssertEventuallyAsync(Action assertion)
    {
        var timeout = DateTime.UtcNow.AddSeconds(2);
        while (true)
        {
            try
            {
                assertion();
                return;
            }
            catch when (DateTime.UtcNow < timeout)
            {
                await Task.Delay(10);
            }
        }
    }
}
