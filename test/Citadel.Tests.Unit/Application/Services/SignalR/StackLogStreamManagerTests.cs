using System.Text;
using Application.Services.SignalR;

namespace Tests.Unit.Application.Services.SignalR;

public sealed class StackLogStreamManagerTests
{
    [Fact]
    public void PrefixContainerName_PreservesMessagesWithoutIntermediateStrings()
    {
        var input = Encoding.UTF8.GetBytes(
            "2026-07-27T12:00:00.000000000Z first line\r\nplain line\n");
        var prefix = "[api] "u8;

        using var result = StackLogStreamManager.PrefixContainerName(input, prefix);

        Assert.NotNull(result);
        Assert.Equal(
            "2026-07-27T12:00:00.000000000Z [api] first line\n[api] plain line",
            Encoding.UTF8.GetString(result.Span));
    }

    [Fact]
    public void PrefixContainerName_SuppressesTimestampOnlyLines()
    {
        var input = Encoding.UTF8.GetBytes(
            "2026-07-27T12:00:00.000000000Z\n2026-07-27T12:00:01.000000000Z  \r\n");

        using var result = StackLogStreamManager.PrefixContainerName(input, "[api] "u8);

        Assert.Null(result);
    }
}
