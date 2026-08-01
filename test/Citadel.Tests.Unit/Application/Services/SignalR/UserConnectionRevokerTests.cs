using Application.Services.SignalR;

namespace Tests.Unit.Application.Services.SignalR;

public sealed class UserConnectionRevokerTests
{
    [Fact]
    public void RevokeUsers_ShouldAbortEveryRegisteredConnectionOnce()
    {
        var revoker = new UserConnectionRevoker();
        var userId = Guid.CreateVersion7();
        var firstAbortCount = 0;
        var secondAbortCount = 0;

        revoker.Register(userId, "first", () => firstAbortCount++);
        revoker.Register(userId, "second", () => secondAbortCount++);

        revoker.RevokeUsers([userId, userId]);
        revoker.RevokeUsers([userId]);

        Assert.Equal(1, firstAbortCount);
        Assert.Equal(1, secondAbortCount);
    }

    [Fact]
    public void Unregister_ShouldPreventConnectionAbort()
    {
        var revoker = new UserConnectionRevoker();
        var userId = Guid.CreateVersion7();
        var abortCount = 0;

        revoker.Register(userId, "connection", () => abortCount++);
        revoker.Unregister(userId, "connection");
        revoker.RevokeUsers([userId]);

        Assert.Equal(0, abortCount);
    }

    [Fact]
    public void RevokeUsers_ShouldContinueWhenAnAbortCallbackThrows()
    {
        var revoker = new UserConnectionRevoker();
        var userId = Guid.CreateVersion7();
        var successfulAbortCount = 0;

        revoker.Register(userId, "throwing", () => throw new InvalidOperationException("test"));
        revoker.Register(userId, "successful", () => successfulAbortCount++);

        revoker.RevokeUsers([userId]);

        Assert.Equal(1, successfulAbortCount);
    }
}
