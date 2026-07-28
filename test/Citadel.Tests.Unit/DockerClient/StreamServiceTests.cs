using Hosting.DockerClient.Services;
using System.Text;
using System.Text.Json.Serialization;

namespace Tests.Unit.DockerClient;

public sealed class StreamServiceTests
{
    [Fact]
    public async Task RawStream_EmitsFinalLineWithoutNewline()
    {
        var items = await ReadStringsAsync("final line");

        Assert.Equal(["final line"], items);
    }

    [Fact]
    public async Task RawStream_DoesNotTrimUtf8ContinuationBytes()
    {
        var items = await ReadStringsAsync("hello\u0420");

        Assert.Equal(["hello\u0420"], items);
    }

    [Fact]
    public async Task MessageStream_RejectsMalformedCompletedJson()
    {
        var service = new StreamService();
        var stream = Task.FromResult<Stream>(
            new MemoryStream(Encoding.UTF8.GetBytes("{not-json")));

        async Task EnumerateAsync()
        {
            await foreach (var _ in service.MonitorStreamForMessagesAsync(
                               stream,
                               StreamServiceTestJsonContext.Default.StreamServiceTestMessage,
                               TestContext.Current.CancellationToken))
            {
            }
        }

        await Assert.ThrowsAsync<InvalidDataException>(EnumerateAsync);
    }

    private static async Task<List<string>> ReadStringsAsync(string value)
    {
        var service = new StreamService();
        var stream = Task.FromResult<Stream>(
            new MemoryStream(Encoding.UTF8.GetBytes(value)));
        var items = new List<string>();

        await foreach (var item in service.MonitorStreamForStringsAsync(
                           stream,
                           TestContext.Current.CancellationToken))
        {
            items.Add(item);
        }

        return items;
    }
}

internal sealed record StreamServiceTestMessage(string Value);

[JsonSerializable(typeof(StreamServiceTestMessage))]
internal partial class StreamServiceTestJsonContext : JsonSerializerContext;
