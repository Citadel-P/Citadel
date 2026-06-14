using Application.Services;
using System.Threading.Channels;

namespace Tests.Unit.Application.Services;

public class MulticastChannelTests
{
    private static async Task<T> ReadOneWithTimeout<T>(ChannelReader<T> reader, TimeSpan timeout)
    {
        using var cts = new CancellationTokenSource(timeout);
        return await reader.ReadAsync(cts.Token).ConfigureAwait(false);
    }

    [Fact]
    public async Task Publish_ToAllSubscribers_DeliversMessageToEach()
    {
        var mc = new MulticastChannel<string>();

        var r1 = mc.AddSubscriber();
        var r2 = mc.AddSubscriber();

        await mc.PublishAsync("hello", TestContext.Current.CancellationToken);

        var v1 = await ReadOneWithTimeout(r1, TimeSpan.FromSeconds(1));
        var v2 = await ReadOneWithTimeout(r2, TimeSpan.FromSeconds(1));

        Assert.Equal("hello", v1);
        Assert.Equal("hello", v2);
    }

    [Fact]
    public async Task RemoveSubscriber_ByReader_StopsReceivingAndCompletesChannel()
    {
        var mc = new MulticastChannel<int>();

        var r1 = mc.AddSubscriber();
        var r2 = mc.AddSubscriber();

        // Remove subscriber r1 by reader before any publish (should complete r1)
        mc.RemoveSubscriber(r1);

        // r1 should be completed (no items, completed -> WaitToReadAsync returns false)
        Assert.False(await r1.WaitToReadAsync());

        // r2 should still receive published items
        await mc.PublishAsync(42, TestContext.Current.CancellationToken);
        var v2 = await ReadOneWithTimeout(r2, TimeSpan.FromSeconds(1));
        Assert.Equal(42, v2);
    }

    [Fact]
    public async Task Complete_CompletesAllSubscribersAndStopsFurtherDelivery()
    {
        var mc = new MulticastChannel<long>();

        var r1 = mc.AddSubscriber();
        var r2 = mc.AddSubscriber();

        // CommandCompleted all subscribers
        mc.Complete();

        // Both readers should be completed
        Assert.False(await r1.WaitToReadAsync(TestContext.Current.CancellationToken));
        Assert.False(await r2.WaitToReadAsync(TestContext.Current.CancellationToken));

        // Further publishes should not throw and should not deliver anything
        await mc.PublishAsync(99L, TestContext.Current.CancellationToken);
        Assert.False(await r1.WaitToReadAsync(TestContext.Current.CancellationToken));
        Assert.False(await r2.WaitToReadAsync(TestContext.Current.CancellationToken));
    }

    [Fact]
    public async Task Publish_IgnoresClosedSubscribers()
    {
        var mc = new MulticastChannel<string>();

        var r1 = mc.AddSubscriber();
        var r2 = mc.AddSubscriber();

        // Remove r1 which completes its channel writer
        mc.RemoveSubscriber(r1);

        // Publishing should still deliver to r2 and not throw because r1 is closed
        await mc.PublishAsync("x", TestContext.Current.CancellationToken);
        var v2 = await ReadOneWithTimeout(r2, TimeSpan.FromSeconds(1));
        Assert.Equal("x", v2);

        // r1 remains completed
        Assert.False(await r1.WaitToReadAsync(TestContext.Current.CancellationToken));
    }
}