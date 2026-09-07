using System.Text;
using Hosting.DockerClient.Services;
using Moq;

namespace Tests.Unit.DockerClient;

public sealed class DockerConnectionTests
{
    [Fact]
    public async Task RejectedRequest_ShouldReportBoundedBodyAndDisposeSocket()
    {
        const string body = "{\"message\":\"Invalid Docker build request\"}";
        var stream = new ResponseStream($"HTTP/1.1 400 Bad Request\r\nContent-Length: {Encoding.UTF8.GetByteCount(body)}\r\n\r\n{body}");
        var transport = new Mock<IDockerTransport>();
        transport.Setup(x => x.OpenRawStreamAsync(It.IsAny<CancellationToken>())).ReturnsAsync(stream);
        using var request = new HttpRequestMessage(HttpMethod.Post, "http://localhost/build");

        var error = await Assert.ThrowsAsync<InvalidOperationException>(() =>
            new DockerConnection(transport.Object).OpenHijackedStreamAsync(request, TestContext.Current.CancellationToken));

        Assert.Contains(body, error.Message);
        Assert.False(stream.CanRead);
    }

    [Fact]
    public async Task OversizedHeaders_ShouldFailAndDisposeSocket()
    {
        var stream = new ResponseStream("HTTP/1.1 200 OK\r\nX-Padding: " + new string('a', 20_000));
        var transport = new Mock<IDockerTransport>();
        transport.Setup(x => x.OpenRawStreamAsync(It.IsAny<CancellationToken>())).ReturnsAsync(stream);
        using var request = new HttpRequestMessage(HttpMethod.Post, "http://localhost/build");

        var error = await Assert.ThrowsAsync<InvalidOperationException>(() =>
            new DockerConnection(transport.Object).OpenHijackedStreamAsync(request, TestContext.Current.CancellationToken));

        Assert.Contains("16 KiB", error.Message);
        Assert.False(stream.CanRead);
    }

    private sealed class ResponseStream(string response) : MemoryStream(Encoding.UTF8.GetBytes(response))
    {
        public override ValueTask WriteAsync(ReadOnlyMemory<byte> buffer, CancellationToken cancellationToken = default)
            => ValueTask.CompletedTask;

        public override ValueTask<int> ReadAsync(Memory<byte> buffer, CancellationToken cancellationToken = default)
            => base.ReadAsync(buffer[..Math.Min(buffer.Length, 31)], cancellationToken);
    }
}
