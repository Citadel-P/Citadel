using Application.Services.SignalR.Context;

namespace Tests.Unit.Application.Services.SignalR;

public class ChannelStreamContextTests
{
    [Fact]
    public void RemoveSubscriber_LastSubscriber_CancelsAndCompletesChannelAndResetsState()
    {
        var ctx = new ChannelStreamContext<string>();

        // add a single subscriber and start
        ctx.AddSubscriber("conn-1");
        Assert.False(ctx.IsEmpty);
        Assert.True(ctx.TryStart());

        // set a dummy stream task and capture current cancellation source
        var tcs = new TaskCompletionSource();
        ctx.StreamTask = tcs.Task;
        var oldCts = ctx.Cancellation;

        // remove last subscriber -> should cancel old CTS, complete channel writer and reset state
        ctx.RemoveSubscriber("conn-1");

        Assert.True(oldCts.IsCancellationRequested, "Old CancellationTokenSource should have been cancelled.");
        Assert.NotSame(oldCts, ctx.Cancellation);
        Assert.Null(ctx.StreamTask);

        // writer should be completed; TryWrite must fail after completion
        var wrote = ctx.Channel.Writer.TryWrite("payload");
        Assert.False(wrote, "Channel writer should not accept writes after TryComplete.");

        // after reset, TryStart should be allowed again
        Assert.True(ctx.TryStart());
    }

    [Fact]
    public void RemoveSubscriber_WhenMultipleSubscribers_DoesNotStopStream()
    {
        
        var ctx = new ChannelStreamContext<Dummy>();

        ctx.AddSubscriber("c1");
        ctx.AddSubscriber("c2");
        Assert.False(ctx.IsEmpty);

        // start and capture cts and stream task
        Assert.True(ctx.TryStart());
        var existingTask = Task.CompletedTask;
        ctx.StreamTask = existingTask;
        var existingCts = ctx.Cancellation;

        // remove one subscriber - should not cancel or complete
        ctx.RemoveSubscriber("c1");

        Assert.False(existingCts.IsCancellationRequested, "Cancellation should not be requested when there are remaining subscribers.");
        Assert.Same(existingCts, ctx.Cancellation);
        Assert.Same(existingTask, ctx.StreamTask);

        // channel should still accept writes (subject to bounded options) -- TryWrite may succeed
        // We assert that the writer is not completed by checking that WaitToReadAsync does not immediately return false.
        var wait = ctx.Channel.Reader.WaitToReadAsync(TestContext.Current.CancellationToken);
        Assert.False(wait.IsCompletedSuccessfully && wait.Result == false, "Reader should not be completed while subscribers remain.");
    }

    [Fact]
    public void RemoveSubscriber_RepeatedCalls_AreSafe()
    {
        var ctx = new ChannelStreamContext<object>();

        // add and remove same subscriber multiple times (idempotency / safety)
        ctx.AddSubscriber("s");
        ctx.RemoveSubscriber("s");

        // second remove should be a no-op and not throw
        var ex = Record.Exception(() => ctx.RemoveSubscriber("s"));
        Assert.Null(ex);

        // channel writer should remain completed after first removal
        Assert.False(ctx.Channel.Writer.TryWrite(new object()));
    }

    private record Dummy();
}
